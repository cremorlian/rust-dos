use std::error::Error;
use std::fmt;
use std::fs;
use std::process::exit;

use elf2mz::Converter;

#[derive(Debug)]
enum CliError {
    TooFewArguments,
    ExtraArguments,
    UnknownOption(String),
    MissingOptionValue(String),
    DuplicateOption(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewArguments => write!(f, "expected input.elf and output.exe"),
            Self::ExtraArguments => write!(f, "more than input.elf and output.exe given"),
            Self::UnknownOption(option) => write!(f, "unknown option `--{option}`"),
            Self::MissingOptionValue(option) => write!(f, "option `--{option}` requires a value"),
            Self::DuplicateOption(option) => write!(f, "option `--{option}` given more than once"),
        }
    }
}

#[derive(Debug)]
struct Opt<'a> {
    name: &'a str,
    value: &'a str,
}

#[derive(Debug)]
struct ConvertParams<'a> {
    input: &'a str,
    output: &'a str,
    options: Vec<Opt<'a>>,
}

fn main() {
    let args = match std::env::args_os()
        .skip(1)
        .map(|arg| arg.into_string())
        .collect::<Result<Vec<String>, _>>()
    {
        Ok(args) => args,
        Err(_) => {
            eprintln!("elf2mz: argument is not valid UTF-8");
            exit(2);
        }
    };
    let params = match run(&args) {
        Ok(params) => params,
        Err(err) => {
            eprintln!("elf2mz: {err}");
            exit(2);
        }
    };
    if let Err(err) = convert(&params) {
        eprintln!("elf2mz: {err}");
        exit(1);
    }
}

fn partition<'a>(args: &'a [String]) -> Result<(Vec<&'a str>, Vec<Opt<'a>>), CliError> {
    let mut positionals = Vec::new();
    let mut options = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let Some(body) = arg.strip_prefix("--") else {
            positionals.push(arg.as_str());
            continue;
        };
        let (name, value) = match body.split_once('=') {
            Some((name, value)) => (name, value),
            None => {
                let Some(value) = iter.next() else {
                    return Err(CliError::MissingOptionValue(body.to_string()));
                };
                (body, value.as_str())
            }
        };
        match name {
            "stub" | "entry" | "stack" | "min-alloc" | "max-alloc" | "strictness" => {}
            _ => return Err(CliError::UnknownOption(name.to_string())),
        }
        options.push(Opt { name, value });
        if options.iter().filter(|opt| opt.name == name).count() > 1 {
            return Err(CliError::DuplicateOption(name.to_string()));
        }
    }
    Ok((positionals, options))
}

fn run(args: &[String]) -> Result<ConvertParams<'_>, CliError> {
    let (positionals, options) = partition(args)?;
    if positionals.len() < 2 {
        return Err(CliError::TooFewArguments);
    }
    if positionals.len() > 2 {
        return Err(CliError::ExtraArguments);
    }
    let input = positionals[0];
    let output = positionals[1];
    Ok(ConvertParams {
        input,
        output,
        options,
    })
}

fn convert(params: &ConvertParams) -> Result<(), Box<dyn Error>> {
    let elf = fs::read(params.input)?;
    let exe = if let Some(opt) = params.options.iter().find(|opt| opt.name == "stub") {
        let stub = fs::read(opt.value)?;
        Converter::new().stub(&stub)?.convert(&elf)?
    } else {
        Converter::new().convert(&elf)?
    };
    fs::write(params.output, exe)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_parses_input_and_output() {
        let args = ["in.elf".to_string(), "out.exe".to_string()];
        let params = run(&args).unwrap();
        assert_eq!(params.input, "in.elf");
        assert_eq!(params.output, "out.exe");
        assert!(params.options.is_empty());
    }

    #[test]
    fn run_parses_multiple_options_with_space_values() {
        let args = [
            "in.elf".to_string(),
            "--stub".to_string(),
            "s.bin".to_string(),
            "out.exe".to_string(),
            "--min-alloc".to_string(),
            "0x10".to_string(),
        ];
        let params = run(&args).unwrap();
        assert_eq!(params.input, "in.elf");
        assert_eq!(params.output, "out.exe");
        assert_eq!(params.options.len(), 2);
        assert_eq!(
            (params.options[0].name, params.options[0].value),
            ("stub", "s.bin")
        );
        assert_eq!(
            (params.options[1].name, params.options[1].value),
            ("min-alloc", "0x10")
        );
    }

    #[test]
    fn run_parses_option_with_equals_value() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--stub=s.bin".to_string(),
        ];
        let params = run(&args).unwrap();
        assert_eq!(params.input, "in.elf");
        assert_eq!(params.output, "out.exe");
        assert_eq!(params.options.len(), 1);
        assert_eq!(
            (params.options[0].name, params.options[0].value),
            ("stub", "s.bin")
        );
    }

    #[test]
    fn run_returns_error_for_too_few_arguments() {
        let args = ["in.elf".to_string()];
        assert!(matches!(run(&args), Err(CliError::TooFewArguments)));
    }

    #[test]
    fn run_returns_extra_arguments() {
        let args = ["a.elf".to_string(), "b.exe".to_string(), "c".to_string()];
        assert!(matches!(run(&args), Err(CliError::ExtraArguments)));
    }

    #[test]
    fn run_returns_missing_option_value() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--stub".to_string(),
        ];
        assert!(matches!(
            run(&args),
            Err(CliError::MissingOptionValue(option)) if option == "stub"
        ));
    }

    #[test]
    fn run_returns_duplicate_option() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--stub".to_string(),
            "a.bin".to_string(),
            "--stub".to_string(),
            "b.bin".to_string(),
        ];
        assert!(matches!(
            run(&args),
            Err(CliError::DuplicateOption(option)) if option == "stub"
        ));
    }

    #[test]
    fn run_returns_unknown_option() {
        let cases = [
            vec![
                "in.elf".to_string(),
                "out.exe".to_string(),
                "--bogus".to_string(),
                "v".to_string(),
            ],
            vec![
                "in.elf".to_string(),
                "out.exe".to_string(),
                "--bogus=foo".to_string(),
            ],
        ];
        for args in cases {
            assert!(matches!(
                run(&args),
                Err(CliError::UnknownOption(option)) if option == "bogus"
            ));
        }
    }
}

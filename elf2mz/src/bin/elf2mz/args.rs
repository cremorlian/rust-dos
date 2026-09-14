use std::fmt;

#[derive(Debug)]
pub(crate) enum CliError {
    TooFewArguments,
    ExtraArguments,
    UnknownOption(String),
    MissingOptionValue(String),
    DuplicateOption(String),
    InvalidOptionValue(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewArguments => write!(f, "expected input.elf and output.exe"),
            Self::ExtraArguments => write!(f, "more than input.elf and output.exe given"),
            Self::UnknownOption(option) => write!(f, "unknown option `--{option}`"),
            Self::MissingOptionValue(option) => write!(f, "option `--{option}` requires a value"),
            Self::DuplicateOption(option) => write!(f, "option `--{option}` given more than once"),
            Self::InvalidOptionValue(option) => {
                write!(f, "option `--{option}` has an invalid value")
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Opt<'a> {
    pub(crate) name: &'a str,
    pub(crate) value: &'a str,
}

#[derive(Debug)]
pub(crate) struct ConvertParams<'a> {
    pub(crate) input: &'a str,
    pub(crate) output: &'a str,
    pub(crate) stub: Option<&'a str>,
    pub(crate) min_alloc: Option<u16>,
}

pub(crate) fn parse(args: &[String]) -> Result<ConvertParams<'_>, CliError> {
    let (positionals, options) = partition(args)?;
    if positionals.len() < 2 {
        return Err(CliError::TooFewArguments);
    }
    if positionals.len() > 2 {
        return Err(CliError::ExtraArguments);
    }
    let input = positionals[0];
    let output = positionals[1];
    let stub = options
        .iter()
        .find(|opt| opt.name == "stub")
        .map(|opt| opt.value);
    let min_alloc = match options.iter().find(|opt| opt.name == "min-alloc") {
        Some(opt) => Some(
            parse_hex_u16(opt.value)
                .ok_or_else(|| CliError::InvalidOptionValue("min-alloc".to_string()))?,
        ),
        None => None,
    };
    Ok(ConvertParams {
        input,
        output,
        stub,
        min_alloc,
    })
}

fn parse_hex_u16(value: &str) -> Option<u16> {
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    u16::from_str_radix(digits, 16).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_input_and_output() {
        let args = ["in.elf".to_string(), "out.exe".to_string()];
        let params = parse(&args).unwrap();
        assert_eq!(params.input, "in.elf");
        assert_eq!(params.output, "out.exe");
        assert_eq!(params.stub, None);
        assert_eq!(params.min_alloc, None);
    }

    #[test]
    fn parse_multiple_options_with_space_values() {
        let args = [
            "in.elf".to_string(),
            "--stub".to_string(),
            "s.bin".to_string(),
            "out.exe".to_string(),
            "--min-alloc".to_string(),
            "0x10".to_string(),
        ];
        let params = parse(&args).unwrap();
        assert_eq!(params.input, "in.elf");
        assert_eq!(params.output, "out.exe");
        assert_eq!(params.stub, Some("s.bin"));
        assert_eq!(params.min_alloc, Some(0x10));
    }

    #[test]
    fn parse_option_with_equals_value() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--stub=s.bin".to_string(),
        ];
        let params = parse(&args).unwrap();
        assert_eq!(params.input, "in.elf");
        assert_eq!(params.output, "out.exe");
        assert_eq!(params.stub, Some("s.bin"));
    }

    #[test]
    fn parse_stub_value() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--stub".to_string(),
            "s.bin".to_string(),
        ];
        let params = parse(&args).unwrap();
        assert_eq!(params.stub, Some("s.bin"));
    }

    #[test]
    fn parse_min_alloc_value() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--min-alloc".to_string(),
            "0x10".to_string(),
        ];
        let params = parse(&args).unwrap();
        assert_eq!(params.min_alloc, Some(0x10));
    }

    #[test]
    fn parse_returns_invalid_option_value() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--min-alloc".to_string(),
            "zzz".to_string(),
        ];
        assert!(matches!(
            parse(&args),
            Err(CliError::InvalidOptionValue(option)) if option == "min-alloc"
        ));
    }

    #[test]
    fn parse_returns_error_for_too_few_arguments() {
        let args = ["in.elf".to_string()];
        assert!(matches!(parse(&args), Err(CliError::TooFewArguments)));
    }

    #[test]
    fn parse_returns_extra_arguments() {
        let args = ["a.elf".to_string(), "b.exe".to_string(), "c".to_string()];
        assert!(matches!(parse(&args), Err(CliError::ExtraArguments)));
    }

    #[test]
    fn parse_returns_missing_option_value() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--stub".to_string(),
        ];
        assert!(matches!(
            parse(&args),
            Err(CliError::MissingOptionValue(option)) if option == "stub"
        ));
    }

    #[test]
    fn parse_returns_duplicate_option() {
        let args = [
            "in.elf".to_string(),
            "out.exe".to_string(),
            "--stub".to_string(),
            "a.bin".to_string(),
            "--stub".to_string(),
            "b.bin".to_string(),
        ];
        assert!(matches!(
            parse(&args),
            Err(CliError::DuplicateOption(option)) if option == "stub"
        ));
    }

    #[test]
    fn parse_returns_unknown_option() {
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
                parse(&args),
                Err(CliError::UnknownOption(option)) if option == "bogus"
            ));
        }
    }
}

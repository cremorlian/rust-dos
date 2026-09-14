mod args;

use std::error::Error;
use std::fs;
use std::process::exit;

use elf2mz::Converter;

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
    let params = match args::parse(&args) {
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

fn convert(params: &args::ConvertParams) -> Result<(), Box<dyn Error>> {
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

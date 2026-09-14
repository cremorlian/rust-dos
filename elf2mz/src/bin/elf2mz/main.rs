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
    let mut converter = Converter::new();
    if let Some(min_alloc) = params.min_alloc {
        converter = converter.min_alloc(min_alloc);
    }
    if let Some(stub) = params.stub {
        let stub = fs::read(stub)?;
        converter = converter.stub(&stub)?;
    }
    let exe = converter.convert(&elf)?;
    fs::write(params.output, exe)?;
    Ok(())
}

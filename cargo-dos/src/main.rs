use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use cargo_dos::{init, mz};

fn main() {
    match env::args().nth(1).as_deref() {
        Some("init") => {
            if let Err(e) = cmd_init() {
                eprintln!("{e}");
                process::exit(1);
            }
        }
        Some("postlink") => {
            if let Err(e) = cmd_postlink() {
                eprintln!("{e}");
                process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: cargo dos [init|postlink]");
            process::exit(1);
        }
    }
}

fn cmd_postlink() -> Result<(), std::io::Error> {
    let input = env::args().nth(2).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "usage: cargo dos postlink <elf>",
        )
    })?;
    let stem = Path::new(&input).file_stem().unwrap_or_default();
    let out = format!("{}.exe", stem.to_string_lossy());
    fs::write(out, mz::header())
}

fn cmd_init() -> Result<(), std::io::Error> {
    let cargo_home = cargo_home();
    let project = env::current_dir()?;
    let target_json = include_str!("../assets/i486-dos.json");
    let plan = init::plan(&cargo_home, &project, target_json);

    write(plan.consumer_config)?;
    write(plan.toolchain)?;
    write(plan.target_json)?;
    Ok(())
}

fn write(file: init::File) -> Result<(), std::io::Error> {
    if let Some(parent) = file.path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&file.path, file.content)?;
    Ok(())
}

fn cargo_home() -> PathBuf {
    env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .expect("CARGO_HOME not set")
}

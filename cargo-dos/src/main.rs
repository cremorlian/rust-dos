use std::env;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

use cargo_dos::{artifact, elf, fixups, init, mz, packer, stub};
use cargo_metadata::Message;

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("dos") {
        return Err("usage: cargo dos [init|build]".to_string());
    }

    match args.next().as_deref() {
        Some("init") => cmd_init().map_err(|e| e.to_string()),
        Some("build") => cmd_build(),
        _ => Err("usage: cargo dos [init|build]".to_string()),
    }
}

fn cmd_build() -> Result<(), String> {
    for elf in cargo_build()? {
        write_exe(&elf)?;
    }
    Ok(())
}

fn cargo_build() -> Result<Vec<PathBuf>, String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut child = Command::new(cargo)
        .args(["build", "--message-format=json-render-diagnostics"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "cargo stdout was not captured".to_string())?;
    let messages = Message::parse_stream(BufReader::new(stdout))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("cargo build failed".to_string());
    }

    artifact::executables(messages).map_err(|e| e.to_string())
}

fn write_exe(elf: &Path) -> Result<(), String> {
    let exe = elf.with_extension("exe");
    let bytes = build_exe(elf)?;
    fs::write(exe, bytes).map_err(|e| e.to_string())
}

fn build_exe(input: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(input).map_err(|e| e.to_string())?;
    let data = elf::extract(&bytes).map_err(|e| format!("{e:?}"))?;
    let stub = stub::build(data.image.len()).map_err(|e| format!("{e:?}"))?;
    let table = fixups::table(&data.fixups).map_err(|e| format!("{e:?}"))?;
    let file_len = 32 + stub.len() + data.image.len() + table.len();
    let header = mz::header(file_len).map_err(|e| format!("{e:?}"))?;
    Ok(packer::compose(header, &stub, &data.image, &table))
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

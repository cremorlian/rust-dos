use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_elf2mz"))
}

fn temp_out(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

#[test]
fn converts_minimal_elf_to_mz_output() {
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.elf");
    let output = temp_out("elf2mz-cli-minimal.exe");

    assert!(bin().arg(&input).arg(&output).status().unwrap().success());

    let out = fs::read(&output).unwrap();
    assert_eq!(out.len(), 35);
    assert_eq!(&out[0..2], b"MZ");
    assert_eq!(u16::from_le_bytes([out[2], out[3]]), 35);
    assert_eq!(u16::from_le_bytes([out[4], out[5]]), 1);
    assert_eq!(&out[32..35], &[0x90, 0x90, 0x90]);
}

#[test]
fn converts_elf_with_stub_to_mz_output() {
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.elf");
    let stub = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stub.bin");
    let output = temp_out("elf2mz-cli-stub.exe");

    assert!(
        bin()
            .arg(&input)
            .arg(&output)
            .arg("--stub")
            .arg(&stub)
            .status()
            .unwrap()
            .success()
    );

    let out = fs::read(&output).unwrap();
    assert_eq!(out.len(), 37);
    assert_eq!(&out[0..2], b"MZ");
    assert_eq!(u16::from_le_bytes([out[2], out[3]]), 37);
    assert_eq!(u16::from_le_bytes([out[4], out[5]]), 1);
    assert_eq!(&out[32..34], &[0xFA, 0xFB]);
    assert_eq!(&out[34..37], &[0x90, 0x90, 0x90]);
    assert_eq!(u16::from_le_bytes([out[20], out[21]]), 0);
    assert_eq!(u16::from_le_bytes([out[22], out[23]]), 0);
}

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

mod fixtures;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("cargo-dos-test-{}-{nanos}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn init_in(project: &Path, cargo_home: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-dos"))
        .args(["dos", "init"])
        .current_dir(project)
        .env("CARGO_HOME", cargo_home)
        .output()
        .unwrap()
}

fn run_dos(args: &[&str], project: &Path, cargo_home: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-dos"))
        .arg("dos")
        .args(args)
        .current_dir(project)
        .env("CARGO_HOME", cargo_home)
        .output()
        .unwrap()
}

fn run_dos_with_cargo(
    args: &[&str],
    project: &Path,
    cargo_home: &Path,
    cargo: &Path,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-dos"))
        .arg("dos")
        .args(args)
        .current_dir(project)
        .env("CARGO_HOME", cargo_home)
        .env("CARGO", cargo)
        .output()
        .unwrap()
}

fn install_fake_cargo(project: &Path, fixture: &[u8]) -> PathBuf {
    std::fs::write(project.join("fixture-elf"), fixture).unwrap();

    let fake_cargo = project.join("cargo");
    std::fs::write(
        &fake_cargo,
        r#"#!/bin/sh
set -eu
mkdir -p target/i486-dos/debug
cp fixture-elf target/i486-dos/debug/hello
cat <<EOF
{"reason":"compiler-artifact","package_id":"hello 0.1.0 (path+file:///tmp/hello)","manifest_path":"/tmp/hello/Cargo.toml","target":{"name":"hello","kind":["bin"],"src_path":"/tmp/hello/src/main.rs"},"profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["$PWD/target/i486-dos/debug/hello"],"executable":"$PWD/target/i486-dos/debug/hello","fresh":false}
EOF
"#,
    )
    .unwrap();

    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(&fake_cargo).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_cargo, permissions).unwrap();

    fake_cargo
}

fn restore_writable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path).unwrap().permissions().mode();
    let mut writable = std::fs::metadata(path).unwrap().permissions();
    writable.set_mode(mode | 0o200);
    std::fs::set_permissions(path, writable).unwrap();
}

#[test]
fn unknown_subcommand_prints_usage_and_exits_nonzero() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    let cargo_home = tmp.path().join("cargo-home");
    std::fs::create_dir_all(&project).unwrap();

    let out = run_dos(&["foo"], &project, &cargo_home);

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("usage"));
}

#[test]
fn missing_cargo_home_prints_error_and_exits_nonzero() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_cargo-dos"))
        .args(["dos", "init"])
        .current_dir(&project)
        .env_remove("CARGO_HOME")
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("CARGO_HOME"));
}

#[test]
fn io_error_surfaces_cleanly_and_exits_one() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    let cargo_home = tmp.path().join("cargo-home-io-fail");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&cargo_home).unwrap();

    let target_dir = cargo_home.join("rust-dos");
    std::fs::create_dir_all(&target_dir).unwrap();
    let mut perms = std::fs::metadata(&target_dir).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&target_dir, perms).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_cargo-dos"))
        .args(["dos", "init"])
        .current_dir(&project)
        .env("CARGO_HOME", &cargo_home)
        .output()
        .unwrap();

    restore_writable(&target_dir);

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Permission denied"));
    assert!(!stderr.contains("thread 'main'"));
}

#[test]
fn init_scaffolds_a_consumer_project() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    let cargo_home = tmp.path().join("cargo-home");
    std::fs::create_dir_all(&project).unwrap();

    init_in(&project, &cargo_home);

    assert!(project.join(".cargo/config.toml").exists());
    assert!(project.join("rust-toolchain.toml").exists());
    assert!(cargo_home.join("rust-dos/i486-dos.json").exists());
}

#[test]
fn build_writes_packaged_exe_next_to_elf() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let fake_cargo = install_fake_cargo(
        &project,
        &fixtures::build_elf(
            &[(vec![1, 2, 3, 4, 5, 6], 0x400000)],
            &[(
                0,
                &[
                    (0x400000, object::elf::R_386_32),
                    (0x400004, object::elf::R_386_PC32),
                    (0x400005, object::elf::R_386_32),
                ],
            )],
        ),
    );

    let out = run_dos_with_cargo(
        &["build"],
        &project,
        &tmp.path().join("cargo-home"),
        &fake_cargo,
    );

    assert!(out.status.success());
    let bytes = std::fs::read(project.join("target/i486-dos/debug/hello.exe")).unwrap();

    assert_eq!(&bytes[..2], b"MZ");

    let file_len = bytes.len();
    assert_eq!(
        u16::from_le_bytes([bytes[2], bytes[3]]),
        (file_len % 512).max(1) as u16,
        "e_cblp"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[4], bytes[5]]),
        file_len.div_ceil(512) as u16,
        "e_cp"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[16], bytes[17]]),
        u16::MAX,
        "e_sp is the top of the segment"
    );
    assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 0, "e_ip");
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 0, "e_cs");

    const STUB_FRAME_PARAGRAPHS: u16 = 4096;
    let e_minalloc = u16::from_le_bytes([bytes[10], bytes[11]]);
    assert_eq!(
        e_minalloc, STUB_FRAME_PARAGRAPHS,
        "e_minalloc reserves the stub's real-mode frame"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[12], bytes[13]]),
        e_minalloc,
        "e_maxalloc equals e_minalloc"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[14], bytes[15]]),
        (file_len - 32).div_ceil(16) as u16,
        "e_ss is the paragraph after the load image"
    );
    const HEADER_LEN: usize = 32;
    const IMAGE_LEN: usize = 6;

    let stub = cargo_dos::stub::build(IMAGE_LEN).expect("the table offset should fit");
    let image_offset = HEADER_LEN + stub.len();
    let table_offset = image_offset + IMAGE_LEN;
    let table_off_at = HEADER_LEN + stub.len() - 4;
    let table_off = u32::from_le_bytes(bytes[table_off_at..table_off_at + 4].try_into().unwrap());

    assert_eq!(
        table_off as usize,
        stub.len() + IMAGE_LEN,
        "TABLE_OFF is module-relative"
    );
    assert_eq!(
        HEADER_LEN + table_off as usize,
        table_offset,
        "TABLE_OFF points at the fixup table"
    );
    assert_eq!(
        &bytes[HEADER_LEN..image_offset],
        &stub[..],
        "the image begins immediately after the stub ends"
    );
    assert_eq!(
        &bytes[image_offset..table_offset],
        [1, 2, 3, 4, 5, 6],
        "the image follows the stub"
    );

    let mut entries = 0;
    let mut cursor = table_offset;
    loop {
        let entry = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]);
        if entry == u32::MAX {
            break;
        }
        assert!(
            entry < IMAGE_LEN as u32,
            "every entry lands inside the emitted image"
        );
        entries += 1;
        cursor += 4;
    }

    assert_eq!(entries, 2, "table holds one u32 per loaded R_386_32 site");
    assert_eq!(
        &bytes[table_offset..table_offset + 4],
        0u32.to_le_bytes(),
        "first entry is the affine image offset of the first R_386_32 site"
    );
    assert_eq!(
        &bytes[table_offset + 4..table_offset + 8],
        5u32.to_le_bytes(),
        "second entry is the affine image offset of the second R_386_32 site"
    );
    assert_eq!(cursor + 4, file_len, "sentinel terminates the file");
}

#[test]
fn build_with_non_elf_input_exits_one_without_panicking() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let fake_cargo = install_fake_cargo(&project, b"definitely not an elf");

    let out = run_dos_with_cargo(
        &["build"],
        &project,
        &tmp.path().join("cargo-home"),
        &fake_cargo,
    );

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("NotAnElf"));
    assert!(!stderr.contains("thread 'main'"));
    assert!(!project.join("target/i486-dos/debug/hello.exe").exists());
}

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
        .arg("init")
        .current_dir(project)
        .env("CARGO_HOME", cargo_home)
        .output()
        .unwrap()
}

fn run_dos(args: &[&str], project: &Path, cargo_home: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-dos"))
        .args(args)
        .current_dir(project)
        .env("CARGO_HOME", cargo_home)
        .output()
        .unwrap()
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
        .arg("init")
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
        .arg("init")
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
fn postlink_emits_stub_then_image_then_fixup_table_with_self_consistent_header() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("hello"),
        fixtures::build_elf(
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
    )
    .unwrap();

    let out = run_dos(
        &["postlink", "hello"],
        &project,
        &tmp.path().join("cargo-home"),
    );

    assert!(out.status.success());
    let bytes = std::fs::read(project.join("hello.exe")).unwrap();

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
    assert_eq!(u16::from_le_bytes([bytes[16], bytes[17]]), 0, "e_sp");
    assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 0, "e_ip");
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 0, "e_cs");

    const HEADER_LEN: usize = 32;
    const STUB_LEN: usize = 64;
    const IMAGE_LEN: usize = 6;

    let image_offset = HEADER_LEN + STUB_LEN;
    let table_offset = image_offset + IMAGE_LEN;

    assert_eq!(
        &bytes[HEADER_LEN..image_offset],
        [0u8; STUB_LEN],
        "the stub sits between the header and the image"
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
fn postlink_without_input_arg_prints_usage_and_exits_one() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    let out = run_dos(&["postlink"], &project, &tmp.path().join("cargo-home"));

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("usage"));
    assert!(!stderr.contains("thread 'main'"));
}

#[test]
fn postlink_with_non_elf_input_exits_one_without_panicking() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("hello"), b"definitely not an elf").unwrap();

    let out = run_dos(
        &["postlink", "hello"],
        &project,
        &tmp.path().join("cargo-home"),
    );

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("NotAnElf"));
    assert!(!stderr.contains("thread 'main'"));
    assert!(!project.join("hello.exe").exists());
}

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

mod common;

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
fn postlink_emits_exe_with_self_consistent_header_and_load_image() {
    let tmp = TempDir::new();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("hello"), common::build_elf(&[vec![1, 2, 3, 4, 5, 6]])).unwrap();

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
    assert_eq!(&bytes[32..], [1, 2, 3, 4, 5, 6]);
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

    let out = run_dos(&["postlink", "hello"], &project, &tmp.path().join("cargo-home"));

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("NotAnElf"));
    assert!(!stderr.contains("thread 'main'"));
    assert!(!project.join("hello.exe").exists());
}

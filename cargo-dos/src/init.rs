use std::path::{Path, PathBuf};

pub struct File {
    pub path: PathBuf,
    pub content: String,
}

pub struct InitPlan {
    pub consumer_config: File,
    pub toolchain: File,
    pub target_json: File,
}

pub fn plan(cargo_home: &Path, project: &Path, target_json: &str) -> InitPlan {
    InitPlan {
        consumer_config: File {
            path: project.join(".cargo/config.toml"),
            content: consumer_config(&asset_dir(cargo_home)),
        },
        toolchain: File {
            path: project.join("rust-toolchain.toml"),
            content: String::from("[toolchain]\nchannel = \"nightly\"\n"),
        },
        target_json: File {
            path: asset_dir(cargo_home).join("i486-dos.json"),
            content: String::from(target_json),
        },
    }
}

fn asset_dir(cargo_home: &Path) -> PathBuf {
    cargo_home.join("rust-dos")
}

fn consumer_config(asset_dir: &Path) -> String {
    format!(
        "[build]
target = \"i486-dos\"
rustflags = [\"-Zunstable-options\", \"-Cdebuginfo=0\"]

[unstable]
build-std = [\"core\"]
json-target-spec = true

[target.i486-dos]
runner = [\"cargo-dos\", \"postlink\"]

[env]
RUST_TARGET_PATH = \"{}\"
",
        asset_dir.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_derives_paths_from_input_dirs() {
        let p = plan(Path::new("/ch"), Path::new("/proj"), "{}");
        assert_eq!(
            p.consumer_config.path,
            Path::new("/proj/.cargo/config.toml")
        );
        assert_eq!(p.toolchain.path, Path::new("/proj/rust-toolchain.toml"));
        assert_eq!(p.target_json.path, Path::new("/ch/rust-dos/i486-dos.json"));
    }

    #[test]
    fn consumer_config_reaches_the_dir_target_json_is_installed_into() {
        let p = plan(Path::new("/ch"), Path::new("/proj"), "{}");
        let config = &p.consumer_config.content;
        let install_dir = p.target_json.path.parent().unwrap().display();
        assert!(config.contains(&format!("RUST_TARGET_PATH = \"{install_dir}\"")));
    }

    #[test]
    fn plan_copies_the_target_json_arg_into_the_install_file() {
        let p = plan(Path::new("/ch"), Path::new("/proj"), "[json content]");
        assert_eq!(p.target_json.content, "[json content]");
    }
}

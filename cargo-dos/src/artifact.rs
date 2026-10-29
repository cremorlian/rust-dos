use std::fmt;
use std::path::PathBuf;

use cargo_metadata::Message;

#[derive(Debug, PartialEq)]
pub enum Error {
    NoExecutable,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoExecutable => f.write_str("cargo build produced no executable"),
        }
    }
}

impl std::error::Error for Error {}

pub fn executables(messages: impl IntoIterator<Item = Message>) -> Result<Vec<PathBuf>, Error> {
    let mut executables = Vec::new();

    for message in messages {
        if let Message::CompilerArtifact(artifact) = message
            && let Some(path) = artifact.executable
        {
            executables.push(PathBuf::from(path.as_str()));
        }
    }

    if executables.is_empty() {
        Err(Error::NoExecutable)
    } else {
        Ok(executables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_binary_executable() {
        let message = r#"{"reason":"compiler-artifact","package_id":"hello 0.1.0 (path+file:///tmp/hello)","manifest_path":"/tmp/hello/Cargo.toml","target":{"name":"hello","kind":["bin"],"src_path":"/tmp/hello/src/main.rs"},"profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/tmp/hello/target/i486-dos/debug/hello"],"executable":"/tmp/hello/target/i486-dos/debug/hello","fresh":false}"#;
        let messages = Message::parse_stream(message.as_bytes()).map(|message| message.unwrap());

        assert_eq!(
            executables(messages),
            Ok(vec![PathBuf::from(
                "/tmp/hello/target/i486-dos/debug/hello"
            )])
        );
    }

    #[test]
    fn selects_the_binary_executable_when_it_is_not_first() {
        let library = r#"{"reason":"compiler-artifact","package_id":"dep 0.1.0 (path+file:///tmp/dep)","manifest_path":"/tmp/dep/Cargo.toml","target":{"name":"dep","kind":["lib"],"src_path":"/tmp/dep/src/lib.rs"},"profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/tmp/dep/target/i486-dos/debug/libdep.rlib"],"executable":null,"fresh":false}"#;
        let binary = r#"{"reason":"compiler-artifact","package_id":"hello 0.1.0 (path+file:///tmp/hello)","manifest_path":"/tmp/hello/Cargo.toml","target":{"name":"hello","kind":["bin"],"src_path":"/tmp/hello/src/main.rs"},"profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/tmp/hello/target/i486-dos/debug/hello"],"executable":"/tmp/hello/target/i486-dos/debug/hello","fresh":false}"#;
        let input = format!("{library}\n{binary}");
        let messages = Message::parse_stream(input.as_bytes()).map(|message| message.unwrap());

        assert_eq!(
            executables(messages),
            Ok(vec![PathBuf::from(
                "/tmp/hello/target/i486-dos/debug/hello"
            )])
        );
    }

    #[test]
    fn collects_every_binary_executable() {
        let one = r#"{"reason":"compiler-artifact","package_id":"one 0.1.0 (path+file:///tmp/one)","manifest_path":"/tmp/one/Cargo.toml","target":{"name":"one","kind":["bin"],"src_path":"/tmp/one/src/main.rs"},"profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/tmp/one/target/i486-dos/debug/one"],"executable":"/tmp/one/target/i486-dos/debug/one","fresh":false}"#;
        let two = r#"{"reason":"compiler-artifact","package_id":"two 0.1.0 (path+file:///tmp/two)","manifest_path":"/tmp/two/Cargo.toml","target":{"name":"two","kind":["bin"],"src_path":"/tmp/two/src/main.rs"},"profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/tmp/two/target/i486-dos/debug/two"],"executable":"/tmp/two/target/i486-dos/debug/two","fresh":false}"#;
        let input = format!("{one}\n{two}");
        let messages = Message::parse_stream(input.as_bytes()).map(|message| message.unwrap());

        assert_eq!(
            executables(messages),
            Ok(vec![
                PathBuf::from("/tmp/one/target/i486-dos/debug/one"),
                PathBuf::from("/tmp/two/target/i486-dos/debug/two"),
            ])
        );
    }

    #[test]
    fn rejects_when_cargo_reports_no_artifacts() {
        assert_eq!(executables(Vec::new()), Err(Error::NoExecutable));
    }
}

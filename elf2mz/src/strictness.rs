use crate::Error;

#[derive(Debug, Copy, Clone, Default)]
pub enum Strictness {
    #[default]
    Error,
    Warn,
    Allow,
}

pub(crate) fn enforce(strictness: Strictness, error: Error) -> Result<(), Error> {
    match strictness {
        Strictness::Error => Err(error),
        Strictness::Warn => {
            eprintln!("{error}");
            Ok(())
        }
        Strictness::Allow => Ok(()),
    }
}

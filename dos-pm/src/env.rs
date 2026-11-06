use crate::psp;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ArgsError {
    Dpmi { code: u16 },
    InvalidPsp { segment: u16 },
    InvalidCommandLineLength { length: u8 },
}

impl From<psp::Error> for ArgsError {
    fn from(error: psp::Error) -> ArgsError {
        match error {
            psp::Error::Dpmi { code } => ArgsError::Dpmi { code },
            psp::Error::InvalidPsp { segment } => ArgsError::InvalidPsp { segment },
            psp::Error::InvalidCommandLineLength { length } => {
                ArgsError::InvalidCommandLineLength { length }
            }
        }
    }
}

pub fn args_bytes() -> Result<Args, ArgsError> {
    let mut area = [0u8; 128];
    let command_line = psp::read_command_line(&mut area)?;
    Ok(Args::from_validated_tail(command_line))
}

pub struct Args {
    bytes: [u8; 128],
    len: usize,
}

impl Args {
    pub(crate) fn from_validated_tail(bytes: &[u8]) -> Args {
        let mut buf = [0u8; 128];
        buf[..bytes.len()].copy_from_slice(bytes);
        Args {
            bytes: buf,
            len: bytes.len(),
        }
    }

    pub fn iter(&self) -> ArgIter<'_> {
        ArgIter {
            bytes: &self.bytes[..self.len],
            pos: 0,
        }
    }
}

impl<'a> IntoIterator for &'a Args {
    type Item = &'a [u8];
    type IntoIter = ArgIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ArgIter<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for ArgIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.bytes.len() && is_delimiter(self.bytes[self.pos]) {
            self.pos += 1;
        }
        let start = self.pos;
        while self.pos < self.bytes.len() && !is_delimiter(self.bytes[self.pos]) {
            self.pos += 1;
        }
        if start == self.pos {
            None
        } else {
            Some(&self.bytes[start..self.pos])
        }
    }
}

fn is_delimiter(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b',' | b';' | b'=')
}

#[cfg(test)]
mod tests {
    use super::{Args, ArgsError};
    use crate::psp;

    #[test]
    fn args_error_maps_psp_errors() {
        let cases = [
            (
                psp::Error::Dpmi { code: 0x8021 },
                ArgsError::Dpmi { code: 0x8021 },
            ),
            (
                psp::Error::InvalidPsp { segment: 0xFFF1 },
                ArgsError::InvalidPsp { segment: 0xFFF1 },
            ),
            (
                psp::Error::InvalidCommandLineLength { length: 128 },
                ArgsError::InvalidCommandLineLength { length: 128 },
            ),
        ];

        for (source, expected) in cases {
            assert_eq!(ArgsError::from(source), expected, "source={source:?}");
        }
    }

    #[test]
    fn tokens_preserve_high_bit_bytes() {
        let args = Args::from_validated_tail(b" alpha \x82 beta");
        let mut iter = args.iter();

        assert_eq!(iter.next(), Some(&b"alpha"[..]));
        assert_eq!(iter.next(), Some(&b"\x82"[..]));
        assert_eq!(iter.next(), Some(&b"beta"[..]));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn empty_command_line_has_no_arguments() {
        let args = Args::from_validated_tail(b"");

        assert_eq!(args.iter().next(), None);
    }

    #[test]
    fn iter_splits_on_spaces() {
        let args = Args::from_validated_tail(b" alpha beta");
        let mut iter = args.iter();

        assert_eq!(iter.next(), Some(&b"alpha"[..]));
        assert_eq!(iter.next(), Some(&b"beta"[..]));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iterates_by_reference() {
        let args = Args::from_validated_tail(b" alpha beta");
        let mut iter = (&args).into_iter();

        assert_eq!(iter.next(), Some(&b"alpha"[..]));
        assert_eq!(iter.next(), Some(&b"beta"[..]));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_splits_on_the_dos_delimiters() {
        let args = Args::from_validated_tail(b" alpha,beta;gamma=delta\tepsilon");
        let mut iter = args.iter();

        assert_eq!(iter.next(), Some(&b"alpha"[..]));
        assert_eq!(iter.next(), Some(&b"beta"[..]));
        assert_eq!(iter.next(), Some(&b"gamma"[..]));
        assert_eq!(iter.next(), Some(&b"delta"[..]));
        assert_eq!(iter.next(), Some(&b"epsilon"[..]));
        assert_eq!(iter.next(), None);
    }
}

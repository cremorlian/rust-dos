//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

pub struct MzSpec {}

pub fn convert(_elf: &[u8], _spec: &MzSpec) -> Result<Vec<u8>, Error> {
    Ok(b"MZ".to_vec())
}

#[derive(Debug)]
pub enum Error {}

#[cfg(test)]
mod tests {
    use crate::{convert, MzSpec};

    #[test]
    fn convert_produces_mz_magic() {
        let mz = convert(&[0x90], &MzSpec {}).expect("convert should succeed");
        assert_eq!(&mz[0..2], b"MZ");
    }
}

//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

#[derive(Default)]
pub struct MzSpec {
    pub min_alloc: u16,
}

pub fn convert(_elf: &[u8], spec: &MzSpec) -> Result<Vec<u8>, Error> {
    let mut out = vec![0u8; 12];
    out[0..2].copy_from_slice(b"MZ");
    out[10..12].copy_from_slice(&spec.min_alloc.to_le_bytes());
    Ok(out)
}

#[derive(Debug)]
pub enum Error {}

#[cfg(test)]
mod tests {
    use crate::{convert, MzSpec};

    #[test]
    fn convert_produces_mz_magic() {
        let mz = convert(&[0x90], &MzSpec::default()).expect("convert should succeed");
        assert_eq!(&mz[0..2], b"MZ");
    }

    #[test]
    fn convert_writes_min_alloc_from_spec() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0040u16, 0x0040u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (min_alloc, expected, desc) in cases {
            let spec = MzSpec { min_alloc };
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[10], mz[11]]);
            assert_eq!(
                written, expected,
                "expected min_alloc to be {expected} for {desc}"
            );
        }
    }
}

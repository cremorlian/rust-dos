//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

#[derive(Default)]
pub struct MzSpec {
    pub min_alloc: u16,
    pub max_alloc: u16,
}

pub fn convert(_elf: &[u8], spec: &MzSpec) -> Result<Vec<u8>, Error> {
    const WORD_WIDTH: usize = 2;

    let mut out = vec![0u8; 14];

    const OFFSET_MAGIC: usize = 0;
    const MZ_MAGIC: &[u8; 2] = b"MZ";
    out[OFFSET_MAGIC..OFFSET_MAGIC + WORD_WIDTH].copy_from_slice(MZ_MAGIC);

    const OFFSET_MIN_ALLOC: usize = 10;
    out[OFFSET_MIN_ALLOC..OFFSET_MIN_ALLOC + WORD_WIDTH]
        .copy_from_slice(&spec.min_alloc.to_le_bytes());

    const OFFSET_MAX_ALLOC: usize = 12;
    out[OFFSET_MAX_ALLOC..OFFSET_MAX_ALLOC + WORD_WIDTH]
        .copy_from_slice(&spec.max_alloc.to_le_bytes());
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
            let spec = MzSpec {
                min_alloc,
                ..Default::default()
            };
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[10], mz[11]]);
            assert_eq!(
                written, expected,
                "expected min_alloc to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn convert_writes_max_alloc_from_spec() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0040u16, 0x0040u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (max_alloc, expected, desc) in cases {
            let spec = MzSpec {
                max_alloc,
                ..Default::default()
            };
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[12], mz[13]]);
            assert_eq!(
                written, expected,
                "expected max_alloc to be {expected} for {desc}"
            );
        }
    }
}

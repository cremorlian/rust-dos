//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

pub struct MzSpec {
    min_alloc: u16,
    max_alloc: u16,
}

impl MzSpec {
    pub fn builder() -> MzSpecBuilder {
        MzSpecBuilder::default()
    }

    pub fn min_alloc(&self) -> u16 {
        self.min_alloc
    }

    pub fn max_alloc(&self) -> u16 {
        self.max_alloc
    }
}

pub struct MzSpecBuilder {
    min_alloc: u16,
    max_alloc: u16,
}

impl Default for MzSpecBuilder {
    fn default() -> Self {
        Self {
            min_alloc: 0,
            max_alloc: 0xFFFF,
        }
    }
}

impl MzSpecBuilder {
    pub fn min_alloc(mut self, min_alloc: u16) -> Self {
        self.min_alloc = min_alloc;
        self
    }

    pub fn max_alloc(mut self, max_alloc: u16) -> Self {
        self.max_alloc = max_alloc;
        self
    }

    pub fn build(self) -> Result<MzSpec, Error> {
        if self.max_alloc < self.min_alloc {
            return Err(Error::MaxAllocLessThanMinAlloc);
        }
        Ok(MzSpec {
            min_alloc: self.min_alloc,
            max_alloc: self.max_alloc,
        })
    }
}

pub fn convert(_elf: &[u8], spec: &MzSpec) -> Result<Vec<u8>, Error> {
    const WORD_WIDTH: usize = 2;

    let mut out = vec![0u8; 14];

    const OFFSET_MAGIC: usize = 0;
    const MZ_MAGIC: &[u8; 2] = b"MZ";
    out[OFFSET_MAGIC..OFFSET_MAGIC + WORD_WIDTH].copy_from_slice(MZ_MAGIC);

    const OFFSET_MIN_ALLOC: usize = 10;
    out[OFFSET_MIN_ALLOC..OFFSET_MIN_ALLOC + WORD_WIDTH]
        .copy_from_slice(&spec.min_alloc().to_le_bytes());

    const OFFSET_MAX_ALLOC: usize = 12;
    out[OFFSET_MAX_ALLOC..OFFSET_MAX_ALLOC + WORD_WIDTH]
        .copy_from_slice(&spec.max_alloc().to_le_bytes());
    Ok(out)
}

#[derive(Debug)]
pub enum Error {
    MaxAllocLessThanMinAlloc,
}

#[cfg(test)]
mod tests {
    use crate::{convert, Error, MzSpec};

    #[test]
    fn convert_produces_mz_magic() {
        let spec = MzSpec::builder().build().expect("builder should succeed");
        let mz = convert(&[0x90], &spec).expect("convert should succeed");
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
            let spec = MzSpec::builder()
                .min_alloc(min_alloc)
                .build()
                .expect("builder should succeed");
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
            let spec = MzSpec::builder()
                .max_alloc(max_alloc)
                .build()
                .expect("builder should succeed");
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[12], mz[13]]);
            assert_eq!(
                written, expected,
                "expected max_alloc to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn builder_uses_correct_default_values() {
        let spec = MzSpec::builder().build().expect("builder should succeed");
        assert_eq!(spec.min_alloc(), 0);
        assert_eq!(spec.max_alloc(), 0xFFFF);
    }

    #[test]
    fn builder_sets_valid_values() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0040u16, 0x0100u16, "typical values"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (min_alloc, max_alloc, desc) in cases {
            let spec = MzSpec::builder()
                .min_alloc(min_alloc)
                .max_alloc(max_alloc)
                .build()
                .expect("builder should succeed");
            assert_eq!(spec.min_alloc(), min_alloc, "min_alloc for {desc}");
            assert_eq!(spec.max_alloc(), max_alloc, "max_alloc for {desc}");
        }
    }

    #[test]
    fn builder_rejects_min_alloc_greater_than_max_alloc() {
        let result = MzSpec::builder()
            .min_alloc(0x0100)
            .max_alloc(0x00ff)
            .build();
        assert!(matches!(result, Err(Error::MaxAllocLessThanMinAlloc)));
    }
}

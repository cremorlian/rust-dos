//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

pub struct MzSpec {
    min_alloc: u16,
    max_alloc: u16,
    entry_ip: u16,
    entry_cs: u16,
    stack_ss: u16,
    stack_sp: u16,
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

    pub fn entry_ip(&self) -> u16 {
        self.entry_ip
    }

    pub fn entry_cs(&self) -> u16 {
        self.entry_cs
    }

    pub fn stack_ss(&self) -> u16 {
        self.stack_ss
    }

    pub fn stack_sp(&self) -> u16 {
        self.stack_sp
    }
}

pub struct MzSpecBuilder {
    min_alloc: u16,
    max_alloc: u16,
    entry_ip: u16,
    entry_cs: u16,
    stack_ss: u16,
    stack_sp: u16,
    strictness: Strictness,
}

#[derive(Debug, Copy, Clone, Default)]
pub enum Strictness {
    #[default]
    Error,
    Warn,
    Allow,
}

impl Default for MzSpecBuilder {
    fn default() -> Self {
        Self {
            min_alloc: 0,
            max_alloc: 0xFFFF,
            entry_ip: 0,
            entry_cs: 0,
            stack_ss: 0,
            stack_sp: 0,
            strictness: Strictness::Error,
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

    pub fn entry_ip(mut self, entry_ip: u16) -> Self {
        self.entry_ip = entry_ip;
        self
    }

    pub fn entry_cs(mut self, entry_cs: u16) -> Self {
        self.entry_cs = entry_cs;
        self
    }

    pub fn stack_ss(mut self, stack_ss: u16) -> Self {
        self.stack_ss = stack_ss;
        self
    }

    pub fn stack_sp(mut self, stack_sp: u16) -> Self {
        self.stack_sp = stack_sp;
        self
    }

    pub fn strictness(mut self, strictness: Strictness) -> Self {
        self.strictness = strictness;
        self
    }

    pub fn build(self) -> Result<MzSpec, Error> {
        if self.max_alloc < self.min_alloc {
            enforce(
                self.strictness,
                Error::MaxAllocLessThanMinAlloc {
                    min_alloc: self.min_alloc,
                    max_alloc: self.max_alloc,
                },
            )?;
        }
        if self.entry_ip == 0xFFFF {
            enforce(self.strictness, Error::EntryIpAtLastByteOfSegment)?;
        }
        if self.entry_cs == 0xFFFF {
            enforce(self.strictness, Error::EntryCsAtTopOfMemory)?;
        }
        if self.stack_ss == 0xFFFF {
            enforce(self.strictness, Error::StackSsAtTopOfMemory)?;
        }
        Ok(MzSpec {
            min_alloc: self.min_alloc,
            max_alloc: self.max_alloc,
            entry_ip: self.entry_ip,
            entry_cs: self.entry_cs,
            stack_ss: self.stack_ss,
            stack_sp: self.stack_sp,
        })
    }
}

fn enforce(strictness: Strictness, error: Error) -> Result<(), Error> {
    match strictness {
        Strictness::Error => Err(error),
        Strictness::Warn => {
            eprintln!("{error}");
            Ok(())
        }
        Strictness::Allow => Ok(()),
    }
}

pub fn convert(_elf: &[u8], spec: &MzSpec) -> Result<Vec<u8>, Error> {
    const WORD_WIDTH: usize = 2;

    let mut out = vec![0u8; 24];

    const OFFSET_MAGIC: usize = 0;
    const MZ_MAGIC: &[u8; 2] = b"MZ";
    out[OFFSET_MAGIC..OFFSET_MAGIC + WORD_WIDTH].copy_from_slice(MZ_MAGIC);

    const OFFSET_MIN_ALLOC: usize = 10;
    out[OFFSET_MIN_ALLOC..OFFSET_MIN_ALLOC + WORD_WIDTH]
        .copy_from_slice(&spec.min_alloc().to_le_bytes());

    const OFFSET_MAX_ALLOC: usize = 12;
    out[OFFSET_MAX_ALLOC..OFFSET_MAX_ALLOC + WORD_WIDTH]
        .copy_from_slice(&spec.max_alloc().to_le_bytes());

    const OFFSET_STACK_SS: usize = 14;
    out[OFFSET_STACK_SS..OFFSET_STACK_SS + WORD_WIDTH]
        .copy_from_slice(&spec.stack_ss().to_le_bytes());

    const OFFSET_STACK_SP: usize = 16;
    out[OFFSET_STACK_SP..OFFSET_STACK_SP + WORD_WIDTH]
        .copy_from_slice(&spec.stack_sp().to_le_bytes());

    const OFFSET_ENTRY_IP: usize = 20;
    out[OFFSET_ENTRY_IP..OFFSET_ENTRY_IP + WORD_WIDTH]
        .copy_from_slice(&spec.entry_ip().to_le_bytes());

    const OFFSET_ENTRY_CS: usize = 22;
    out[OFFSET_ENTRY_CS..OFFSET_ENTRY_CS + WORD_WIDTH]
        .copy_from_slice(&spec.entry_cs().to_le_bytes());
    Ok(out)
}

#[derive(Debug)]
pub enum Error {
    MaxAllocLessThanMinAlloc { min_alloc: u16, max_alloc: u16 },
    EntryIpAtLastByteOfSegment,
    EntryCsAtTopOfMemory,
    StackSsAtTopOfMemory,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxAllocLessThanMinAlloc {
                min_alloc,
                max_alloc,
            } => write!(
                f,
                "min_alloc (0x{min_alloc:04X}) must not exceed max_alloc (0x{max_alloc:04X}); \
                 DOS cannot load the program if the minimum allocation cannot be met"
            ),
            Self::EntryIpAtLastByteOfSegment => write!(
                f,
                "entry_ip 0xFFFF is the last byte of a 64 KiB segment; the first fetch wraps to \
                 the segment start, so valid code cannot begin there"
            ),
            Self::EntryCsAtTopOfMemory => write!(
                f,
                "entry_cs 0xFFFF aims the code segment at the top of the 1 MiB map (0xFFFF0); the \
                 loader would relocate it into unmapped memory"
            ),
            Self::StackSsAtTopOfMemory => write!(
                f,
                "stack_ss 0xFFFF aims the stack segment at the top of the 1 MiB map (0xFFFF0); \
                 the loader would relocate it into unmapped memory"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{convert, Error, MzSpec, Strictness};

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
    fn convert_writes_entry_ip_from_spec() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (entry_ip, expected, desc) in cases {
            let spec = MzSpec::builder()
                .strictness(Strictness::Allow)
                .entry_ip(entry_ip)
                .build()
                .expect("builder should succeed");
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[20], mz[21]]);
            assert_eq!(
                written, expected,
                "expected entry_ip to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn convert_writes_entry_cs_from_spec() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (entry_cs, expected, desc) in cases {
            let spec = MzSpec::builder()
                .strictness(Strictness::Allow)
                .entry_cs(entry_cs)
                .build()
                .expect("builder should succeed");
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[22], mz[23]]);
            assert_eq!(
                written, expected,
                "expected entry_cs to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn convert_writes_stack_ss_from_spec() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (stack_ss, expected, desc) in cases {
            let spec = MzSpec::builder()
                .strictness(Strictness::Allow)
                .stack_ss(stack_ss)
                .build()
                .expect("builder should succeed");
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[14], mz[15]]);
            assert_eq!(
                written, expected,
                "expected stack_ss to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn convert_writes_stack_sp_from_spec() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (stack_sp, expected, desc) in cases {
            let spec = MzSpec::builder()
                .stack_sp(stack_sp)
                .build()
                .expect("builder should succeed");
            let mz = convert(&[0x90], &spec).expect("convert should succeed");
            let written = u16::from_le_bytes([mz[16], mz[17]]);
            assert_eq!(
                written, expected,
                "expected stack_sp to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn builder_uses_correct_default_values() {
        let spec = MzSpec::builder().build().expect("builder should succeed");
        assert_eq!(spec.min_alloc(), 0);
        assert_eq!(spec.max_alloc(), 0xFFFF);
        assert_eq!(spec.entry_ip(), 0);
        assert_eq!(spec.entry_cs(), 0);
        assert_eq!(spec.stack_ss(), 0);
        assert_eq!(spec.stack_sp(), 0);
    }

    #[test]
    fn builder_sets_valid_values() {
        let cases = [
            (
                0x0000,
                0x0000,
                0x0000,
                0x0000,
                0x0000,
                0x0000,
                "minimum boundary",
            ),
            (
                0x0040,
                0x0100,
                0x0010,
                0x0020,
                0x0000,
                0x1000,
                "typical values",
            ),
            (
                0xffff,
                0xffff,
                0xfffe,
                0xfffe,
                0xfffe,
                0xfffe,
                "maximum valid boundary",
            ),
        ];
        for (min_alloc, max_alloc, entry_ip, entry_cs, stack_ss, stack_sp, desc) in cases {
            let spec = MzSpec::builder()
                .min_alloc(min_alloc)
                .max_alloc(max_alloc)
                .entry_ip(entry_ip)
                .entry_cs(entry_cs)
                .stack_ss(stack_ss)
                .stack_sp(stack_sp)
                .build()
                .expect("builder should succeed");
            assert_eq!(spec.min_alloc(), min_alloc, "min_alloc for {desc}");
            assert_eq!(spec.max_alloc(), max_alloc, "max_alloc for {desc}");
            assert_eq!(spec.entry_ip(), entry_ip, "entry_ip for {desc}");
            assert_eq!(spec.entry_cs(), entry_cs, "entry_cs for {desc}");
            assert_eq!(spec.stack_ss(), stack_ss, "stack_ss for {desc}");
            assert_eq!(spec.stack_sp(), stack_sp, "stack_sp for {desc}");
        }
    }

    #[test]
    fn builder_rejects_min_alloc_greater_than_max_alloc() {
        let result = MzSpec::builder()
            .min_alloc(0x0100)
            .max_alloc(0x00ff)
            .build();
        assert!(matches!(
            result,
            Err(Error::MaxAllocLessThanMinAlloc {
                min_alloc: 0x0100,
                max_alloc: 0x00ff,
            })
        ));
    }

    #[test]
    fn builder_rejects_entry_ip_at_last_byte_of_segment() {
        let result = MzSpec::builder().entry_ip(0xffff).build();
        assert!(matches!(result, Err(Error::EntryIpAtLastByteOfSegment)));
    }

    #[test]
    fn builder_rejects_entry_cs_at_top_of_memory() {
        let result = MzSpec::builder().entry_cs(0xffff).build();
        assert!(matches!(result, Err(Error::EntryCsAtTopOfMemory)));
    }

    #[test]
    fn builder_rejects_stack_ss_at_top_of_memory() {
        let result = MzSpec::builder().stack_ss(0xffff).build();
        assert!(matches!(result, Err(Error::StackSsAtTopOfMemory)));
    }

    #[test]
    fn builder_strictness_allow_skips_validation() {
        let result = MzSpec::builder()
            .strictness(Strictness::Allow)
            .min_alloc(0x0100)
            .max_alloc(0x00ff)
            .entry_ip(0xffff)
            .entry_cs(0xffff)
            .stack_ss(0xffff)
            .build();
        assert!(result.is_ok());
    }
}

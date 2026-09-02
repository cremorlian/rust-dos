//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

pub mod error;

pub use crate::error::Error;

#[derive(Debug, Copy, Clone, Default)]
pub enum Strictness {
    #[default]
    Error,
    Warn,
    Allow,
}

pub struct Converter {
    min_alloc: u16,
    max_alloc: u16,
    entry_ip: u16,
    entry_cs: u16,
    stack_ss: u16,
    stack_sp: u16,
    strictness: Strictness,
}

impl Default for Converter {
    fn default() -> Self {
        Self::new()
    }
}

impl Converter {
    pub fn new() -> Self {
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

    pub fn convert(&self, _elf: &[u8]) -> Result<Vec<u8>, Error> {
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

        const WORD_WIDTH: usize = 2;
        const PARAGRAPH: usize = 16;
        const HEADER_SIZE: usize = 32;

        let mut out = vec![0u8; HEADER_SIZE];

        let write_word = |out: &mut [u8], offset: usize, value: u16| {
            out[offset..offset + WORD_WIDTH].copy_from_slice(&value.to_le_bytes());
        };

        const OFFSET_MAGIC: usize = 0;
        const MZ_MAGIC: &[u8; 2] = b"MZ";
        out[OFFSET_MAGIC..OFFSET_MAGIC + WORD_WIDTH].copy_from_slice(MZ_MAGIC);

        const OFFSET_CRLC: usize = 6;
        const NO_RELOCATIONS: u16 = 0;
        write_word(&mut out, OFFSET_CRLC, NO_RELOCATIONS);

        const OFFSET_CPARHDR: usize = 8;
        const HEADER_PARAGRAPHS: u16 = (HEADER_SIZE / PARAGRAPH) as u16;
        write_word(&mut out, OFFSET_CPARHDR, HEADER_PARAGRAPHS);

        const OFFSET_MIN_ALLOC: usize = 10;
        write_word(&mut out, OFFSET_MIN_ALLOC, self.min_alloc);

        const OFFSET_MAX_ALLOC: usize = 12;
        write_word(&mut out, OFFSET_MAX_ALLOC, self.max_alloc);

        const OFFSET_STACK_SS: usize = 14;
        write_word(&mut out, OFFSET_STACK_SS, self.stack_ss);

        const OFFSET_STACK_SP: usize = 16;
        write_word(&mut out, OFFSET_STACK_SP, self.stack_sp);

        const OFFSET_CSUM: usize = 18;
        const NO_CHECKSUM: u16 = 0;
        write_word(&mut out, OFFSET_CSUM, NO_CHECKSUM);

        const OFFSET_ENTRY_IP: usize = 20;
        write_word(&mut out, OFFSET_ENTRY_IP, self.entry_ip);

        const OFFSET_ENTRY_CS: usize = 22;
        write_word(&mut out, OFFSET_ENTRY_CS, self.entry_cs);

        const OFFSET_LFARLC: usize = 24;
        const NO_RELOCATION_TABLE: u16 = 0;
        write_word(&mut out, OFFSET_LFARLC, NO_RELOCATION_TABLE);

        const OFFSET_OVNO: usize = 26;
        const NO_OVERLAY: u16 = 0;
        write_word(&mut out, OFFSET_OVNO, NO_OVERLAY);

        Ok(out)
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

#[cfg(test)]
mod tests {
    use crate::{Converter, Error, Strictness};

    #[test]
    fn converter_produces_mz_magic() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        assert_eq!(&mz[0..2], b"MZ");
    }

    #[test]
    fn converter_writes_min_alloc() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0040u16, 0x0040u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (min_alloc, expected, desc) in cases {
            let mz = Converter::new()
                .min_alloc(min_alloc)
                .convert(&[0x90])
                .expect("convert should succeed");
            let written = u16::from_le_bytes([mz[10], mz[11]]);
            assert_eq!(
                written, expected,
                "expected min_alloc to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn converter_writes_max_alloc() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0040u16, 0x0040u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (max_alloc, expected, desc) in cases {
            let mz = Converter::new()
                .max_alloc(max_alloc)
                .convert(&[0x90])
                .expect("convert should succeed");
            let written = u16::from_le_bytes([mz[12], mz[13]]);
            assert_eq!(
                written, expected,
                "expected max_alloc to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn converter_writes_entry_ip() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (entry_ip, expected, desc) in cases {
            let mz = Converter::new()
                .strictness(Strictness::Allow)
                .entry_ip(entry_ip)
                .convert(&[0x90])
                .expect("convert should succeed");
            let written = u16::from_le_bytes([mz[20], mz[21]]);
            assert_eq!(
                written, expected,
                "expected entry_ip to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn converter_writes_entry_cs() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (entry_cs, expected, desc) in cases {
            let mz = Converter::new()
                .strictness(Strictness::Allow)
                .entry_cs(entry_cs)
                .convert(&[0x90])
                .expect("convert should succeed");
            let written = u16::from_le_bytes([mz[22], mz[23]]);
            assert_eq!(
                written, expected,
                "expected entry_cs to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn converter_writes_stack_ss() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (stack_ss, expected, desc) in cases {
            let mz = Converter::new()
                .strictness(Strictness::Allow)
                .stack_ss(stack_ss)
                .convert(&[0x90])
                .expect("convert should succeed");
            let written = u16::from_le_bytes([mz[14], mz[15]]);
            assert_eq!(
                written, expected,
                "expected stack_ss to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn converter_writes_stack_sp() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (stack_sp, expected, desc) in cases {
            let mz = Converter::new()
                .stack_sp(stack_sp)
                .convert(&[0x90])
                .expect("convert should succeed");
            let written = u16::from_le_bytes([mz[16], mz[17]]);
            assert_eq!(
                written, expected,
                "expected stack_sp to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn converter_writes_crlc_as_zero() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        let written = u16::from_le_bytes([mz[6], mz[7]]);
        assert_eq!(written, 0, "e_crlc must be zero (no relocations)");
    }

    #[test]
    fn converter_writes_csum_as_zero() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        let written = u16::from_le_bytes([mz[18], mz[19]]);
        assert_eq!(written, 0, "e_csum must be zero (no checksum)");
    }

    #[test]
    fn converter_writes_cparhdr_as_two() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        let written = u16::from_le_bytes([mz[8], mz[9]]);
        assert_eq!(written, 2, "e_cparhdr must be 2 (32-byte padded header)");
    }

    #[test]
    fn converter_pads_header_to_paragraph_boundary() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        assert_eq!(
            mz.len(),
            32,
            "28-byte header must be padded to a whole 16-byte paragraph"
        );
    }

    #[test]
    fn converter_writes_lfarlc_as_zero() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        let written = u16::from_le_bytes([mz[24], mz[25]]);
        assert_eq!(written, 0, "e_lfarlc must be zero (no relocs)");
    }

    #[test]
    fn converter_writes_ovno_as_zero() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        let written = u16::from_le_bytes([mz[26], mz[27]]);
        assert_eq!(written, 0, "e_ovno must be zero (no overlay)");
    }

    #[test]
    fn converter_uses_correct_default_values() {
        let mz = Converter::new()
            .convert(&[0x90])
            .expect("convert should succeed");
        assert_eq!(u16::from_le_bytes([mz[10], mz[11]]), 0, "min_alloc");
        assert_eq!(u16::from_le_bytes([mz[12], mz[13]]), 0xFFFF, "max_alloc");
        assert_eq!(u16::from_le_bytes([mz[20], mz[21]]), 0, "entry_ip");
        assert_eq!(u16::from_le_bytes([mz[22], mz[23]]), 0, "entry_cs");
        assert_eq!(u16::from_le_bytes([mz[14], mz[15]]), 0, "stack_ss");
        assert_eq!(u16::from_le_bytes([mz[16], mz[17]]), 0, "stack_sp");
    }

    #[test]
    fn converter_writes_all_set_values() {
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
                0xfffe,
                0xffff,
                0xfffe,
                0xfffe,
                0xfffe,
                0xfffe,
                "maximum valid boundary",
            ),
        ];
        for (min_alloc, max_alloc, entry_ip, entry_cs, stack_ss, stack_sp, desc) in cases {
            let mz = Converter::new()
                .strictness(Strictness::Allow)
                .min_alloc(min_alloc)
                .max_alloc(max_alloc)
                .entry_ip(entry_ip)
                .entry_cs(entry_cs)
                .stack_ss(stack_ss)
                .stack_sp(stack_sp)
                .convert(&[0x90])
                .expect("convert should succeed");
            assert_eq!(
                u16::from_le_bytes([mz[10], mz[11]]),
                min_alloc,
                "min_alloc for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([mz[12], mz[13]]),
                max_alloc,
                "max_alloc for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([mz[20], mz[21]]),
                entry_ip,
                "entry_ip for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([mz[22], mz[23]]),
                entry_cs,
                "entry_cs for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([mz[14], mz[15]]),
                stack_ss,
                "stack_ss for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([mz[16], mz[17]]),
                stack_sp,
                "stack_sp for {desc}"
            );
        }
    }

    #[test]
    fn converter_rejects_min_alloc_greater_than_max_alloc() {
        let result = Converter::new()
            .min_alloc(0x0100)
            .max_alloc(0x00ff)
            .convert(&[0x90]);
        assert!(matches!(
            result,
            Err(Error::MaxAllocLessThanMinAlloc {
                min_alloc: 0x0100,
                max_alloc: 0x00ff,
            })
        ));
    }

    #[test]
    fn converter_rejects_entry_ip_at_last_byte_of_segment() {
        let result = Converter::new().entry_ip(0xffff).convert(&[0x90]);
        assert!(matches!(result, Err(Error::EntryIpAtLastByteOfSegment)));
    }

    #[test]
    fn converter_rejects_entry_cs_at_top_of_memory() {
        let result = Converter::new().entry_cs(0xffff).convert(&[0x90]);
        assert!(matches!(result, Err(Error::EntryCsAtTopOfMemory)));
    }

    #[test]
    fn converter_rejects_stack_ss_at_top_of_memory() {
        let result = Converter::new().stack_ss(0xffff).convert(&[0x90]);
        assert!(matches!(result, Err(Error::StackSsAtTopOfMemory)));
    }

    #[test]
    fn converter_strictness_allow_skips_validation() {
        let result = Converter::new()
            .strictness(Strictness::Allow)
            .min_alloc(0x0100)
            .max_alloc(0x00ff)
            .entry_ip(0xffff)
            .entry_cs(0xffff)
            .stack_ss(0xffff)
            .convert(&[0x90]);
        assert!(result.is_ok());
    }
}

//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

pub mod error;

mod compose;
mod elf;
mod layout;
mod mz_header;
mod strictness;

pub use crate::error::Error;
pub use crate::strictness::Strictness;

pub struct Converter {
    min_alloc: u16,
    max_alloc: u16,
    entry_ip: u16,
    entry_cs: u16,
    stack: layout::StackSpec,
    strictness: Strictness,
    layout: layout::LayoutConfig,
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
            stack: layout::StackSpec::Default,
            strictness: Strictness::Error,
            layout: layout::LayoutConfig::ImageOnly,
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

    pub fn stack(mut self, ss: u16, sp: u16) -> Self {
        self.stack = layout::StackSpec::Set { ss, sp };
        self
    }

    pub fn strictness(mut self, strictness: Strictness) -> Self {
        self.strictness = strictness;
        self
    }

    pub fn stub(mut self, shell: &[u8]) -> Result<Self, Error> {
        if shell.is_empty() {
            return Err(Error::EmptyStub);
        }
        self.layout = layout::LayoutConfig::Shell {
            shell: shell.to_vec(),
        };
        Ok(self)
    }

    pub fn convert(&self, elf: &[u8]) -> Result<Vec<u8>, Error> {
        let image = elf::extract_image(elf)?;
        let spec = layout::LayoutSpec {
            layout: &self.layout,
            entry_cs: self.entry_cs,
            entry_ip: self.entry_ip,
            image_len: image.len(),
            min_alloc: self.min_alloc,
            strictness: self.strictness,
            stack: self.stack,
        };
        let resolved = layout::resolve(&spec)?;
        let stub = match &self.layout {
            layout::LayoutConfig::ImageOnly => &[][..],
            layout::LayoutConfig::Shell { shell } => shell.as_slice(),
        };
        let header = mz_header::build_headers(
            &mz_header::HeaderSpecs {
                min_alloc: self.min_alloc,
                max_alloc: self.max_alloc,
                stack_ss: resolved.stack.0,
                stack_sp: resolved.stack.1,
                entry_ip: resolved.entry.1,
                entry_cs: resolved.entry.0,
            },
            resolved.module_len,
            self.strictness,
        )?;
        Ok(compose::pack(&header, stub, &image))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_ELF: &[u8] = include_bytes!("../tests/fixtures/minimal.elf");

    #[test]
    fn convert_rejects_block_end_over_limit_under_error() {
        let result = Converter::new()
            .stub(&[0u8; 12])
            .unwrap()
            .min_alloc(0xFFFF)
            .convert(MINIMAL_ELF);
        assert!(matches!(
            result,
            Err(Error::StackBlockEndUnrepresentable { block_end })
                if block_end == 0xFFFFF
        ));
    }

    #[test]
    fn convert_rejects_block_end_over_limit_under_allow() {
        let result = Converter::new()
            .stub(&[0u8; 12])
            .unwrap()
            .min_alloc(0xFFFF)
            .strictness(Strictness::Allow)
            .convert(MINIMAL_ELF);
        assert!(matches!(
            result,
            Err(Error::StackBlockEndUnrepresentable { block_end })
                if block_end == 0xFFFFF
        ));
    }

    #[test]
    fn convert_writes_max_expressible_default_stack() {
        let out = Converter::new()
            .stub(&[0u8; 12])
            .unwrap()
            .min_alloc(0xFFFE)
            .convert(MINIMAL_ELF)
            .unwrap();
        let e_ss = u16::from_le_bytes([out[14], out[15]]);
        let e_sp = u16::from_le_bytes([out[16], out[17]]);
        assert_eq!((e_ss, e_sp), (0xFFFE, 0x000F));
    }

    #[test]
    fn convert_rejects_wrapping_canary_stack_under_allow() {
        let result = Converter::new()
            .stack(0xFFFF, 0)
            .strictness(Strictness::Allow)
            .convert(MINIMAL_ELF);
        assert!(matches!(result, Err(Error::StackSsWrapsDuringRelocation)));
    }
}

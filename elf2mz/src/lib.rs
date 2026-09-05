//! Convert a static ELF32 executable into an MZ (MS-DOS) executable.

pub mod error;

mod elf;
mod mz_header;
mod strictness;

pub use crate::error::Error;
pub use crate::strictness::Strictness;

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
        let header = mz_header::build_headers(
            &mz_header::HeaderSpecs {
                min_alloc: self.min_alloc,
                max_alloc: self.max_alloc,
                stack_ss: self.stack_ss,
                stack_sp: self.stack_sp,
                entry_ip: self.entry_ip,
                entry_cs: self.entry_cs,
            },
            0,
            self.strictness,
        )?;
        Ok(header.to_vec())
    }
}

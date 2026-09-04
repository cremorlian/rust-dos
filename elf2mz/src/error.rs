//! The single public error type for the `elf2mz` packer.

#[derive(Debug)]
pub enum Error {
    MaxAllocLessThanMinAlloc { min_alloc: u16, max_alloc: u16 },
    EntryIpAtLastByteOfSegment,
    EntryCsAtTopOfMemory,
    StackSsAtTopOfMemory,
    NotAnElfFile,
    UnsupportedElfClass,
    UnsupportedElfEndian,
    Truncated,
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
            Self::NotAnElfFile => write!(
                f,
                "input does not begin with the ELF magic (\\x7FELF); expected an ELF32 executable"
            ),
            Self::UnsupportedElfClass => write!(
                f,
                "ELF class is ELF64; only ELF32 (ELFCLASS32) executables are supported"
            ),
            Self::UnsupportedElfEndian => write!(
                f,
                "ELF data encoding is big-endian; only little-endian (ELFDATA2LSB) is supported"
            ),
            Self::Truncated => write!(
                f,
                "ELF file is truncated: fewer bytes than required to hold the header and/or program headers"
            ),
        }
    }
}

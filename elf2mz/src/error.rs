//! The single public error type for the `elf2mz` packer.

#[derive(Debug)]
pub enum Error {
    MaxAllocLessThanMinAlloc {
        min_alloc: u16,
        max_alloc: u16,
    },
    EntryOutsideImage {
        entry_cs: u16,
        entry_ip: u16,
        module_len: usize,
    },
    StackSsWrapsDuringRelocation,
    EmptyStub,
    EntryOwnedByShell {
        entry_cs: u16,
        entry_ip: u16,
    },
    NotAnElfFile,
    UnsupportedElfClass,
    UnsupportedElfEndian,
    UnsupportedElfType {
        e_type: u16,
    },
    Truncated,
    NoLoadableSegments,
    InvalidSegmentSize,
    InvalidSegmentRange,
    OutputTooLarge {
        pages: u32,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxAllocLessThanMinAlloc {
                min_alloc,
                max_alloc,
            } => write!(
                f,
                "min_alloc (0x{min_alloc:04X}) exceeds max_alloc (0x{max_alloc:04X})"
            ),
            Self::EntryOutsideImage {
                entry_cs,
                entry_ip,
                module_len,
            } => {
                let offset = (*entry_cs as u32) * 16 + (*entry_ip as u32);
                write!(
                    f,
                    "entry CS:IP (0x{entry_cs:04X}:0x{entry_ip:04X}) resolves to offset \
                     0x{offset:X} of the module, past its {module_len} bytes"
                )
            }
            Self::StackSsWrapsDuringRelocation => write!(
                f,
                "stack_ss 0xFFFF wraps into low memory once the load segment is added"
            ),
            Self::EmptyStub => write!(f, "stub is empty; a shell must contain at least one byte"),
            Self::EntryOwnedByShell { entry_cs, entry_ip } => write!(
                f,
                "shell mode fixes the entry at 0:0; a caller-supplied CS:IP \
                 (0x{entry_cs:04X}:0x{entry_ip:04X}) is a contradiction"
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
            Self::UnsupportedElfType { e_type } => write!(
                f,
                "ELF type is {e_type}; only static executables (ET_EXEC, value 2) are supported. \
                 PIE (ET_DYN) and relocatable (ET_REL) inputs require dynamic linking"
            ),
            Self::OutputTooLarge { pages } => write!(
                f,
                "output is too large: it spans {pages} pages, but the MZ page count (e_cp) is a u16 \
                 and can represent at most 65535 pages"
            ),
            Self::Truncated => write!(
                f,
                "ELF file is truncated: fewer bytes than required to hold the header and/or program headers"
            ),
            Self::NoLoadableSegments => write!(
                f,
                "ELF has no loadable program segments; there is no program image to load"
            ),
            Self::InvalidSegmentSize => write!(
                f,
                "program segment is malformed: file size (p_filesz) exceeds its memory size (p_memsz)"
            ),
            Self::InvalidSegmentRange => write!(
                f,
                "program segment address range (p_vaddr + p_memsz) overflows the 32-bit address space"
            ),
        }
    }
}

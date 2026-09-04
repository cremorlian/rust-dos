use crate::Error;

pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
    if !bytes.starts_with(ELF_MAGIC) {
        return Err(Error::NotAnElfFile);
    }

    const ELFCLASS32: u8 = 1;
    const IDENT_CLASS: usize = 4;
    if bytes.get(IDENT_CLASS) != Some(&ELFCLASS32) {
        return Err(Error::UnsupportedElfClass);
    }

    const ELFDATA2LSB: u8 = 1;
    const IDENT_DATA: usize = 5;
    if bytes.get(IDENT_DATA) != Some(&ELFDATA2LSB) {
        return Err(Error::UnsupportedElfEndian);
    }

    const ELF32_HEADER_SIZE: usize = 52;
    if bytes.len() < ELF32_HEADER_SIZE {
        return Err(Error::Truncated);
    }

    let read_u32 = |offset: usize| -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    };

    const E_PHOFF: usize = 28;

    let phoff = read_u32(E_PHOFF) as usize;

    const P_OFFSET: usize = 4;
    const P_FILESZ: usize = 16;
    const P_MEMSZ: usize = 20;

    let seg_offset = read_u32(phoff + P_OFFSET) as usize;
    let filesz = read_u32(phoff + P_FILESZ) as usize;
    let memsz = read_u32(phoff + P_MEMSZ) as usize;

    let mut image = vec![0u8; memsz];
    image[0..filesz].copy_from_slice(&bytes[seg_offset..seg_offset + filesz]);

    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn parse_rejects_non_elf() {
        let bytes = b"this is not an ELF file at all";
        assert!(matches!(parse(bytes), Err(Error::NotAnElfFile)));
    }

    #[test]
    fn parse_rejects_elf64_class() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        assert!(matches!(parse(&bytes), Err(Error::UnsupportedElfClass)));
    }

    #[test]
    fn parse_rejects_big_endian() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 1;
        bytes[5] = 2;
        assert!(matches!(parse(&bytes), Err(Error::UnsupportedElfEndian)));
    }

    #[test]
    fn parse_rejects_truncated_header() {
        let mut bytes = vec![0u8; 16];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 1;
        bytes[5] = 1;
        assert!(matches!(parse(&bytes), Err(Error::Truncated)));
    }

    struct Segment {
        vaddr: u32,
        data: Vec<u8>,
    }

    fn build_elf(segments: &[Segment]) -> Vec<u8> {
        const HEADER_SIZE: usize = 52;
        const PHENT_SIZE: usize = 32;
        const PT_LOAD: u32 = 1;

        let data_offset = HEADER_SIZE + segments.len() * PHENT_SIZE;

        let mut out = vec![0u8; data_offset];
        out[0..4].copy_from_slice(b"\x7fELF");
        out[4] = 1;
        out[5] = 1;
        out[28..32].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        out[42..44].copy_from_slice(&(PHENT_SIZE as u16).to_le_bytes());
        out[44..46].copy_from_slice(&(segments.len() as u16).to_le_bytes());

        for (i, seg) in segments.iter().enumerate() {
            let ph = HEADER_SIZE + i * PHENT_SIZE;
            out[ph..ph + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
            out[ph + 4..ph + 8].copy_from_slice(&(data_offset as u32).to_le_bytes());
            out[ph + 8..ph + 12].copy_from_slice(&seg.vaddr.to_le_bytes());
            out[ph + 12..ph + 16].copy_from_slice(&seg.vaddr.to_le_bytes());
            out[ph + 16..ph + 20].copy_from_slice(&(seg.data.len() as u32).to_le_bytes());
            out[ph + 20..ph + 24].copy_from_slice(&(seg.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&seg.data);
        }
        out
    }

    #[test]
    fn parse_extracts_single_load_segment() {
        let elf = build_elf(&[Segment {
            vaddr: 0x1000,
            data: vec![1, 2, 3, 4],
        }]);
        let image = parse(&elf).expect("parse should succeed");
        assert_eq!(image, &[1, 2, 3, 4]);
    }
}

pub fn extract_image(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    use object::Object;
    use object::ObjectSegment;

    let file = object::File::parse(bytes).map_err(|_| Error::NotAnElf)?;

    let mut image = Vec::new();
    for segment in file.segments() {
        let data = segment.data().map_err(|_| Error::NotAnElf)?;
        image.extend_from_slice(data);
    }

    if image.is_empty() {
        return Err(Error::NoLoadableSegments);
    }

    Ok(image)
}

#[derive(Debug, PartialEq)]
pub enum Error {
    NotAnElf,
    NoLoadableSegments,
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::{build, elf, Endianness};

    #[test]
    fn extract_image_concatenates_load_segments_in_program_header_order() {
        let elf = build_elf(&[vec![1, 2, 3, 4], vec![5, 6]]);

        let image = extract_image(&elf).expect("parse should succeed");

        assert_eq!(image, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn extract_image_rejects_segment_range_past_end_of_file() {
        let mut elf = build_elf(&[vec![1, 2, 3, 4]]);
        elf[0x34 + 16..0x34 + 16 + 4].copy_from_slice(&(u32::MAX - 1).to_le_bytes());

        assert!(matches!(
            extract_image(&elf),
            Err(Error::NotAnElf)
        ));
    }

    #[test]
    fn extract_image_rejects_truncated_input() {
        let elf = build_elf(&[vec![1, 2, 3, 4]]);
        let truncated = &elf[..0x34 + 32 + 2];

        assert!(matches!(
            extract_image(truncated),
            Err(Error::NotAnElf)
        ));
    }

    fn build_elf(segments: &[Vec<u8>]) -> Vec<u8> {
        let mut builder = build::elf::Builder::new(Endianness::Little, false);
        builder.header.e_phoff = 0x34;

        let shstrtab = builder.sections.add();
        shstrtab.name = b".shstrtab"[..].into();
        shstrtab.sh_type = elf::SHT_STRTAB;
        shstrtab.data = build::elf::SectionData::SectionString;

        let mut file_offset = 0x34 + segments.len() as u64 * 32;
        for seg in segments.iter() {
            let segment = builder.segments.add();
            segment.p_type = elf::PT_LOAD;
            segment.p_offset = file_offset;
            segment.p_filesz = seg.len() as u64;

            let section = builder.sections.add();
            section.sh_addralign = 1;
            section.data = build::elf::SectionData::Data(seg[..].into());

            segment.sections.push(section.id());
            file_offset += seg.len() as u64;
        }

        let mut buf = Vec::new();
        builder.write(&mut buf).expect("write elf");
        buf
    }
}
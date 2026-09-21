pub fn extract_image(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    use object::Object;
    use object::ObjectSegment;

    let file = object::File::parse(bytes).map_err(|_| Error::NotAnElf)?;

    let mut image = Vec::new();
    let mut expected: Option<u64> = None;

    for (i, segment) in file.segments().enumerate() {
        let data = segment.data().map_err(|_| Error::NotAnElf)?;
        let offset = segment.file_range().0;
        let filesz = segment.file_range().1;

        if let Some(expected_offset) = expected
            && offset != expected_offset
        {
            return Err(Error::UnexpectedSegmentOffset {
                index: i,
                expected_offset,
                actual_offset: offset,
            });
        }
        expected = Some(offset + filesz);
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
    UnexpectedSegmentOffset {
        index: usize,
        expected_offset: u64,
        actual_offset: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::{Endianness, build, elf};

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

        assert!(matches!(extract_image(&elf), Err(Error::NotAnElf)));
    }

    #[test]
    fn extract_image_rejects_truncated_input() {
        let elf = build_elf(&[vec![1, 2, 3, 4]]);
        let truncated = &elf[..0x34 + 32 + 2];

        assert!(matches!(extract_image(truncated), Err(Error::NotAnElf)));
    }

    #[test]
    fn extract_image_rejects_gap_between_segments() {
        let mut elf = build_elf(&[vec![1, 2, 3, 4], vec![5, 6]]);

        const PHDR_SIZE: usize = 32;
        const P_OFFSET_OFF: usize = 4;
        let second_phdr = 0x34 + PHDR_SIZE;
        elf[second_phdr + P_OFFSET_OFF..second_phdr + P_OFFSET_OFF + 8]
            .copy_from_slice(&0x110u64.to_le_bytes());

        assert!(matches!(
            extract_image(&elf),
            Err(Error::UnexpectedSegmentOffset {
                index: 1,
                expected_offset: 0x78,
                actual_offset: 0x110,
            })
        ));
    }

    #[test]
    fn extract_image_rejects_overlapping_segments() {
        let mut elf = build_elf(&[vec![1, 2, 3, 4], vec![5, 6]]);

        const PHDR_SIZE: usize = 32;
        const P_OFFSET_OFF: usize = 4;
        let second_phdr = 0x34 + PHDR_SIZE;
        elf[second_phdr + P_OFFSET_OFF..second_phdr + P_OFFSET_OFF + 8]
            .copy_from_slice(&0x76u64.to_le_bytes());

        assert!(matches!(
            extract_image(&elf),
            Err(Error::UnexpectedSegmentOffset {
                index: 1,
                expected_offset: 0x78,
                actual_offset: 0x76,
            })
        ));
    }

    #[test]
    fn extract_image_rejects_segments_out_of_program_order() {
        let mut elf = build_elf(&[vec![1, 2, 3, 4], vec![5, 6]]);

        const P_OFFSET_OFF: usize = 4;
        let first_phdr = 0x34;

        elf[first_phdr + P_OFFSET_OFF..first_phdr + P_OFFSET_OFF + 8]
            .copy_from_slice(&0x100u64.to_le_bytes());

        assert!(matches!(
            extract_image(&elf),
            Err(Error::UnexpectedSegmentOffset {
                index: 1,
                expected_offset: 0x104,
                actual_offset: 0x78,
            })
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

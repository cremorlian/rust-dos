pub struct ElfData {
    pub image: Vec<u8>,
    pub fixups: Vec<u32>,
}

pub fn extract(bytes: &[u8]) -> Result<ElfData, Error> {
    let object::File::Elf32(file) = object::File::parse(bytes).map_err(|_| Error::NotAnElf)? else {
        return Err(Error::NotAnElf);
    };
    let image = extract_image(&file)?;
    let fixups = collect_fixups(&file)?;
    Ok(ElfData { image, fixups })
}

fn extract_image(file: &object::read::elf::ElfFile32<'_>) -> Result<Vec<u8>, Error> {
    use object::{Object, ObjectSegment};

    let image_size: u64 = file.segments().map(|s| s.file_range().1).sum();
    if image_size > u64::from(u32::MAX) {
        return Err(Error::ImageTooLarge { image_size });
    }

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

    Ok(image)
}

fn collect_fixups(file: &object::read::elf::ElfFile32<'_>) -> Result<Vec<u32>, Error> {
    use object::{Object, ObjectSection, ObjectSegment};

    let mut image_start = 0u64;
    let mut spans: Vec<(u64, u64, u64)> = Vec::new();
    for segment in file.segments() {
        let filesz = segment.file_range().1;
        spans.push((segment.address(), filesz, image_start));
        image_start += filesz;
    }

    let mut fixups = Vec::new();
    for section in file.sections() {
        let is_reloc = matches!(
            section.flags(),
            object::SectionFlags::Elf { sh_type, .. }
                if sh_type == object::elf::SHT_REL || sh_type == object::elf::SHT_RELA
        );
        if is_reloc {
            continue;
        }
        for (r_offset, relocation) in section.relocations() {
            let Some((address, _filesz, image_start)) = spans
                .iter()
                .copied()
                .find(|&(address, filesz, _)| r_offset >= address && r_offset < address + filesz)
            else {
                continue;
            };
            let object::RelocationFlags::Elf { r_type } = relocation.flags() else {
                unreachable!("non-ELF flags from an ELF32 file");
            };
            match r_type {
                object::elf::R_386_32 => {
                    fixups.push((image_start + (r_offset - address)) as u32);
                }
                object::elf::R_386_PC32 => {}
                _ => {
                    return Err(Error::UnrepresentableRelocType {
                        offset: r_offset,
                        r_type: r_type.0,
                    });
                }
            }
        }
    }

    Ok(fixups)
}

#[derive(Debug, PartialEq)]
pub enum Error {
    NotAnElf,
    UnrepresentableRelocType {
        offset: u64,
        r_type: u32,
    },
    ImageTooLarge {
        image_size: u64,
    },
    UnexpectedSegmentOffset {
        index: usize,
        expected_offset: u64,
        actual_offset: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use object::elf;

    #[test]
    fn extract_concatenates_load_segments_in_program_header_order() {
        let elf = fixtures::build_elf(&[(vec![1, 2, 3, 4], 0x400000), (vec![5, 6], 0x400000)], &[]);

        let data = extract(&elf).expect("parse should succeed");

        assert_eq!(data.image, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn extract_rejects_segment_range_past_end_of_file() {
        let mut elf = fixtures::build_elf(&[(vec![1, 2, 3, 4], 0x400000)], &[]);
        elf[0x34 + 16..0x34 + 16 + 4].copy_from_slice(&(u32::MAX - 1).to_le_bytes());

        assert!(matches!(extract(&elf), Err(Error::NotAnElf)));
    }

    #[test]
    fn extract_rejects_image_larger_than_u32_max() {
        let mut elf =
            fixtures::build_elf(&[(vec![1, 2, 3, 4], 0x400000), (vec![5], 0x400004)], &[]);
        elf[0x34 + 16..0x34 + 16 + 4].copy_from_slice(&0x8000_0000u32.to_le_bytes());
        elf[0x54 + 16..0x54 + 16 + 4].copy_from_slice(&0x8000_0000u32.to_le_bytes());

        assert!(matches!(
            extract(&elf),
            Err(Error::ImageTooLarge {
                image_size: 0x1_0000_0000
            })
        ));
    }

    #[test]
    fn extract_rejects_truncated_input() {
        let elf = fixtures::build_elf(&[(vec![1, 2, 3, 4], 0x400000)], &[]);
        let truncated = &elf[..0x34 + 32 + 2];

        assert!(matches!(extract(truncated), Err(Error::NotAnElf)));
    }

    #[test]
    fn extract_rejects_gap_between_segments() {
        let mut elf =
            fixtures::build_elf(&[(vec![1, 2, 3, 4], 0x400000), (vec![5, 6], 0x400000)], &[]);

        const PHDR_SIZE: usize = 32;
        const P_OFFSET_OFF: usize = 4;
        let second_phdr = 0x34 + PHDR_SIZE;
        elf[second_phdr + P_OFFSET_OFF..second_phdr + P_OFFSET_OFF + 8]
            .copy_from_slice(&0x110u64.to_le_bytes());

        assert!(matches!(
            extract(&elf),
            Err(Error::UnexpectedSegmentOffset {
                index: 1,
                expected_offset: 0x78,
                actual_offset: 0x110,
            })
        ));
    }

    #[test]
    fn extract_rejects_overlapping_segments() {
        let mut elf =
            fixtures::build_elf(&[(vec![1, 2, 3, 4], 0x400000), (vec![5, 6], 0x400000)], &[]);

        const PHDR_SIZE: usize = 32;
        const P_OFFSET_OFF: usize = 4;
        let second_phdr = 0x34 + PHDR_SIZE;
        elf[second_phdr + P_OFFSET_OFF..second_phdr + P_OFFSET_OFF + 8]
            .copy_from_slice(&0x76u64.to_le_bytes());

        assert!(matches!(
            extract(&elf),
            Err(Error::UnexpectedSegmentOffset {
                index: 1,
                expected_offset: 0x78,
                actual_offset: 0x76,
            })
        ));
    }

    #[test]
    fn extract_builds_image_and_records_loaded_r386_32_offsets() {
        let elf = fixtures::build_elf(
            &[(vec![0xAA; 0x10001], 0x400000), (vec![0xBB; 8], 0x410001)],
            &[
                (
                    0,
                    &[
                        (0x400000, elf::R_386_32),
                        (0x400004, elf::R_386_32),
                        (0x410000, elf::R_386_32),
                    ],
                ),
                (1, &[(0x410001, elf::R_386_32), (0x410008, elf::R_386_32)]),
            ],
        );

        let data = extract(&elf).expect("parse should succeed");

        let image = [vec![0xAA; 0x10001], vec![0xBB; 8]].concat();
        assert_eq!(data.image, image);
        assert_eq!(data.fixups, [0, 4, 0x10000, 0x10001, 0x10008]);
    }

    #[test]
    fn extract_ignores_r386_pc32_at_loaded_sites() {
        let elf = fixtures::build_elf(
            &[(vec![0xAA; 8], 0x400000)],
            &[(0, &[(0x400000, elf::R_386_32), (0x400004, elf::R_386_PC32)])],
        );

        let data = extract(&elf).expect("parse should succeed");

        assert_eq!(data.fixups, [0]);
    }

    #[test]
    fn extract_rejects_known_but_unrepresentable_reloc_types_at_loaded_sites() {
        for (r_type, expected) in [
            (elf::R_386_NONE, 0),
            (elf::R_386_PLT32, elf::R_386_PLT32.0),
            (elf::R_386_PC16, elf::R_386_PC16.0),
            (elf::R_386_PC8, elf::R_386_PC8.0),
        ] {
            let elf =
                fixtures::build_elf(&[(vec![0xAA; 8], 0x400000)], &[(0, &[(0x400000, r_type)])]);

            assert!(matches!(
                extract(&elf),
                Err(Error::UnrepresentableRelocType {
                    offset: 0x400000,
                    r_type
                }) if r_type == expected
            ));
        }
    }

    #[test]
    fn extract_rejects_unrepresentable_reloc_type_at_loaded_sites() {
        let elf = fixtures::build_elf(
            &[(vec![0xAA; 8], 0x400000)],
            &[(0, &[(0x400000, elf::RelocationType(99))])],
        );

        assert!(matches!(
            extract(&elf),
            Err(Error::UnrepresentableRelocType {
                offset: 0x400000,
                r_type: 99
            })
        ));
    }

    #[test]
    fn extract_records_only_relocs_within_load_segment_range() {
        let elf = fixtures::build_elf(
            &[(vec![0xAA; 8], 0x400000)],
            &[(
                0,
                &[
                    (0x3fffff, elf::R_386_32),
                    (0x400000, elf::R_386_32),
                    (0x400007, elf::R_386_32),
                    (0x400008, elf::R_386_32),
                ],
            )],
        );

        let data = extract(&elf).expect("parse should succeed");

        assert_eq!(data.fixups, [0, 7]);
    }

    #[test]
    fn extract_rejects_segments_out_of_program_order() {
        let mut elf =
            fixtures::build_elf(&[(vec![1, 2, 3, 4], 0x400000), (vec![5, 6], 0x400000)], &[]);

        const P_OFFSET_OFF: usize = 4;
        let first_phdr = 0x34;

        elf[first_phdr + P_OFFSET_OFF..first_phdr + P_OFFSET_OFF + 8]
            .copy_from_slice(&0x100u64.to_le_bytes());

        assert!(matches!(
            extract(&elf),
            Err(Error::UnexpectedSegmentOffset {
                index: 1,
                expected_offset: 0x104,
                actual_offset: 0x78,
            })
        ));
    }
}

#[derive(Debug, PartialEq)]
pub enum Error {
    FixupCollidesWithSentinel { offset: u32 },
}

pub fn table(fixups: &[u32]) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::with_capacity(fixups.len() * 4 + 4);
    for fixup in fixups {
        if *fixup == u32::MAX {
            return Err(Error::FixupCollidesWithSentinel { offset: *fixup });
        }
        bytes.extend_from_slice(&fixup.to_le_bytes());
    }
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_encodes_little_endian_offsets_and_terminating_sentinel() {
        let table = table(&[0, 7, 0x10008]).expect("encode should succeed");

        assert_eq!(
            table,
            [
                0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0xff, 0xff,
                0xff, 0xff,
            ]
        );
    }

    #[test]
    fn table_encodes_just_the_sentinel_for_no_fixups() {
        assert_eq!(table(&[]).expect("encode should succeed"), [0xff; 4]);
    }

    #[test]
    fn table_rejects_offset_equal_to_sentinel() {
        assert!(matches!(
            table(&[0x1122_3344, u32::MAX]),
            Err(Error::FixupCollidesWithSentinel { offset: u32::MAX })
        ));
    }

    #[test]
    fn table_encodes_largest_offset_below_sentinel() {
        let table = table(&[0x00ff_fffe]).expect("encode should succeed");

        assert_eq!(table, [0xfe, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn table_encodes_all_four_byte_lanes() {
        let table = table(&[0x1122_3344]).expect("encode should succeed");

        assert_eq!(table, [0x44, 0x33, 0x22, 0x11, 0xff, 0xff, 0xff, 0xff]);
    }
}

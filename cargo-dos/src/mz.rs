pub(crate) const HEADER_BYTES: usize = 32;
const PARAGRAPH_BYTES: usize = 16;

pub fn header(file_len: usize) -> Result<[u8; HEADER_BYTES], Error> {
    if file_len == 0 {
        return Err(Error::FileLengthIsZero);
    }
    if file_len > 65535 * 512 {
        return Err(Error::FileLengthTooLarge);
    }

    let mut out = [0u8; HEADER_BYTES];

    const MZ_MAGIC: [u8; 2] = *b"MZ";
    out[..2].copy_from_slice(&MZ_MAGIC);

    const LAST_PAGE_BYTES: usize = 2;
    out[LAST_PAGE_BYTES..LAST_PAGE_BYTES + 2]
        .copy_from_slice(&last_page_bytes(file_len).to_le_bytes());

    const PAGE_COUNT: usize = 4;
    out[PAGE_COUNT..PAGE_COUNT + 2].copy_from_slice(&page_count(file_len).to_le_bytes());

    const E_CPARHDR: usize = 8;
    let header_paragraphs = (HEADER_BYTES / PARAGRAPH_BYTES) as u16;
    out[E_CPARHDR..E_CPARHDR + 2].copy_from_slice(&header_paragraphs.to_le_bytes());

    Ok(out)
}

#[derive(Debug, PartialEq)]
pub enum Error {
    FileLengthIsZero,
    FileLengthTooLarge,
}

fn last_page_bytes(file_len: usize) -> u16 {
    const PAGE_SIZE: usize = 512;
    match file_len % PAGE_SIZE {
        0 => PAGE_SIZE as u16,
        n => n as u16,
    }
}

fn page_count(file_len: usize) -> u16 {
    const PAGE_SIZE: usize = 512;
    file_len.div_ceil(PAGE_SIZE) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_starts_with_mz() {
        let out = header(32).unwrap();
        assert_eq!(&out[..2], b"MZ");
    }

    #[test]
    fn header_claims_no_reloc_table_and_no_overlay() {
        let out = header(32).unwrap();

        assert_eq!(u16::from_le_bytes([out[6], out[7]]), 0, "e_crlc");
        assert_eq!(u16::from_le_bytes([out[24], out[25]]), 0, "e_lfarlc");
        assert_eq!(u16::from_le_bytes([out[26], out[27]]), 0, "e_ovno");
    }

    #[test]
    fn header_paragraph_count_tracks_the_header_length() {
        let out = header(4096).unwrap();

        assert_eq!(
            u16::from_le_bytes([out[8], out[9]]),
            out.len().div_ceil(PARAGRAPH_BYTES) as u16,
            "e_cparhdr"
        );
    }

    #[test]
    fn header_rejects_zero_length() {
        assert!(matches!(header(0), Err(Error::FileLengthIsZero)));
    }

    #[test]
    fn header_rejects_length_that_exceeds_the_page_count_limit() {
        assert!(matches!(
            header(65535 * 512 + 1),
            Err(Error::FileLengthTooLarge)
        ));
    }

    #[test]
    fn header_page_fields_account_for_the_file_length() {
        let cases = [
            (1, 1, 1, "lowest boundary"),
            (32 + 0x1000, 32, 9, "typical value"),
            (65535 * 512, 512, 65535, "highest boundary"),
        ];
        for (file_len, last_page_bytes, page_count, desc) in cases {
            let out = header(file_len).unwrap();
            assert_eq!(
                u16::from_le_bytes([out[2], out[3]]),
                last_page_bytes,
                "e_cblp for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([out[4], out[5]]),
                page_count,
                "e_cp for {desc}"
            );
        }
    }
}

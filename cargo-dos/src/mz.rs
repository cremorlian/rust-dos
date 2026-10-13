pub(crate) const HEADER_BYTES: usize = 32;
const PARAGRAPH_BYTES: usize = 16;

pub fn header(file_len: usize) -> Result<[u8; HEADER_BYTES], Error> {
    if file_len == 0 {
        return Err(Error::FileLengthIsZero);
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

    const STUB_FRAME_PARAGRAPHS: u16 = 4096;
    let reservation = STUB_FRAME_PARAGRAPHS.to_le_bytes();

    const E_MINALLOC: usize = 10;
    out[E_MINALLOC..E_MINALLOC + 2].copy_from_slice(&reservation);

    const E_MAXALLOC: usize = 12;
    out[E_MAXALLOC..E_MAXALLOC + 2].copy_from_slice(&reservation);

    const E_SS: usize = 14;
    let load_module = file_len
        .saturating_sub(HEADER_BYTES)
        .div_ceil(PARAGRAPH_BYTES);

    const MEGABYTE_BYTES: usize = 1024 * 1024;
    const LOAD_MODULE_LIMIT: usize =
        (MEGABYTE_BYTES - STUB_FRAME_PARAGRAPHS as usize * PARAGRAPH_BYTES) / PARAGRAPH_BYTES;
    if load_module > LOAD_MODULE_LIMIT {
        return Err(Error::FileLengthTooLarge);
    }
    out[E_SS..E_SS + 2].copy_from_slice(&(load_module as u16).to_le_bytes());

    const E_SP: usize = 16;
    out[E_SP..E_SP + 2].copy_from_slice(&u16::MAX.to_le_bytes());

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

    const STUB_FRAME_PARAGRAPHS: u16 = 4096;
    const TYPICAL_FILE_LEN: usize = 4112;
    const MEGABYTE_BYTES: usize = 1024 * 1024;
    const LOAD_MODULE_LIMIT: usize =
        (MEGABYTE_BYTES - STUB_FRAME_PARAGRAPHS as usize * PARAGRAPH_BYTES) / PARAGRAPH_BYTES;
    const LARGEST_ACCEPTED_FILE_LEN: usize = LOAD_MODULE_LIMIT * PARAGRAPH_BYTES + HEADER_BYTES;

    #[test]
    fn header_starts_with_mz() {
        let out = header(HEADER_BYTES).unwrap();
        assert_eq!(&out[..2], b"MZ");
    }

    #[test]
    fn header_claims_no_reloc_table_and_no_overlay() {
        let out = header(HEADER_BYTES).unwrap();

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
    fn e_minalloc_reserves_the_stub_frame() {
        let out = header(TYPICAL_FILE_LEN).unwrap();
        assert_eq!(
            u16::from_le_bytes([out[10], out[11]]),
            STUB_FRAME_PARAGRAPHS,
            "e_minalloc"
        );
    }

    #[test]
    fn e_maxalloc_matches_e_minalloc() {
        let out = header(TYPICAL_FILE_LEN).unwrap();
        assert_eq!(
            u16::from_le_bytes([out[12], out[13]]),
            u16::from_le_bytes([out[10], out[11]]),
            "e_maxalloc"
        );
    }

    #[test]
    fn e_sp_names_the_top_of_the_stack() {
        let out = header(TYPICAL_FILE_LEN).unwrap();
        assert_eq!(
            u16::from_le_bytes([out[16], out[17]]),
            u16::MAX,
            "e_sp is the top of the segment"
        );
    }

    #[test]
    fn e_ss_accounts_for_the_file_length() {
        let cases = [
            (1, 0, "shorter than the header, so the load module is empty"),
            (HEADER_BYTES, 0, "exactly the header"),
            (48, 1, "one whole paragraph of load module"),
            (TYPICAL_FILE_LEN, 255, "typical value"),
            (
                LARGEST_ACCEPTED_FILE_LEN,
                LOAD_MODULE_LIMIT as u16,
                "largest accepted, where the frame reaches the megabyte",
            ),
        ];
        for (file_len, e_ss, desc) in cases {
            let out = header(file_len).unwrap();
            assert_eq!(
                u16::from_le_bytes([out[14], out[15]]),
                e_ss,
                "e_ss for {desc}"
            );
        }
    }

    #[test]
    fn e_ss_and_e_sp_span_exactly_the_reserved_block() {
        let out = header(TYPICAL_FILE_LEN).unwrap();
        let load_module = (TYPICAL_FILE_LEN - HEADER_BYTES).div_ceil(PARAGRAPH_BYTES);
        let e_minalloc = u16::from_le_bytes([out[10], out[11]]) as usize;
        let e_ss = u16::from_le_bytes([out[14], out[15]]) as usize;
        let e_sp = u16::from_le_bytes([out[16], out[17]]) as usize;
        let block = load_module * PARAGRAPH_BYTES;

        assert_eq!(
            e_ss * PARAGRAPH_BYTES,
            block,
            "SS:0 is the block's first byte"
        );
        assert_eq!(
            e_ss * PARAGRAPH_BYTES + e_sp,
            block + e_minalloc * PARAGRAPH_BYTES - 1,
            "SS:SP is the block's last byte"
        );
    }

    #[test]
    fn e_ss_reaches_the_megabyte_at_the_largest_accepted_file() {
        let out = header(LARGEST_ACCEPTED_FILE_LEN).unwrap();
        let e_ss = u16::from_le_bytes([out[14], out[15]]) as usize;
        assert_eq!(e_ss, LOAD_MODULE_LIMIT, "load module fills to 1 MB");
    }

    #[test]
    fn header_rejects_zero_length() {
        assert!(matches!(header(0), Err(Error::FileLengthIsZero)));
    }

    #[test]
    fn header_rejects_length_too_large_to_describe() {
        assert!(matches!(
            header(LARGEST_ACCEPTED_FILE_LEN + 1),
            Err(Error::FileLengthTooLarge)
        ));
    }

    #[test]
    fn header_page_fields_account_for_the_file_length() {
        let cases = [
            (1, 1, 1, "lowest boundary"),
            (HEADER_BYTES + 0x1000, 32, 9, "typical value"),
            (
                LARGEST_ACCEPTED_FILE_LEN,
                (LARGEST_ACCEPTED_FILE_LEN % 512) as u16,
                LARGEST_ACCEPTED_FILE_LEN.div_ceil(512) as u16,
                "highest boundary, set by e_ss rather than e_cp",
            ),
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

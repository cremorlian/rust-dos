use crate::strictness::enforce;
use crate::{Error, Strictness};

pub(crate) struct HeaderSpecs {
    pub min_alloc: u16,
    pub max_alloc: u16,
    pub stack_ss: u16,
    pub stack_sp: u16,
    pub entry_ip: u16,
    pub entry_cs: u16,
}

pub(crate) fn build_headers(
    specs: &HeaderSpecs,
    module_len: usize,
    strictness: Strictness,
) -> Result<[u8; 32], Error> {
    if specs.max_alloc < specs.min_alloc {
        enforce(
            strictness,
            Error::MaxAllocLessThanMinAlloc {
                min_alloc: specs.min_alloc,
                max_alloc: specs.max_alloc,
            },
        )?;
    }
    if specs.stack_ss == 0xFFFF {
        enforce(strictness, Error::StackSsWrapsDuringRelocation)?;
    }

    const WORD_WIDTH: usize = 2;
    const PARAGRAPH: usize = 16;
    const PAGE: usize = 512;
    const HEADER_SIZE: usize = 32;

    const OFFSET_MAGIC: usize = 0;
    const OFFSET_CBLP: usize = 2;
    const OFFSET_CP: usize = 4;
    const OFFSET_CRLC: usize = 6;
    const OFFSET_CPARHDR: usize = 8;
    const OFFSET_MIN_ALLOC: usize = 10;
    const OFFSET_MAX_ALLOC: usize = 12;
    const OFFSET_STACK_SS: usize = 14;
    const OFFSET_STACK_SP: usize = 16;
    const OFFSET_CSUM: usize = 18;
    const OFFSET_ENTRY_IP: usize = 20;
    const OFFSET_ENTRY_CS: usize = 22;
    const OFFSET_LFARLC: usize = 24;
    const OFFSET_OVNO: usize = 26;

    const NO_RELOCATIONS: u16 = 0;
    const HEADER_PARAGRAPHS: u16 = (HEADER_SIZE / PARAGRAPH) as u16;
    const NO_CHECKSUM: u16 = 0;
    const NO_RELOCATION_TABLE: u16 = 0;
    const NO_OVERLAY: u16 = 0;

    let total_len = HEADER_SIZE + module_len;
    let last_page_bytes = match total_len % PAGE {
        0 => PAGE as u16,
        n => n as u16,
    };
    let page_count_full = total_len.div_ceil(PAGE);
    if page_count_full > u16::MAX as usize {
        return Err(Error::OutputTooLarge {
            pages: page_count_full as u32,
        });
    }
    let page_count = page_count_full as u16;

    let mut out = [0u8; HEADER_SIZE];

    const MZ_MAGIC: [u8; 2] = *b"MZ";
    out[OFFSET_MAGIC..OFFSET_MAGIC + MZ_MAGIC.len()].copy_from_slice(&MZ_MAGIC);

    let mut write_word = |offset: usize, value: u16| {
        out[offset..offset + WORD_WIDTH].copy_from_slice(&value.to_le_bytes());
    };

    write_word(OFFSET_CBLP, last_page_bytes);
    write_word(OFFSET_CP, page_count);
    write_word(OFFSET_CRLC, NO_RELOCATIONS);
    write_word(OFFSET_CPARHDR, HEADER_PARAGRAPHS);
    write_word(OFFSET_MIN_ALLOC, specs.min_alloc);
    write_word(OFFSET_MAX_ALLOC, specs.max_alloc);
    write_word(OFFSET_STACK_SS, specs.stack_ss);
    write_word(OFFSET_STACK_SP, specs.stack_sp);
    write_word(OFFSET_CSUM, NO_CHECKSUM);
    write_word(OFFSET_ENTRY_IP, specs.entry_ip);
    write_word(OFFSET_ENTRY_CS, specs.entry_cs);
    write_word(OFFSET_LFARLC, NO_RELOCATION_TABLE);
    write_word(OFFSET_OVNO, NO_OVERLAY);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    const TEST_IMAGE_LEN: usize = 0x1000;

    fn base_specs() -> HeaderSpecs {
        HeaderSpecs {
            min_alloc: 0,
            max_alloc: 0xFFFF,
            stack_ss: 0,
            stack_sp: 0,
            entry_ip: 0,
            entry_cs: 0,
        }
    }

    fn with(
        specs: &HeaderSpecs,
        strictness: Strictness,
        f: impl FnOnce(&mut HeaderSpecs),
    ) -> [u8; 32] {
        let mut s = HeaderSpecs {
            min_alloc: specs.min_alloc,
            max_alloc: specs.max_alloc,
            stack_ss: specs.stack_ss,
            stack_sp: specs.stack_sp,
            entry_ip: specs.entry_ip,
            entry_cs: specs.entry_cs,
        };
        f(&mut s);
        build_headers(&s, TEST_IMAGE_LEN, strictness).expect("build should succeed")
    }

    fn try_build(specs: &HeaderSpecs, strictness: Strictness) -> Result<[u8; 32], Error> {
        build_headers(specs, TEST_IMAGE_LEN, strictness)
    }

    #[test]
    fn build_headers_produces_mz_magic() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        assert_eq!(&out[0..2], b"MZ");
    }

    #[test]
    fn build_headers_writes_min_alloc() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0040u16, 0x0040u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (min_alloc, expected, desc) in cases {
            let out = with(&base_specs(), Strictness::Error, |s| {
                s.min_alloc = min_alloc
            });
            let written = u16::from_le_bytes([out[10], out[11]]);
            assert_eq!(
                written, expected,
                "expected min_alloc to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn build_headers_writes_max_alloc() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0040u16, 0x0040u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (max_alloc, expected, desc) in cases {
            let out = with(&base_specs(), Strictness::Error, |s| {
                s.max_alloc = max_alloc
            });
            let written = u16::from_le_bytes([out[12], out[13]]);
            assert_eq!(
                written, expected,
                "expected max_alloc to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn build_headers_writes_entry_ip() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (
                0xffffu16,
                0xffffu16,
                "maximum boundary (sentinel, Allow writes it)",
            ),
        ];
        for (entry_ip, expected, desc) in cases {
            let out = with(&base_specs(), Strictness::Allow, |s| s.entry_ip = entry_ip);
            let written = u16::from_le_bytes([out[20], out[21]]);
            assert_eq!(
                written, expected,
                "expected entry_ip to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn build_headers_writes_entry_cs() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (
                0xffffu16,
                0xffffu16,
                "maximum boundary (sentinel, Allow writes it)",
            ),
        ];
        for (entry_cs, expected, desc) in cases {
            let out = with(&base_specs(), Strictness::Allow, |s| s.entry_cs = entry_cs);
            let written = u16::from_le_bytes([out[22], out[23]]);
            assert_eq!(
                written, expected,
                "expected entry_cs to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn build_headers_writes_stack_ss() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (
                0xffffu16,
                0xffffu16,
                "maximum boundary (sentinel, Allow writes it)",
            ),
        ];
        for (stack_ss, expected, desc) in cases {
            let out = with(&base_specs(), Strictness::Allow, |s| s.stack_ss = stack_ss);
            let written = u16::from_le_bytes([out[14], out[15]]);
            assert_eq!(
                written, expected,
                "expected stack_ss to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn build_headers_writes_stack_sp() {
        let cases = [
            (0x0000u16, 0x0000u16, "minimum boundary"),
            (0x0010u16, 0x0010u16, "typical value"),
            (0xffffu16, 0xffffu16, "maximum boundary"),
        ];
        for (stack_sp, expected, desc) in cases {
            let out = with(&base_specs(), Strictness::Error, |s| s.stack_sp = stack_sp);
            let written = u16::from_le_bytes([out[16], out[17]]);
            assert_eq!(
                written, expected,
                "expected stack_sp to be {expected} for {desc}"
            );
        }
    }

    #[test]
    fn build_headers_writes_crlc_as_zero() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        let written = u16::from_le_bytes([out[6], out[7]]);
        assert_eq!(written, 0, "e_crlc must be zero (no relocations)");
    }

    #[test]
    fn build_headers_writes_csum_as_zero() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        let written = u16::from_le_bytes([out[18], out[19]]);
        assert_eq!(written, 0, "e_csum must be zero (no checksum)");
    }

    #[test]
    fn build_headers_writes_cparhdr_as_two() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        let written = u16::from_le_bytes([out[8], out[9]]);
        assert_eq!(written, 2, "e_cparhdr must be 2 (32-byte padded header)");
    }

    #[test]
    fn build_headers_pads_to_paragraph_boundary() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        assert_eq!(
            out.len(),
            32,
            "28-byte header must be padded to a whole 16-byte paragraph"
        );
    }

    #[test]
    fn build_headers_writes_lfarlc_as_zero() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        let written = u16::from_le_bytes([out[24], out[25]]);
        assert_eq!(written, 0, "e_lfarlc must be zero (no relocs)");
    }

    #[test]
    fn build_headers_writes_ovno_as_zero() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        let written = u16::from_le_bytes([out[26], out[27]]);
        assert_eq!(written, 0, "e_ovno must be zero (no overlay)");
    }

    #[test]
    fn build_headers_writes_cblp_as_full_page_on_exact_multiple() {
        const FULL_PAGE: u16 = 0x0200;
        let out = build_headers(&base_specs(), 480, Strictness::Error).expect("build succeeds");
        let written = u16::from_le_bytes([out[2], out[3]]);
        assert_eq!(
            written, FULL_PAGE,
            "a 512-byte output must report e_cblp = 512"
        );
    }

    #[test]
    fn build_headers_rejects_output_larger_than_u16_pages() {
        const MAX_PAGES: usize = 65535;
        const PAGE: usize = 512;
        const HEADER_SIZE: usize = 32;
        const OVERFLOW_PAGES: u32 = 65536;
        let over_image_len = MAX_PAGES * PAGE - HEADER_SIZE + 1;
        assert!(matches!(
            build_headers(&base_specs(), over_image_len, Strictness::Error),
            Err(Error::OutputTooLarge {
                pages: OVERFLOW_PAGES
            })
        ));
    }

    #[test]
    fn build_headers_writes_cp_at_max_u16_pages() {
        const MAX_PAGES: usize = 65535;
        const PAGE: usize = 512;
        const HEADER_SIZE: usize = 32;
        const MAX_PAGE_COUNT: u16 = 65535;
        let max_image_len = MAX_PAGES * PAGE - HEADER_SIZE;
        let out =
            build_headers(&base_specs(), max_image_len, Strictness::Error).expect("build succeeds");
        assert_eq!(
            u16::from_le_bytes([out[4], out[5]]),
            MAX_PAGE_COUNT,
            "e_cp must still hold 65535 pages at the representable boundary"
        );
    }

    #[test]
    fn build_headers_uses_correct_default_values() {
        let out = build_headers(&base_specs(), TEST_IMAGE_LEN, Strictness::Error)
            .expect("build succeeds");
        assert_eq!(u16::from_le_bytes([out[10], out[11]]), 0, "min_alloc");
        assert_eq!(u16::from_le_bytes([out[12], out[13]]), 0xFFFF, "max_alloc");
        assert_eq!(u16::from_le_bytes([out[20], out[21]]), 0, "entry_ip");
        assert_eq!(u16::from_le_bytes([out[22], out[23]]), 0, "entry_cs");
        assert_eq!(u16::from_le_bytes([out[14], out[15]]), 0, "stack_ss");
        assert_eq!(u16::from_le_bytes([out[16], out[17]]), 0, "stack_sp");
    }

    #[test]
    fn build_headers_writes_all_set_values() {
        let cases = [
            (
                0x0000u16,
                0x0000u16,
                0x0000u16,
                0x0000u16,
                0x0000u16,
                0x0000u16,
                "minimum boundary",
            ),
            (
                0x0040u16,
                0x0100u16,
                0x0010u16,
                0x0020u16,
                0x0000u16,
                0x1000u16,
                "typical values",
            ),
            (
                0xfffeu16,
                0xffffu16,
                0xfffeu16,
                0xfffeu16,
                0xfffeu16,
                0xfffeu16,
                "maximum valid boundary",
            ),
        ];
        for (min_alloc, max_alloc, entry_ip, entry_cs, stack_ss, stack_sp, desc) in cases {
            let specs = HeaderSpecs {
                min_alloc,
                max_alloc,
                stack_ss,
                stack_sp,
                entry_ip,
                entry_cs,
            };
            let out = build_headers(&specs, 0, Strictness::Allow).expect("build succeeds");
            assert_eq!(
                u16::from_le_bytes([out[10], out[11]]),
                min_alloc,
                "min_alloc for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([out[12], out[13]]),
                max_alloc,
                "max_alloc for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([out[20], out[21]]),
                entry_ip,
                "entry_ip for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([out[22], out[23]]),
                entry_cs,
                "entry_cs for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([out[14], out[15]]),
                stack_ss,
                "stack_ss for {desc}"
            );
            assert_eq!(
                u16::from_le_bytes([out[16], out[17]]),
                stack_sp,
                "stack_sp for {desc}"
            );
        }
    }

    #[test]
    fn build_headers_rejects_min_alloc_greater_than_max_alloc() {
        let specs = HeaderSpecs {
            min_alloc: 0x0100,
            max_alloc: 0x00ff,
            ..base_specs()
        };
        let result = try_build(&specs, Strictness::Error);
        assert!(matches!(
            result,
            Err(Error::MaxAllocLessThanMinAlloc {
                min_alloc: 0x0100,
                max_alloc: 0x00ff,
            })
        ));
    }

    #[test]
    fn build_headers_rejects_stack_ss_wraps_during_relocation() {
        let specs = HeaderSpecs {
            stack_ss: 0xffff,
            ..base_specs()
        };
        let result = try_build(&specs, Strictness::Error);
        assert!(matches!(result, Err(Error::StackSsWrapsDuringRelocation)));
    }

    #[test]
    fn build_headers_strictness_allow_skips_validation() {
        let specs = HeaderSpecs {
            min_alloc: 0x0100,
            max_alloc: 0x00ff,
            stack_ss: 0xffff,
            stack_sp: 0,
            entry_ip: 0,
            entry_cs: 0,
        };
        let result = try_build(&specs, Strictness::Allow);
        assert!(result.is_ok());
    }
}

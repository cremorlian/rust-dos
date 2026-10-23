#[derive(Debug, PartialEq)]
pub enum Error {
    TableOffsetTooLarge { image_len: usize },
}

pub fn build(image_len: usize) -> Result<Vec<u8>, Error> {
    let raw = include_bytes!(concat!(env!("OUT_DIR"), "/stub.bin"));
    let table_off = raw
        .len()
        .checked_add(image_len)
        .and_then(|offset| u32::try_from(offset).ok())
        .ok_or(Error::TableOffsetTooLarge { image_len })?;
    let mut out = raw.to_vec();

    const TABLE_OFF_BYTES: usize = 4;
    out[raw.len() - TABLE_OFF_BYTES..].copy_from_slice(&table_off.to_le_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW_STUB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/stub.bin"));

    #[test]
    fn build_writes_the_module_relative_table_offset_into_the_final_stub_word() {
        let max_image_len = u32::MAX as usize - RAW_STUB.len();

        for (image_len, table_off) in [
            (0, RAW_STUB.len()),
            (6, RAW_STUB.len() + 6),
            (max_image_len, u32::MAX as usize),
        ] {
            let built = build(image_len).expect("the table offset should fit");

            assert_eq!(
                &built[RAW_STUB.len() - 4..RAW_STUB.len()],
                &(table_off as u32).to_le_bytes(),
                "table offset for image length {image_len}"
            );
        }
    }

    #[test]
    fn build_rejects_the_first_table_offset_above_u32() {
        let image_len = u32::MAX as usize - RAW_STUB.len() + 1;

        assert_eq!(
            build(image_len),
            Err(Error::TableOffsetTooLarge { image_len })
        );
    }
}

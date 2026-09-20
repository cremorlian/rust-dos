pub fn header() -> [u8; 32] {
    let mut out = [0u8; 32];
    out[..2].copy_from_slice(b"MZ");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_32_bytes_starting_with_mz() {
        let out = header();
        assert_eq!(out.len(), 32);
        assert_eq!(&out[..2], b"MZ");
    }
}

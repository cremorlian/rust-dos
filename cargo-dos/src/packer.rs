pub fn compose(header: [u8; 32], image: &[u8], table: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + image.len() + table.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(image);
    out.extend_from_slice(table);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_outputs_header_then_image_then_fixup_table() {
        let header = [0xAB; 32];
        let image = [1, 2, 3, 4];
        let table = [0xff; 4];

        let out = compose(header, &image, &table);

        assert_eq!(out.len(), 40);
        assert_eq!(&out[..32], &header[..]);
        assert_eq!(&out[32..36], image);
        assert_eq!(&out[36..], table);
    }
}

pub fn compose(header: [u8; 32], stub: &[u8], image: &[u8], table: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(header.len() + stub.len() + image.len() + table.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(stub);
    out.extend_from_slice(image);
    out.extend_from_slice(table);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_outputs_header_then_stub_then_image_then_fixup_table() {
        let header = [0xAB; 32];
        let stub = [0u8; 8];
        let image = [1, 2, 3, 4];
        let table = [0xff; 4];

        let out = compose(header, &stub, &image, &table);

        assert_eq!(out.len(), 48);
        assert_eq!(&out[..32], &header[..]);
        assert_eq!(&out[32..40], &stub[..]);
        assert_eq!(&out[40..44], image);
        assert_eq!(&out[44..], table);
    }
}

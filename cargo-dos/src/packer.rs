pub fn compose(
    header: [u8; crate::mz::HEADER_BYTES],
    stub: &[u8],
    image: &[u8],
    table: &[u8],
) -> Vec<u8> {
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
        let header = [0xAB; crate::mz::HEADER_BYTES];
        let stub = [0u8; 8];
        let image = [1, 2, 3, 4];
        let table = [0xff; 4];

        let out = compose(header, &stub, &image, &table);

        let stub_at = crate::mz::HEADER_BYTES;
        let image_at = stub_at + stub.len();
        let table_at = image_at + image.len();
        assert_eq!(out.len(), table_at + table.len());
        assert_eq!(&out[..stub_at], &header[..]);
        assert_eq!(&out[stub_at..image_at], &stub[..]);
        assert_eq!(&out[image_at..table_at], image);
        assert_eq!(&out[table_at..], table);
    }
}

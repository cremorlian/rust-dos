pub(crate) fn pack(header: &[u8], stub: &[u8], image: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(header.len() + stub.len() + image.len());
    out.extend_from_slice(header);
    out.extend_from_slice(stub);
    out.extend_from_slice(image);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_prepends_header_stub_and_image_in_order() {
        let header = [0x4D, 0x5A];
        let stub = [0xFA, 0xFB];
        let image = [0x90, 0x90, 0x90];
        assert_eq!(
            pack(&header, &stub, &image),
            &[0x4D, 0x5A, 0xFA, 0xFB, 0x90, 0x90, 0x90]
        );
    }

    #[test]
    fn pack_without_stub_is_header_then_image() {
        let header = [0x4D, 0x5A];
        let image = [0x90, 0x90, 0x90];
        assert_eq!(pack(&header, &[], &image), &[0x4D, 0x5A, 0x90, 0x90, 0x90]);
    }
}

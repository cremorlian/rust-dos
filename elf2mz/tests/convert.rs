use elf2mz::{Converter, Error};

const MINIMAL_ELF: &[u8] = include_bytes!("fixtures/minimal.elf");

#[test]
fn convert_minimal_elf_returns_mz_with_image_appended() {
    let out = Converter::new().convert(MINIMAL_ELF).unwrap();
    assert_eq!(out.len(), 35);
    assert_eq!(&out[0..2], b"MZ");
    assert_eq!(u16::from_le_bytes([out[2], out[3]]), 35);
    assert_eq!(u16::from_le_bytes([out[4], out[5]]), 1);
    assert_eq!(&out[32..35], &[0x90, 0x90, 0x90]);
}

#[test]
fn convert_rejects_non_elf() {
    assert!(matches!(
        Converter::new().convert(b"not an elf file"),
        Err(Error::NotAnElfFile)
    ));
}

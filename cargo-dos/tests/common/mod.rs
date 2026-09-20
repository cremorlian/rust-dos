use object::{build, elf, Endianness};

pub fn build_elf(segments: &[Vec<u8>]) -> Vec<u8> {
    let mut builder = build::elf::Builder::new(Endianness::Little, false);
    builder.header.e_phoff = 0x34;

    let shstrtab = builder.sections.add();
    shstrtab.name = b".shstrtab"[..].into();
    shstrtab.sh_type = elf::SHT_STRTAB;
    shstrtab.data = build::elf::SectionData::SectionString;

    let mut file_offset = 0x34 + segments.len() as u64 * 32;
    for seg in segments.iter() {
        let segment = builder.segments.add();
        segment.p_type = elf::PT_LOAD;
        segment.p_offset = file_offset;
        segment.p_filesz = seg.len() as u64;

        let section = builder.sections.add();
        section.sh_addralign = 1;
        section.data = build::elf::SectionData::Data(seg[..].into());

        segment.sections.push(section.id());
        file_offset += seg.len() as u64;
    }

    let mut buf = Vec::new();
    builder.write(&mut buf).expect("write elf");
    buf
}
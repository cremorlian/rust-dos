use object::{Endianness, build, elf};

pub fn build_elf(
    segments: &[(Vec<u8>, u64)],
    relocs: &[(usize, &[(u64, elf::RelocationType)])],
) -> Vec<u8> {
    let mut builder = build::elf::Builder::new(Endianness::Little, false);
    builder.header.e_phoff = 0x34;

    let shstrtab = builder.sections.add();
    shstrtab.name = b".shstrtab"[..].into();
    shstrtab.sh_type = elf::SHT_STRTAB;
    shstrtab.data = build::elf::SectionData::SectionString;

    let mut file_offset = 0x34 + segments.len() as u64 * 32;
    let mut data_sections = Vec::new();
    for (data, vaddr) in segments.iter() {
        let segment = builder.segments.add();
        segment.p_type = elf::PT_LOAD;
        segment.p_offset = file_offset;
        segment.p_vaddr = *vaddr;
        segment.p_filesz = data.len() as u64;

        let section = builder.sections.add();
        section.sh_addralign = 1;
        section.data = build::elf::SectionData::Data(data[..].into());

        segment.sections.push(section.id());
        data_sections.push(section.id());
        file_offset += data.len() as u64;
    }

    for (section_idx, section_relocs) in relocs.iter() {
        let reloc = builder.sections.add();
        reloc.sh_type = elf::SHT_REL;
        reloc.sh_info_section = Some(data_sections[*section_idx]);
        reloc.data = build::elf::SectionData::Relocation(
            section_relocs
                .iter()
                .map(|&(r_offset, r_type)| build::elf::Relocation {
                    r_offset,
                    symbol: None,
                    r_type,
                    r_addend: 0,
                })
                .collect(),
        );
    }

    let mut buf = Vec::new();
    builder.write(&mut buf).expect("write elf");
    buf
}

use crate::strictness::enforce;
use crate::{Error, Strictness};

pub(crate) enum LayoutConfig {
    ImageOnly,
    Shell { shell: Vec<u8> },
}

pub(crate) struct ImageOnlyLayout {
    pub entry_cs: u16,
    pub entry_ip: u16,
    pub image_len: usize,
}

pub(crate) struct ShellImageLayout<'a> {
    pub stub: &'a [u8],
    pub image_len: usize,
}

pub(crate) enum Layout<'a> {
    ImageOnly(ImageOnlyLayout),
    ShellImage(ShellImageLayout<'a>),
}

impl Layout<'_> {
    pub fn entry(&self) -> (u16, u16) {
        match self {
            Layout::ImageOnly(layout) => (layout.entry_cs, layout.entry_ip),
            Layout::ShellImage(_) => (0, 0),
        }
    }

    pub fn module_len(&self) -> usize {
        match self {
            Layout::ImageOnly(layout) => layout.image_len,
            Layout::ShellImage(layout) => layout.stub.len() + layout.image_len,
        }
    }

    pub fn stub(&self) -> &[u8] {
        match self {
            Layout::ImageOnly(_) => &[],
            Layout::ShellImage(layout) => layout.stub,
        }
    }
}

pub(crate) fn shell(shell: &[u8]) -> Result<LayoutConfig, Error> {
    if shell.is_empty() {
        return Err(Error::EmptyStub);
    }
    Ok(LayoutConfig::Shell {
        shell: shell.to_vec(),
    })
}

pub(crate) fn resolve(
    config: &LayoutConfig,
    entry_cs: u16,
    entry_ip: u16,
    image_len: usize,
    strictness: Strictness,
) -> Result<Layout<'_>, Error> {
    match config {
        LayoutConfig::ImageOnly => {
            if entry_cs as u32 * 16 + entry_ip as u32 >= image_len as u32 {
                enforce(
                    strictness,
                    Error::EntryOutsideImage {
                        entry_cs,
                        entry_ip,
                        module_len: image_len,
                    },
                )?;
            }
            Ok(Layout::ImageOnly(ImageOnlyLayout {
                entry_cs,
                entry_ip,
                image_len,
            }))
        }
        LayoutConfig::Shell { shell } => {
            if entry_cs != 0 || entry_ip != 0 {
                enforce(strictness, Error::EntryOwnedByShell { entry_cs, entry_ip })?;
            }
            Ok(Layout::ShellImage(ShellImageLayout {
                stub: shell,
                image_len,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: &[u8] = &[0x90, 0x90, 0x90];
    const IMAGE_LEN: usize = 0x1000;

    #[test]
    fn resolve_rejects_entry_outside_image() {
        let config = LayoutConfig::ImageOnly;
        assert!(matches!(
            resolve(&config, 0x0100, 0x0001, IMAGE_LEN, Strictness::Error),
            Err(Error::EntryOutsideImage {
                entry_cs: 0x0100,
                entry_ip: 0x0001,
                module_len: IMAGE_LEN,
            })
        ));
    }

    #[test]
    fn resolve_rejects_entry_at_image_end() {
        let config = LayoutConfig::ImageOnly;
        assert!(matches!(
            resolve(&config, 0x0100, 0x0000, IMAGE_LEN, Strictness::Error),
            Err(Error::EntryOutsideImage { .. })
        ));
    }

    #[test]
    fn resolve_rejects_entry_ip_far_outside_module() {
        let config = LayoutConfig::ImageOnly;
        assert!(matches!(
            resolve(&config, 0, 0xFFFF, IMAGE_LEN, Strictness::Error),
            Err(Error::EntryOutsideImage { .. })
        ));
    }

    #[test]
    fn resolve_rejects_entry_cs_far_outside_module() {
        let config = LayoutConfig::ImageOnly;
        assert!(matches!(
            resolve(&config, 0xFFFF, 0, IMAGE_LEN, Strictness::Error),
            Err(Error::EntryOutsideImage { .. })
        ));
    }

    #[test]
    fn resolve_accepts_entry_at_last_valid_offset() {
        let config = LayoutConfig::ImageOnly;
        let layout =
            resolve(&config, 0x00FF, 0x000F, IMAGE_LEN, Strictness::Error).expect("resolve");
        assert_eq!(layout.entry(), (0x00FF, 0x000F));
    }

    #[test]
    fn resolve_under_allow_skips_module_bound() {
        let config = LayoutConfig::ImageOnly;
        assert_eq!(
            resolve(&config, 0xFFFF, 0xFFFF, IMAGE_LEN, Strictness::Allow)
                .expect("resolve")
                .entry(),
            (0xFFFF, 0xFFFF)
        );
    }

    #[test]
    fn resolve_image_only_echoes_caller_entry() {
        let config = LayoutConfig::ImageOnly;
        let layout =
            resolve(&config, 0, 2, IMAGE.len(), Strictness::Error).expect("image-only resolve");
        assert_eq!(layout.entry(), (0, 2));
        assert_eq!(layout.module_len(), IMAGE.len());
        assert_eq!(layout.stub(), &[] as &[u8]);
    }

    #[test]
    fn resolve_shell_image_module_len_includes_stub() {
        let config = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        let layout = resolve(&config, 0, 0, IMAGE.len(), Strictness::Error).expect("shell resolve");
        assert_eq!(layout.module_len(), 2 + IMAGE.len());
    }

    #[test]
    fn resolve_shell_image_under_error_rejects_conflicting_caller_entry() {
        let config = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        assert!(matches!(
            resolve(&config, 0x0100, 0x0001, IMAGE.len(), Strictness::Error),
            Err(Error::EntryOwnedByShell {
                entry_cs: 0x0100,
                entry_ip: 0x0001,
            })
        ));
    }

    #[test]
    fn resolve_shell_image_under_allow_forces_zero_entry() {
        let config = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        let layout = resolve(&config, 0x0100, 0x0001, IMAGE.len(), Strictness::Allow)
            .expect("shell resolve");
        assert_eq!(layout.entry(), (0, 0));
    }

    #[test]
    fn shell_rejects_empty_stub() {
        assert!(matches!(shell(&[]), Err(Error::EmptyStub)));
    }

    #[test]
    fn shell_construction_of_non_empty_stub_returns_a_shell_config() {
        let config = shell(&[0xFA, 0xFB]).expect("non-empty shell");
        assert!(matches!(
            config,
            LayoutConfig::Shell { shell } if shell == [0xFA, 0xFB]
        ));
    }

    #[test]
    fn resolve_shell_image_stub_returns_shell_bytes() {
        let config = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        let layout = resolve(&config, 0, 0, IMAGE.len(), Strictness::Error).expect("shell resolve");
        assert_eq!(layout.stub(), &[0xFA, 0xFB]);
    }
}

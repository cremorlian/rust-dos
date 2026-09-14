use crate::strictness::enforce;
use crate::{Error, Strictness};

const MAX_EXPRESSIBLE_BLOCK_END: u32 = 0xFFFEF;

pub(crate) enum LayoutConfig {
    ImageOnly,
    Shell { shell: Vec<u8> },
}

#[derive(Clone, Copy)]
pub(crate) enum StackSpec {
    Default,
    Set { ss: u16, sp: u16 },
}

pub(crate) struct LayoutSpec<'a> {
    pub layout: &'a LayoutConfig,
    pub entry_cs: u16,
    pub entry_ip: u16,
    pub image_len: usize,
    pub min_alloc: u16,
    pub strictness: Strictness,
    pub stack: StackSpec,
}

pub(crate) struct Resolution {
    pub entry: (u16, u16),
    pub stack: (u16, u16),
    pub module_len: usize,
}

pub(crate) fn resolve<'a>(spec: &LayoutSpec<'a>) -> Result<Resolution, Error> {
    match spec.layout {
        LayoutConfig::ImageOnly => {
            if spec.entry_cs as u32 * 16 + spec.entry_ip as u32 >= spec.image_len as u32 {
                enforce(
                    spec.strictness,
                    Error::EntryOutsideImage {
                        entry_cs: spec.entry_cs,
                        entry_ip: spec.entry_ip,
                        module_len: spec.image_len,
                    },
                )?;
            }
            Ok(Resolution {
                entry: (spec.entry_cs, spec.entry_ip),
                stack: resolve_stack(spec, spec.image_len)?,
                module_len: spec.image_len,
            })
        }
        LayoutConfig::Shell { shell } => {
            if shell.is_empty() {
                return Err(Error::EmptyStub);
            }
            if spec.entry_cs != 0 || spec.entry_ip != 0 {
                enforce(
                    spec.strictness,
                    Error::EntryOwnedByShell {
                        entry_cs: spec.entry_cs,
                        entry_ip: spec.entry_ip,
                    },
                )?;
            }
            let module_len = shell.len() + spec.image_len;
            Ok(Resolution {
                entry: (0, 0),
                stack: resolve_stack(spec, module_len)?,
                module_len,
            })
        }
    }
}

fn resolve_stack(spec: &LayoutSpec<'_>, module_len: usize) -> Result<(u16, u16), Error> {
    match spec.stack {
        StackSpec::Set { ss, sp } => Ok((ss, sp)),
        StackSpec::Default => match spec.layout {
            LayoutConfig::ImageOnly => Ok((0, 0)),
            LayoutConfig::Shell { .. } => {
                let block_end = module_len as u32 + spec.min_alloc as u32 * 16;
                if block_end > MAX_EXPRESSIBLE_BLOCK_END {
                    return Err(Error::StackBlockEndUnrepresentable { block_end });
                }
                Ok(((block_end >> 4) as u16, (block_end & 0xF) as u16))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: &[u8] = &[0x90, 0x90, 0x90];
    const IMAGE_LEN: usize = 0x1000;

    fn spec(
        layout: &LayoutConfig,
        entry_cs: u16,
        entry_ip: u16,
        image_len: usize,
    ) -> LayoutSpec<'_> {
        LayoutSpec {
            layout,
            entry_cs,
            entry_ip,
            image_len,
            min_alloc: 0,
            strictness: Strictness::Error,
            stack: StackSpec::Default,
        }
    }

    #[test]
    fn resolve_rejects_entry_outside_image() {
        let cfg = LayoutConfig::ImageOnly;
        let s = spec(&cfg, 0x0100, 0x0001, IMAGE_LEN);
        assert!(matches!(
            resolve(&s),
            Err(Error::EntryOutsideImage {
                entry_cs: 0x0100,
                entry_ip: 0x0001,
                module_len: IMAGE_LEN,
            })
        ));
    }

    #[test]
    fn resolve_rejects_entry_at_image_end() {
        let cfg = LayoutConfig::ImageOnly;
        let s = spec(&cfg, 0x0100, 0x0000, IMAGE_LEN);
        assert!(matches!(resolve(&s), Err(Error::EntryOutsideImage { .. })));
    }

    #[test]
    fn resolve_rejects_entry_ip_far_outside_module() {
        let cfg = LayoutConfig::ImageOnly;
        let s = spec(&cfg, 0, 0xFFFF, IMAGE_LEN);
        assert!(matches!(resolve(&s), Err(Error::EntryOutsideImage { .. })));
    }

    #[test]
    fn resolve_rejects_entry_cs_far_outside_module() {
        let cfg = LayoutConfig::ImageOnly;
        let s = spec(&cfg, 0xFFFF, 0, IMAGE_LEN);
        assert!(matches!(resolve(&s), Err(Error::EntryOutsideImage { .. })));
    }

    #[test]
    fn resolve_accepts_entry_at_last_valid_offset() {
        let cfg = LayoutConfig::ImageOnly;
        let s = spec(&cfg, 0x00FF, 0x000F, IMAGE_LEN);
        assert!(matches!(
            resolve(&s),
            Ok(Resolution { entry, .. }) if entry == (0x00FF, 0x000F)
        ));
    }

    #[test]
    fn resolve_under_allow_skips_module_bound() {
        let cfg = LayoutConfig::ImageOnly;
        let s = LayoutSpec {
            strictness: Strictness::Allow,
            ..spec(&cfg, 0xFFFF, 0xFFFF, IMAGE_LEN)
        };
        assert!(matches!(
            resolve(&s),
            Ok(Resolution { entry, .. }) if entry == (0xFFFF, 0xFFFF)
        ));
    }

    #[test]
    fn resolve_image_only_echoes_caller_entry() {
        let cfg = LayoutConfig::ImageOnly;
        let s = spec(&cfg, 0, 2, IMAGE.len());
        let r = resolve(&s).expect("image-only resolve");
        assert_eq!(r.entry, (0, 2));
        assert_eq!(r.module_len, IMAGE.len());
    }

    #[test]
    fn resolve_shell_image_module_len_includes_stub() {
        let cfg = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        let s = spec(&cfg, 0, 0, IMAGE.len());
        let r = resolve(&s).expect("shell resolve");
        assert_eq!(r.module_len, 2 + IMAGE.len());
    }

    #[test]
    fn resolve_shell_image_under_error_rejects_conflicting_caller_entry() {
        let cfg = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        let s = spec(&cfg, 0x0100, 0x0001, IMAGE.len());
        assert!(matches!(
            resolve(&s),
            Err(Error::EntryOwnedByShell {
                entry_cs: 0x0100,
                entry_ip: 0x0001,
            })
        ));
    }

    #[test]
    fn resolve_shell_image_under_allow_forces_zero_entry() {
        let cfg = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        let s = LayoutSpec {
            strictness: Strictness::Allow,
            ..spec(&cfg, 0x0100, 0x0001, IMAGE.len())
        };
        let r = resolve(&s).expect("shell resolve");
        assert_eq!(r.entry, (0, 0));
    }

    #[test]
    fn resolve_shell_rejects_empty_stub() {
        let cfg = LayoutConfig::Shell { shell: vec![] };
        let s = spec(&cfg, 0, 0, IMAGE_LEN);
        assert!(matches!(resolve(&s), Err(Error::EmptyStub)));
    }

    #[test]
    fn resolve_stack_shell_mode_default_is_block_end() {
        let cfg = LayoutConfig::Shell {
            shell: b"\xFA\xFB".to_vec(),
        };
        let s = spec(&cfg, 0, 0, IMAGE_LEN);
        assert!(matches!(
            resolve(&s),
            Ok(Resolution { stack, .. }) if stack == (0x0100, 0x0002)
        ));
    }

    #[test]
    fn resolve_stack_image_only_default_is_zero_pair() {
        let cfg = LayoutConfig::ImageOnly;
        let s = spec(&cfg, 0, 0, IMAGE_LEN);
        assert!(matches!(
            resolve(&s),
            Ok(Resolution { stack, .. }) if stack == (0, 0)
        ));
    }

    #[test]
    fn resolve_stack_caller_set_pair_wins() {
        let cfg = LayoutConfig::ImageOnly;
        let s = LayoutSpec {
            stack: StackSpec::Set {
                ss: 0x1234,
                sp: 0x5678,
            },
            ..spec(&cfg, 0, 0, IMAGE_LEN)
        };
        assert!(matches!(
            resolve(&s),
            Ok(Resolution { stack, .. }) if stack == (0x1234, 0x5678)
        ));
    }

    #[test]
    fn resolve_stack_block_end_over_limit_is_unconditional_error() {
        let cfg = LayoutConfig::Shell {
            shell: vec![0x00; 0xF],
        };
        let s = LayoutSpec {
            min_alloc: 0xFFFF,
            ..spec(&cfg, 0, 0, IMAGE_LEN)
        };
        assert!(matches!(
            resolve(&s),
            Err(Error::StackBlockEndUnrepresentable { block_end })
                if block_end == 0x100FFF
        ));
    }

    #[test]
    fn resolve_stack_max_expressible_block_end_is_split() {
        let cfg = LayoutConfig::Shell {
            shell: vec![0x00; 0xF],
        };
        let s = LayoutSpec {
            min_alloc: 0xFEFE,
            ..spec(&cfg, 0, 0, IMAGE_LEN)
        };
        assert!(matches!(
            resolve(&s),
            Ok(Resolution { stack, .. }) if stack == (0xFFFE, 0xF)
        ));
    }
}

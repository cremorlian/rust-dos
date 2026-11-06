use crate::raw::{HostOutcome, PreparedCall, Rmcs, copy_linear, execute};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum Error {
    Dpmi { code: u16 },
    InvalidPsp { segment: u16 },
    InvalidCommandLineLength { length: u8 },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct Psp(u16);

impl Psp {
    pub(crate) fn new(segment: u16) -> Result<Psp, Error> {
        if segment == 0 || (segment as u32) * 16 + 0x100 > 0x10_0000 {
            return Err(Error::InvalidPsp { segment });
        }
        Ok(Psp(segment))
    }
}

pub(crate) fn read_command_line(area: &mut [u8; 128]) -> Result<&[u8], Error> {
    let psp = get()?;
    let linear = (psp.0 as u32) * 16 + 0x80;
    copy_linear(linear, area);
    command_line_tail(area)
}

pub(crate) fn command_line_tail(area: &[u8; 128]) -> Result<&[u8], Error> {
    let length = area[0];
    if length > 127 {
        return Err(Error::InvalidCommandLineLength { length });
    }
    Ok(&area[1..1 + length as usize])
}

pub(crate) fn prepare_get_psp() -> PreparedCall {
    const AH_GET_PSP: u32 = 0x6200;
    let mut rmcs = Rmcs::zeroed();
    rmcs.eax = AH_GET_PSP;
    const INT_DOS: u8 = 0x21;
    PreparedCall {
        rmcs,
        int_no: INT_DOS,
    }
}

fn get() -> Result<Psp, Error> {
    let mut call = prepare_get_psp();
    let outcome = execute(&mut call);
    surface_psp(&call, outcome)
}

pub(crate) fn surface_psp(call: &PreparedCall, o: HostOutcome) -> Result<Psp, Error> {
    if o.host_cf {
        return Err(Error::Dpmi { code: o.host_ax });
    }
    Psp::new(call.rmcs.ebx as u16)
}

#[cfg(test)]
mod tests {
    use super::{Error, PreparedCall, Psp, Rmcs, command_line_tail, prepare_get_psp, surface_psp};
    use crate::raw::HostOutcome;

    fn call(rmcs: Rmcs) -> PreparedCall {
        PreparedCall { rmcs, int_no: 0x21 }
    }

    fn host_ok() -> HostOutcome {
        HostOutcome {
            host_cf: false,
            host_ax: 0,
        }
    }

    fn rmcs(eax: u32, ebx: u32, flags: u16) -> Rmcs {
        let mut rmcs = Rmcs::zeroed();
        rmcs.eax = eax;
        rmcs.ebx = ebx;
        rmcs.flags = flags;
        rmcs
    }

    fn area(tail: &[u8]) -> [u8; 128] {
        let mut area = [0u8; 128];
        area[0] = tail.len() as u8;
        area[1..1 + tail.len()].copy_from_slice(tail);
        area
    }

    #[test]
    fn command_line_tail_validates_the_tail_length() {
        assert_eq!(command_line_tail(&area(b"")).unwrap(), b"");
        assert_eq!(
            command_line_tail(&area(&[b'a'; 127])).unwrap(),
            &[b'a'; 127][..]
        );
        assert_eq!(
            command_line_tail(&area(b" alpha \x82 beta")).unwrap(),
            b" alpha \x82 beta"
        );

        let mut too_long = [0u8; 128];
        too_long[0] = 128;
        assert!(matches!(
            command_line_tail(&too_long),
            Err(Error::InvalidCommandLineLength { length: 128 })
        ));
    }

    #[test]
    fn psp_validates_segment_boundaries() {
        let cases = [
            (0x0000, Err(Error::InvalidPsp { segment: 0x0000 })),
            (0x0001, Ok(Psp(0x0001))),
            (0x1234, Ok(Psp(0x1234))),
            (0xFFF0, Ok(Psp(0xFFF0))),
            (0xFFF1, Err(Error::InvalidPsp { segment: 0xFFF1 })),
            (0xFFFF, Err(Error::InvalidPsp { segment: 0xFFFF })),
        ];

        for (segment, expected) in cases {
            assert_eq!(Psp::new(segment), expected, "segment={segment:#06X}");
        }
    }

    #[test]
    fn prepare_get_psp_targets_the_dos_get_psp_function() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.eax = 0x6200;
        let expected = PreparedCall { rmcs, int_no: 0x21 };

        assert_eq!(prepare_get_psp(), expected);
    }

    #[test]
    fn surface_psp_covers_host_and_segment_cases() {
        let cases = [
            (rmcs(0, 0x1234, 0), host_ok(), Ok(Psp(0x1234))),
            (rmcs(0, 0x0001, 0), host_ok(), Ok(Psp(0x0001))),
            (rmcs(0, 0xFFF0, 0), host_ok(), Ok(Psp(0xFFF0))),
            (rmcs(0, 0x0001_2345, 0), host_ok(), Ok(Psp(0x2345))),
            (
                rmcs(0, 0, 0),
                host_ok(),
                Err(Error::InvalidPsp { segment: 0 }),
            ),
            (
                rmcs(0, 0xFFF1, 0),
                host_ok(),
                Err(Error::InvalidPsp { segment: 0xFFF1 }),
            ),
            (
                rmcs(0, 0xFFFF, 0),
                host_ok(),
                Err(Error::InvalidPsp { segment: 0xFFFF }),
            ),
            (
                rmcs(0, 0x1234, 0),
                HostOutcome {
                    host_cf: true,
                    host_ax: 0,
                },
                Err(Error::Dpmi { code: 0 }),
            ),
            (
                rmcs(6, 0x1234, 1),
                HostOutcome {
                    host_cf: true,
                    host_ax: 0x8021,
                },
                Err(Error::Dpmi { code: 0x8021 }),
            ),
            (
                rmcs(0, 0x1234, 0),
                HostOutcome {
                    host_cf: true,
                    host_ax: 0xFFFF,
                },
                Err(Error::Dpmi { code: 0xFFFF }),
            ),
            (rmcs(0, 0x1234, 1), host_ok(), Ok(Psp(0x1234))),
            (rmcs(0xFFFF, 0x1234, 1), host_ok(), Ok(Psp(0x1234))),
            (rmcs(0x0001_0006, 0x1234, 1), host_ok(), Ok(Psp(0x1234))),
            (rmcs(6, 0x1234, 0x8001), host_ok(), Ok(Psp(0x1234))),
            (rmcs(0, 0x1234, 0x0002), host_ok(), Ok(Psp(0x1234))),
            (
                rmcs(0, 0x1234, 0),
                HostOutcome {
                    host_cf: false,
                    host_ax: 0xFFFF,
                },
                Ok(Psp(0x1234)),
            ),
            (rmcs(0xFFFF, 0x1234, 0), host_ok(), Ok(Psp(0x1234))),
        ];

        for (rmcs, outcome, expected) in cases {
            assert_eq!(
                surface_psp(&call(rmcs), outcome),
                expected,
                "rmcs={rmcs:?} outcome={outcome:?}"
            );
        }
    }
}

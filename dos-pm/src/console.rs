use embedded_io as eio;

use crate::raw::{HostOutcome, PreparedCall, Rmcs, execute};

const MAX_RM_TRANSFER: usize = 0xFFF0;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    WriteZero,
    Dos { code: u16 },
    Dpmi { code: u16 },
    BufferUnreachable,
}

fn is_reachable(lin: u32, len: usize) -> bool {
    len == 0 || lin as u64 + len as u64 <= 0x1_00000
}

pub(crate) fn prepare_write(handle: u16, lin: u32, len: usize) -> Result<PreparedCall, Error> {
    if !is_reachable(lin, len) {
        return Err(Error::BufferUnreachable);
    }
    const AH_WRITE: u32 = 0x4000;
    let mut rmcs = Rmcs::zeroed();
    rmcs.eax = AH_WRITE;
    rmcs.ebx = handle as u32;
    rmcs.ecx = len.min(MAX_RM_TRANSFER) as u32;
    rmcs.edx = lin & 0xF;
    rmcs.ds = (lin >> 4) as u16;
    const INT_DOS: u8 = 0x21;
    Ok(PreparedCall {
        rmcs,
        int_no: INT_DOS,
    })
}

pub(crate) fn prepare_read(handle: u16, lin: u32, len: usize) -> Result<PreparedCall, Error> {
    if !is_reachable(lin, len) {
        return Err(Error::BufferUnreachable);
    }
    const AH_READ: u32 = 0x3F00;
    let mut rmcs = Rmcs::zeroed();
    rmcs.eax = AH_READ;
    rmcs.ebx = handle as u32;
    rmcs.ecx = len.min(MAX_RM_TRANSFER) as u32;
    rmcs.edx = lin & 0xF;
    rmcs.ds = (lin >> 4) as u16;
    const INT_DOS: u8 = 0x21;
    Ok(PreparedCall {
        rmcs,
        int_no: INT_DOS,
    })
}

pub(crate) fn surface_read(call: &PreparedCall, o: HostOutcome) -> Result<usize, Error> {
    if o.host_cf {
        return Err(Error::Dpmi { code: o.host_ax });
    }
    if call.rmcs.cf & 1 != 0 {
        return Err(Error::Dos {
            code: call.rmcs.eax as u16,
        });
    }
    Ok(call.rmcs.eax as usize)
}

pub(crate) fn surface_write(call: &PreparedCall, o: HostOutcome) -> Result<usize, Error> {
    if o.host_cf {
        return Err(Error::Dpmi { code: o.host_ax });
    }
    if call.rmcs.cf & 1 != 0 {
        return Err(Error::Dos {
            code: call.rmcs.eax as u16,
        });
    }
    let n = call.rmcs.eax as usize;
    if n == 0 && call.rmcs.ecx != 0 {
        return Err(Error::WriteZero);
    }
    Ok(n)
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::WriteZero => f.write_str("DOS write returned zero bytes for a non-empty buffer"),
            Error::Dos { code } => write!(f, "DOS call failed with error 0x{code:04X}"),
            Error::Dpmi { code } => write!(f, "DPMI call failed with error 0x{code:04X}"),
            Error::BufferUnreachable => f.write_str(
                "buffer extends past the 1 MiB real-mode limit; the DOS call would access wrapped memory",
            ),
        }
    }
}

impl core::error::Error for Error {}

impl eio::Error for Error {
    fn kind(&self) -> eio::ErrorKind {
        match self {
            Error::WriteZero => eio::ErrorKind::WriteZero,
            Error::Dos { .. } | Error::Dpmi { .. } => eio::ErrorKind::Other,
            Error::BufferUnreachable => eio::ErrorKind::InvalidInput,
        }
    }
}

/// Zero-sized handle to the DOS console standard output, used by value.
pub struct Stdout;

impl eio::ErrorType for Stdout {
    type Error = Error;
}

/// Writes `buf` in one DOS `AH=40h` call, capped at 64 KiB.
///
/// A buffer extending past the 1 MiB real-mode limit returns
/// `Error::BufferUnreachable` without writing.
impl eio::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let lin = buf.as_ptr() as usize as u32;
        let mut call = prepare_write(1, lin, buf.len())?;
        let o = execute(&mut call);
        surface_write(&call, o)
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// Zero-sized handle to the DOS console standard input, used by value.
pub struct Stdin;

impl eio::ErrorType for Stdin {
    type Error = Error;
}

/// Reads into `buf` in one DOS `AH=3Fh` call, capped at 64 KiB.
///
/// `Ok(0)` is end of file. A buffer extending past the 1 MiB real-mode
/// limit returns `Error::BufferUnreachable` without reading.
impl eio::Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let lin = buf.as_mut_ptr() as usize as u32;
        let mut call = prepare_read(0, lin, buf.len())?;
        let o = execute(&mut call);
        surface_read(&call, o)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, prepare_read, prepare_write, surface_read, surface_write};
    use crate::raw::{HostOutcome, PreparedCall, Rmcs};

    fn call(rmcs: Rmcs) -> PreparedCall {
        PreparedCall { rmcs, int_no: 0x21 }
    }

    #[test]
    fn read_caps_a_chunk_at_the_abi_window() {
        for (len, expected) in [(0x0_FFF1, 0xFFF0), (0x10_0000, 0xFFF0)] {
            let cmd = prepare_read(0, 0, len).expect("reachable");
            assert_eq!(cmd.rmcs.ecx, expected, "len={len:#x}");
        }
    }

    #[test]
    fn read_targets_the_caller_buffer() {
        let cmd = prepare_read(0, 0x12345, 2).expect("reachable");

        let mut rmcs = Rmcs::zeroed();
        rmcs.eax = 0x3F00;
        rmcs.ebx = 0;
        rmcs.ecx = 2;
        rmcs.edx = 0x5;
        rmcs.ds = 0x1234;
        let expected = PreparedCall { rmcs, int_no: 0x21 };

        assert_eq!(cmd, expected);
    }

    #[test]
    fn surface_read_maps_dpmi_error() {
        let host_fail = HostOutcome {
            host_cf: true,
            host_ax: 0x8021,
        };
        assert_eq!(
            surface_read(&call(Rmcs::zeroed()), host_fail),
            Err(Error::Dpmi { code: 0x8021 })
        );
    }

    #[test]
    fn surface_read_prefers_dpmi_error() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.ecx = 3;
        rmcs.eax = 6;
        rmcs.cf = 1;
        let host_fail = HostOutcome {
            host_cf: true,
            host_ax: 0x8021,
        };
        assert_eq!(
            surface_read(&call(rmcs), host_fail),
            Err(Error::Dpmi { code: 0x8021 })
        );
    }

    #[test]
    fn surface_read_allows_zero_len() {
        let rmcs = Rmcs::zeroed();
        assert_eq!(surface_read(&call(rmcs), host_ok()), Ok(0));
    }

    #[test]
    fn surface_read_returns_bytes_read() {
        for (eax, expected) in [(0x0000, 0), (0x0003, 3), (0xFFFF, 0xFFFF as usize)] {
            let mut rmcs = Rmcs::zeroed();
            rmcs.ecx = 3;
            rmcs.eax = eax;
            assert_eq!(
                surface_read(&call(rmcs), host_ok()),
                Ok(expected),
                "eax={eax:#x}"
            );
        }
    }

    #[test]
    fn surface_read_maps_dos_error() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.ecx = 3;
        rmcs.eax = 6;
        rmcs.cf = 1;
        assert_eq!(
            surface_read(&call(rmcs), host_ok()),
            Err(Error::Dos { code: 6 })
        );
    }

    #[test]
    fn surface_read_maps_zero_bytes_to_eof() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.ecx = 3;
        assert_eq!(surface_read(&call(rmcs), host_ok()), Ok(0));
    }

    #[test]
    fn write_targets_the_caller_buffer() {
        let cmd = prepare_write(1, 0x12345, 2).expect("reachable");

        let mut rmcs = Rmcs::zeroed();
        rmcs.eax = 0x4000;
        rmcs.ebx = 1;
        rmcs.ecx = 2;
        rmcs.edx = 0x5;
        rmcs.ds = 0x1234;
        let expected = PreparedCall { rmcs, int_no: 0x21 };

        assert_eq!(cmd, expected);
    }

    #[test]
    fn write_caps_a_chunk_at_the_abi_window() {
        for (len, expected) in [(0x0_FFF1, 0xFFF0), (0x10_0000, 0xFFF0)] {
            let cmd = prepare_write(1, 0, len).expect("reachable");
            assert_eq!(cmd.rmcs.ecx, expected, "len={len:#x}");
        }
    }

    #[test]
    fn read_errors_when_unreachable() {
        for (lin, len) in [
            (0x0_FFFFF, 0x2),
            (0x0_FF000, 0x1001),
            (0x1_00000, 0x1),
            (0xFFFF_FFFF, 0x1),
            (0x0_0000, usize::MAX),
        ] {
            assert_eq!(
                prepare_read(0, lin, len),
                Err(Error::BufferUnreachable),
                "lin={lin:#x} len={len:#x}"
            );
        }
    }

    #[test]
    fn can_read_buffers_from_stdin() {
        for (lin, len) in [
            (0x0_0000, 0x0),
            (0x0_0000, 0x1_00000),
            (0x0_FFFFF, 0x1),
            (0x0_FF000, 0x1000),
            (0xFFFF_FFFF, 0),
        ] {
            assert!(
                prepare_read(0, lin, len).is_ok(),
                "lin={lin:#x} len={len:#x}"
            );
        }
    }

    #[test]
    fn can_write_buffers_to_stdout() {
        for (lin, len) in [
            (0x0_0000, 0x0),
            (0x0_0000, 0x1_00000),
            (0x0_FFFFF, 0x1),
            (0x0_FF000, 0x1000),
            (0xFFFF_FFFF, 0),
        ] {
            assert!(
                prepare_write(1, lin, len).is_ok(),
                "lin={lin:#x} len={len:#x}"
            );
        }
    }

    #[test]
    fn write_errors_when_unreachable() {
        for (lin, len) in [
            (0x0_FFFFF, 0x2),
            (0x0_FF000, 0x1001),
            (0x1_00000, 0x1),
            (0xFFFF_FFFF, 0x1),
            (0x0_0000, usize::MAX),
        ] {
            assert_eq!(
                prepare_write(1, lin, len),
                Err(Error::BufferUnreachable),
                "lin={lin:#x} len={len:#x}"
            );
        }
    }

    fn host_ok() -> HostOutcome {
        HostOutcome {
            host_cf: false,
            host_ax: 0,
        }
    }

    #[test]
    fn surface_write_returns_bytes_written() {
        for (eax, expected) in [(0x0001, 1), (0x0003, 3), (0xFFFF, 0xFFFF as usize)] {
            let mut rmcs = Rmcs::zeroed();
            rmcs.ecx = 3;
            rmcs.eax = eax;
            assert_eq!(
                surface_write(&call(rmcs), host_ok()),
                Ok(expected),
                "eax={eax:#x}"
            );
        }
    }

    #[test]
    fn surface_write_maps_dos_error() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.ecx = 3;
        rmcs.eax = 5;
        rmcs.cf = 1;
        assert_eq!(
            surface_write(&call(rmcs), host_ok()),
            Err(Error::Dos { code: 5 })
        );
    }

    #[test]
    fn surface_write_prefers_dpmi_error() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.ecx = 3;
        rmcs.eax = 5;
        rmcs.cf = 1;
        let host_fail = HostOutcome {
            host_cf: true,
            host_ax: 0x8021,
        };
        assert_eq!(
            surface_write(&call(rmcs), host_fail),
            Err(Error::Dpmi { code: 0x8021 })
        );
    }

    #[test]
    fn surface_write_maps_dpmi_error() {
        let host_fail = HostOutcome {
            host_cf: true,
            host_ax: 0x8021,
        };
        assert_eq!(
            surface_write(&call(Rmcs::zeroed()), host_fail),
            Err(Error::Dpmi { code: 0x8021 })
        );
    }

    #[test]
    fn surface_write_maps_zero_write() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.ecx = 3;
        assert_eq!(surface_write(&call(rmcs), host_ok()), Err(Error::WriteZero));
    }

    #[test]
    fn surface_write_allows_zero_len() {
        assert_eq!(surface_write(&call(Rmcs::zeroed()), host_ok()), Ok(0));
    }
}

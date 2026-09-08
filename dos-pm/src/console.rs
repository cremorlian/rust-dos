use crate::raw::{HostOutcome, PreparedCall, Rmcs};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    WriteZero,
    Dos { code: u16 },
    Dpmi { code: u16 },
}

pub(crate) fn prepare_write(handle: u16, chunk: &[u8], buf: &mut [u8]) -> PreparedCall {
    buf[..chunk.len()].copy_from_slice(chunk);
    let lin = buf.as_ptr() as usize as u32;
    let mut rmcs = Rmcs::zeroed();
    rmcs.eax = 0x4000;
    rmcs.ebx = handle as u32;
    rmcs.ecx = chunk.len() as u32;
    rmcs.edx = lin & 0xF;
    rmcs.ds = (lin >> 4) as u16;
    PreparedCall { rmcs, int_no: 0x21 }
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

#[cfg(test)]
mod tests {
    use super::{Error, prepare_write, surface_write};
    use crate::raw::{HostOutcome, PreparedCall, Rmcs};

    fn call(rmcs: Rmcs) -> PreparedCall {
        PreparedCall { rmcs, int_no: 0x21 }
    }

    #[test]
    fn prepare_write_builds_full_ah40h_command() {
        let mut staging = [0u8; 8];
        let chunk = *b"hi";

        let cmd = prepare_write(1, &chunk, &mut staging);

        let lin = staging.as_ptr() as usize as u32;
        let mut rmcs = Rmcs::zeroed();
        rmcs.eax = 0x4000;
        rmcs.ebx = 1;
        rmcs.ecx = 2;
        rmcs.edx = lin & 0xF;
        rmcs.ds = (lin >> 4) as u16;
        let expected = PreparedCall { rmcs, int_no: 0x21 };

        assert_eq!(cmd, expected);
        assert_eq!(&staging[..2], &chunk[..]);
    }

    fn host_ok() -> HostOutcome {
        HostOutcome {
            host_cf: false,
            host_ax: 0,
        }
    }

    #[test]
    fn surface_write_returns_bytes_written() {
        let mut rmcs = Rmcs::zeroed();
        rmcs.ecx = 3;
        rmcs.eax = 3;
        assert_eq!(surface_write(&call(rmcs), host_ok()), Ok(3));
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

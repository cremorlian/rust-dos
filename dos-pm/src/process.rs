use crate::raw::{PreparedCall, Rmcs, execute};

pub(crate) fn prepare_exit(code: u8) -> PreparedCall {
    const AH_EXIT: u32 = 0x4C00;
    let mut rmcs = Rmcs::zeroed();
    rmcs.eax = AH_EXIT | code as u32;
    const INT_DOS: u8 = 0x21;
    PreparedCall {
        rmcs,
        int_no: INT_DOS,
    }
}

pub fn exit(code: u8) -> ! {
    let mut call = prepare_exit(code);
    let _ = execute(&mut call);
    unreachable!("INT 21h AH=4Ch terminates the process; execution cannot return")
}

#[cfg(test)]
mod tests {
    use super::prepare_exit;
    use crate::raw::{PreparedCall, Rmcs};

    #[test]
    fn exit_encodes_the_ah_4c_call() {
        for (code, expected_ax) in [(0x00u8, 0x4C00u32), (0x07, 0x4C07), (0xFF, 0x4CFF)] {
            let mut rmcs = Rmcs::zeroed();
            rmcs.eax = expected_ax;
            let expected = PreparedCall { rmcs, int_no: 0x21 };
            assert_eq!(prepare_exit(code), expected, "code={code:#x}");
        }
    }
}

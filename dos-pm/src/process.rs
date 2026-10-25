use crate::raw::{PreparedExit, exit_to_dos};

pub(crate) fn prepare_exit(code: u8) -> PreparedExit {
    const AH_EXIT: u32 = 0x4C00;
    PreparedExit {
        eax: AH_EXIT | code as u32,
    }
}

pub fn exit(code: u8) -> ! {
    exit_to_dos(prepare_exit(code))
}

#[cfg(test)]
mod tests {
    use super::prepare_exit;
    use crate::raw::PreparedExit;

    #[test]
    fn prepare_exit_encodes_the_4c_function_and_code() {
        for (code, expected_eax) in [(0u8, 0x4C00u32), (1, 0x4C01), (22, 0x4C16), (0xFF, 0x4CFF)] {
            let expected = PreparedExit { eax: expected_eax };
            assert_eq!(prepare_exit(code), expected, "code={code:#x}");
        }
    }
}

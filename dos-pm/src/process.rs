use crate::raw::exit_to_dos;

pub fn exit(code: u8) -> ! {
    exit_to_dos(code)
}

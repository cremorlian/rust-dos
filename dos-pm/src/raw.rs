#![allow(unsafe_code)]

#[cfg(target_arch = "x86")]
use core::arch::asm;

#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct Rmcs {
    pub(crate) edi: u32,
    pub(crate) esi: u32,
    pub(crate) ebp: u32,
    pub(crate) reserved: u32,
    pub(crate) ebx: u32,
    pub(crate) edx: u32,
    pub(crate) ecx: u32,
    pub(crate) eax: u32,
    pub(crate) ds: u16,
    pub(crate) es: u16,
    pub(crate) fs: u16,
    pub(crate) gs: u16,
    pub(crate) cf: u8,
    pub(crate) pad: u8,
    pub(crate) flags: u16,
}

impl Rmcs {
    pub(crate) const fn zeroed() -> Rmcs {
        Rmcs {
            edi: 0,
            esi: 0,
            ebp: 0,
            reserved: 0,
            ebx: 0,
            edx: 0,
            ecx: 0,
            eax: 0,
            ds: 0,
            es: 0,
            fs: 0,
            gs: 0,
            cf: 0,
            pad: 0,
            flags: 0,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PreparedCall {
    pub(crate) rmcs: Rmcs,
    pub(crate) int_no: u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct HostOutcome {
    pub(crate) host_cf: bool,
    pub(crate) host_ax: u16,
}

#[cfg(target_arch = "x86")]
pub(crate) fn execute(call: &mut PreparedCall) -> HostOutcome {
    let rmcs_ptr = core::ptr::addr_of_mut!(call.rmcs) as usize as u32;
    let mut host_cf = 0u8;
    let mut host_ax = 0u32;

    unsafe {
        asm!(
            "int $0x31",
            "setc {cf}",
            cf = out(reg_byte) host_cf,
            inlateout("eax") 0x0300u32 => host_ax,
            inlateout("ebx") (call.int_no as u32) => _,
            inlateout("edi") rmcs_ptr => _,
            lateout("ecx") _, lateout("edx") _, lateout("esi") _, lateout("ebp") _,
            options(nostack),
        );
    }

    HostOutcome {
        host_cf: host_cf != 0,
        host_ax: host_ax as u16,
    }
}

#[cfg(not(target_arch = "x86"))]
pub(crate) fn execute(_call: &mut PreparedCall) -> HostOutcome {
    unreachable!(
        "execute() raises int $0x31 to simulate a real-mode DOS call, and only \
         that can ever run on a 32-bit x86 DPMI target; host builds stub it out"
    )
}

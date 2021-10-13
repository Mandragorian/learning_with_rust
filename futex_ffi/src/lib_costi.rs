use std::sync::atomic::AtomicU32;

#[allow(non_camel_case_types)]
pub type c_long = i64;
#[allow(non_camel_case_types)]
pub type c_time_t = i64;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct c_timespec {
    pub tv_sec: c_time_t,
    pub tv_nsec: c_long,
}

extern "C" {
    pub fn syscall(
        syscall: i64,
        futex_addr: *const AtomicU32,
        op: i32,
        val: u32,
        timeout: *const c_timespec,
        uaddr2: *const u32,
        val3: u32,
    ) -> c_long;
}

pub const SYS_FUTEX: i64 = 202;
pub const FUTEX_WAIT: i32 = 0;
pub const FUTEX_WAKE: i32 = 1;

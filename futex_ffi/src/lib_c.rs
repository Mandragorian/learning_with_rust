use std::sync::atomic::AtomicU32;

use libc::{c_long, syscall as syscall_untyped};

pub use libc::{timespec as c_timespec, SYS_futex as SYS_FUTEX, FUTEX_WAIT, FUTEX_WAKE};

pub unsafe fn syscall(
    syscall: i64,
    futex_addr: *const AtomicU32,
    op: i32,
    val: u32,
    timeout: *const c_timespec,
    uaddr2: *const u32,
    val3: u32,
) -> c_long {
    syscall_untyped(syscall, futex_addr, op, val, timeout, uaddr2, val3)
}

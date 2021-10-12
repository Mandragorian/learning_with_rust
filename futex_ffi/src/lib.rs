extern "C" {
    fn syscall(syscall: u64, futex_addr: u64, op: u32, val: u32, timespec: u64, uaddr2: u64, val3: u32) -> i32;
}

const SYS_FUTEX: u64 = 202;
const FUTEX_WAIT: u32 = 0;

unsafe fn futex(futex_addr: u64, op: u32, val: u32, timespec: u64) -> i32 {
    syscall(SYS_FUTEX, futex_addr, op, val, timespec, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syscall_basic_functionality() {
        let shared_int: u32 = 0;
        let shared_int_addr = &shared_int as *const u32;
        let shared_int_addr_u64 = shared_int_addr as u64;
        let res = unsafe { syscall(SYS_FUTEX, shared_int_addr_u64, FUTEX_WAIT, 1, 0, 0, 0) };
        assert_eq!(res, -1);
    }

    #[test]
    fn futext_basic_functionality() {
        let shared_int: u32 = 0;
        let shared_int_addr = &shared_int as *const u32;
        let shared_int_addr_u64 = shared_int_addr as u64;
        let res = unsafe { futex(shared_int_addr_u64, FUTEX_WAIT, 1, 0) };
        assert_eq!(res, -1);
    }
}

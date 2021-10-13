use std::sync::atomic::AtomicU32;

pub struct FutexTimeout(u32, u32);

#[allow(non_camel_case_types)]
type c_time_t = u32;

#[repr(C)]
#[allow(non_camel_case_types)]
struct c_timespec {
    tv_sec: c_time_t,
    tv_nsec: u32,
}

extern "C" {
    fn syscall(syscall: u64, futex_addr: u64, op: u32, val: u32, timespec: u64, uaddr2: u64, val3: u32) -> i32;
}

const SYS_FUTEX: u64 = 202;
const FUTEX_WAIT: u32 = 0;
const FUTEX_WAKE: u32 = 1;

unsafe fn futex(futex_ref: &AtomicU32, op: u32, val: u32, timeout: Option<FutexTimeout>) -> i32 {
    let futex_addr = (futex_ref as *const AtomicU32) as u64;
    let timeout_ptr = match timeout {
        None => 0,
        Some(duration) => {
            let timespec = c_timespec {
                tv_sec: duration.0,
                tv_nsec: duration.1,
            };
            (&timespec as *const c_timespec) as u64
        },
    };
    syscall(SYS_FUTEX, futex_addr, op, val, timeout_ptr, 0, 0)
}

pub unsafe fn futex_wait(futex_addr: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i32 {
    futex(futex_addr, FUTEX_WAIT, val, timeout)
}

pub unsafe fn futex_wake(futex_addr: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i32 {
    futex(futex_addr, FUTEX_WAKE, val, timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

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
        let shared_int = AtomicU32::new(0);
        let res = unsafe { futex(&shared_int, FUTEX_WAIT, 1, None) };
        assert_eq!(res, -1);

        let res = unsafe { futex(&shared_int, FUTEX_WAKE, 1, None) };
        assert_eq!(res, 0);
    }

    #[test]
    fn futex_wake_after_wait() {
        let shared_int = Arc::new(AtomicU32::new(0));
        let shared_int2 = Arc::clone(&shared_int);

        let handle = std::thread::spawn(move || {
            unsafe { futex_wait(shared_int2.as_ref(), 0, None) }
        });

        std::thread::sleep(Duration::from_millis(2000));
        let res = unsafe { futex_wake(&shared_int, 1, None) };
        assert_eq!(res, 1);

        // Checking that the return value is zero checks both that 
        // the thread was woken up, and with no errors
        assert_eq!(handle.join().unwrap(), 0);
    }

    #[test]
    fn futex_wakes_up_after_timeout() {
        let shared_int = AtomicU32::new(1);
        let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let finished2 = Arc::clone(&finished);

        std::thread::spawn(move || {
            unsafe { futex_wait(&shared_int, 1, Some(FutexTimeout(0,500000000))) };
            finished2.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        std::thread::sleep(Duration::from_secs(1));
        assert!(finished.load(std::sync::atomic::Ordering::Relaxed));
    }
}

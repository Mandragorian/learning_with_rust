#[cfg(all(feature = "libc", feature = "libcosti"))]
compile_error!("feature \"libc\" and feature \"libcosti\" cannot be enabled at the same time");

#[cfg(feature = "libc")]
mod lib_c;
#[cfg(feature = "libcosti")]
mod lib_costi;

#[cfg(feature = "libc")]
use crate::lib_c::*;
#[cfg(feature = "libcosti")]
use crate::lib_costi::*;

use std::ptr::null;
use std::sync::atomic::AtomicU32;

pub struct FutexTimeout(i64, i64);

impl From<FutexTimeout> for c_timespec {
    fn from(timeout: FutexTimeout) -> Self {
        println!("here");
        let tv_sec = timeout.0;
        let tv_nsec = timeout.1;
        Self { tv_sec, tv_nsec }
    }
}

unsafe fn futex(futex_ref: &AtomicU32, op: i32, val: u32, timeout: Option<FutexTimeout>) -> i64 {
    let futex_addr = futex_ref as *const AtomicU32;
    let timeout_ptr = match timeout {
        None => null(),
        Some(duration) => {
            let timespec = c_timespec::from(duration);
            (&timespec) as *const c_timespec
        }
    };
    syscall(SYS_FUTEX, futex_addr, op, val, timeout_ptr, null(), 0)
}

pub fn futex_wait(futex_addr: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64 {
    unsafe { futex(futex_addr, FUTEX_WAIT, val, timeout) }
}

pub fn futex_wake(futex_addr: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64 {
    unsafe { futex(futex_addr, FUTEX_WAKE, val, timeout) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    #[test]
    fn syscall_basic_functionality() {
        let shared_int = AtomicU32::new(0);
        let shared_int_addr = &shared_int as *const AtomicU32;
        let res = unsafe { syscall(SYS_FUTEX, shared_int_addr, FUTEX_WAIT, 1, null(), null(), 0) };
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

        let handle = spawn(move || futex_wait(shared_int2.as_ref(), 0, None));

        sleep(Duration::from_millis(2000));
        let res = futex_wake(&shared_int, 1, None);
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

        spawn(move || {
            futex_wait(&shared_int, 1, Some(FutexTimeout(0, 500000000)));
            finished2.store(true, Ordering::Relaxed);
        });

        sleep(Duration::from_secs(1));
        assert!(finished.load(Ordering::Relaxed));
    }
}

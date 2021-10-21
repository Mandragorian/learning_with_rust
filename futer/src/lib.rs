#![feature(dropck_eyepatch)]
#![feature(test)]

extern crate test;

use std::sync::atomic::{AtomicU32, Ordering};
use std::marker::PhantomData;

use futex_ffi::{futex_wait, futex_wake, FutexTimeout};

trait Futex {
    fn futex_wake(lock: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64;
    fn futex_wait(lock: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64;
}

#[derive(Debug)]
struct RealFutexCalls;
impl Futex for RealFutexCalls {
    fn futex_wake(lock: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64 {
        futex_wake(lock, val, timeout)
    }
    fn futex_wait(lock: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64 {
        futex_wait(lock, val, timeout)
    }
}

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
const CONTESTED: u32 = 2;

#[derive(Debug)]
struct FuterGuardInternal<'a, T, F: Futex> {
    ptr: *const T,
    lock: &'a AtomicU32,
    _futex: PhantomData<fn() -> F>,
}

impl<'a, T, F: Futex> FuterGuardInternal<'a, T, F> {
    fn new(ptr: *const T, lock: &'a AtomicU32) -> Self {
        Self { ptr, lock, _futex: PhantomData }
    }
}

impl<'a, T, F: Futex> std::ops::Deref for FuterGuardInternal<'a, T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: the ptr was created through a box, thus it has valid alignment etc.
        // Since self still exists, the owner of the lock is alive ('a constrains us
        // to live at most as long as the lock). Thus ptr still points to valid
        // memory.
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<'a, T, F: Futex> std::ops::DerefMut for FuterGuardInternal<'a, T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr_mut = self.ptr as *mut T;
        // Safety: Since self exists, we have gained access to the lock, and we can
        // mutate the memory content. For validity/alignment look at Deref implementation
        unsafe { ptr_mut.as_mut().unwrap() }
    }
}

// Safety: T is never accessed in drop, so it is safe to let it dangle
unsafe impl<'a, #[may_dangle] T, #[may_dangle] F: Futex> Drop for FuterGuardInternal<'a, T, F> {
    fn drop(&mut self) {
        if self.lock.fetch_sub(1, Ordering::Release) != 1 {
            self.lock.store(0, Ordering::Release);
            F::futex_wake(self.lock, u32::MAX, None);
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TryLockError {
    WouldBlock,
}

struct FuterInternal<T, F: Futex> {
    val: Box<T>,
    lock: Box<AtomicU32>,
    _futex: PhantomData<fn() -> F>,
}

impl<T, F: Futex> FuterInternal<T, F> {
    fn new(unboxed_val: T) -> Self {
        let val = Box::new(unboxed_val);
        let lock = Box::new(AtomicU32::new(UNLOCKED));
        Self { val, lock, _futex: PhantomData }
    }

    fn lock(&self) -> Result<FuterGuardInternal<T, F>, ()> {
        match self
            .lock
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Acquire) {
                Ok(_) => {
                    return Ok(FuterGuardInternal::new(
                        self.val.as_ref() as *const T,
                        self.lock.as_ref(),
                    ))
                }
                Err(val) => {
                    let mut c = val;
                    loop {
                        if (c == 2) || (self.lock.compare_exchange(LOCKED, CONTESTED, Ordering::Acquire, Ordering::Acquire) == Err(2))  {
                            F::futex_wait(&self.lock, CONTESTED, None);
                        }
                        c = match self.lock.compare_exchange(UNLOCKED, CONTESTED, Ordering::Acquire, Ordering::Acquire) {
                            Ok(_) => break Ok(FuterGuardInternal::new(
                                    self.val.as_ref() as *const T,
                                    self.lock.as_ref(),
                                )),
                            Err(val) => val,
                        }
                    }
                }
            }
    }

    fn try_lock(&self) -> Result<FuterGuardInternal<T, F>, TryLockError> {
        match self.lock.compare_exchange_weak(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Acquire) {
            Ok(_) =>
                Ok(FuterGuardInternal::new(
                    self.val.as_ref() as *const T,
                    self.lock.as_ref(),
                )),
            Err(_) => Err(TryLockError::WouldBlock)
        }
    }

    fn unlock(guard: FuterGuardInternal<T, F>) {
        drop(guard)
    }
}

pub struct Futer<T>(FuterInternal<T, RealFutexCalls>);

impl<T> Futer<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Futer(FuterInternal::new(val))
    }

    #[inline]
    pub fn lock(&self) -> Result<FuterGuard<T>, ()> {
        self.0.lock().map(|guard| FuterGuard(guard))
    }

    #[inline]
    pub fn try_lock(&self) -> Result<FuterGuard<T>, TryLockError> {
        self.0.try_lock().map(|guard| FuterGuard(guard))
    }

    #[inline]
    pub fn unlock(guard: FuterGuard<T>) {
        FuterInternal::unlock(guard.0)
    }
}

#[derive(Debug)]
pub struct FuterGuard<'a, T>(FuterGuardInternal<'a, T, RealFutexCalls>);

impl<'a, T> std::ops::Deref for FuterGuard<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, T> std::ops::DerefMut for FuterGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static FUTEX_WAKE_CALL_COUNTER: AtomicU32 = AtomicU32::new(0);
    static FUTEX_WAIT_CALL_COUNTER: AtomicU32 = AtomicU32::new(0);

    struct MockFutexCalls;

    impl Futex for MockFutexCalls {
        fn futex_wake(lock: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64 {
            FUTEX_WAKE_CALL_COUNTER.fetch_add(1, Ordering::SeqCst);
            futex_wake(lock, val, timeout)
        }
        fn futex_wait(lock: &AtomicU32, val: u32, timeout: Option<FutexTimeout>) -> i64 {
            FUTEX_WAIT_CALL_COUNTER.fetch_add(1, Ordering::SeqCst);
            futex_wait(lock, val, timeout)
        }
    }

    #[test]
    fn futer_can_be_correctly_constructed() {
        let _futer = Futer::new(32);
    }

    #[test]
    fn futer_is_generic() {
        let _futer_int: Futer<u32> = Futer::new(32);
        let _futer_string: Futer<String> = Futer::new(String::from("hello"));
    }

    #[test]
    fn futer_lock_api() {
        let futer = Futer::new(32);
        assert_eq!(*futer.lock().unwrap(), 32);

        *futer.lock().unwrap() = 42;
        assert_eq!(*futer.lock().unwrap(), 42);

        let futer = Futer::new(String::from("asdf"));
        assert_eq!(futer.lock().unwrap().as_str(), "asdf");
    }

    #[test]
    fn futer_unlock_api() {
        let futer = Futer::new(32);
        let lock = futer.lock().unwrap();

        Futer::unlock(lock);
    }

    #[test]
    fn futer_is_send_and_sync() {
        // Arc<T> is Sync + Send only if T is Sync + Send, so
        // if Arc<Futer<T>> is Send, Futer<T> is Sync + Send.
        let futer = std::sync::Arc::new(Futer::new(32));

        std::thread::spawn(move || {
            let v = futer.lock().unwrap();
            assert_eq!(*v, 32);
        });
    }

    #[test]
    fn basic_allocate_on_heap() {
        fn dummy_stack() -> (Futer<u32>, *const u32) {
            let f = Futer::new(32);
            let f_ptr = {
                let x = &*f.lock().unwrap() as *const u32;
                x
            };
            (f, f_ptr)
        }

        let (futer, ptr) = dummy_stack();

        assert_eq!(&*futer.lock().unwrap() as *const u32, ptr);
    }

    #[test]
    fn basic_sync_test() {
        use std::sync::{Arc, Barrier};
        use std::thread::spawn;

        const NUM_THREADS: usize = 5;
        const NUM_ITER: usize = 1000;

        for _ in 0..1 {
            let barrier = Arc::new(Barrier::new(NUM_THREADS));
            let finished_barrier = Arc::new(Barrier::new(NUM_THREADS + 1));
            let futer = Arc::new(Futer::new(0));

            for _ in 0..NUM_THREADS {
                let futer_clone = Arc::clone(&futer);
                let barrier_clone = Arc::clone(&barrier);
                let finished_barrier_clone = Arc::clone(&finished_barrier);
                spawn(move || {
                    barrier_clone.wait();
                    for _ in 0..NUM_ITER {
                        let mut lock = futer_clone.lock().unwrap();
                        *lock = *lock + 1;
                        Futer::unlock(lock);
                    }
                    finished_barrier_clone.wait();
                });
            }

            finished_barrier.wait();
            assert_eq!(*futer.lock().unwrap(), NUM_THREADS * NUM_ITER);
        }
    }

    #[test]
    fn try_lock_api() {
        let futer = Futer::new(32);
        let mut lock = futer.try_lock().unwrap();

        assert_eq!(*lock, 32);

        *lock = 42;
        assert_eq!(*lock, 42);


        Futer::unlock(lock);
    }

    #[test]
    fn try_lock_would_block() {
        let futer = Futer::new(32);
        let futer2 = &futer;
        let _lock = futer.lock();

        {
            let lock2 = futer2.try_lock();
            if let Err(err) = lock2 {
                assert_eq!(TryLockError::WouldBlock, err);
            } else {
                panic!("try_lock did not return error");
            }
        }
    }

    #[test]
    fn only_syscalls_when_contested() {
        let futer_internal = FuterInternal::<u32, MockFutexCalls>::new(0);

        let lock = futer_internal.lock().unwrap();
        FuterInternal::unlock(lock);

        assert_eq!(FUTEX_WAIT_CALL_COUNTER.load(Ordering::SeqCst), 0);
        assert_eq!(FUTEX_WAKE_CALL_COUNTER.load(Ordering::SeqCst), 0);
    }
}

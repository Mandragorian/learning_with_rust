#![feature(dropck_eyepatch)]

use std::sync::atomic::AtomicU32;

use futex_ffi::{futex_wait, futex_wake};

#[derive(Debug)]
pub struct FuterGuard<'a, T> {
    ptr: *const T,
    lock: &'a AtomicU32,
}

impl<'a, T> FuterGuard<'a, T> {
    fn new(ptr: *const T, lock: &'a AtomicU32) -> Self {
        Self { ptr, lock }
    }
}

impl<'a, T> std::ops::Deref for FuterGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: the ptr was created through a box, thus it has valid alignment etc.
        // Since self still exists, the owner of the lock is alive ('a constrains us
        // to live at most as long as the lock). Thus ptr still points to valid
        // memory.
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<'a, T> std::ops::DerefMut for FuterGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr_mut = self.ptr as *mut T;
        // Safety: Since self exists, we have gained access to the lock, and we can
        // mutate the memory content. For validity/alignment look at Deref implementation
        unsafe { ptr_mut.as_mut().unwrap() }
    }
}

// Safety: T is never accessed in drop, so it is safe to let it dangle
unsafe impl<'a, #[may_dangle] T> Drop for FuterGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(0, std::sync::atomic::Ordering::Relaxed);
        futex_wake(self.lock, u32::MAX, None);
    }
}

pub struct Futer<T> {
    val: Box<T>,
    lock: Box<AtomicU32>,
}

impl<T> Futer<T> {
    pub fn new(unboxed_val: T) -> Self {
        let val = Box::new(unboxed_val);
        let lock = Box::new(AtomicU32::new(0));
        Self { val, lock }
    }

    pub fn lock(&self) -> Result<FuterGuard<T>, ()> {
        use std::sync::atomic::Ordering;

        loop {
            match self
                .lock
                .compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => {
                    break Ok(FuterGuard::new(
                        self.val.as_ref() as *const T,
                        self.lock.as_ref(),
                    ))
                }
                Err(_) => {
                    futex_wait(&self.lock, 1, None);
                }
            }
        }
    }

    pub fn unlock(guard: FuterGuard<T>) {
        drop(guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        const NUM_THREADS: usize = 1000;

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
                    let mut lock = futer_clone.lock().unwrap();
                    *lock = *lock + 1;
                    Futer::unlock(lock);
                    finished_barrier_clone.wait();
                });
            }

            finished_barrier.wait();
            assert_eq!(*futer.lock().unwrap(), NUM_THREADS);
        }
    }
}

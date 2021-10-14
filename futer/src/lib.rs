#[derive(Debug)]
pub struct FuterGuard<T> {
    ptr: *const T,
}

impl<T> std::ops::Deref for FuterGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T> std::ops::DerefMut for FuterGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr_mut = self.ptr as *mut T;
        unsafe { ptr_mut.as_mut().unwrap() }
    }
}

pub struct Futer<T> {
    val: Box<T>,
}

impl<T> Futer<T> {
    pub fn new(unboxed_val: T) -> Self {
        let val = Box::new(unboxed_val);
        Self { val }
    }

    pub fn lock(&self) -> Result<FuterGuard<T>, ()> {
        Ok(FuterGuard { ptr: self.val.as_ref() as *const T })
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
            let f_ptr = &*f.lock().unwrap() as *const u32;
            (f, f_ptr)
        }

        let (futer, ptr) = dummy_stack();

        assert_eq!(&*futer.lock().unwrap() as *const u32, ptr);
    }
}

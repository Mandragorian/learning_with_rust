pub struct Futer<T> {
    val: T,
}

impl<T> Futer<T> {
    pub fn new(val: T) -> Self {
        Self { val }
    }

    pub fn lock(&self) -> Result<&T, ()> {
        Ok(&self.val)
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
        assert_eq!(futer.lock().unwrap(), &32);

        let futer = Futer::new(String::from("asdf"));
        assert_eq!(futer.lock().unwrap().as_str(), "asdf");
    }
}

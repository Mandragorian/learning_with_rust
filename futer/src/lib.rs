pub struct Futer<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Futer<T> {
    pub fn new(val: T) -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn futer_can_be_correctly_constructed() {
        let futer = Futer::new(32);
    }

    #[test]
    fn futer_is_generic() {
        let _futer_int: Futer<u32> = Futer::new(32);
        let _futer_string: Futer<String> = Futer::new(String::from("hello"));
    }
}

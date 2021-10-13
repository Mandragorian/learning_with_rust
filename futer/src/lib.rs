pub struct Futer;

impl Futer {
    pub fn new(val: u32) -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn futer_can_be_correctly_constructed() {
        let futer = Futer::new(32);
    }
}

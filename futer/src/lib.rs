pub struct Futer;

impl Futer {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_exists() {
        let futer = Futer::new();
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub fn should_panic_okay() {
    panic!("Panic");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn assert_true() {
        assert!(true, "it must be true");
    }

    #[test]
    #[should_panic]
    fn assert_should_panic() {
        should_panic_okay()
    }
}

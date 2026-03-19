fn main() {
    println!("true: {}", valid_braces("()"));
    println!("false: {}", valid_braces("[(])"));
    println!("true: {}", valid_braces("([{}])"));
    println!("true: {}", valid_braces("({})[({})]"));
}

fn valid_braces(s: &str) -> bool {
    let mut string = s.to_string();
    let braces = vec!["{}", "[]", "()"];

    let mut i = 0;
    let mut cycle = 0;
    while cycle < 2 {
        let b = braces[i];
        while string.matches(b).count() > 0 {
            string = string.replace(b, "");
        }

        i += 1;

        if i >= braces.len() {
            i = 0;
            cycle += 1;
        }

        if string == "" {
            return true;
        }
    }

    string == ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tests() {
        expect_true("()");
        expect_false("[(])");
    }

    fn expect_true(s: &str) {
        assert!(
            valid_braces(s),
            "Expected {s:?} to be valid. Got false",
            s = s
        );
    }

    fn expect_false(s: &str) {
        assert!(
            !valid_braces(s),
            "Expected {s:?} to be invalid. Got true",
            s = s
        );
    }
}

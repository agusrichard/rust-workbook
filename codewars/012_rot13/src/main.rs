fn main() {
    println!("{:?}", rot13("abcde!<>"));
    println!("{:?}", rot13("ABCDE"));
}

fn rot13(message: &str) -> String {
    message
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                let offset = (c as u8) - base;
                let rotated = (offset + 13) % 26;
                return (base + rotated) as char;
            } else {
                return c;
            }
        })
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::rot13;

    const ERR_MSG: &str = "\nYour result (left) did not match the expected output (right)";

    fn dotest(s: &str, expected: &str) {
        assert_eq!(rot13(s), expected, "{ERR_MSG} with message = \"{s}\"")
    }

    #[test]
    fn sample_tests() {
        dotest("test", "grfg");
        dotest("Test", "Grfg");
    }
}

fn main() {
    println!("Hello, world!");
}

fn expanded_form(n: u64) -> String {
    let result: Vec<String> = format!("{}", n)
        .chars()
        .rev()
        .enumerate()
        .filter_map(|(i, c)| {
            if c == '0' {
                None
            } else {
                Some(format!(
                    "{}",
                    c.to_digit(10).unwrap() as u64 * 10u64.pow(i as u32)
                ))
            }
        })
        .collect();

    result
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<String>>()
        .join(" + ")
}

fn expanded_form_simplified(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    s.chars()
        .enumerate()
        .filter_map(|(i, c)| {
            let digit = c.to_digit(10).unwrap() as u64;
            let place = 10u64.pow((len - 1 - i) as u32);
            (digit != 0).then(|| (digit * place).to_string())
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

#[cfg(test)]
mod tests {
    use super::expanded_form;
    use super::expanded_form_simplified;

    #[test]
    fn examples() {
        assert_eq!(expanded_form(12), "10 + 2");
        assert_eq!(expanded_form(42), "40 + 2");
        assert_eq!(expanded_form(70304), "70000 + 300 + 4");
    }

    #[test]
    fn examples_simplified() {
        assert_eq!(expanded_form_simplified(12), "10 + 2");
        assert_eq!(expanded_form_simplified(42), "40 + 2");
        assert_eq!(expanded_form_simplified(70304), "70000 + 300 + 4");
    }
}

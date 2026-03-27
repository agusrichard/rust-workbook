fn main() {
    println!("Hello, world!");
}

fn luck_check_v2(ticket: &str) -> Option<bool> {
    let digits: Vec<u32> = ticket
        .chars()
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<_>>>()?;

    if digits.is_empty() {
        return None;
    }

    let mid = digits.len() / 2;
    let (left, right) = (&digits[..mid], &digits[digits.len() - mid..]);
    Some(left.iter().sum::<u32>() == right.iter().sum::<u32>())
}

fn luck_check(ticket: &str) -> Option<bool> {
    let contain_char = ticket.chars().any(|c| !c.is_ascii_digit());
    if contain_char || ticket.len() == 0 {
        return None;
    }

    let (left, right) = if ticket.len() % 2 == 0 {
        let mid = ticket.len() / 2;
        (&ticket[..mid], &ticket[mid..])
    } else {
        let mid = ticket.len() / 2;
        (&ticket[..mid], &ticket[mid + 1..])
    };

    let left_sum = left.chars().map(|c| c as u64).sum::<u64>();
    let right_sum = right.chars().map(|c| c as u64).sum::<u64>();

    Some(left_sum == right_sum)
}

#[cfg(test)]
mod tests {
    use super::luck_check;

    fn dotest(s: &str, expected: Option<bool>) {
        let actual = luck_check(s);
        assert!(
            actual == expected,
            "With ticket = \"{s}\"\nExpected {expected:?} but got {actual:?}"
        )
    }

    #[test]
    fn fixed_tests() {
        dotest("683179", Some(true));
        dotest("683000", Some(false));
        dotest("6F43E8", None);
        dotest("91856399083", Some(true))
    }
}

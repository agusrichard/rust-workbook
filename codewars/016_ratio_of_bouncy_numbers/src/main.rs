fn main() {
    println!("Hello, world!");
}

fn number_check_v2(n: u32) -> bool {
    let digits: Vec<u32> = n
        .to_string()
        .chars()
        .map(|c| c.to_digit(10).unwrap())
        .collect();

    let is_incr = digits.windows(2).all(|w| w[0] <= w[1]);
    let is_dcr = digits.windows(2).all(|w| w[0] >= w[1]);

    !is_incr && !is_dcr
}

fn number_check(n: u32) -> bool {
    let (_, is_incr) =
        n.to_string()
            .chars()
            .into_iter()
            .fold((0, true), |(prev, state): (u32, bool), x: char| {
                let digit = x.to_digit(10).unwrap();
                if state && (digit) >= prev {
                    return (digit, true);
                }

                (prev, false)
            });

    let (_, is_dcr) =
        n.to_string()
            .chars()
            .into_iter()
            .fold((9, true), |(prev, state): (u32, bool), x: char| {
                let digit = x.to_digit(10).unwrap();
                if state && (digit) <= prev {
                    return (digit, true);
                }

                (prev, false)
            });

    !is_incr && !is_dcr
}

fn bouncy_ratio(ratio: f64) -> Option<u32> {
    if ratio < 0.0 || ratio > 0.99 {
        return None;
    };

    if ratio == 0.0 {
        return Some(1);
    }

    let mut i = 100u32;
    let mut counter = 0;

    while (counter as f64 / i as f64) < ratio {
        i += 1;

        if number_check(i) {
            counter += 1;
        }
    }

    Some(i)
}

#[cfg(test)]
mod tests {
    use super::bouncy_ratio;

    const ERR_MSG: &str = "\nYour result (left) did not match the expected output (right)";

    fn dotest(p: f64, expected: Option<u32>) {
        assert_eq!(bouncy_ratio(p), expected, "{ERR_MSG} with ratio = {p}")
    }

    #[test]
    fn fixed_tests() {
        dotest(0.0, Some(1));
        dotest(0.999, None);
        dotest(0.15, Some(160));
        dotest(0.5, Some(538));
        dotest(0.75, Some(3088));
        dotest(0.9, Some(21780));
    }
}

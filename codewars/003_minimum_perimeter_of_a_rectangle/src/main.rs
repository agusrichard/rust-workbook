fn main() {
    let result = minimum_perimeter(45);
    println!("{result}");
}

fn minimum_perimeter(area: u64) -> u64 {
    let sqrt = (area as f64).sqrt() as u64;
    let (a, b) = (1..=sqrt)
        .filter(|x| area % x == 0)
        .map(|x| (x, area / x))
        .min_by_key(|&(a, b)| a + b)
        .unwrap();
    2 * (a + b)
}

#[cfg(test)]
mod tests {
    use super::minimum_perimeter;

    fn dotest(n: u64, expected: u64) {
        let actual = minimum_perimeter(n);
        assert!(
            actual == expected,
            "With n = {n}\nExpected {expected} but got {actual}"
        )
    }

    #[test]
    fn sample_tests() {
        dotest(45, 28);
        dotest(30, 22);
        dotest(81, 36);
        dotest(89, 180);
    }

    #[test]
    fn perfect_square() {
        dotest(1, 4);   // 1x1
        dotest(4, 8);   // 2x2
        dotest(9, 12);  // 3x3
        dotest(16, 16); // 4x4
        dotest(25, 20); // 5x5
    }

    #[test]
    fn prime_area() {
        // primes only have (1, p) as divisor pair
        dotest(2, 6);
        dotest(7, 16);
        dotest(13, 28);
        dotest(97, 196);
    }

    #[test]
    fn large_area() {
        dotest(100, 40);  // 10x10
        dotest(120, 44);  // 10x12
        dotest(1000, 130); // 25x40
    }
}

fn main() {
    println!("Hello, world! {:?}", powers(5));
}

fn powers(n: u32) -> Vec<u32> {
    format!("{:b}", n)
        .chars()
        .rev()
        .enumerate()
        .map(|(i, x)| if x == '1' { 2u32.pow(i as u32) } else { 0 })
        .filter(|&x| x != 0)
        .collect()
}

#[cfg(test)]
mod samples {
    use super::powers;

    fn dotest(n: u32, expected: Vec<u32>) {
        let actual = powers(n);
        assert!(
            actual == expected,
            "With n = {n}\nExpected {expected:?}\nGot {actual:?}"
        )
    }

    #[test]
    fn example_cases() {
        dotest(1, vec![1]);
        dotest(5, vec![1, 4]);
        dotest(7, vec![1, 2, 4]);
        dotest(8, vec![8]);
        dotest(10, vec![2, 8]);

        dotest(21, vec![1, 4, 16]);
        dotest(53, vec![1, 4, 16, 32]);
        dotest(63, vec![1, 2, 4, 8, 16, 32]);
        dotest(99, vec![1, 2, 32, 64]);
        dotest(100, vec![4, 32, 64]);
    }
}

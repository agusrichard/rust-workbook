fn main() {
    println!("Hello, world!");
}

pub fn solution(array_a: &[i64], array_b: &[i64]) -> f64 {
    array_a
        .iter()
        .zip(array_b)
        .map(|(a, b)| (*a as f64 - *b as f64).powi(2))
        .sum::<f64>()
        / array_a.len() as f64
}

#[cfg(test)]
mod sample_tests {
    use super::solution;

    macro_rules! assert_approx_eq {
        ($sol:expr, $exp:expr, $epsilon:expr) => {
            assert!(
                ($exp - $sol).abs() <= $epsilon,
                "Expected {}, got {}. Allowed error margin: {}",
                $exp,
                $sol,
                $epsilon
            )
        };
        ($sol:expr, $exp:expr) => {
            assert_approx_eq!($sol, $exp, 1e-9)
        };
    }

    #[test]
    fn test_fixed() {
        assert_approx_eq!(solution(&[1, 2, 3], &[4, 5, 6]), 9.);
        assert_approx_eq!(solution(&[10, 20, 10, 2], &[10, 25, 5, -2]), 16.5);
        assert_approx_eq!(solution(&[0, -1], &[-1, 0]), 1.);
        assert_approx_eq!(solution(&[10, 10], &[10, 10]), 0.);
    }
}

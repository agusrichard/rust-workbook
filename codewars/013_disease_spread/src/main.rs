fn main() {
    let result = epidemic(18, 432, 1004, 1, 0.00209, 0.51);
    println!("420 == {:?}", result);
}

fn epidemic(tm: i32, n: i32, s0: i32, i0: i32, b: f64, a: f64) -> i32 {
    let dt = tm as f64 / n as f64;
    let mut ik = i0 as f64;
    let mut sk = s0 as f64;
    let mut max = i0 as f64;
    for _ in 0..(n + 1) {
        (sk, ik) = (sk - (dt * b * sk * ik), ik + (dt * (b * sk * ik - a * ik)));
        if ik > max {
            max = ik
        }
    }

    max as i32
}

#[cfg(test)]
mod tests {
    use super::epidemic;

    #[test]
    fn basic() {
        assert_eq!(epidemic(18, 432, 1004, 1, 0.00209, 0.51), 420);
        assert_eq!(epidemic(12, 288, 1007, 2, 0.00206, 0.45), 461);
        assert_eq!(epidemic(13, 312, 999, 1, 0.00221, 0.55), 409);
    }
}

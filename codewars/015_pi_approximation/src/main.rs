use std::f64::consts::PI;

fn main() {
    println!("Hello, world!");
}

fn rnd10(f: f64) -> f64 {
    (f * 1e10).round() / 1e10
}

fn iter_pi(epsilon: f64) -> (i32, f64) {
    let mut apr = 1f64;
    let mut apr_pi = apr * 4f64;
    let (mut denom, mut i): (f64, i32) = (3.0, 0);
    while (apr_pi - PI).abs() > epsilon {
        let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
        apr = apr + (sign * (1f64 / denom));

        i += 1;
        denom += 2f64;
        apr_pi = apr * 4f64;
    }

    (i + 1, rnd10(apr_pi))
}

fn testing(epsilon: f64, exp: (i32, f64)) -> () {
    assert_eq!(iter_pi(epsilon), exp)
}

#[test]
fn tests_iter_pi() {
    testing(0.1, (10, 3.0418396189));
    testing(0.01, (100, 3.1315929036));
    testing(0.001, (1000, 3.1405926538));
    testing(0.0001, (10000, 3.1414926536));
    testing(0.00001, (100001, 3.1416026535));
    testing(0.000001, (1000001, 3.1415936536));
    testing(0.05, (20, 3.0916238067));
}

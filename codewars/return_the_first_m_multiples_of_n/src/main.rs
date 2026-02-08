fn main() {
    let result1 = multiples(3, 5.0);
    println!("{:?}", result1);
    let result2 = oneline_multiples(3, 5.0);
    println!("{:?}", result2);
}

fn multiples(m: i32, n: f64) -> Vec<f64> {
    let mut v: Vec<f64> = Vec::new();
    for i in 1..=m {
        v.push((i as f64) * n);
    }

    return v.clone();
}

fn oneline_multiples(m: i32, n: f64) -> Vec<f64> {
    (1..=m).map(|x| (x as f64) * n).collect()
}

#[cfg(test)]
mod tests {
    use super::multiples;
}

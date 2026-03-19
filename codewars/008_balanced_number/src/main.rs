fn main() {
    println!("{}", balanced_num(7));
    println!("{}", balanced_num(959));
    println!("{}", balanced_num(13));
    println!("{}", balanced_num(432));
    println!("{}", balanced_num(1230987));
    println!("{}", balanced_num(56239814));
}

fn balanced_num(n: u64) -> String {
    let n_str = n.to_string();
    let n_len = n_str.len();
    if n_len == 1 || n_len == 2 {
        return String::from("Balanced");
    }

    let (mut head, mut tail) = ("", "");
    if n_len % 2 == 1 {
        let mid = n_len / 2;
        head = &n_str[..mid];
        tail = &n_str[mid + 1..];
    } else {
        let mid = (n_len / 2) - 1;
        head = &n_str[..mid];
        tail = &n_str[mid + 2..];
    };

    let head_sum = head.chars().map(|x| x as i32).sum::<i32>();
    let tail_sum = tail.chars().map(|x| x as i32).sum::<i32>();
    if head_sum == tail_sum {
        return String::from("Balanced");
    }

    String::from("Not Balanced")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_number() {
        assert_eq!(balanced_num(7), "Balanced");
        assert_eq!(balanced_num(959), "Balanced");
        assert_eq!(balanced_num(13), "Balanced");
        assert_eq!(balanced_num(432), "Not Balanced");
        assert_eq!(balanced_num(424), "Balanced");
    }

    #[test]
    fn larger_number() {
        assert_eq!(balanced_num(1024), "Not Balanced");
        assert_eq!(balanced_num(66545), "Not Balanced");
        assert_eq!(balanced_num(295591), "Not Balanced");
        assert_eq!(balanced_num(1230987), "Not Balanced");
        assert_eq!(balanced_num(56239814), "Balanced");
    }
}

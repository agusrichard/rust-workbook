fn main() {
    let result1 = word_value(&["abc", "abc", "abc", "abc"]);
    println!("{:?}", result1);
    let result2 = word_value(&["codewars", "abc", "xyz"]);
    println!("{:?}", result2);
}

fn word_value(words: &[&str]) -> Vec<i32> {
    words.iter().enumerate().map(|(i, &word)| {
        let score: i32 = word.chars().filter(|c| c.is_alphabetic()).map(|c| c as i32 - 'a' as i32 + 1).sum();
        (i as i32 + 1) * score
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tests() {
        assert_eq!(word_value(&["abc", "abc abc"]), [6, 24]);
        assert_eq!(word_value(&["abc", "abc", "abc", "abc"]), [6, 12, 18, 24]);
        assert_eq!(word_value(&["codewars", "abc", "xyz"]), [88, 12, 225]);
    }
}

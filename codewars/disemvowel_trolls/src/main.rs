fn main() {
    let result = disemvowel("This website is for losers LOL!");
    println!("{result}");
}

fn disemvowel(s: &str) -> String {
    s.chars().filter(|c| !"aeiouAEIOU".contains(*c)).collect()
}

#[cfg(test)]
mod tests {
    use super::disemvowel;

    #[test]
    fn test_example() {
        assert_eq!(
            disemvowel("This website is for losers LOL!"),
            "Ths wbst s fr lsrs LL!"
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(disemvowel(""), "");
    }

    #[test]
    fn test_no_vowels() {
        assert_eq!(disemvowel("bcd fgh jkl"), "bcd fgh jkl");
    }

    #[test]
    fn test_all_vowels() {
        assert_eq!(disemvowel("aeiouAEIOU"), "");
    }

    #[test]
    fn test_mixed_content() {
        assert_eq!(
            disemvowel("No offense but,\nYour writing is among the worst I've ever read"),
            "N ffns bt,\nYr wrtng s mng th wrst 'v vr rd"
        );
        assert_eq!(
            disemvowel("What are you, a communist?"),
            "Wht r y,  cmmnst?"
        );
    }
}

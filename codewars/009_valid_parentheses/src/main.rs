fn main() {
    println!("{}", valid_parentheses("((()))"));
    println!("{}", valid_parentheses("((())"));
    println!("{}", valid_parentheses(")("));
    println!("{}", valid_parentheses(""));
    println!("{}", valid_parentheses("((()(()))())()(())(()))"));
    println!("{}", valid_parentheses("))()((()()(())))"));
    println!("{}", valid_parentheses("()"));
}

fn valid_parentheses_v2(parens: &str) -> bool {
    let mut depth: i32 = 0;
    for c in parens.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

fn valid_parentheses(parens: &str) -> bool {
    match parens.chars().next() {
        Some(c) => {
            if c == ')' {
                return false;
            }
        }
        _ => {
            return true;
        }
    }

    let mut stacks: Vec<char> = vec![];
    for c in parens.chars() {
        if c == '(' {
            stacks.push(c);
        } else if stacks.len() != 0 && c == ')' {
            stacks.pop();
        } else {
            return false;
        }
    }

    if stacks.is_empty() {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn do_test(expected: bool, input: &str) {
        assert_eq!(
            valid_parentheses(input),
            expected,
            "\nYour result (left) did not match the expected output (right) for the input: {input:?}"
        );
    }

    #[test]
    fn valid_cases() {
        do_test(true, "()");
        do_test(true, "((()))");
        do_test(true, "()()()");
        do_test(true, "(()())()");
        do_test(true, "()(())((()))(())()");
    }

    #[test]
    fn invalid_cases() {
        do_test(false, ")(");
        do_test(false, "()()(");
        do_test(false, "((())");
        do_test(false, "())(()");
        do_test(false, ")()");
        do_test(false, ")");
    }

    #[test]
    fn empty_string() {
        do_test(true, "");
    }

    fn do_test_v2(expected: bool, input: &str) {
        assert_eq!(
            valid_parentheses_v2(input),
            expected,
            "\nYour result (left) did not match the expected output (right) for the input: {input:?}"
        );
    }

    #[test]
    fn valid_cases_v2() {
        do_test_v2(true, "()");
        do_test_v2(true, "((()))");
        do_test_v2(true, "()()()");
        do_test_v2(true, "(()())()");
        do_test_v2(true, "()(())((()))(())()");
    }

    #[test]
    fn invalid_cases_v2() {
        do_test_v2(false, ")(");
        do_test_v2(false, "()()(");
        do_test_v2(false, "((())");
        do_test_v2(false, "())(()");
        do_test_v2(false, ")()");
        do_test_v2(false, ")");
    }

    #[test]
    fn empty_string_v2() {
        do_test_v2(true, "");
    }
}

use std::collections::HashMap;

fn run_vector() {
    let mut v: Vec<i32> = vec![1, 2, 3];

    v.push(4);
    v.push(5);

    let first = &v[0];
    println!("{:?}", v);
    println!("{}", first);

    let second = v.get(1).unwrap_or(&0);
    println!("{}", second);

    let third_squared = match v.get(2) {
        Some(&x) => x.pow(2),
        _ => 0
    };
    println!("{}", third_squared);

    for i in &v {
        println!("{}", i);
    }

    for i in &mut v {
        *i *= 2;
    }

    println!("{:?}", v);

}

fn run_string() {
    let mut str1 = String::new();
    str1.push_str("Something");
    println!("{str1}");

    let str2 = "Damara Astiningtyas".to_string();
    println!("{str2}");

    let str3 = String::from("Hey Jude");
    println!("{str3}");

    let title = format!("{str1} and {str3}");
    println!("{title}");

    for c in str2.chars() {
        println!("{c}")
    }
}

fn run_hash_map() {
    use std::collections::HashMap;

    let mut numbers = HashMap::new();
    numbers.insert(String::from("One"), 1);
    numbers.insert(String::from("Two"), 2);
    println!("{:?}", numbers);

    match numbers.get(&String::from("One")) {
        Some(i) => println!("{i}"),
        _ => println!("Nothing")
    }

    let result = numbers.get(&String::from("One")).copied().unwrap_or(0);
    println!("{result}");

    for (k, v) in &numbers {
        println!("{k}-{v}");
    }

    let richard = String::from("Richard");
    let damara = String::from("Damara");

    let mut couples = HashMap::new();
    couples.insert(&richard, &damara);
    println!("{richard}");
    println!("{damara}");
    println!("{:?}", couples);

    let sentence = "hello world richard damara hello richard world damara world";
    let mut maps = HashMap::new();
    for word in sentence.split_whitespace() {
        let count = maps.entry(word).or_insert(0);
        *count += 1;
    }
    println!("{:?}", maps);
}

fn main() {
    run_vector();
    run_string();
    run_hash_map();
}

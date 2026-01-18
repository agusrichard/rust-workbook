fn main() {
    // println!("Hello, world!");

    // let s = return_a_static_string();
    // println!("{}", s);

    // // 1. Create a destination vector with some initial data
    // let mut my_dst = vec![
    //     String::from("cat"),
    //     String::from("sheep"), // Max length is 5
    // ];

    // // 2. Create a source slice
    // let my_src = vec![
    //     String::from("cow"),          // len 3 (too short)
    //     String::from("elephant"),     // len 8 (BIGGER! will add)
    //     String::from("dog"),          // len 3 (too short)
    //     String::from("hippopotamus"), // len 12 (BIGGER! will add)
    // ];

    // // 3. Call the function
    // add_big_strings(&mut my_dst, &my_src);

    // // 4. Print the result
    // // Expected: cat, sheep, elephant, hippopotamus
    // println!("Result: {:?}", my_dst);

    // let num = 1;
    // let ref_num_1 = &num;
    // let ref_num_2 = &num;
    // println!("{} {}", ref_num_1, ref_num_2);

    // let mut name = (String::from("Ferris"), String::from("Rustacean"));
    // let first = &name.0;
    // name.1.push_str(", Esq.");
    // println!("{first} {}", name.1);

    // let mut v = vec![1, 2, 3];
    // copy_to_prev(&mut v, 1);

    // let name = String::from("Ferris");
    // award_phd(&name);
    // println!("{}", name);

    let mut point = [0, 1];
    let mut x = point[0];
    let y = &mut point[1];
    x += 1;
    *y += 1;
    println!("{} {}", point[0], point[1]);
}

fn return_a_static_string() -> &'static str {
    "Hello world from a function"
}

/// Adds "Ph.D." to a person's name
// fn award_phd(name: &String) {
//     let mut name = *name;
//     name.push_str(", Ph.D.");
// }

fn add_big_strings(dst: &mut Vec<String>, src: &[String]) {
    let largest_len: usize = dst.iter().map(|s| s.len()).max().unwrap_or(0);
    for s in src {
        if s.len() > largest_len {
            dst.push(s.clone());
        }
    }
}

// fn copy_to_prev(v: &mut Vec<i32>, i: usize) {
// let n = &mut v[i];
// *n = v[i - 1];
// }

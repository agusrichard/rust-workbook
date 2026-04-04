use std::io;

fn main() {
    let tup: (u32, i32, char) = (1, 1, 'a');
    println!("{:?}", tup);

    let (x, y, z) = tup;
    println!("The value of y is: {y}");

    let first_val = tup.0;
    println!("The value of first_val is: {first_val}");

    let mut x: (i32, i32) = (1, 2);
    x.0 = 0;
    x.1 += 5;
    println!("{:?}", x);

    let a = [1, 2, 3, 4, 5];
    println!("{:?}", a);
    println!("{:?}", a[3]);

    println!("Please enter an array index!");

    let mut index = String::new();

    io::stdin()
        .read_line(&mut index)
        .expect("Failed to read line");

    let index: usize = index
        .trim()
        .parse()
        .expect("Index entered was not a number");

    let element = a[index];

    println!("The value of the element at index {index} is: {element}");
}

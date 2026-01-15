fn main() {
    println!("Hello, world!");
    another_function(42, 'A');

    let y = {
        let x = 3;
        x + 1
    };

    println!("The value of y is: {y}");
    println!("Add one value {}", add_one(9));
}

fn another_function(x: i32, y: char) {
    println!("Another function has x = {x}, y = {y}");
}

fn add_one(x: i32) -> i32 {
    x + 1
}

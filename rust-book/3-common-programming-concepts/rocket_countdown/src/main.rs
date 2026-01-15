fn main() {
    let mut count = 10;

    while count > 0 {
        println!("T-minus {}", count);
        count -= 1;
    }
    println!("Liftoff!");
}

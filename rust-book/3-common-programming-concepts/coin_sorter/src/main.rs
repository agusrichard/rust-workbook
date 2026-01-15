fn main() {
    let coin = 10;

    let y = match coin {
        1 => "Penny",
        5 => "Nickel",
        10 => "Dime",
        25 => "Quarter",
        _ => "Unknown Coin",
    };

    println!("{}", y);
}

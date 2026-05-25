pub fn run() {
    let some_option_value: Option<i32> = None;
    let Some(x) = some_option_value else {return};
    if let Some(y) = some_option_value {
        println!("{},", y);
    }
}
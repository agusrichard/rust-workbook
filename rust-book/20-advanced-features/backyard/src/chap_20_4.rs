fn square(x: i32) -> i32 {
    x * x
}


fn do_twice(f: fn(i32) -> i32, arg: i32) -> i32 {
    f(arg) + f(arg)
}

#[derive(Debug)]
enum Status {
    Value(u32),
    Stop,
}

fn create_list_of_operations() -> Vec<Box<dyn Fn(i32) -> i32>>{
    vec![Box::new(|x| x + 1), Box::new(|x| x * 2)]
}


pub fn run() {
    let result = do_twice(square, 5);
    println!("Square value: {}", result);

    // Using a closure
    let list_of_numbers = vec![1, 2, 3];
    let list_of_strings: Vec<String> =
        list_of_numbers.iter().map(|i| i.to_string()).collect();
    println!("List of numbers: {:?}", list_of_numbers);

    // Using a function pointer
    let list_of_numbers = vec![1, 2, 3];
    let list_of_strings: Vec<String> =
        list_of_numbers.iter().map(ToString::to_string).collect();
    println!("List of numbers: {:?}", list_of_numbers);

    let list_of_statuses: Vec<Status> = (0u32..20).map(Status::Value).collect();
    println!("List of status: {:?}", list_of_statuses);

    let operations = create_list_of_operations();
    let mut value = 5;
    for op in &operations {
        value = op(value);
    }
    println!("value: {}", value);
}

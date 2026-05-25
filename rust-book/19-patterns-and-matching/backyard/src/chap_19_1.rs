enum RGB {
    Red(u16),
    Green(u16),
    Blue(u16)
}

fn print_coordinates(&(x, y): &(i32, i32)) {
    println!("Current location: ({},{})", x, y);
}

pub fn run() {
    let x = Some(10);
    let fun = move |x| {
        match x {
            Some(x) => format!("Good day. it's x={x}"),
            None => "Good day, it's None".to_string(),
        }
    };

    let result = fun(x);
    println!("result: {:?}", result);

    let green = RGB::Green(0);
    if let RGB::Green(r) = green {
        println!("Green: {:?}", r);
    }

    let point = (3, 5);
    print_coordinates(&point);
}
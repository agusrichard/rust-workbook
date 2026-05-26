pub fn run() {
    let x = Some(5);
    let y = 10;

    match x {
        Some(50) => println!("Got 50"),
        Some(y) => println!("Matched, y = {y}"),   // new `y`, shadows outer `y`
        _ => println!("Default case, x = {x:?}"),
    }

    println!("at the end: x = {x:?}, y = {y}");    // outer `y` is still 10

    let x = 4;
    match x {
        4 | 5 | 6 => println!("Got 4 or 5 or 6"),
        _ => println!("Default case, x = {x:?}"),
    }

    match x {
        1..=5 => println!("one through five"),
        _ => println!("something else"),
    }

    let x = Some(5);
    let y = 10;

    match x {
        Some(50) => println!("Got 50"),
        Some(n) if n == y => println!("Matched, n = {n}"),   // compares to outer y
        _ => println!("Default case, x = {x:?}"),
    }

    println!("at the end: x = {x:?}, y = {y}");

    enum Message {
        Hello { id: i32 },
    }

    let msg = Message::Hello { id: 5 };

    match msg {
        Message::Hello { id: id @ 3..=7 } => {
            println!("Found an id in range: {id}")   // id is bound AND tested
        }
        Message::Hello { id: 10..=12 } => {
            println!("Found an id in another range")  // id not captured, can't use it
        }
        Message::Hello { id } => println!("Found some other id: {id}"),
    }
}
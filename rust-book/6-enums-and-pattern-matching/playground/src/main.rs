#[derive(Debug)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(i32, i32, i32),
}

impl Message {
    fn call(&self) {
        println!("Something is here {:?}", self);
    }
}

fn main() {
    let m = Message::Move { x: 10, y: 10 };
    m.call()
}

use std::ops::Add;
use std::fmt;


trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}


struct Counter {
    count: u32,
    limit: u32
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.limit {
            self.count += 1;
            Some(self.count)
        } else {
            None
        }
    }
}

trait OutlinePrint: fmt::Display {
    fn outline_print(&self) {
        let output = self.to_string();  // uses Display::to_string
        let len = output.len();
        println!("{}", "*".repeat(len + 4));
        println!("*{}*", " ".repeat(len + 2));
        println!("* {output} *");
        println!("*{}*", " ".repeat(len + 2));
        println!("{}", "*".repeat(len + 4));
    }
}


impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl OutlinePrint for Point {}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Point {
    x: i32,
    y: i32,
    z: i32
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z
        }
    }
}

trait Pilot {
    fn fly(&self);
}

trait Wizard {
    fn fly(&self);
}

struct Human;

impl Pilot for Human {
    fn fly(&self) {
        println!("This is your captain speaking.");
    }
}


impl Wizard for Human {
    fn fly(&self) {
        println!("Up!");
    }
}


impl Human {
    fn fly(&self) {
        println!("*waving arms furiously*");
    }
}


trait Animal {
    fn baby_name() -> String;
}

struct Dog;

impl Dog {
    fn baby_name() -> String {
        String::from("Spot")
    }
}

impl Animal for Dog {
    fn baby_name() -> String {
        String::from("puppy")
    }
}

struct Wrapper(Vec<String>);

impl fmt::Display for Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]", self.0.join(", "))
    }
}

pub fn run() {
    let mut counter = Counter { count: 0, limit: 10 };
    while let Some(i) = counter.next() {
        println!("{}", i);
    }

    let p1 = Point { x: 1, y: 2, z: 3 };
    let p2 = Point { x: -1, y: -2, z: -3 };
    let p3 = p1 + p2;
    println!("{:?}", p3);
    p3.outline_print();

    let person = Human;
    Pilot::fly(&person);
    Wizard::fly(&person);
    person.fly();

    Dog::baby_name();
    <Dog as Animal>::baby_name();

    let w = Wrapper(vec![String::from("hello"), String::from("world")]);
    println!("w = {w}");  // → w = [hello, world]
}
use std::ops::Deref;

struct MyBox<T>(T);

impl<T> MyBox<T> {
    fn new(x: T) -> MyBox<T> {
        MyBox(x)
    }
}

impl<T> Deref for MyBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


struct MyPoint {
    x: i32,
    y: i32
}

impl Drop for MyPoint {
    fn drop(&mut self) {
        println!("Dropping MyPoint with coordinate {} and {}", self.x, self.y);
    }
}


fn move_closure() {
    let x = 5;
    let y = Box::new(x);
    let s = String::from("hello");

    println!("{x}");
    println!("{y}");

    let f = move || println!("{x}");
    let g = move || println!("{s}");
    f();
    g();
    println!("{x}");
    // println!("{s}");
}

fn impl_deref() {
    let x = 5;
    let y = MyBox::new(x);

    assert_eq!(5, x);
    assert_eq!(5, *y);  // now works!
}

fn impl_drop() {
    let x = MyPoint{x: 10, y: 10};
    let y = MyPoint{x: 1, y: 1};

    println!("impl_drop is donw");
}

fn impl_drop2() {
    let x = MyPoint{x: 10, y: 10};

    drop(x);
    println!("impl_drop2 is done");
}

fn main() {
    move_closure();
    impl_deref();
    impl_drop();
    impl_drop2();
}

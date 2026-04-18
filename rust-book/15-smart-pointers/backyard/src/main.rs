fn main() {
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

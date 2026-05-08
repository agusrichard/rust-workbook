use std::sync::{Arc, Mutex};
use std::thread;

fn mutex() {
    let m = Mutex::new(0);

    for i in 0..5 {
        let mut num = m.lock().unwrap();
        *num += i;
    }

    println!("m: {}", m.lock().unwrap());
}

fn arc_mutex() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for i in 0..10 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            println!("num: {num}");
            *num += i;
        });
        handles.push(handle);
    }

    for handle in handles {
        let r = handle.join().unwrap();
    }

    println!("Result {}", *counter.lock().unwrap());
    println!("Sum {}", (0..10).sum::<i32>());
}

fn main() {
    // mutex();
    arc_mutex();
}

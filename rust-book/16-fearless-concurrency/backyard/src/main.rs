use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

fn threads() {
    let handler1 = thread::spawn(|| {
        for i in 1..10 {
            println!("number {i} from the spawned thread 1");
            thread::sleep(Duration::from_millis(1));
        }
    });

    let handler2 = thread::spawn(|| {
        let mut result = 0;
        for i in 1..10 {
            println!("number {i} from the spawned thread 2");
            thread::sleep(Duration::from_millis(1));
            result += i;
        };
        result
    });

    for i in 1..5 {
        println!("number {i} from the main thread");
        thread::sleep(Duration::from_millis(1));
    };

    handler1.join().unwrap();
    let handler2_result = handler2.join().unwrap();
    println!("handler1_result: {handler2_result}");

}

fn single_iterable_producer() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        for i in 1..5 {
            tx.send(i).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });

    for i in rx {
        println!("{i}");
    }
}

fn producer(tx: Sender<i32>, start: i32, end: i32) {
    thread::spawn(move || {
        for i in start..end {
            tx.send(i).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });
}

fn consumer(rx: Receiver<i32>) {
    for i in rx {
        println!("Received: {i}");
    }
}

fn multiple_consumers() {
    let (tx1, rx) = mpsc::channel();

    let tx2 = tx1.clone();

    producer(tx1, 0, 5);
    producer(tx2, 5, 10);
    consumer(rx);
}

fn main() {
    // threads();
    // single_iterable_producer();
    multiple_consumers();
}

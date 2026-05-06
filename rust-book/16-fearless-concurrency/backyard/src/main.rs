use std::thread;
use std::time::Duration;

fn main() {
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

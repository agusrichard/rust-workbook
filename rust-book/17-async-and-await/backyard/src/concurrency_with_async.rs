use std::time::Duration;

pub fn sequential() {
    trpl::block_on(async {
        let f1 = async {
            for i in 1..5 {
                println!("f1 {i}");
            }
        };

        let f2 = async {
            for i in 1..5 {
                println!("f2 {i}");
            }
        };

        f1.await;
        f2.await;
    })
}

pub fn blocking_single_task() {
    trpl::block_on(async {
        let handle = trpl::spawn_task(async {
            for i in 1..10 {
                println!("hi number {i} from the first task!");
                trpl::sleep(Duration::from_millis(500)).await;
            }
        });

        for i in 1..5 {
            println!("hi number {i} from the second task!");
            trpl::sleep(Duration::from_millis(500)).await;
        }

        handle.await.unwrap();
    });}

pub fn concurrent() {
    trpl::block_on(async {
        let f1 = async {
            for i in 0..5 {
                println!("f1 {i}");
                trpl::sleep(Duration::from_secs(1)).await;
            }
        };

        let f2 = async {
            for i in 0..5 {
                println!("f2 {i}");
                trpl::sleep(Duration::from_secs(1)).await;
            }
        };

        trpl::join(f1, f2).await;
    })
}


pub fn run() {
    // sequential();
    // concurrent();
    blocking_single_task();
}
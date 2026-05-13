use std::{thread, time::Duration};


fn slow(name: &str, ms: u64) {
    thread::sleep(Duration::from_millis(ms));  // blocks the thread — no await!
    println!("'{name}' ran for {ms}ms");
}

fn mostly_sequential() {
    trpl::block_on(async {
        let a = async {
            println!("'a' started.");
            slow("a", 30);
            slow("a", 10);
            slow("a", 20);
            trpl::sleep(Duration::from_millis(50)).await;  // FIRST yield in 'a'
            println!("'a' finished.");
        };

        let b = async {
            println!("'b' started.");
            slow("b", 75);
            slow("b", 10);
            slow("b", 15);
            slow("b", 350);
            trpl::sleep(Duration::from_millis(50)).await;  // FIRST yield in 'b'
            println!("'b' finished.");
        };

        trpl::select(a, b).await;
    });
}

fn alternating() {
    trpl::block_on(async {
        let one_ms = Duration::from_millis(1);

        let a = async {
            println!("'a' started.");
            slow("a", 30);
            trpl::sleep(one_ms).await;   // yield → runtime can switch to 'b'
            slow("a", 10);
            trpl::sleep(one_ms).await;
            slow("a", 20);
            trpl::sleep(one_ms).await;
            println!("'a' finished.");
        };

        let b = async {
            println!("'b' started.");
            slow("b", 75);
            trpl::sleep(one_ms).await;
            slow("b", 10);
            trpl::sleep(one_ms).await;
            slow("b", 15);
            trpl::sleep(one_ms).await;
            slow("b", 350);
            trpl::sleep(one_ms).await;
            println!("'b' finished.");
        };

        trpl::select(a, b).await;
    });
}

pub fn run() {
    // mostly_sequential();
    alternating();
}
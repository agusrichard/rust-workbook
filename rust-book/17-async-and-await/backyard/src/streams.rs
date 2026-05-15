use trpl::StreamExt;

pub fn run() {
    trpl::block_on(async {
        let values = (0..10).map(|x| x * 2);
        let mut stream = trpl::stream_from_iter(values);

        while let Some(value) = stream.next().await {
            println!("{value}");
        }
    })
}
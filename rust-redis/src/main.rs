use redis::Commands;

fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let mut con = client.get_connection()?;

    let _: () = con.set_ex("greeting", "hello", 60)?;
    let value: Option<String> = con.get("greeting")?;
    println!("{value:?}");

    Ok(())
}

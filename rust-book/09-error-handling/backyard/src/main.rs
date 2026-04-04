use std::io;
use std::fs::File;
use std::io::{ErrorKind, Read};

fn read_username_from_file() -> Result<String, io::Error> {
    let mut file = File::open("username.txt")?;
    let mut username = String::new();
    file.read_to_string(&mut username);
    Ok(username.clone())

}

fn main() {
    let greeting_file = match File::open("hello.txt") {
        Ok(file) => file,
        Err(error) => match error.kind() {
            ErrorKind::NotFound => match File::create("hello.txt") {
                Ok(file) => file,
                Err(e) => panic!("Problem when creating the file: {e:?}")
            },
            other_error => panic!("Problem when opening the file: {other_error:?}")
        }
    };

    let hi_file = File::open("hi.txt").unwrap_or_else(|err| {
        if err.kind() == ErrorKind::NotFound {
            File::create("hi.txt").unwrap_or_else(|e| {
                panic!("Error when creating the file: {e:?}");
            })
        } else {
            panic!("Error when opening the file: {err:?}");
        }
    });

    let username = read_username_from_file().expect("Error when reading the username file");
    println!("{username}")
}

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("Connection established!");
        handle_connection(stream);
    }
}

fn generate_response(status_line: &str, html_file: &str) -> String {
    let contents = fs::read_to_string(html_file).unwrap();
    let length = contents.len();

    format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}")

}


fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let mut response = String::new();
    if request_line == "GET / HTTP/1.1" {
        response = generate_response("HTTP/1.1 200 OK", "hello.html")
    } else {
        response = generate_response("HTTP/1.1 404 NOT FOUND", "404.html");
    }

    stream.write_all(response.as_bytes()).unwrap();
}

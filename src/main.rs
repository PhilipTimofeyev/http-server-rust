#[allow(unused_imports)]
use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();
    let echo = request_line.split("echo/");
    let content = echo.last().unwrap().split_whitespace().next();

    let (status_line, content) = if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK", "")
        } else if content.is_some() {
        println!("{}",content.unwrap());
            ("HTTP/1.1 200 OK", content.unwrap())
        } else {
            ("HTTP/1.1 404 Not Found", "")
        };

    let content_length = content.len();
    let response = format!("{status_line}\r\nContent-Type: text/plain\r\nContent-Length: {content_length}\r\n\r\n{content}");

    stream.write_all(response.as_bytes()).unwrap();
}

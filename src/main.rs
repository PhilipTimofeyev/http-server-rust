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
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let start_line = &http_request[0];
    let start_line: Vec<_> = start_line.split(" ").collect(); 
    
    let user_agent = http_request.iter().find(|el| el.contains("User-Agent"));
    
    let mut status_line = "HTTP/1.1 200 OK";
    let content = match start_line[1] {
        "/" =>  "",
        "/user-agent" =>  {
            let (_key, user_agent) = user_agent.unwrap().split_once(": ").unwrap();
            user_agent
        },
        _ => {
            if start_line[1].contains("/echo") {
                let echo = start_line[1].split("/echo/").last().unwrap();
                echo
            } else {
                status_line = "HTTP/1.1 404 Not Found";
                ""
            }

        }
    };

    let content_length = content.len();
    let response = format!("{status_line}\r\nContent-Type: text/plain\r\nContent-Length: {content_length}\r\n\r\n{content}");

    stream.write_all(response.as_bytes()).unwrap();
}


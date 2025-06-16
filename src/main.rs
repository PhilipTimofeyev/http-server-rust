#[allow(unused_imports)]
use std::{
    env,
    path::Path,
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};
use std::sync::Arc;
use codecrafters_http_server::ThreadPool;

fn main() {
    println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    let dir = parse_args(args);
    let dir = Arc::new(dir);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                let dir = Arc::clone(&dir);
                pool.execute(move|| {
                    handle_connection(stream, dir);
                    });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn parse_args(args: Vec<String>) -> String {
    if args.iter().any(|arg| *arg == "--directory") {
        let dir = args.iter().last().unwrap();
        dir.to_string()
    } else {
        "".to_string()
    }
} 


fn handle_connection(mut stream: TcpStream, dir: Arc<String>) {
    let buf_reader = BufReader::new(&stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
        
    let start_line = &http_request[0];
    let start_line: Vec<_> = start_line.split(" ").collect(); 
    let mut content_type = "text/plain".to_string();
    
    let user_agent = http_request.iter().find(|el| el.contains("User-Agent"));
    
    let mut status_line = "HTTP/1.1 200 OK";
    let content: String = match start_line[1] {
        "/" =>  "".to_string(),
        "/user-agent" =>  {
            let (_key, user_agent) = user_agent.unwrap().split_once(": ").unwrap();
            user_agent.to_string()
        },
        _ => {
            if start_line[1].contains("/echo") {
                let echo = start_line[1].split("/echo/").last().unwrap();
                echo.to_string()
            } else if start_line[1].contains("/files") {
                let file = start_line[1].split("/files/").last().unwrap();
                let filepath = format!("{dir}{file}");
                let filepath = Path::new(&filepath);
                let read_file = fs::read_to_string(filepath);

                if let Ok(read_file) = read_file {
                    content_type = "application/octet-stream".to_string();
                    read_file.to_string()
                } else {
                     status_line = "HTTP/1.1 404 Not Found";
                     "".to_string()
                }
            } else {
                status_line = "HTTP/1.1 404 Not Found";
                "".to_string()  
            }

        }
    };

    let content_length = content.len();
    let response = format!("{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n\r\n{content}");

    stream.write_all(response.as_bytes()).unwrap();
}


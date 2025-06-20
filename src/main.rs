use codecrafters_http_server::ThreadPool;
use std::{
    env,
    io::{prelude::*, BufReader},
    net::{Shutdown, TcpListener, TcpStream},
    sync::Arc,
};
pub mod codec;
pub mod request;
pub mod response;
pub mod status;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir_arg = parse_args(args);
    let shared_dir = Arc::new(dir_arg);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("accepted new connection");
        let dir = Arc::clone(&shared_dir);
        pool.execute(move || {
            handle_connection(stream, dir);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream, dir: Arc<String>) {
    let mut buf_reader = BufReader::new(stream.try_clone().unwrap());

    loop {
        let request = buf_reader.fill_buf().unwrap();
        let length = request.len();

        let headers_present = request.windows(4).any(|w| w == b"\r\n\r\n");
        if !headers_present {
            buf_reader.consume(length);
            continue;
        };

        let mut request = request::parse_request(request);
        let mut response = request.handle_request(&dir);

        codec::encode(&mut response);

        buf_reader.consume(length);

        let connection_is_closed = response.headers.connection == Some("close".to_string());

        let response_header = response.build_response_header();
        let response_body = response.body.unwrap_or_default();

        stream.write_all(response_header.as_bytes()).unwrap();
        stream.write_all(&response_body).unwrap();

        if connection_is_closed {
            stream.shutdown(Shutdown::Both).unwrap()
        };
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



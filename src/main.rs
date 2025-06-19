use codecrafters_http_server::ThreadPool;
use std::{
    env,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    sync::Arc,
};
pub mod codec;
pub mod request;
pub mod response;
pub mod status;

fn main() {
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
                pool.execute(move || {
                    handle_connection(stream, dir);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, dir: Arc<String>) {
    let mut buf_reader = BufReader::new(&stream);
    let request = buf_reader.fill_buf().unwrap();
    let length = request.len();

    let mut request = request::parse_request(request);
    let mut response = request.handle_request(&dir);

    if let Some(encoder) = &response.headers.content_encoding {
        match encoder {
            codec::Encoder::Gzip => {
                if let Some(body) = response.body.take() {
                    let encoded = codec::gzip_encoder(body);
                    let content_length = encoded.len();

                    response.body = Some(encoded);
                    response.headers.content_length = Some(content_length);
                }
            }
        };
    };

    buf_reader.consume(length);

    let response_header = response.build_response_header();
    let response_body = response.body.unwrap_or_default();

    stream.write_all(response_header.as_bytes()).unwrap();
    stream.write_all(&response_body).unwrap()
}

fn parse_args(args: Vec<String>) -> String {
    if args.iter().any(|arg| *arg == "--directory") {
        let dir = args.iter().last().unwrap();
        dir.to_string()
    } else {
        "".to_string()
    }
}

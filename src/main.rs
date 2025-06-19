use codecrafters_http_server::ThreadPool;
use serde::de::{self, Deserializer};
use std::{
    collections::HashMap,
    env, fmt, fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::Arc,
};
pub mod codec;
pub mod request;
pub mod status;

use request::request::Request;
// use request::Encoder;
// use request::request::Headers;

// use request::StatusCode;

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

    let mut request = parse_request(request);
    let mut response = request.handle_request(&dir);
    //
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

fn parse_request(request: &[u8]) -> Request {
    let mut headers: Vec<String> = request
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let request_line = headers.remove(0);
    let request_line = parse_request_line(request_line);

    let headers = parse_headers(headers);
    let json_request = serde_json::to_string(&headers).unwrap();
    let headers: request::request::Headers = serde_json::from_str(&json_request).unwrap();

    let body_start = request.len() - headers.content_length.unwrap_or(0);
    let body_end = body_start + headers.content_length.unwrap_or(0);

    let body = &request[body_start..body_end];
    let body = body.to_vec();

    Request {
        request_line,
        headers,
        body,
    }
}

fn parse_headers(headers: Vec<String>) -> HashMap<String, String> {
    let mut headers_hash: HashMap<String, String> = HashMap::new();

    for header in &headers[1..] {
        let header = header.split_once(": ");
        match header {
            Some(header) => {
                let (key, value) = header;
                headers_hash.insert(key.to_string(), value.to_string())
            }
            None => continue,
        };
    }

    headers_hash
}

fn parse_request_line(request_line: String) -> request::request::RequestLine {
    let mut request_line: Vec<&str> = request_line.split_whitespace().collect();
    let (_html, endpoint, verb) = (
        request_line.pop().unwrap().to_string(),
        request_line.pop().unwrap().to_string(),
        request_line.pop().unwrap(),
    );

    let verb = match verb {
        "GET" => request::request::Verb::GET,
        "POST" => request::request::Verb::POST,
        &_ => todo!(),
    };

    request::request::RequestLine { verb, endpoint }
}

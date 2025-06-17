use codecrafters_http_server::ThreadPool;
use std::{
    env, fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::Arc,
};

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

    let (mut headers, body) = parse_request(request);

    let start_line = headers.remove(0);
    let start_line = parse_start_line(start_line);

    let (content, status) = parse_endpoint(start_line, &headers[..], &dir, &body);

    let response = format!(
        "{}\r\nContent-Type: {}\r\nContent-Encoding: {}\r\nContent-Length: {}\r\n\r\n{}",
        status.as_str(),
        content.content_type,
        content.encoding,
        content.length,
        content.content
    );

    stream.write_all(response.as_bytes()).unwrap();
}

enum StatusCode {
    _200,
    _404,
    _201,
}

impl StatusCode {
    fn as_str(&self) -> &'static str {
        match self {
            StatusCode::_200 => "HTTP/1.1 200 OK",
            StatusCode::_404 => "HTTP/1.1 404 Not Found",
            StatusCode::_201 => "HTTP/1.1 201 Created",
        }
    }
}

#[derive(PartialEq)]
enum Verb {
    GET,
    POST,
}

fn parse_args(args: Vec<String>) -> String {
    if args.iter().any(|arg| *arg == "--directory") {
        let dir = args.iter().last().unwrap();
        dir.to_string()
    } else {
        "".to_string()
    }
}

fn parse_request(request: &[u8]) -> (Vec<String>, String) {
    let mut request: Vec<String> = request.lines().map(|result| result.unwrap()).collect();

    let body = request.split_off(request.len() - 1).pop().unwrap();
    let headers = request;

    (headers, body)
}

fn parse_start_line(start_line: String) -> StartLine {
    let mut start_line: Vec<&str> = start_line.split_whitespace().collect();

    let (_html, endpoint, verb) = (
        start_line.pop().unwrap().to_string(),
        start_line.pop().unwrap().to_string(),
        start_line.pop().unwrap(),
    );

    let verb = match verb {
        "GET" => Verb::GET,
        "POST" => Verb::POST,
        &_ => todo!(),
    };

    StartLine { verb, endpoint }
}

fn parse_content_type(headers: &[String]) -> String {
    if let Some(content_type) = headers
        .iter()
        .find(|header| header.contains("Content-Type"))
    {
        let (_key, content_type) = content_type.split_once(": ").unwrap();
        content_type.to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

fn parse_encoding(headers: &[String]) -> String {
    if let Some(encoding) = headers
        .iter()
        .find(|header| header.contains("Accept-Encoding"))
    {
        let (_key, encoding) = encoding.split_once(": ").unwrap();
        match encoding {
            "gzip" => encoding.to_string(),
            _ => "".to_string()

        }
    } else {
        "".to_string()
    }
}

fn parse_endpoint(
    start_line: StartLine,
    headers: &[String],
    dir: &str,
    body: &str,
) -> (Content, StatusCode) {
    let encoding = parse_encoding(headers);
    let (content, status) = match start_line.endpoint.as_str() {
        "/" => (Content::new("", "", &encoding), StatusCode::_200),
        "/user-agent" => {
            let user_agent = headers.iter().find(|el| el.contains("User-Agent"));
            let (_key, user_agent) = user_agent.unwrap().split_once(": ").unwrap();
            let content = Content::new(user_agent, "text/plain", &encoding);
            (content, StatusCode::_200)
        }
        endpoint if endpoint.contains("/echo") => {
            let message = endpoint.split("/echo/").last().unwrap();
            let content = Content::new(message, "text/plain", &encoding);
            (content, StatusCode::_200)
        }
        endpoint if endpoint.contains("/files") => {
            let filename = endpoint.split("/files/").last().unwrap();
            let filepath = format!("{dir}{filename}");
            let filepath = Path::new(&filepath);

            match start_line.verb {
                Verb::GET => {
                    let read_file = fs::read_to_string(filepath);
                    let content_type = parse_content_type(headers);
                    if let Ok(read_file) = read_file {
                        let content = Content::new(&read_file, &content_type, &encoding);
                        (content, StatusCode::_200)
                    } else {
                        (Content::new("", "", &encoding), StatusCode::_404)
                    }
                }
                Verb::POST => {
                    let _ = fs::write(filepath, body);
                    (Content::new("", "", &encoding), StatusCode::_201)
                }
            }
        }
        _ => {
            return (Content::new("", "", &encoding), StatusCode::_404);
        }
    };
    (content, status)
}

struct StartLine {
    verb: Verb,
    endpoint: String,
}

struct Content {
    content: String,
    content_type: String,
    length: usize,
    encoding: String
}

impl Content {
    fn new(content: &str, content_type: &str, encoding: &str) -> Content {
        let content = content.to_string();
        let content_type = content_type.to_string();
        let length = content.len();
        let encoding = encoding.to_string();

        Content {
            content,
            content_type,
            length,
            encoding,
        }
    }
}


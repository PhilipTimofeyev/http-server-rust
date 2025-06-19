use codecrafters_http_server::ThreadPool;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::de::{self, Deserializer};
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fmt, fs,
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
    let length = request.len();

    let mut request = parse_request(request);
    let status = request.parse_endpoint(&dir);

    if let Some(encoder) = &request.headers.accept_encoding {
        match encoder {
            Encoder::Gzip => request.gzip_encoder(),
        }
    }

    buf_reader.consume(length);
    let response = format!(
        "{}\r\nContent-Type: {}\r\nContent-Encoding: {}\r\nContent-Length: {}\r\n\r\n",
        status,
        request
            .headers
            .content_type
            .unwrap_or_else(|| "text/plain".to_string()),
        request
            .headers
            .accept_encoding
            .map_or("".to_string(), |a| a.to_string()),
        request.headers.content_length.unwrap_or_default(),
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.write_all(&request.body).unwrap()
}

#[derive(Debug)]
enum Encoder {
    Gzip,
}

struct Request {
    headers: Headers,
    body: Vec<u8>,
}

#[derive(Deserialize, Debug)]
struct Headers {
    #[serde(deserialize_with = "string_to_startline")]
    #[serde(rename = "Start-Line")]
    start_line: StartLine,
    #[serde(default)]
    #[serde(deserialize_with = "encoder_type")]
    #[serde(rename = "Accept-Encoding")]
    accept_encoding: Option<Encoder>,
    #[serde(rename = "User-Agent")]
    user_agent: Option<String>,
    #[serde(rename = "Content-Type")]
    content_type: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "string_to_option_usize")]
    #[serde(rename = "Content-Length")]
    content_length: Option<usize>,
}

fn encoder_type<'de, D>(deserializer: D) -> Result<Option<Encoder>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<&str> = Option::deserialize(deserializer)?;
    match s {
        Some(encoder) if !encoder.is_empty() => match encoder {
            encoder if encoder.contains("gzip") => Ok(Some(Encoder::Gzip)),
            &_ => Ok(None),
        },
        _ => Ok(None),
    }
}

impl fmt::Display for Encoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Encoder::Gzip => write!(f, "gzip"),
        }
    }
}

fn string_to_option_usize<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<&str> = Option::deserialize(deserializer)?;
    match s {
        Some(text) if !text.is_empty() => {
            text.parse::<usize>().map(Some).map_err(de::Error::custom)
        }
        _ => Ok(None),
    }
}

fn string_to_startline<'de, D>(deserializer: D) -> Result<StartLine, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    let mut start_line: Vec<&str> = s.split_whitespace().collect();
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

    Ok(StartLine { verb, endpoint })
}

impl Request {
    fn parse_endpoint(&mut self, dir: &str) -> StatusCode {
        match self.headers.start_line.endpoint.as_str() {
            "/" => StatusCode::_200,
            "/user-agent" => self.handle_user_agent(),
            endpoint if endpoint.contains("/echo") => self.handle_echo(),
            endpoint if endpoint.contains("/files") => {
                let filename = endpoint.split("/files/").last().unwrap();
                let filepath = format!("{dir}{filename}");
                let filepath = Path::new(&filepath);

                match self.headers.start_line.verb {
                    Verb::GET => self.handle_get_file(filepath),
                    Verb::POST => self.handle_post_file(filepath),
                }
            }
            _ => StatusCode::_404,
        }
    }

    fn handle_user_agent(&mut self) -> StatusCode {
        let user_agent = self.headers.user_agent.clone().unwrap_or_default();
        self.headers.content_length = Some(user_agent.len());
        self.body = user_agent.as_bytes().to_vec();
        StatusCode::_200
    }

    fn gzip_encoder(&mut self) {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&self.body).unwrap();
        let compressed_body = encoder.finish().unwrap();
        self.headers.content_length = Some(compressed_body.len());
        self.body = compressed_body;
    }

    fn handle_echo(&mut self) -> StatusCode {
        let message = self
            .headers
            .start_line
            .endpoint
            .split("/echo/")
            .last()
            .unwrap();
        self.headers.content_type = Some("text/plain".to_string());
        self.headers.content_length = Some(message.len());
        self.body = message.as_bytes().to_vec();
        StatusCode::_200
    }

    fn handle_get_file(&mut self, filepath: &Path) -> StatusCode {
        let file = fs::read(filepath);
        if let Ok(file) = file {
            self.headers.content_type = Some("application/octet-stream".to_string());
            self.headers.content_length = Some(file.len());
            self.body = file;
            StatusCode::_200
        } else {
            StatusCode::_404
        }
    }

    fn handle_post_file(&mut self, filepath: &Path) -> StatusCode {
        let _ = fs::write(filepath, &self.body);
        self.headers.content_length = Some(self.body.len());
        StatusCode::_201
    }
}

enum StatusCode {
    _200,
    _404,
    _201,
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatusCode::_200 => write!(f, "HTTP/1.1 200 OK"),
            StatusCode::_404 => write!(f, "HTTP/1.1 404 Not Found"),
            StatusCode::_201 => write!(f, "HTTP/1.1 201 Created"),
        }
    }
}

#[derive(PartialEq, Debug)]
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

fn parse_request(request: &[u8]) -> Request {
    let headers: Vec<String> = request
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let headers = parse_headers(headers);
    let json_string = serde_json::to_string(&headers).unwrap();
    let headers: Headers = serde_json::from_str(&json_string).unwrap();

    let body_start = request.len() - headers.content_length.unwrap_or(0);
    let body_end = body_start + headers.content_length.unwrap_or(0);
    let body = &request[body_start..body_end];
    let body = body.to_vec();

    Request { headers, body }
}

fn parse_headers(headers: Vec<String>) -> HashMap<String, String> {
    let mut headers_hash: HashMap<String, String> = HashMap::new();

    let (start_line, headers) = headers.split_at(1);
    headers_hash.insert("Start-Line".to_string(), start_line[0].to_string());

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

#[derive(Debug)]
struct StartLine {
    verb: Verb,
    endpoint: String,
}

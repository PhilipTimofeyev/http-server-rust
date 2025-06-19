use codecrafters_http_server::ThreadPool;
// use flate2::write::GzEncoder;
// use flate2::Compression;
use crate::codec::Encoder;
use crate::status::StatusCode;
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

pub struct Request {
    pub request_line: RequestLine,
    pub headers: Headers,
    pub body: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub struct Headers {
    #[serde(default)]
    #[serde(deserialize_with = "encoder_type")]
    #[serde(rename = "Accept-Encoding")]
    pub accept_encoding: Option<Encoder>,
    #[serde(rename = "User-Agent")]
    user_agent: Option<String>,
    #[serde(rename = "Content-Type")]
    pub content_type: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "string_to_option_usize")]
    #[serde(rename = "Content-Length")]
    pub content_length: Option<usize>,
}

#[derive(PartialEq, Debug)]
pub enum Verb {
    GET,
    POST,
}

#[derive(Debug)]
pub struct RequestLine {
    pub verb: Verb,
    pub endpoint: String,
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

impl Request {
    pub fn handle_request(&mut self, dir: &str) -> Response {
        match self.request_line.endpoint.as_str() {
            "/" => {
                let content_encoding = self.headers.accept_encoding.take();
                let response_headers = ResponseHeaders {
                    content_type: None,
                    content_length: None,
                    content_encoding,
                };
                Response {
                    status_line: StatusCode::_200,
                    headers: response_headers,
                    body: None,
                }
            }
            "/user-agent" => self.handle_user_agent(),
            endpoint if endpoint.contains("/echo") => self.handle_echo(),
            endpoint if endpoint.contains("/files") => {
                let filename = endpoint.split("/files/").last().unwrap();
                let filepath = format!("{dir}{filename}");
                let filepath = Path::new(&filepath);

                match self.request_line.verb {
                    Verb::GET => self.handle_get_file(filepath),
                    Verb::POST => self.handle_post_file(filepath),
                }
            }
            _ => {
                let response_headers = ResponseHeaders {
                    content_type: None,
                    content_length: None,
                    content_encoding: None,
                };
                Response {
                    status_line: StatusCode::_404,
                    headers: response_headers,
                    body: None,
                }
            }
        }
    }

    fn handle_user_agent(&mut self) -> Response {
        let user_agent = self.headers.user_agent.take().unwrap_or_default();
        let body = user_agent.as_bytes().to_vec();
        let content_length = Some(body.len());
        let content_encoding = self.headers.accept_encoding.take();
        let body = Some(body);

        let response_headers = ResponseHeaders {
            content_type: None,
            content_length,
            content_encoding,
        };

        Response {
            status_line: StatusCode::_200,
            headers: response_headers,
            body,
        }
    }

    fn handle_echo(&mut self) -> Response {
        let message = self.request_line.endpoint.split("/echo/").last().unwrap();
        let content_type = Some("text/plain".to_string());
        let content_length = Some(message.len());
        let content_encoding = self.headers.accept_encoding.take();
        let body = Some(message.as_bytes().to_vec());

        let response_headers = ResponseHeaders {
            content_type,
            content_length,
            content_encoding,
        };
        Response {
            status_line: StatusCode::_200,
            headers: response_headers,
            body,
        }
    }

    fn handle_get_file(&mut self, filepath: &Path) -> Response {
        let file = fs::read(filepath);
        if let Ok(file) = file {
            let content_type = Some("application/octet-stream".to_string());
            let content_length = Some(file.len());
            let body = Some(file);

            let response_headers = ResponseHeaders {
                content_type,
                content_length,
                content_encoding: None,
            };
            Response {
                status_line: StatusCode::_200,
                headers: response_headers,
                body,
            }
        } else {
            let response_headers = ResponseHeaders {
                content_type: None,
                content_length: None,
                content_encoding: None,
            };
            Response {
                status_line: StatusCode::_404,
                headers: response_headers,
                body: None,
            }
        }
    }

    fn handle_post_file(&mut self, filepath: &Path) -> Response {
        let _ = fs::write(filepath, &self.body);
        let response_headers = ResponseHeaders {
            content_type: None,
            content_length: None,
            content_encoding: None,
        };
        Response {
            status_line: StatusCode::_201,
            headers: response_headers,
            body: None,
        }
    }
}

pub struct Response {
    pub status_line: StatusCode,
    pub headers: ResponseHeaders,
    pub body: Option<Vec<u8>>,
}

impl Response {
    pub fn build_response_header(&mut self) -> String {
        format!(
            "{}\r\nContent-Type: {}\r\nContent-Encoding: {}\r\nContent-Length: {}\r\n\r\n",
            self.status_line,
            self.headers
                .content_type
                .take()
                .unwrap_or_else(|| "text/plain".to_string()),
            self.headers
                .content_encoding
                .take()
                .map_or("".to_string(), |a| a.to_string()),
            self.headers.content_length.unwrap_or_default(),
        )
    }
}

pub struct ResponseHeaders {
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub content_encoding: Option<Encoder>,
}

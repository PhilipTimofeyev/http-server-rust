use crate::codec::Encoder;
use crate::status::StatusCode;

pub struct Response {
    pub status_line: StatusCode,
    pub headers: ResponseHeaders,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct ResponseHeaders {
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub content_encoding: Option<Encoder>,
    pub connection: Option<String>,
}

impl Response {
    pub fn build_response_header(&mut self) -> String {
        format!(
            "{}\r\nContent-Type: {}\r\nContent-Encoding: {}\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n",
            self.status_line,
            self.headers.content_type.take().unwrap_or_default(),
            self.headers
                .content_encoding
                .take()
                .map_or("".to_string(), |encoding| encoding.to_string()),
            self.headers.content_length.unwrap_or_default(),
            self.headers.connection.take().unwrap_or("keep-alive".to_string()),
        )
    }
}

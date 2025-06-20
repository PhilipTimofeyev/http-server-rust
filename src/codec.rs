use crate::response;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fmt;
use std::io::Write;

#[derive(Debug)]
pub enum Encoder {
    Gzip,
}

impl fmt::Display for Encoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Encoder::Gzip => write!(f, "gzip"),
        }
    }
}

pub fn gzip_encoder(data: Vec<u8>) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&data).unwrap();
    encoder.finish().unwrap()
}

pub fn encode(response: &mut response::Response) {
    if let Some(encoder) = &response.headers.content_encoding {
        match encoder {
            Encoder::Gzip => {
                if let Some(body) = response.body.take() {
                    let encoded = gzip_encoder(body);
                    let content_length = encoded.len();

                    response.body = Some(encoded);
                    response.headers.content_length = Some(content_length);
                }
            }
        };
    };
}

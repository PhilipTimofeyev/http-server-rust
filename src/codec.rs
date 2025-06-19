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
    // self.headers.content_length = Some(compressed_body.len());
    // self.body = compressed_body;
}

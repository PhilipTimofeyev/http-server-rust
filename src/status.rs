use std::fmt;

pub enum StatusCode {
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

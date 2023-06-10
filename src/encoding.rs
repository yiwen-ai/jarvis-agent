use axum::http::header;
use libflate::gzip::{Decoder, Encoder};
use std::{io, string::ToString};

pub enum Encoding {
    Zstd,
    Gzip,
    Identity,
}

impl ToString for Encoding {
    fn to_string(&self) -> String {
        match self {
            Self::Zstd => "zstd".to_string(),
            Self::Gzip => "gzip".to_string(),
            Self::Identity => "identity".to_string(),
        }
    }
}

impl Encoding {
    pub fn identity(&self) -> bool {
        match self {
            Self::Identity => true,
            _ => false,
        }
    }

    pub fn from_header(hm: &header::HeaderMap) -> Self {
        match hm.get(header::CONTENT_ENCODING) {
            Some(v) => match v.to_str() {
                Ok(s) => {
                    if s.eq("zstd") {
                        Self::Zstd
                    } else if s.eq("gzip") {
                        Self::Gzip
                    } else {
                        Self::Identity
                    }
                }
                Err(_) => Self::Identity,
            },
            None => match hm.get("accept-encoding") {
                None => Self::Identity,
                Some(v) => match v.to_str() {
                    Ok(s) => {
                        if s.contains("zstd") {
                            Self::Zstd
                        } else if s.contains("gzip") {
                            Self::Gzip
                        } else {
                            Self::Identity
                        }
                    }
                    Err(_) => Self::Identity,
                },
            },
        }
    }

    pub fn header_value(&self) -> header::HeaderValue {
        match self {
            Self::Zstd => header::HeaderValue::from_static("zstd"),
            Self::Gzip => header::HeaderValue::from_static("gzip"),
            Self::Identity => header::HeaderValue::from_static("identity"),
        }
    }

    pub fn encode_all<R: io::Read>(&self, r: R) -> Result<Vec<u8>, io::Error> {
        match self {
            Self::Zstd => {
                let buf = zstd::stream::encode_all(r, 9)?;
                Ok(buf)
            }
            Self::Gzip => {
                let mut encoder = Encoder::new(Vec::new())?;
                let mut r = r;
                let _ = io::copy(&mut r, &mut encoder);
                encoder.finish().into_result()
            }
            Self::Identity => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "identity encoding not supported",
            )),
        }
    }

    pub fn decode_all<R: io::Read>(&self, r: R) -> Result<Vec<u8>, io::Error> {
        use io::Read;
        match self {
            Self::Zstd => {
                let buf = zstd::stream::decode_all(r)?;
                Ok(buf)
            }
            Self::Gzip => {
                let mut decoder = Decoder::new(r)?;
                let mut buf = Vec::new();
                decoder.read_to_end(&mut buf)?;
                Ok(buf)
            }
            Self::Identity => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "identity decoding not supported",
            )),
        }
    }
}

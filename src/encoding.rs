use axum::http::header::{HeaderMap, HeaderValue};
use bytes::Buf;
use libflate::gzip::{Decoder, Encoder};
use std::{io, io::Read, io::Write, string::ToString};

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

    pub fn from_header(hm: &HeaderMap) -> Self {
        match hm.get("accept-encoding") {
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
        }
    }

    pub fn header_value(&self) -> HeaderValue {
        match self {
            Self::Zstd => HeaderValue::from_static("zstd"),
            Self::Gzip => HeaderValue::from_static("gzip"),
            Self::Identity => HeaderValue::from_static("identity"),
        }
    }

    pub fn encode_all(&self, data: &[u8]) -> Result<Vec<u8>, io::Error> {
        match self {
            Self::Zstd => {
                let buf = zstd::stream::encode_all(data.reader(), 9)?;
                Ok(buf)
            }
            Self::Gzip => {
                let mut encoder = Encoder::new(Vec::new())?;
                encoder.write_all(data)?;
                encoder.finish().into_result()
            }
            Self::Identity => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "identity encoding not supported",
            )),
        }
    }

    pub fn decode_all(&self, data: &[u8]) -> Result<Vec<u8>, io::Error> {
        match self {
            Self::Zstd => {
                let buf = zstd::stream::decode_all(data.reader())?;
                Ok(buf)
            }
            Self::Gzip => {
                let mut decoder = Decoder::new(data)?;
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

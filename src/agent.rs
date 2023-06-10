use axum::{
    body::{boxed, HttpBody, StreamBody},
    extract::{Extension, State},
    http,
    response::Response,
    BoxError,
};
use bytes::Buf;
use hyper::body::to_bytes;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

use crate::context::{extract_header, ReqContext};
use crate::encoding;
use crate::erring::HTTPError;

pub async fn handler<B>(
    State(client): State<Client>,
    Extension(ctx): Extension<Arc<ReqContext>>,
    req: http::Request<B>,
) -> Result<Response, HTTPError>
where
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    let host = extract_header(req.headers(), "x-forwarded-host", || "".to_string());
    if host.is_empty() {
        return Err(HTTPError(400, "X-Forwarded-Host is empty".to_string()));
    }

    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(req.uri().path());

    let url = reqwest::Url::parse(&format!("https://{}{}", host, path_query))
        .map_err(|e| HTTPError(400, e.to_string()))?;

    let enc = encoding::Encoding::from_header(req.headers());
    ctx.set("encoding", Value::String(enc.to_string())).await;

    let method = req.method().clone();
    let mut headers = req.headers().clone();
    let mut rreq = reqwest::Request::new(method, url);
    headers.remove(http::header::HOST);
    headers.remove(http::header::FORWARDED);
    headers.remove(http::header::HeaderName::from_static("x-forwarded-for"));
    headers.remove(http::header::HeaderName::from_static("x-forwarded-host"));
    headers.remove(http::header::HeaderName::from_static("x-forwarded-proto"));

    if headers.contains_key(http::header::CONTENT_LENGTH)
        || headers.contains_key(http::header::CONTENT_ENCODING)
    {
        match to_bytes(req.into_body()).await {
            Err(err) => {
                return Err(HTTPError(400, Into::<BoxError>::into(err).to_string()));
            }
            Ok(body) => {
                if headers.contains_key(http::header::CONTENT_ENCODING) {
                    headers.remove(http::header::CONTENT_ENCODING);
                    headers.remove(http::header::CONTENT_LENGTH);

                    let body = enc
                        .decode_all(body.reader())
                        .map_err(|err| HTTPError(400, err.to_string()))?;
                    ctx.set("req_body_decoded_size", Value::from(body.len()))
                        .await;
                    *rreq.body_mut() = Some(body.into());
                } else {
                    *rreq.body_mut() = Some(body.into());
                }
            }
        }
    }

    *rreq.headers_mut() = headers;
    *rreq.version_mut() = http::Version::HTTP_11;

    let rres = client
        .execute(rreq)
        .await
        .map_err(|err| HTTPError(500, err.to_string()))?;
    let status = rres.status();
    let version = rres.version();
    let headers = rres.headers().to_owned();
    ctx.set(
        "res_content_encoding",
        Value::from(extract_header(&headers, "content-encoding", || {
            "".to_string()
        })),
    )
    .await;
    ctx.set(
        "res_content_length",
        Value::from(extract_header(&headers, "content-length", || {
            "".to_string()
        })),
    )
    .await;

    let mut res = Response::new(boxed(StreamBody::new(rres.bytes_stream())));
    *res.status_mut() = status;
    *res.version_mut() = version;
    *res.headers_mut() = headers;

    Ok(res)
}

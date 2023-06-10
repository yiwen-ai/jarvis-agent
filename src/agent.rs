use axum::{
    body::{boxed, StreamBody},
    extract::{Extension, State},
    http::{self, Request, StatusCode},
    response::{IntoResponse, Response},
};
use hyper::Body;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

use crate::context::{extract_header, ReqContext};
use crate::encoding;

pub async fn handler(
    State(client): State<Client>,
    Extension(ctx): Extension<Arc<ReqContext>>,
    req: Request<Body>,
) -> Response {
    let host = extract_header(req.headers(), "x-forwarded-host", || "".to_string());
    if host.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            "X-Forwarded-Host is empty".to_string(),
        )
            .into_response();
    }

    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let url = reqwest::Url::parse(&format!("https://{}{}", host, path_query));
    if url.is_err() {
        return (
            StatusCode::BAD_REQUEST,
            [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            url.err().unwrap().to_string(),
        )
            .into_response();
    }

    let enc = encoding::Encoding::from_header(req.headers());
    ctx.set("encoding", Value::String(enc.to_string())).await;

    let mut headers = req.headers().clone();
    let rreq = reqwest::Request::try_from(req);
    if rreq.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            rreq.err().unwrap().to_string(),
        )
            .into_response();
    }

    let mut rreq = rreq.unwrap();
    headers.remove(http::header::HOST);
    // headers.remove(http::header::ACCEPT_ENCODING);
    headers.remove(http::header::FORWARDED);
    headers.remove(http::header::HeaderName::from_static("x-forwarded-for"));
    headers.remove(http::header::HeaderName::from_static("x-forwarded-host"));
    headers.remove(http::header::HeaderName::from_static("x-forwarded-proto"));
    if headers.contains_key(http::header::CONTENT_ENCODING) {
        if let Some(body) = rreq.body() {
            if let Some(body) = body.as_bytes() {
                headers.remove(http::header::CONTENT_ENCODING);
                headers.remove(http::header::CONTENT_LENGTH);

                ctx.set("req_body_size", Value::from(body.len())).await;
                let body = enc.decode_all(body).unwrap();
                ctx.set("req_body_decoded_size", Value::from(body.len()))
                    .await;
                *rreq.body_mut() = Some(reqwest::Body::from(body));
            }
        }
    }
    *rreq.headers_mut() = headers;
    *rreq.version_mut() = http::Version::HTTP_11;
    *rreq.url_mut() = url.unwrap();

    let rres = client.execute(rreq).await;
    if rres.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            rres.err().unwrap().to_string(),
        )
            .into_response();
    }

    let rres = rres.unwrap();
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

    res
}

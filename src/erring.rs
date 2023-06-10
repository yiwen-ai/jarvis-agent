use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, Clone)]
pub struct HTTPError(pub u16, pub String);

impl std::fmt::Display for HTTPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.0, &self.1)
    }
}

impl std::error::Error for HTTPError {}

impl IntoResponse for HTTPError {
    fn into_response(self) -> Response {
        let code = StatusCode::from_u16(self.0).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (code, self.1).into_response()
    }
}

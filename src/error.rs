use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    Conflict,
    Internal,
    BadRequest,
    Forbidden,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized",
            ),
            AppError::Conflict => (
                StatusCode::CONFLICT,
                "conflict",
                "Resource conflict",
            ),
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Internal server error",
            ),
            AppError::BadRequest => (
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Bad request",
            ),
            AppError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "Forbidden",
            ),
        };

        let body = Json(ErrorResponse {
            code,
            message,
        });

        (status, body).into_response()
    }
}
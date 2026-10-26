use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Unauthorized(&'static str),
    Conflict(&'static str),
    Invalid(&'static str),
    Internal(&'static str),
    BadRequest(&'static str),
    Forbidden(&'static str),
}

// 错误响应结构体
#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: &'static str,
}

// 将 AppError 转换为响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 根据错误类型构造响应
        let (status, code, message) = match self {
            // 鉴权无效
            AppError::Unauthorized(message) => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                message,
            ),
            // 冲突错误
            AppError::Conflict(message) => (
                StatusCode::CONFLICT,
                "conflict",
                message,
            ),
            // 内部错误
            AppError::Internal(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                message,
            ),
            // 请求参数错误
            AppError::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                "bad_request",
                message,
            ),
            // 禁止访问
            AppError::Forbidden(message) => (
                StatusCode::FORBIDDEN,
                "forbidden",
                message,
            ),
        };

        // 创建 JSON 响应体
        let body = Json(ErrorResponse {
            code,
            message,
        });

        // 返回响应
        (status, body).into_response()
    }
}
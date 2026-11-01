use axum::http::HeaderMap;
use uuid::Uuid;

use crate::{
    core::token,
    error::AppError,
};

/// 提取请求头中的用户ID
pub fn require_actor_id(
    headers: &HeaderMap,
    jwt_secret: &str,
) -> Result<Uuid, AppError> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized("missing authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized("invalid authorization header format"))?;

    let claims = token::verify_access_token(token, jwt_secret)
        .map_err(|_| AppError::Unauthorized("invalid access token"))?;

    Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("invalid token subject"))
}
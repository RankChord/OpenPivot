use axum::{extract::State, http::StatusCode, Json};

use crate::{
    app::AppState,
    core::{password, token},
    models::auth::{RegisterRequest, RegisterResponse, 
        LoginRequest, TokenResponse, RefreshRequest, MeResponse},
    // repository::{user, session},
    // models::user::UserStatus,
};

use crate::error::AppError;
use crate::api::v1::extractors::require_user_id;

use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        // .route("/login", post(login))
        // .route("/refresh", post(refresh))
        // .route("/logout", post(logout))
        // .route("/me", get(me))
}

// --------------------- 注册函数 --------------------
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    // 验证请求体
    payload.validate().then_some(()).ok_or(AppError::BadRequest)?;

    // 检查用户名是否已存在
    user::find_user_by_username(&state.db, &payload.username)
        .await
        .map_err(|_| AppError::Internal)?
        .map(|_| AppError::Conflict)
        .map_or(Ok(()), Err)?;

    // 密码哈希
    let password_hash = password::hash_password(&payload.password)
        .map_err(|_| AppError::Internal)?;

    let created_user = user::create_user(
        &state.db,
        &payload.username,
        &payload.nickname,
        &password_hash,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(Json(RegisterResponse {
        id: created_user.id,
        username: created_user.username,
        nickname: created_user.nickname,
    }))
}

// pub async fn login(
//     State(state): State<AppState>,
//     Json(payload): Json<LoginRequest>,
// ) -> Result<Json<TokenResponse>, AppError> {
//     if !payload.validate() {
//         return Err(AppError::BadRequest);
//     }

//     let found_user = user::find_user_by_username(&state.db, &payload.username)
//     .await
//     .map_err(|_| AppError::Internal)?;

//     let user = match found_user {
//         Some(user) => user,
//         None => return Err(AppError::Unauthorized),
//     };

//     if !matches!(user.status, UserStatus::Active) {
//         return Err(AppError::Forbidden);
//     }

//     let password_ok = password::verify_password(
//         &payload.password,
//         &user.password_hash,
//     )
//     .map_err(|_| AppError::Internal)?;

//     if !password_ok {
//         return Err(AppError::Unauthorized);
//     }

//     let access_token = token::create_access_token(
//         user.id,
//         &state.config.auth.jwt_secret,
//         state.config.auth.access_token_ttl_minutes,
//     )
//     .map_err(|_| AppError::Internal)?;

//     let refresh_token = token::generate_refresh_token();
//     let refresh_token_hash = token::hash_refresh_token(&refresh_token);

//     let expires_at = time::OffsetDateTime::now_utc()
//         + time::Duration::days(state.config.auth.refresh_token_ttl_days as i64);

//     session::create_session(
//         &state.db,
//         user.id,
//         &refresh_token_hash,
//         expires_at,
//         None,
//         None,
//     )
//     .await
//     .map_err(|_| AppError::Internal)?;

//     user::update_last_login_at(&state.db, user.id)
//     .await
//     .map_err(|_| AppError::Internal)?;

//     Ok(Json(TokenResponse {
//         access_token,
//         refresh_token,
//         token_type: "Bearer".to_string(),
//         expires_in: state.config.auth.access_token_ttl_minutes * 60,
//     }))
// }

// pub async fn refresh(
//     State(state): State<AppState>,
//     Json(payload): Json<RefreshRequest>,
// ) -> Result<Json<TokenResponse>, AppError> {
//     let refresh_token_hash = token::hash_refresh_token(&payload.refresh_token);

//     let found_session = session::find_session_by_refresh_token_hash(
//         &state.db,
//         &refresh_token_hash,
//     )
//     .await
//     .map_err(|_| AppError::Internal)?;

//     let old_session = match found_session {
//         Some(session) => session,
//         None => return Err(AppError::Unauthorized),
//     };

//     if old_session.revoked_at.is_some() {
//         return Err(AppError::Unauthorized);
//     }

//     let now = time::OffsetDateTime::now_utc();

//     if old_session.expires_at <= now {
//         return Err(AppError::Unauthorized);
//     }

//     let access_token = token::create_access_token(
//         old_session.user_id,
//         &state.config.auth.jwt_secret,
//         state.config.auth.access_token_ttl_minutes,
//     )
//     .map_err(|_| AppError::Internal)?;

//     let new_refresh_token = token::generate_refresh_token();
//     let new_refresh_token_hash = token::hash_refresh_token(&new_refresh_token);

//     let new_expires_at = now
//         + time::Duration::days(state.config.auth.refresh_token_ttl_days as i64);

//     session::revoke_session(&state.db, old_session.id)
//         .await
//         .map_err(|_| AppError::Internal)?;

//     session::create_session(
//         &state.db,
//         old_session.user_id,
//         &new_refresh_token_hash,
//         new_expires_at,
//         None,
//         None,
//     )
//     .await
//     .map_err(|_| AppError::Internal)?;

//     Ok(Json(TokenResponse {
//         access_token,
//         refresh_token: new_refresh_token,
//         token_type: "Bearer".to_string(),
//         expires_in: state.config.auth.access_token_ttl_minutes * 60,
//     }))
// }

// pub async fn logout(
//     State(state): State<AppState>,
//     Json(payload): Json<RefreshRequest>,
// ) -> Result<StatusCode, AppError> {
//     let refresh_token_hash = token::hash_refresh_token(&payload.refresh_token);

//     let found_session = session::find_session_by_refresh_token_hash(
//         &state.db,
//         &refresh_token_hash,
//     )
//     .await;

//     if let Ok(Some(session)) = found_session {
//         let _ = session::revoke_session(&state.db, session.id).await;
//     }

//     Ok(StatusCode::NO_CONTENT)
// }

// pub async fn me(
//     State(state): State<AppState>,
//     headers: axum::http::HeaderMap,
// ) -> Result<Json<MeResponse>, AppError> {
//     let user_id = require_user_id(
//         &headers,
//         &state.config.auth.jwt_secret,
//     )?;

//     Ok(Json(MeResponse {
//         user_id,
//     }))
// }
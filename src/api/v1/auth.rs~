use axum::{extract::State, http::StatusCode, Json};

use crate::{
    app::AppState,
    core::{password, token},
    models::auth::{RegisterRequest, RegisterResponse, LoginIdentifier,
        LoginRequest, TokenResponse, RefreshRequest, MeResponse},
    repository::{user, session},
    models::user::UserStatus,
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
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/me", get(me))
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

    // 创建用户
    let created_user = user::create_user(
        &state.db,
        &payload.username,
        &payload.nickname,
        &password_hash,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 返回注册响应
    Ok(Json(RegisterResponse {
        id: created_user.id,
        username: created_user.username,
        nickname: created_user.nickname,
    }))
}

// --------------------- 登录函数 --------------------
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // 验证请求体
    payload.validate().then_some(()).ok_or(AppError::BadRequest)?;

    // 获取用户信息
    let authenticated_user = match &payload.identifier {
        LoginIdentifier::Username(username) => {
            user::find_user_by_username(&state.db, username).await
        }
        LoginIdentifier::Id(user_id) => {
            user::find_user_by_id(&state.db, *user_id).await
        }
    }
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::Unauthorized)?;

    // 判断用户状态是否为活跃
    if !matches!(authenticated_user.status, UserStatus::Active) {
        return Err(AppError::Forbidden);
    }

    // 验证用户密码
    let password_ok = password::verify_password(
        &payload.password,
        &authenticated_user.password_hash,
    )
    .map_err(|_| AppError::Internal)?;

    // 如果密码不正确，返回未授权错误
    if !password_ok {
        return Err(AppError::Unauthorized);
    }

    // 生成访问令牌
    let access_token = token::create_access_token(
        authenticated_user.id,
        &state.config.auth.jwt_secret,
        state.config.auth.access_token_ttl_minutes,
    )
    .map_err(|_| AppError::Internal)?;

    // 生成刷新令牌
    let refresh_token = token::generate_refresh_token();
    let refresh_token_hash = token::hash_refresh_token(&refresh_token);

    // 计算刷新令牌的过期时间
    let expires_at = time::OffsetDateTime::now_utc()
        + time::Duration::days(state.config.auth.refresh_token_ttl_days as i64);

    // 创建连接会话
    session::create_session(
        &state.db,                  // 数据库连接
        authenticated_user.id,      // 用户ID
        &refresh_token_hash,        // 刷新令牌哈希
        expires_at,                 // 过期时间
        None,
        None,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 更新用户的最后登录时间
    user::update_last_login_at(&state.db, authenticated_user.id)
    .await
    .map_err(|_| AppError::Internal)?;

    // 返回令牌响应
    Ok(Json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.auth.access_token_ttl_minutes * 60,
    }))
}

// --------------------- 刷新函数 --------------------
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // 验证刷新令牌
    let refresh_token_hash = token::hash_refresh_token(&payload.refresh_token);

    // 查找会话
    let found_session = session::find_session_by_refresh_token_hash(
        &state.db,
        &refresh_token_hash,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 如果会话不存在，返回未授权错误
    let old_session = match found_session {
        Some(session) => session,
        None => return Err(AppError::Unauthorized),
    };

    // 检查会话是否已被撤销或过期
    if old_session.revoked_at.is_some() {
        return Err(AppError::Unauthorized);
    }

    // 检查会话是否已过期
    let now = time::OffsetDateTime::now_utc();

    // 如果会话已过期，返回未授权错误
    if old_session.expires_at <= now {
        return Err(AppError::Unauthorized);
    }

    // 生成新的访问令牌
    let access_token = token::create_access_token(
        old_session.user_id,
        &state.config.auth.jwt_secret,
        state.config.auth.access_token_ttl_minutes,
    )
    .map_err(|_| AppError::Internal)?;

    // 生成新的刷新令牌
    let new_refresh_token = token::generate_refresh_token();
    let new_refresh_token_hash = token::hash_refresh_token(&new_refresh_token);

    // 计算新的刷新令牌的过期时间
    let new_expires_at = now
        + time::Duration::days(state.config.auth.refresh_token_ttl_days as i64);

    // 撤销旧的会话
    session::revoke_session(&state.db, old_session.id)
        .await
        .map_err(|_| AppError::Internal)?;

    // 创建新的会话
    session::create_session(
        &state.db,
        old_session.user_id,
        &new_refresh_token_hash,
        new_expires_at,
        None,
        None,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 返回新的令牌响应
    Ok(Json(TokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.auth.access_token_ttl_minutes * 60,
    }))
}

// --------------------- 登出函数 --------------------
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<StatusCode, AppError> {
    // 验证刷新令牌
    let refresh_token_hash = token::hash_refresh_token(&payload.refresh_token);

    // 查找会话
    let found_session = session::find_session_by_refresh_token_hash(
        &state.db,
        &refresh_token_hash,
    )
    .await;

    // 如果会话存在，撤销会话
    if let Ok(Some(session)) = found_session {
        let _ = session::revoke_session(&state.db, session.id).await;
    }

    // 返回204 No Content，表示登出成功
    Ok(StatusCode::NO_CONTENT)
}

// --------------------- 获取当前用户信息函数 --------------------
pub async fn me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<MeResponse>, AppError> {
    // 从请求头中获取用户ID
    let user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    // 返回当前用户信息
    Ok(Json(MeResponse {
        id: user_id,
    }))
}
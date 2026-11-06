use axum::{extract::State, http::StatusCode, Json};

use crate::{
    app::AppState,
    core::{password, token},
    models::auth::{RegisterRequest, RegisterResponse, LoginIdentifier,
        LoginRequest, TokenResponse, RefreshRequest, MeResponse},
    repository::{actor, credential, session},
    models::actor::ActorStatus,
};

use crate::error::AppError;
use crate::api::v1::extractors::require_actor_id;

use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))                 // 注册接口
        .route("/login", post(login))                       // 登录接口
        .route("/refresh", post(refresh))                   // 刷新令牌
        .route("/logout", post(logout))                     // 登出令牌
        .route("/me", get(me))                              // 获取个人信息
}

// --------------------- 注册函数 --------------------
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    // 验证请求体
    payload.validate().map_err(AppError::Invalid)?;
    
    let RegisterRequest {
        username,
        display_name,
        password,
        actor_type,
    } = payload;

    // 用户名统一小写，避免 Alice 和 alice 被当成两个账号
    let username = username.to_ascii_lowercase();

    // Argon2 是 CPU/内存密集型操作，交给阻塞线程池
    let password_hash = tokio::task::spawn_blocking(move || {
        password::hash_password(&password)
    })
    .await
    .map_err(|err| {
        eprintln!("[auth] password hash task failed: {err}");
        AppError::Internal("password hash task failed")
    })?
    .map_err(|err| {
        eprintln!("[auth] password hash failed: {err}");
        AppError::Internal("password hashing failed")
    })?;

    // 开启事务
    let mut tx = state.db.begin().await.map_err(|err| {
        eprintln!("[auth] begin transaction failed: {err:?}");
        AppError::Internal("transaction begin failed")
    })?;

    // 创建 Actor
    let created_actor = actor::create_actor(
        &mut *tx,
        &actor_type,
        &display_name,
    )
    .await
    .map_err(|err| {
        eprintln!("[auth] create actor failed: {err:?}");
        AppError::Internal("actor creation failed")
    })?;

    credential::create_credential(
        &mut *tx,
        created_actor.id,
        &username,
        &password_hash,
    )
    .await
    .map_err(|err| match &err {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::Conflict("username already taken")
        }
        _ => {
            eprintln!("[auth] create credential failed: {err:?}");
            AppError::Internal("credential creation failed")
        }
    })?;

    // 提交事务
    tx.commit().await.map_err(|err| {
        eprintln!("[auth] commit transaction failed: {err:?}");
        AppError::Internal("transaction commit failed")
    })?;

    // 返回注册成功响应
    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id: created_actor.id,
            username,
            display_name,
        }),
    ))
}

// --------------------- 登录函数 --------------------
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // 验证请求体
    payload.validate().map_err(AppError::Invalid)?;

    // 获取用户信息
    let authenticated_actor = match &payload.identifier {
        LoginIdentifier::Username(username) => {
            let normalized = username.to_ascii_lowercase();

            credential::find_authenticated_actor_by_username(
                &state.db,
                &normalized,
            )
            .await
        }
        LoginIdentifier::Id(actor_id) => {
            credential::find_authenticated_actor_by_id(
                &state.db,
                *actor_id,
            )
            .await
        }
    }
    .map_err(|err| {
        eprintln!("[auth] find authenticated actor failed: {err:?}");
        AppError::Internal("authentication lookup failed")
    })?
    .ok_or(AppError::Unauthorized("account not found"))?;

    // 判断用户状态是否为活跃
    if !matches!(authenticated_actor.status, ActorStatus::Active) {
        return Err(AppError::Forbidden("account is not active"));
    }
    
    // 验证用户密码
    let password_ok = password::verify_password(
        &payload.password,
        &authenticated_actor.password_hash,
    )
    .map_err(|_| AppError::Internal("password verification failed"))?;

    // 如果密码不正确，返回未授权错误
    if !password_ok {
        return Err(AppError::Unauthorized("incorrect password"));
    }

    // 生成访问令牌
    let access_token = token::create_access_token(
        authenticated_actor.actor_id,
        &state.config.auth.jwt_secret,
        state.config.auth.access_token_ttl_minutes,
    )
    .map_err(|_| AppError::Internal("token generation failed"))?;

    // 生成刷新令牌
    let refresh_token = token::generate_refresh_token();
    let refresh_token_hash = token::hash_refresh_token(&refresh_token);

    // 计算刷新令牌的过期时间
    let expires_at = time::OffsetDateTime::now_utc()
        + time::Duration::days(state.config.auth.refresh_token_ttl_days as i64);

    // 开启事务
    let mut tx = state.db.begin().await.map_err(|err| {
        eprintln!("[auth] begin transaction failed: {err:?}");
        AppError::Internal("transaction begin failed")
    })?;

    // 创建连接会话
    session::create_session(
        &mut *tx,                          // 事务连接
        authenticated_actor.actor_id,      // 用户ID
        &refresh_token_hash,               // 刷新令牌哈希
        expires_at,                        // 过期时间
        None,
        None,
    )
    .await
    .map_err(|_| AppError::Internal("session creation failed"))?;

    // 更新用户的最后登录时间
    credential::update_last_login_at(
        &mut *tx,
        authenticated_actor.actor_id,
    )
    .await
    .map_err(|err| {
        eprintln!("[auth] update last login time failed: {err:?}");
        AppError::Internal("last login update failed")
    })?;

    // 提交事务
    tx.commit().await.map_err(|err| {
        eprintln!("[auth] commit transaction failed: {err:?}");
        AppError::Internal("transaction commit failed")
    })?;

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
    .map_err(|_| AppError::Internal("session lookup failed"))?;

    // 如果会话不存在，返回未授权错误
    let old_session = match found_session {
        Some(session) => session,
        None => return Err(AppError::Unauthorized("session not found")),
    };

    // 检查会话是否已被撤销或过期
    if old_session.revoked_at.is_some() {
        return Err(AppError::Unauthorized("session revoked"));
    }

    // 检查会话是否已过期
    let now = time::OffsetDateTime::now_utc();

    // 如果会话已过期，返回未授权错误
    if old_session.expires_at <= now {
        return Err(AppError::Unauthorized("session expired"));
    }

    // 生成新的访问令牌
    let access_token = token::create_access_token(
        old_session.actor_id,
        &state.config.auth.jwt_secret,
        state.config.auth.access_token_ttl_minutes,
    )
    .map_err(|_| AppError::Internal("token generation failed"))?;

    // 生成新的刷新令牌
    let new_refresh_token = token::generate_refresh_token();
    let new_refresh_token_hash = token::hash_refresh_token(&new_refresh_token);

    // 计算新的刷新令牌的过期时间
    let new_expires_at = now
        + time::Duration::days(state.config.auth.refresh_token_ttl_days as i64);

    // 开启事务
    let mut tx = state.db.begin().await.map_err(|err| {
        eprintln!("[auth] begin transaction failed: {err:?}");
        AppError::Internal("transaction begin failed")
    })?;

    // 撤销旧的会话
    session::revoke_session(&mut *tx, old_session.id)
        .await
        .map_err(|_| AppError::Internal("session revoke failed"))?;

    // 创建新的会话
    session::create_session(
        &mut *tx,
        old_session.actor_id,
        &new_refresh_token_hash,
        new_expires_at,
        None,
        None,
    )
    .await
    .map_err(|_| AppError::Internal("session creation failed"))?;

    // 提交事务
    tx.commit().await.map_err(|err| {
        eprintln!("[auth] commit transaction failed: {err:?}");
        AppError::Internal("transaction commit failed")
    })?;

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
    .await
    .map_err(|_| AppError::Internal("session lookup failed"))?;

    // 如果会话存在，撤销会话
    if let Some(session) = found_session {
        session::revoke_session(&state.db, session.id)
            .await
            .map_err(|_| AppError::Internal("session revoke failed"))?;
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
    let actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;
    // 返回当前用户信息
    Ok(Json(MeResponse {
        id: actor_id,
    }))
}

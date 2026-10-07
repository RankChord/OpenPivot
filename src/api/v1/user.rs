use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Router,
    Json,
};

use crate::{
    api::v1::extractors::require_user_id,
    app::AppState,
    error::AppError,
    models::user::{UserSearchItem, SearchUsersQuery},
    repository::user,
};



pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(search_users))
}

// --------------------- 搜索函数 --------------------
pub async fn search_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<Vec<UserSearchItem>>, AppError> {
    // 从 Authorization 请求头中解析并验证 JWT，获取当前登录用户的 UUID
    let current_user_id = require_user_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 去掉搜索关键字首尾的空白字符
    let keyword = query.q.trim();

    // 检查处理后的搜索关键字是否为空
    if keyword.is_empty() {
        return Err(AppError::BadRequest);
    }

    // 调用用户 Repository，在数据库中搜索符合条件的用户
    let users = user::search_users(
        &state.db,
        current_user_id,
        keyword,
    )
    .await
    .map_err(|_| AppError::Internal)?;

    // 将搜索结果序列化为 JSON，并作为成功响应返回
    Ok(Json(users))
}
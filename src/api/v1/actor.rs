use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Router,
    Json,
};

use crate::{
    api::v1::extractors::require_actor_id,
    app::AppState,
    error::AppError,
    models::actor::{ActorSearchItem, SearchActorsQuery},
    repository::actor,
};



pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(search_actors))
}

// --------------------- 搜索函数 --------------------
pub async fn search_actors(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchActorsQuery>,
) -> Result<Json<Vec<ActorSearchItem>>, AppError> {
    // 从 Authorization 请求头中解析并验证 JWT，获取当前登录用户的 UUID
    let current_actor_id = require_actor_id(
        &headers,
        &state.config.auth.jwt_secret,
    )?;

    // 去掉搜索关键字首尾的空白字符
    let keyword = query.q.trim();

    // 检查处理后的搜索关键字是否为空
    if keyword.is_empty() {
        return Err(AppError::BadRequest("search keyword is empty"));
    }

    // 调用用户 Repository，在数据库中搜索符合条件的用户
    let actors = actor::search_actors(
        &state.db,
        current_actor_id,
        keyword,
    )
    .await
    .map_err(|_| AppError::Internal("actor search failed"))?;

    // 将搜索结果序列化为 JSON，并作为成功响应返回
    Ok(Json(actors))
}
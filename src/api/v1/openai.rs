use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get},
    Json,
    Router,
};

use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    let chat = post(chat_completions);

    let files = get(list_files)
        .post(upload_file)
        .delete(delete_file);

    let responses = post(responses);

    let uploads = post(upload_file_multipart);

    Router::new()
        .route("/chat/completions", chat)
        .route("/responses", responses)
        .route("/responses/{response_id}", responses)

        .route("/realtime", )
        .route("/files", files)
        .route("/files/{id}", files)
        .route("/files/{id}/content", files)
        .route("/uploads", uploads)
}


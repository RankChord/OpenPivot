pub mod auth;
pub mod user;
pub mod friend;
pub mod system;
pub mod extractors;
pub mod conversation;

use axum::Router;
use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/users", user::routes())
        .nest("/system", system::routes())
        .nest("/friends", friend::routes())
        .nest("/conversations", conversation::routes())
}
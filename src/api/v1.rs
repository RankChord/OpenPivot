pub mod auth;
pub mod user;
pub mod flow;
pub mod space;
pub mod friend;
pub mod system;
pub mod extractors;
pub mod conversation;

use axum::Router;
use crate::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("", flow::routes())
        .nest("/auth", auth::routes())
        .nest("/users", user::routes())
        .nest("/spaces", space::routes())
        .nest("/system", system::routes())
        .nest("/friends", friend::routes())
        .nest("/conversations", conversation::routes())
}
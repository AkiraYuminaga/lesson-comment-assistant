use crate::api::handlers::{generate_comment_handler, health_check, list_courses, AppState};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn create_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/courses", get(list_courses))
        .route("/v1/comments/generate", post(generate_comment_handler))
        .with_state(state)
}

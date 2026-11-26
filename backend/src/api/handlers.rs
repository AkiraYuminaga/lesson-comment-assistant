use crate::api::schemas::{GenerateRequest, GenerateResponse, HealthResponse};
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::security::signature::sign;
use crate::security::validator::validate_course_name;
use crate::services::comment::generate_comment;
use crate::services::gemini::GeminiClient;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;
use validator::Validate;

pub struct AppState {
    pub gemini_client: GeminiClient,
    pub config: AppConfig,
}

pub async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    (StatusCode::OK, Json(response))
}

/// Course list for the extension's menus.
pub async fn list_courses() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(crate::profile::get().catalogue.summary()),
    )
}

pub async fn generate_comment_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateRequest>,
) -> AppResult<impl IntoResponse> {
    // Validate request
    request.validate().map_err(|e| {
        tracing::warn!("Validation failed: {}", e);
        AppError::ValidationError(format!("Invalid request: {}", e))
    })?;

    // Additional validation for course name
    if !request.course.is_empty() {
        validate_course_name(&request.course)?;
    }

    // Inputs only go into the LLM prompt (never rendered as HTML), so no HTML sanitizing here.
    let comment = generate_comment(&request, &state.gemini_client).await?;

    // Sign the response
    let signature = sign(&comment, &state.config.security.hmac_secret).map_err(|e| {
        tracing::error!("Failed to sign response: {}", e);
        AppError::InternalError
    })?;

    let response = GenerateResponse {
        comment,
        signature: Some(signature),
    };

    Ok((StatusCode::OK, Json(response)))
}

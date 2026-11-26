use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct GenerateRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Student name must be between 1 and 100 characters"
    ))]
    pub student: String,

    #[validate(length(max = 100, message = "Course name must not exceed 100 characters"))]
    pub course: String,

    #[validate(length(max = 20, message = "Lesson must not exceed 20 characters"))]
    pub lesson: String,

    #[validate(length(max = 500, message = "Progress must not exceed 500 characters"))]
    pub progress: String,

    #[validate(length(max = 2000, message = "Performance must not exceed 2000 characters"))]
    pub performance: String,

    #[validate(length(max = 500, message = "Issue must not exceed 500 characters"))]
    pub issue: String,

    #[serde(default, alias = "lastUndone")]
    pub last_undone: bool,

    #[serde(default = "default_polish")]
    pub polish: bool,
}

fn default_polish() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub comment: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

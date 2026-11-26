use crate::config::GeminiConfig;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_ATTEMPTS: u32 = 3;
const BASE_BACKOFF_MS: u64 = 1000;

#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    config: GeminiConfig,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
    generation_config: GenerationConfig,
}

/// Higher temperature so repeated generations for the same input read differently.
#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    top_p: f32,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: String,
}

impl GeminiClient {
    pub fn new(config: GeminiConfig) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to build HTTP client"),
            config,
        }
    }

    pub async fn generate_text(&self, system_prompt: &str, user_prompt: &str) -> AppResult<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.config.model
        );

        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: user_prompt.to_string(),
                }],
            }],
            system_instruction: Some(SystemInstruction {
                parts: vec![Part {
                    text: system_prompt.to_string(),
                }],
            }),
            generation_config: GenerationConfig {
                temperature: 0.8,
                top_p: 0.95,
            },
        };

        // Retry transient upstream overload (429 / 5xx) with exponential backoff.
        let mut attempt = 0;
        let response = loop {
            attempt += 1;
            let response = self
                .client
                .post(&url)
                // Send key in a header so it never appears in URLs or error logs
                .header(
                    "x-goog-api-key",
                    self.config.api_key.expose_secret().as_str(),
                )
                .json(&request_body)
                .send()
                .await
                .map_err(|e| {
                    // without_url(): avoid logging the request URL
                    let e = e.without_url();
                    tracing::error!("Gemini API request failed: {}", e);
                    AppError::GeminiApiError(format!("Request failed: {}", e))
                })?;

            let status = response.status();
            if status.is_success() {
                break response;
            }

            let error_text = response.text().await.unwrap_or_default();
            let transient = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
            if transient && attempt < MAX_ATTEMPTS {
                let delay = Duration::from_millis(BASE_BACKOFF_MS << (attempt - 1));
                tracing::warn!(
                    "Gemini API {} (attempt {}/{}), retrying in {:?}",
                    status,
                    attempt,
                    MAX_ATTEMPTS,
                    delay
                );
                tokio::time::sleep(delay).await;
                continue;
            }

            tracing::error!("Gemini API error ({}): {}", status, error_text);
            return Err(if transient {
                AppError::UpstreamBusy
            } else {
                AppError::GeminiApiError(format!("API returned error: {}", status))
            });
        };

        let gemini_response: GeminiResponse = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Gemini response: {}", e);
            AppError::GeminiApiError("Failed to parse response".to_string())
        })?;

        gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| AppError::GeminiApiError("No text in response".to_string()))
    }
}

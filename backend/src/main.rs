mod api;
mod config;
mod data;
mod error;
mod middleware;
mod profile;
mod prompts;
mod security;
mod services;

use crate::api::handlers::AppState;
use crate::api::routes::create_routes;
use crate::config::AppConfig;
use crate::security::cors::create_cors_layer;
use crate::security::rate_limit::create_rate_limit_layer;
use crate::services::gemini::GeminiClient;
use axum::middleware as axum_middleware;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

/// Max accepted request body size (the largest field is 2000 chars ≈ 6 KB in UTF-8)
const MAX_BODY_BYTES: usize = 32 * 1024;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lesson_comment_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting lesson comment backend...");

    // Load configuration
    let config = AppConfig::from_env()?;
    tracing::info!("Configuration loaded successfully");

    // Load courses / prompt / style; fail fast on a broken file instead of at the first request
    profile::init(&config.config_dir)?;

    // Create Gemini client
    let gemini_client = GeminiClient::new(config.gemini.clone());

    // Create app state
    let state = Arc::new(AppState {
        gemini_client,
        config: config.clone(),
    });

    // Build our application with routes and middleware
    let app = create_routes(state)
        // Reject oversized bodies before parsing
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        // Rate limiting (protects Gemini quota)
        .layer(create_rate_limit_layer(config.rate_limit.per_minute))
        // CORS
        .layer(create_cors_layer(config.security.allowed_origins.clone()))
        // Security headers
        .layer(axum_middleware::from_fn(
            middleware::security_headers::add_security_headers,
        ))
        // Logging
        .layer(axum_middleware::from_fn(middleware::logging::log_requests))
        // Tracing
        .layer(TraceLayer::new_for_http());

    // Start the server
    let addr = SocketAddr::from((
        config
            .server
            .host
            .parse::<std::net::IpAddr>()
            .unwrap_or_else(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        config.server.port,
    ));

    tracing::info!("Server listening on http://{}", addr);
    println!("🚀 Server started successfully!");
    println!("📍 Listening on: http://{}", addr);
    println!("🏥 Health check: http://{}/health", addr);
    println!("📚 Courses:      http://{}/v1/courses", addr);
    println!("📝 API endpoint: http://{}/v1/comments/generate", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

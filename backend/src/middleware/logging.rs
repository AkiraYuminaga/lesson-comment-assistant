use axum::{body::Body, http::Request, middleware::Next, response::Response};
use tracing::info;

/// Middleware to log all incoming requests
pub async fn log_requests(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    info!(
        method = %method,
        uri = %uri,
        version = ?version,
        "Incoming request"
    );

    let response = next.run(request).await;

    info!(
        method = %method,
        uri = %uri,
        status = response.status().as_u16(),
        "Request completed"
    );

    response
}

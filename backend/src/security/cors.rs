use axum::http::{header, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Create a CORS layer with whitelisted origins
pub fn create_cors_layer(allowed_origins: Vec<String>) -> CorsLayer {
    let origins: Vec<_> = allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(false)
        // Chrome Private Network Access: public pages calling 127.0.0.1 need this on preflight
        .allow_private_network(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cors_layer() {
        let origins = vec![
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
        ];

        let _layer = create_cors_layer(origins);
        // If we get here without panicking, the layer was created successfully
    }
}

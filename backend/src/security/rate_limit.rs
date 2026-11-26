use governor::middleware::NoOpMiddleware;
use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::GlobalKeyExtractor, GovernorLayer,
};

/// Create a global rate limiting layer: `requests_per_minute` sustained, bursting up to the same amount.
///
/// The server binds to localhost and serves a single user, so a global bucket is used
/// (per-IP keys would all be 127.0.0.1 anyway and would require ConnectInfo).
pub fn create_rate_limit_layer(
    requests_per_minute: u32,
) -> GovernorLayer<GlobalKeyExtractor, NoOpMiddleware> {
    let rpm = requests_per_minute.max(1);
    let config = GovernorConfigBuilder::default()
        .key_extractor(GlobalKeyExtractor)
        // Replenish one token every (60s / rpm)
        .per_millisecond((60_000 / rpm as u64).max(1))
        .burst_size(rpm)
        .finish()
        .expect("Failed to build rate limit config");

    GovernorLayer {
        config: Arc::new(config),
    }
}

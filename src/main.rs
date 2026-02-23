//! Binary entry point for Brag Frog. Initializes config and starts the server.

use brag_frog::shared::config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::from_env();

    let (state, session_store) = brag_frog::app::build_state(config).await;

    if state.config.public_only {
        tracing::warn!(
            "PUBLIC-ONLY MODE ACTIVE: sync services will only fetch public/non-confidential data"
        );
    }

    let addr = format!("{}:{}", state.config.host, state.config.port);
    tracing::info!("Starting Brag Frog on {}", addr);

    let app = brag_frog::app::build_app(state, session_store);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}

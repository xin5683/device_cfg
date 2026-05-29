use axum::{
    Router,
    routing::{get, post},
};
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    device::wifi::{WifiConfig, WifiService},
    web::{api, assets},
};

pub async fn run() {
    let config = AppConfig::from_env();
    let service = Arc::new(WifiService::new(WifiConfig::from_env()));

    service.print_config();

    let app = Router::new()
        .route("/", get(assets::index))
        .route("/style.css", get(assets::style_css))
        .route("/app.js", get(assets::app_js))
        .route("/api/scan", get(api::scan_handler))
        .route("/api/status", get(api::status_handler))
        .route("/api/connect", post(api::connect_handler))
        .route("/api/disconnect", post(api::disconnect_handler))
        .with_state(service)
        .layer(build_cors());

    let addr = format!("0.0.0.0:{}", config.port);
    println!("Device Config 服务启动: http://{}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

struct AppConfig {
    port: String,
}

impl AppConfig {
    fn from_env() -> Self {
        Self {
            port: env::var("PORT").unwrap_or_else(|_| "80".to_string()),
        }
    }
}

fn build_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

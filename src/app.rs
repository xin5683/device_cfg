use axum::{
    Router,
    routing::{get, post},
};
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    device::{
        daemon::{DaemonConfig, DaemonService},
        update::{UpdateConfig, UpdateService},
        wifi::{WifiConfig, WifiService},
    },
    web::{api, assets},
};

#[derive(Clone)]
pub struct AppState {
    pub wifi: Arc<WifiService>,
    pub update: Arc<UpdateService>,
    pub daemon: Arc<DaemonService>,
}

pub async fn run() {
    let config = AppConfig::from_env();
    let wifi = Arc::new(WifiService::new(WifiConfig::from_env()));
    let update = Arc::new(UpdateService::new(UpdateConfig::from_env()));
    let daemon = Arc::new(DaemonService::new(DaemonConfig::from_env()));
    let state = AppState {
        wifi,
        update,
        daemon,
    };

    state.wifi.print_config();
    state.update.print_config();
    state.daemon.print_config();
    match state.daemon.start_if_auto_start().await {
        Ok(Some(result)) => println!("XPlaneUDP 自启动完成: pid={}", result.pid),
        Ok(None) => {}
        Err(err) => eprintln!("XPlaneUDP 自启动失败: {}", err),
    }

    let app = Router::new()
        .route("/", get(assets::index))
        .route("/assets/{*path}", get(assets::asset))
        .route("/api/system/info", get(api::system_info_handler))
        .route("/api/scan", get(api::scan_handler))
        .route("/api/status", get(api::status_handler))
        .route("/api/connect", post(api::connect_handler))
        .route("/api/disconnect", post(api::disconnect_handler))
        .route("/api/update/check", get(api::update_check_handler))
        .route("/api/update/apply", post(api::update_apply_handler))
        .route("/api/daemon/status", get(api::daemon_status_handler))
        .route("/api/daemon/check", get(api::daemon_check_handler))
        .route("/api/daemon/pull", post(api::daemon_pull_handler))
        .route("/api/daemon/upgrade", post(api::daemon_upgrade_handler))
        .route("/api/daemon/start", post(api::daemon_start_handler))
        .route("/api/daemon/restart", post(api::daemon_restart_handler))
        .route(
            "/api/daemon/auto-start",
            post(api::daemon_auto_start_handler),
        )
        .route("/api/daemon/logs", get(api::daemon_logs_handler))
        .route("/api/daemon/logs/ws", get(api::daemon_logs_ws_handler))
        .with_state(state)
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

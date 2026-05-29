mod handlers;
mod wifi;

use axum::{
    Router,
    routing::{get, post},
};
use std::env;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

const INDEX_HTML: &str = include_str!("static/index.html");
const STYLE_CSS: &str = include_str!("static/style.css");
const APP_JS: &str = include_str!("static/app.js");

async fn index() -> axum::response::Html<&'static str> {
    axum::response::Html(INDEX_HTML)
}

async fn style_css() -> (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        STYLE_CSS,
    )
}

async fn app_js() -> (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        APP_JS,
    )
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(index))
        .route("/style.css", get(style_css))
        .route("/app.js", get(app_js))
        .route("/api/scan", get(handlers::scan_handler))
        .route("/api/status", get(handlers::status_handler))
        .route("/api/connect", post(handlers::connect_handler))
        .route("/api/disconnect", post(handlers::disconnect_handler))
        .layer(cors);

    let port = env::var("PORT").unwrap_or_else(|_| "80".to_string());
    let addr = format!("0.0.0.0:{}", port);

    wifi::print_config();
    println!("WiFi 配置服务启动: http://{}", addr);
    println!("API 端点可独立访问，支持 CORS 跨域请求");

    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

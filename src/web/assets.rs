use axum::{http::StatusCode, response::Html};

const INDEX_HTML: &str = include_str!("../static/index.html");
const STYLE_CSS: &str = include_str!("../static/style.css");
const APP_JS: &str = include_str!("../static/app.js");
const ANSI_UP_JS: &str = include_str!("../static/vendor/ansi_up.js");

pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub async fn style_css() -> (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        STYLE_CSS,
    )
}

pub async fn app_js() -> (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        APP_JS,
    )
}

pub async fn ansi_up_js() -> (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        ANSI_UP_JS,
    )
}

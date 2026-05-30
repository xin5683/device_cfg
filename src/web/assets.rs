use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use include_dir::{Dir, include_dir};

static FRONTEND_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

pub async fn index() -> Response {
    serve_embedded_file("index.html")
}

pub async fn asset(Path(path): Path<String>) -> Response {
    serve_embedded_file(&format!("assets/{path}"))
}

fn serve_embedded_file(path: &str) -> Response {
    let normalized = path.trim_start_matches('/');
    if normalized.is_empty() || normalized.contains("..") {
        return StatusCode::NOT_FOUND.into_response();
    }

    match FRONTEND_DIST.get_file(normalized) {
        Some(file) => {
            let mime = mime_guess::from_path(normalized).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                file.contents(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

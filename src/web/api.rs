use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::app::AppState;

pub struct Utf8Json<T>(pub T);

impl<T: Serialize> IntoResponse for Utf8Json<T> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(bytes) => (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/json; charset=utf-8",
                )],
                bytes,
            )
                .into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> impl IntoResponse {
        Utf8Json(Self {
            success: true,
            data: Some(data),
            message: None,
        })
    }

    fn err(msg: impl Into<String>) -> impl IntoResponse {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Utf8Json(ApiResponse::<()> {
                success: false,
                data: None,
                message: Some(msg.into()),
            }),
        )
    }
}

#[derive(Serialize)]
pub struct NetworkInfo {
    pub ssid: String,
    pub bssid: String,
    pub signal: i32,
    pub security: String,
    pub frequency: u32,
}

#[derive(Serialize)]
pub struct StatusInfo {
    pub state: String,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub ip: Option<String>,
}

#[derive(Serialize)]
pub struct SystemInfo {
    pub version: &'static str,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub ssid: String,
    pub password: Option<String>,
}

pub async fn system_info_handler() -> impl IntoResponse {
    ApiResponse::ok(SystemInfo {
        version: env!("CARGO_PKG_VERSION"),
    })
    .into_response()
}

pub async fn scan_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.wifi.scan().await {
        Ok(networks) => {
            let data: Vec<NetworkInfo> = networks
                .into_iter()
                .map(|network| NetworkInfo {
                    ssid: network.ssid,
                    bssid: network.bssid,
                    signal: network.signal,
                    security: network.security,
                    frequency: network.frequency,
                })
                .collect();
            ApiResponse::ok(data).into_response()
        }
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn status_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.wifi.status().await {
        Ok(status) => {
            let data = StatusInfo {
                state: status.state,
                ssid: status.ssid,
                bssid: status.bssid,
                ip: status.ip,
            };
            ApiResponse::ok(data).into_response()
        }
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn connect_handler(
    State(state): State<AppState>,
    Json(req): Json<ConnectRequest>,
) -> impl IntoResponse {
    let password = req.password.as_deref().filter(|value| !value.is_empty());

    match state.wifi.connect(&req.ssid, password).await {
        Ok(()) => ApiResponse::ok("连接成功").into_response(),
        Err(e) => ApiResponse::<&str>::err(e.to_string()).into_response(),
    }
}

pub async fn disconnect_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.wifi.disconnect().await {
        Ok(()) => ApiResponse::ok("已断开连接").into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn update_check_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.update.check().await {
        Ok(info) => ApiResponse::ok(info).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn update_apply_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.update.apply().await {
        Ok(result) => ApiResponse::ok(result).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

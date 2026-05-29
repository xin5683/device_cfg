use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::device::wifi::WifiService;

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

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub ssid: String,
    pub password: Option<String>,
}

pub async fn scan_handler(State(service): State<Arc<WifiService>>) -> impl IntoResponse {
    match service.scan().await {
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

pub async fn status_handler(State(service): State<Arc<WifiService>>) -> impl IntoResponse {
    match service.status().await {
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
    State(service): State<Arc<WifiService>>,
    Json(req): Json<ConnectRequest>,
) -> impl IntoResponse {
    let password = req.password.as_deref().filter(|value| !value.is_empty());

    match service.connect(&req.ssid, password).await {
        Ok(()) => ApiResponse::ok("连接成功").into_response(),
        Err(e) => ApiResponse::<&str>::err(e.to_string()).into_response(),
    }
}

pub async fn disconnect_handler(State(service): State<Arc<WifiService>>) -> impl IntoResponse {
    match service.disconnect().await {
        Ok(()) => ApiResponse::ok("已断开连接").into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::wifi;

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
        Utf8Json(ApiResponse {
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

pub async fn scan_handler() -> impl IntoResponse {
    match wifi::scan().await {
        Ok(networks) => {
            let data: Vec<NetworkInfo> = networks
                .into_iter()
                .map(|n| NetworkInfo {
                    ssid: n.ssid,
                    bssid: n.bssid,
                    signal: n.signal,
                    security: n.security,
                    frequency: n.frequency,
                })
                .collect();
            ApiResponse::ok(data).into_response()
        }
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn status_handler() -> impl IntoResponse {
    match wifi::status().await {
        Ok(s) => {
            let data = StatusInfo {
                state: s.state,
                ssid: s.ssid,
                bssid: s.bssid,
                ip: s.ip,
            };
            ApiResponse::ok(data).into_response()
        }
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn connect_handler(Json(req): Json<ConnectRequest>) -> impl IntoResponse {
    let password = req.password.as_deref().filter(|p| !p.is_empty());

    match wifi::connect(&req.ssid, password).await {
        Ok(()) => ApiResponse::ok("连接成功").into_response(),
        Err(e) => ApiResponse::<&str>::err(e.to_string()).into_response(),
    }
}

pub async fn disconnect_handler() -> impl IntoResponse {
    match wifi::disconnect().await {
        Ok(()) => ApiResponse::ok("已断开连接").into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

use axum::Json;

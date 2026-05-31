use axum::{
    Json,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader, SeekFrom},
    process::Command,
    sync::watch,
    task::JoinHandle,
};

use crate::{
    app::AppState,
    device::system::{CanStatus, TimeStatus},
};

const LOG_WS_POLL_INTERVAL: Duration = Duration::from_millis(200);
const DIAGNOSTIC_WS_POLL_INTERVAL: Duration = Duration::from_secs(1);
const DIAGNOSTIC_NTP_REFRESH_TICKS: u64 = 30;

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

#[derive(Deserialize)]
pub struct LogQuery {
    pub lines: Option<usize>,
}

#[derive(Deserialize)]
pub struct DaemonAutoStartRequest {
    pub auto_start: bool,
}

#[derive(Serialize)]
pub struct DiagnosticStatus {
    pub can: CanStatus,
    pub time: TimeStatus,
}

pub async fn system_info_handler() -> impl IntoResponse {
    ApiResponse::ok(SystemInfo {
        version: env!("CARGO_PKG_VERSION"),
    })
    .into_response()
}

pub async fn system_reboot_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.system.reboot().await {
        Ok(result) => ApiResponse::ok(result).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
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

pub async fn daemon_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon.status().await {
        Ok(status) => ApiResponse::ok(status).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_check_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon.check().await {
        Ok(info) => ApiResponse::ok(info).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_pull_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon.pull().await {
        Ok(result) => ApiResponse::ok(result).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_upgrade_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon.upgrade().await {
        Ok(result) => ApiResponse::ok(result).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_start_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon.start().await {
        Ok(result) => ApiResponse::ok(result).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_restart_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon.restart().await {
        Ok(result) => ApiResponse::ok(result).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_auto_start_handler(
    State(state): State<AppState>,
    Json(req): Json<DaemonAutoStartRequest>,
) -> impl IntoResponse {
    match state.daemon.set_auto_start(req.auto_start).await {
        Ok(status) => ApiResponse::ok(status).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_logs_handler(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> impl IntoResponse {
    match state.daemon.logs(query.lines.unwrap_or(120)).await {
        Ok(logs) => ApiResponse::ok(logs).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn daemon_logs_ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let log_path = state.daemon.log_path();
    ws.on_upgrade(move |socket| stream_daemon_logs(socket, log_path))
}

pub async fn diagnostic_status_ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_diagnostic_status(socket, state))
}

pub async fn diagnostic_can_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.system.can_status().await {
        Ok(status) => ApiResponse::ok(status).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn diagnostic_time_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.system.time_status().await {
        Ok(status) => ApiResponse::ok(status).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

pub async fn diagnostic_time_sync_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.system.sync_time().await {
        Ok(result) => ApiResponse::ok(result).into_response(),
        Err(e) => ApiResponse::<()>::err(e.to_string()).into_response(),
    }
}

async fn stream_daemon_logs(socket: WebSocket, log_path: PathBuf) {
    let (mut sender, mut receiver) = socket.split();
    let mut offset = tokio::fs::metadata(&log_path)
        .await
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let mut ticker = tokio::time::interval(LOG_WS_POLL_INTERVAL);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                match read_new_log_chunk(&log_path, &mut offset).await {
                    Ok(Some(chunk)) => {
                        if sender.send(Message::Text(chunk.into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        if sender.send(Message::Text(format!("日志读取失败: {err}\n").into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
            message = receiver.next() => {
                match message {
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}
                }
            }
            else => break,
        }
    }
}

async fn stream_diagnostic_status(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut ticker = tokio::time::interval(DIAGNOSTIC_WS_POLL_INTERVAL);
    let mut tick_count = DIAGNOSTIC_NTP_REFRESH_TICKS;
    let mut cached_time: Option<TimeStatus> = None;
    let mut canbusload: Option<CanbusloadSession> = None;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let mut can = match state.system.can_status().await {
                    Ok(status) => status,
                    Err(err) => {
                        if send_diagnostic_error(&mut sender, err.to_string())
                            .await
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                };
                can.load_percent = update_canbusload_session(
                    &mut canbusload,
                    state.system.canbusload_bin(),
                    &can
                ).await;

                let should_refresh_time = tick_count >= DIAGNOSTIC_NTP_REFRESH_TICKS;
                let time = if should_refresh_time {
                    tick_count = 0;
                    match state.system.time_status().await {
                        Ok(status) => {
                            cached_time = Some(status.clone());
                            status
                        }
                        Err(err) => {
                            if send_diagnostic_error(&mut sender, err.to_string())
                                .await
                                .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                    }
                } else {
                    match state.system.time_status_from_cached(cached_time.as_ref()).await {
                        Ok(status) => status,
                        Err(err) => {
                            if send_diagnostic_error(&mut sender, err.to_string())
                                .await
                                .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                    }
                };
                tick_count += 1;

                let payload = ApiResponse {
                    success: true,
                    data: Some(DiagnosticStatus { can, time }),
                    message: None,
                };

                let message = match serde_json::to_string(&payload) {
                    Ok(message) => message,
                    Err(err) => {
                        if send_diagnostic_error(&mut sender, err.to_string())
                            .await
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                };

                if sender.send(Message::Text(message.into())).await.is_err() {
                    break;
                }
            }
            message = receiver.next() => {
                match message {
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}
                }
            }
            else => break,
        }
    }

    if let Some(session) = canbusload {
        session.stop().await;
    }
}

struct CanbusloadSession {
    iface: String,
    bitrate: u64,
    load_rx: watch::Receiver<Option<f64>>,
    child: tokio::process::Child,
    reader_task: JoinHandle<()>,
}

impl CanbusloadSession {
    fn start(bin: &str, iface: &str, bitrate: u64) -> Option<Self> {
        let target = format!("{iface}@{bitrate}");
        let mut child = Command::new(bin)
            .arg(target)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        let (load_tx, load_rx) = watch::channel(None);
        let iface = iface.to_string();
        let reader_iface = iface.clone();
        let reader_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(percent) = parse_canbusload_percent(&line, &reader_iface) {
                    let _ = load_tx.send(Some(percent));
                }
            }
        });

        Some(Self {
            iface,
            bitrate,
            load_rx,
            child,
            reader_task,
        })
    }

    fn matches(&self, iface: &str, bitrate: u64) -> bool {
        self.iface == iface && self.bitrate == bitrate
    }

    fn load_percent(&self) -> Option<f64> {
        *self.load_rx.borrow()
    }

    async fn stop(mut self) {
        let _ = self.child.kill().await;
        self.reader_task.abort();
    }
}

async fn update_canbusload_session(
    session: &mut Option<CanbusloadSession>,
    bin: &str,
    can: &CanStatus,
) -> Option<f64> {
    let Some(bitrate) = can.bitrate else {
        if let Some(session) = session.take() {
            session.stop().await;
        }
        return None;
    };
    if bitrate == 0 || !can.exists || !can.up {
        if let Some(session) = session.take() {
            session.stop().await;
        }
        return None;
    }

    let should_restart = session
        .as_ref()
        .is_none_or(|session| !session.matches(&can.iface, bitrate));
    if should_restart {
        if let Some(session) = session.take() {
            session.stop().await;
        }
        *session = CanbusloadSession::start(bin, &can.iface, bitrate);
    }

    session.as_ref().and_then(CanbusloadSession::load_percent)
}

fn parse_canbusload_percent(line: &str, iface: &str) -> Option<f64> {
    let prefix = format!("{iface}@");
    let mut fields = line.split_whitespace();
    let interface = fields.next()?;
    if !interface.starts_with(&prefix) {
        return None;
    }

    line.split_whitespace()
        .filter_map(|part| part.strip_suffix('%')?.parse::<f64>().ok())
        .last()
}

async fn send_diagnostic_error(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: String,
) -> std::result::Result<(), axum::Error> {
    let payload = ApiResponse::<()> {
        success: false,
        data: None,
        message: Some(message),
    };
    let message = serde_json::to_string(&payload)
        .unwrap_or_else(|_| "{\"success\":false,\"message\":\"诊断状态序列化失败\"}".to_string());
    sender.send(Message::Text(message.into())).await
}

async fn read_new_log_chunk(path: &Path, offset: &mut u64) -> io::Result<Option<String>> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    let file_len = metadata.len();

    if file_len < *offset {
        *offset = 0;
    }
    if file_len <= *offset {
        return Ok(None);
    }

    let mut file = File::open(path).await?;
    file.seek(SeekFrom::Start(*offset)).await?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    *offset += buffer.len() as u64;

    if buffer.is_empty() {
        Ok(None)
    } else {
        Ok(Some(String::from_utf8_lossy(&buffer).into_owned()))
    }
}

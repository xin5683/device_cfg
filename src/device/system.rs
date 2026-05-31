use serde::Serialize;
use std::{env, net::SocketAddr, path::PathBuf, time::SystemTime};
use tokio::{
    net::UdpSocket,
    process::Command,
    time::{Duration, sleep, timeout},
};

const DEFAULT_CAN_IFACE: &str = "can0";
const DEFAULT_CANBUSLOAD_BIN: &str = "canbusload";
const DEFAULT_NTP_SERVER: &str = "pool.ntp.org";
const DEFAULT_NTPD_BIN: &str = "ntpd";
const DEFAULT_REBOOT_BIN: &str = "reboot";
const NTP_UNIX_EPOCH_OFFSET: u64 = 2_208_988_800;
const NTP_QUERY_TIMEOUT: Duration = Duration::from_secs(4);
const NTP_SYNC_TIMEOUT: Duration = Duration::from_secs(20);
const REBOOT_DELAY: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct SystemDiagnosticConfig {
    can_iface: String,
    canbusload_bin: String,
    can_bitrate: Option<u64>,
    ntp_server: String,
    ntpd_bin: String,
    reboot_bin: String,
}

impl SystemDiagnosticConfig {
    pub fn from_env() -> Self {
        Self {
            can_iface: env::var("CAN_IFACE").unwrap_or_else(|_| DEFAULT_CAN_IFACE.to_string()),
            canbusload_bin: env::var("CANBUSLOAD_BIN")
                .unwrap_or_else(|_| DEFAULT_CANBUSLOAD_BIN.to_string()),
            can_bitrate: env::var("CAN_BITRATE")
                .ok()
                .and_then(|value| value.parse::<u64>().ok()),
            ntp_server: env::var("NTP_SERVER").unwrap_or_else(|_| DEFAULT_NTP_SERVER.to_string()),
            ntpd_bin: env::var("NTPD_BIN").unwrap_or_else(|_| DEFAULT_NTPD_BIN.to_string()),
            reboot_bin: env::var("REBOOT_BIN").unwrap_or_else(|_| DEFAULT_REBOOT_BIN.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct SystemDiagnosticError(pub String);

impl From<std::io::Error> for SystemDiagnosticError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl std::fmt::Display for SystemDiagnosticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type Result<T> = std::result::Result<T, SystemDiagnosticError>;

#[derive(Debug, Clone, Serialize)]
pub struct CanStatus {
    pub iface: String,
    pub exists: bool,
    pub operstate: String,
    pub carrier: Option<bool>,
    pub up: bool,
    pub bitrate: Option<u64>,
    pub load_percent: Option<f64>,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeStatus {
    pub board_time: String,
    pub board_unix_ms: u128,
    pub internet_time: Option<String>,
    pub internet_unix_ms: Option<u128>,
    pub internet_connected: bool,
    pub ntp_server: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeSyncResult {
    pub synced: bool,
    pub ntp_server: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RebootResult {
    pub scheduled: bool,
    pub delay_seconds: u64,
}

pub struct SystemDiagnosticService {
    config: SystemDiagnosticConfig,
}

impl SystemDiagnosticService {
    pub fn new(config: SystemDiagnosticConfig) -> Self {
        Self { config }
    }

    pub fn print_config(&self) {
        println!(
            "诊断配置: can_iface={}, canbusload_bin={}, can_bitrate={:?}, ntp_server={}, ntpd_bin={}, reboot_bin={}",
            self.config.can_iface,
            self.config.canbusload_bin,
            self.config.can_bitrate,
            self.config.ntp_server,
            self.config.ntpd_bin,
            self.config.reboot_bin
        );
    }

    pub fn canbusload_bin(&self) -> &str {
        &self.config.canbusload_bin
    }

    pub async fn can_status(&self) -> Result<CanStatus> {
        let base = PathBuf::from("/sys/class/net").join(&self.config.can_iface);
        let exists = tokio::fs::metadata(&base).await.is_ok();

        if !exists {
            return Ok(CanStatus {
                iface: self.config.can_iface.clone(),
                exists: false,
                operstate: "missing".to_string(),
                carrier: None,
                up: false,
                bitrate: None,
                load_percent: None,
                rx_packets: 0,
                tx_packets: 0,
                rx_bytes: 0,
                tx_bytes: 0,
                rx_errors: 0,
                tx_errors: 0,
            });
        }

        let operstate = read_trimmed(base.join("operstate"))
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        let carrier = read_trimmed(base.join("carrier"))
            .await
            .ok()
            .and_then(|value| match value.as_str() {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            });
        let flags = read_trimmed(base.join("flags")).await.unwrap_or_default();
        let up = operstate == "up" || parse_iface_up_flag(&flags);
        let bitrate = read_can_bitrate(&base, &self.config.can_iface)
            .await
            .or(self.config.can_bitrate);

        Ok(CanStatus {
            iface: self.config.can_iface.clone(),
            exists: true,
            operstate,
            carrier,
            up,
            bitrate,
            load_percent: None,
            rx_packets: read_u64(base.join("statistics/rx_packets")).await,
            tx_packets: read_u64(base.join("statistics/tx_packets")).await,
            rx_bytes: read_u64(base.join("statistics/rx_bytes")).await,
            tx_bytes: read_u64(base.join("statistics/tx_bytes")).await,
            rx_errors: read_u64(base.join("statistics/rx_errors")).await,
            tx_errors: read_u64(base.join("statistics/tx_errors")).await,
        })
    }

    pub async fn time_status(&self) -> Result<TimeStatus> {
        let board_time = read_board_time().await.unwrap_or_else(|_| "-".to_string());
        let board_unix_ms = unix_time_ms(SystemTime::now()).unwrap_or(0);

        match query_ntp_time(&self.config.ntp_server).await {
            Ok(internet_time) => Ok(TimeStatus {
                board_time,
                board_unix_ms,
                internet_time: Some(format_unix_time(internet_time).await),
                internet_unix_ms: Some(unix_time_ms(internet_time).unwrap_or(0)),
                internet_connected: true,
                ntp_server: self.config.ntp_server.clone(),
                error: None,
            }),
            Err(err) => Ok(TimeStatus {
                board_time,
                board_unix_ms,
                internet_time: None,
                internet_unix_ms: None,
                internet_connected: false,
                ntp_server: self.config.ntp_server.clone(),
                error: Some(err.to_string()),
            }),
        }
    }

    pub async fn time_status_from_cached(&self, cached: Option<&TimeStatus>) -> Result<TimeStatus> {
        let board_time = read_board_time().await.unwrap_or_else(|_| "-".to_string());
        let board_unix_ms = unix_time_ms(SystemTime::now()).unwrap_or(0);

        if let Some(cached) = cached {
            let internet_unix_ms = cached.internet_unix_ms.map(|internet_ms| {
                let board_delta = board_unix_ms.saturating_sub(cached.board_unix_ms);
                internet_ms.saturating_add(board_delta)
            });
            let internet_time = if let Some(internet_unix_ms) = internet_unix_ms {
                let duration = Duration::from_millis(internet_unix_ms.min(u64::MAX as u128) as u64);
                Some(format_unix_time(SystemTime::UNIX_EPOCH + duration).await)
            } else {
                cached.internet_time.clone()
            };

            return Ok(TimeStatus {
                board_time,
                board_unix_ms,
                internet_time,
                internet_unix_ms,
                internet_connected: cached.internet_connected,
                ntp_server: cached.ntp_server.clone(),
                error: cached.error.clone(),
            });
        }

        Ok(TimeStatus {
            board_time,
            board_unix_ms,
            internet_time: None,
            internet_unix_ms: None,
            internet_connected: false,
            ntp_server: self.config.ntp_server.clone(),
            error: Some("正在检测互联网时间源".to_string()),
        })
    }

    pub async fn sync_time(&self) -> Result<TimeSyncResult> {
        let command = Command::new(&self.config.ntpd_bin)
            .args(["-q", "-p", &self.config.ntp_server])
            .output();

        let output = timeout(NTP_SYNC_TIMEOUT, command)
            .await
            .map_err(|_| SystemDiagnosticError("ntpd 同步超时".to_string()))??;
        let mut text = String::new();
        text.push_str(&String::from_utf8_lossy(&output.stdout));
        text.push_str(&String::from_utf8_lossy(&output.stderr));

        if output.status.success() {
            Ok(TimeSyncResult {
                synced: true,
                ntp_server: self.config.ntp_server.clone(),
                output: text.trim().to_string(),
            })
        } else {
            Err(SystemDiagnosticError(format!(
                "ntpd 同步失败: {}",
                text.trim()
            )))
        }
    }

    pub async fn reboot(&self) -> Result<RebootResult> {
        let reboot_bin = self.config.reboot_bin.clone();
        tokio::spawn(async move {
            sleep(REBOOT_DELAY).await;
            if let Err(err) = Command::new(&reboot_bin).status().await {
                eprintln!("设备重启命令执行失败: {}", err);
            }
        });

        Ok(RebootResult {
            scheduled: true,
            delay_seconds: REBOOT_DELAY.as_secs(),
        })
    }
}

async fn read_trimmed(path: PathBuf) -> Result<String> {
    Ok(tokio::fs::read_to_string(path).await?.trim().to_string())
}

async fn read_u64(path: PathBuf) -> u64 {
    read_optional_u64(path).await.unwrap_or(0)
}

async fn read_optional_u64(path: PathBuf) -> Option<u64> {
    read_trimmed(path)
        .await
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
}

async fn read_can_bitrate(base: &std::path::Path, iface: &str) -> Option<u64> {
    if let Some(bitrate) = read_optional_u64(base.join("can_bittiming/bitrate")).await {
        return Some(bitrate);
    }

    read_can_bitrate_from_ip(iface).await
}

async fn read_can_bitrate_from_ip(iface: &str) -> Option<u64> {
    let output = Command::new("ip")
        .args(["-details", "link", "show", iface])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }

    parse_can_bitrate(&String::from_utf8_lossy(&output.stdout))
}

fn parse_can_bitrate(output: &str) -> Option<u64> {
    let mut tokens = output.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == "bitrate" {
            return tokens.next()?.parse::<u64>().ok();
        }
    }
    None
}

fn parse_iface_up_flag(flags: &str) -> bool {
    let value = flags.strip_prefix("0x").unwrap_or(flags);
    u32::from_str_radix(value, 16)
        .map(|flags| flags & 0x1 == 0x1)
        .unwrap_or(false)
}

async fn read_board_time() -> Result<String> {
    let output = Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S %Z"])
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(SystemDiagnosticError(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

async fn query_ntp_time(server: &str) -> Result<SystemTime> {
    let addr = resolve_ntp_addr(server).await?;
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let mut packet = [0_u8; 48];
    packet[0] = 0b00_100_011;

    timeout(NTP_QUERY_TIMEOUT, socket.send_to(&packet, addr))
        .await
        .map_err(|_| SystemDiagnosticError("NTP 请求超时".to_string()))??;

    let mut response = [0_u8; 48];
    let (len, _) = timeout(NTP_QUERY_TIMEOUT, socket.recv_from(&mut response))
        .await
        .map_err(|_| SystemDiagnosticError("NTP 响应超时".to_string()))??;

    if len < 48 {
        return Err(SystemDiagnosticError("NTP 响应长度无效".to_string()));
    }

    let seconds =
        u32::from_be_bytes([response[40], response[41], response[42], response[43]]) as u64;
    let fraction =
        u32::from_be_bytes([response[44], response[45], response[46], response[47]]) as u64;
    if seconds < NTP_UNIX_EPOCH_OFFSET {
        return Err(SystemDiagnosticError("NTP 时间无效".to_string()));
    }

    let unix_seconds = seconds - NTP_UNIX_EPOCH_OFFSET;
    let nanos = ((fraction as u128 * 1_000_000_000_u128) >> 32) as u32;
    Ok(SystemTime::UNIX_EPOCH + Duration::new(unix_seconds, nanos))
}

async fn resolve_ntp_addr(server: &str) -> Result<SocketAddr> {
    let target = format!("{server}:123");
    let mut addrs = tokio::net::lookup_host(&target).await?;
    addrs
        .next()
        .ok_or_else(|| SystemDiagnosticError(format!("无法解析 NTP 服务器: {server}")))
}

async fn format_unix_time(time: SystemTime) -> String {
    let seconds = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|value| value.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());

    let output = Command::new("date")
        .args(["-d", &format!("@{seconds}"), "+%Y-%m-%d %H:%M:%S UTC"])
        .env("TZ", "UTC")
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => seconds,
    }
}

fn unix_time_ms(time: SystemTime) -> Option<u128> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

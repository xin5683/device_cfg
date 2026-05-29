use std::{collections::HashMap, time::Duration};
use tokio::{process::Command, time::sleep};

const SCAN_RETRY_COUNT: usize = 5;
const SCAN_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const CONNECT_RETRY_COUNT: usize = 20;
const CONNECT_RETRY_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct WifiConfig {
    ctrl_dir: String,
    iface: String,
}

impl WifiConfig {
    pub fn from_env() -> Self {
        Self {
            ctrl_dir: std::env::var("WPA_CTRL_DIR")
                .unwrap_or_else(|_| "/var/run/wpa_supplicant".to_string()),
            iface: std::env::var("WIFI_IFACE").unwrap_or_else(|_| "wlan0".to_string()),
        }
    }

    pub fn iface(&self) -> &str {
        &self.iface
    }

    pub fn ctrl_dir(&self) -> &str {
        &self.ctrl_dir
    }
}

#[derive(Debug)]
pub struct WifiError(pub String);

impl From<std::io::Error> for WifiError {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

impl From<String> for WifiError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for WifiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type Result<T> = std::result::Result<T, WifiError>;

#[derive(Debug, Clone)]
pub struct Network {
    pub ssid: String,
    pub bssid: String,
    pub signal: i32,
    pub security: String,
    pub frequency: u32,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub state: String,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub ip: Option<String>,
}

pub struct WifiService {
    config: WifiConfig,
}

impl WifiService {
    pub fn new(config: WifiConfig) -> Self {
        Self { config }
    }

    pub fn print_config(&self) {
        println!(
            "WiFi 配置: ctrl_dir={}, iface={}",
            self.config.ctrl_dir(),
            self.config.iface()
        );
    }

    pub async fn scan(&self) -> Result<Vec<Network>> {
        let output = self.wpa_cli_cmd(&["scan"]).await?;
        ensure_wpa_ok(&output, "启动扫描")?;

        let mut last_results = String::new();
        for _ in 0..SCAN_RETRY_COUNT {
            sleep(SCAN_RETRY_INTERVAL).await;
            let results = self.wpa_cli_cmd(&["scan_results"]).await?;
            if has_scan_data(&results) {
                return parse_scan_results(&results);
            }
            last_results = results;
        }

        parse_scan_results(&last_results)
    }

    pub async fn status(&self) -> Result<Status> {
        let output = self.wpa_cli_cmd(&["status"]).await?;

        let mut state = String::from("DISCONNECTED");
        let mut ssid = None;
        let mut bssid = None;

        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "wpa_state" => state = value.to_string(),
                    "ssid" => ssid = Some(value.to_string()),
                    "bssid" => bssid = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        let ip = self.get_ip_address().await.ok().flatten();

        Ok(Status {
            state,
            ssid,
            bssid,
            ip,
        })
    }

    pub async fn connect(&self, ssid: &str, password: Option<&str>) -> Result<()> {
        self.remove_existing_networks(ssid).await?;

        let output = self.wpa_cli_cmd(&["add_network"]).await?;
        let net_id = output.trim().parse::<u32>().map_err(|_| {
            WifiError(format!(
                "无法获取网络 ID: {}",
                if output.trim().is_empty() {
                    "空响应".to_string()
                } else {
                    output.trim().to_string()
                }
            ))
        })?;
        let net_id_str = net_id.to_string();

        let output = self
            .wpa_cli_cmd(&["set_network", &net_id_str, "ssid", &format!("\"{}\"", ssid)])
            .await?;
        ensure_wpa_ok(&output, "设置 SSID")?;

        if let Some(pwd) = password {
            if pwd.is_empty() {
                let output = self
                    .wpa_cli_cmd(&["set_network", &net_id_str, "key_mgmt", "NONE"])
                    .await?;
                ensure_wpa_ok(&output, "设置开放网络模式")?;
            } else {
                let output = self
                    .wpa_cli_cmd(&["set_network", &net_id_str, "psk", &format!("\"{}\"", pwd)])
                    .await?;
                ensure_wpa_ok(&output, "设置 WiFi 密码")?;
            }
        } else {
            let output = self
                .wpa_cli_cmd(&["set_network", &net_id_str, "key_mgmt", "NONE"])
                .await?;
            ensure_wpa_ok(&output, "设置开放网络模式")?;
        }

        let output = self.wpa_cli_cmd(&["select_network", &net_id_str]).await?;
        ensure_wpa_ok(&output, "切换目标网络")?;

        let output = self.wpa_cli_cmd(&["enable_network", &net_id_str]).await?;
        ensure_wpa_ok(&output, "启用目标网络")?;

        let output = self.wpa_cli_cmd(&["save_config"]).await?;
        ensure_wpa_ok(&output, "保存网络配置")?;

        let mut last_state = String::from("UNKNOWN");
        for _ in 0..CONNECT_RETRY_COUNT {
            sleep(CONNECT_RETRY_INTERVAL).await;
            if let Ok(status) = self.status().await {
                last_state = status.state.clone();
                if status.state == "COMPLETED" {
                    let _ = Command::new("udhcpc")
                        .args(["-i", self.config.iface(), "-b"])
                        .output()
                        .await;
                    sleep(Duration::from_secs(2)).await;
                    return Ok(());
                }
            }
        }

        let detail = self.collect_connection_diagnostics().await;
        Err(WifiError(format!(
            "连接超时，最后状态: {}，诊断信息: {}",
            last_state,
            detail.replace('\n', "; ")
        )))
    }

    pub async fn disconnect(&self) -> Result<()> {
        let output = self.wpa_cli_cmd(&["disconnect"]).await?;
        ensure_wpa_ok(&output, "断开 WiFi")?;
        Ok(())
    }

    async fn wpa_cli_cmd(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("wpa_cli")
            .arg("-p")
            .arg(self.config.ctrl_dir())
            .arg("-i")
            .arg(self.config.iface())
            .args(args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WifiError(format!("wpa_cli 命令失败: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn get_ip_address(&self) -> Result<Option<String>> {
        let output = Command::new("ip")
            .args(["-4", "addr", "show", self.config.iface()])
            .output()
            .await?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("inet ")
                && let Some(addr) = line.split_whitespace().nth(1)
                && let Some(ip) = addr.split('/').next()
            {
                return Ok(Some(ip.to_string()));
            }
        }

        Ok(None)
    }

    async fn remove_existing_networks(&self, ssid: &str) -> Result<()> {
        let output = self.wpa_cli_cmd(&["list_networks"]).await?;

        for line in output.lines().skip(1) {
            let mut parts = line.split('\t');
            let Some(id) = parts.next() else {
                continue;
            };
            let Some(existing_ssid) = parts.next() else {
                continue;
            };

            if existing_ssid == ssid {
                let output = self.wpa_cli_cmd(&["remove_network", id]).await?;
                ensure_wpa_ok(&output, "清理旧网络配置")?;
            }
        }

        Ok(())
    }

    async fn collect_connection_diagnostics(&self) -> String {
        let status = self
            .wpa_cli_cmd(&["status"])
            .await
            .unwrap_or_else(|e| format!("status: {}", e));
        let networks = self
            .wpa_cli_cmd(&["list_networks"])
            .await
            .unwrap_or_else(|e| format!("list_networks: {}", e));

        format!(
            "status => {} | list_networks => {}",
            status.trim(),
            networks.trim()
        )
    }
}

fn ensure_wpa_ok(output: &str, action: &str) -> Result<()> {
    let trimmed = output.trim();
    if trimmed == "OK" {
        return Ok(());
    }

    Err(WifiError(format!(
        "{} 失败: {}",
        action,
        if trimmed.is_empty() {
            "空响应"
        } else {
            trimmed
        }
    )))
}

fn has_scan_data(results: &str) -> bool {
    results.lines().skip(1).any(|line| !line.trim().is_empty())
}

fn parse_scan_results(results: &str) -> Result<Vec<Network>> {
    let mut networks: HashMap<String, Network> = HashMap::new();

    for line in results.lines().skip(1) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }

        let bssid = parts[0].to_string();
        let frequency = parts[1].parse().unwrap_or(0);
        let signal = parts[2].parse().unwrap_or(-100);
        let flags = parts[3];
        let ssid_raw = parts[4..].join("\t");
        let ssid = decode_wpa_ssid(&ssid_raw);

        if ssid.is_empty() {
            continue;
        }

        let security = if flags.contains("WPA2") {
            "WPA2"
        } else if flags.contains("WPA") {
            "WPA"
        } else if flags.contains("WEP") {
            "WEP"
        } else {
            "开放"
        }
        .to_string();

        if let Some(existing) = networks.get(&ssid)
            && existing.signal >= signal
        {
            continue;
        }

        networks.insert(
            ssid.clone(),
            Network {
                ssid,
                bssid,
                signal,
                security,
                frequency,
            },
        );
    }

    let mut result: Vec<Network> = networks.into_values().collect();
    result.sort_by(|a, b| b.signal.cmp(&a.signal));
    Ok(result)
}

fn decode_wpa_ssid(source: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'x') {
            chars.next();
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                bytes.push(byte);
            }
        } else {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }

    String::from_utf8_lossy(&bytes).to_string()
}

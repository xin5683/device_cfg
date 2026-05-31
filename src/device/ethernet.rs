use serde::{Deserialize, Serialize};
use std::{env, net::Ipv4Addr, path::PathBuf};
use tokio::{fs, process::Command, time::timeout};

const DEFAULT_ETHERNET_IFACE: &str = "eth0";
const DEFAULT_ETHERNET_CONFIG_PATH: &str = "/etc/network/interfaces.d/eth0";
const DEFAULT_NETWORK_RESTART_BIN: &str = "/etc/init.d/S40network";
const NETWORK_RESTART_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Clone)]
pub struct EthernetConfig {
    iface: String,
    config_path: PathBuf,
    network_restart_bin: String,
}

impl EthernetConfig {
    pub fn from_env() -> Self {
        Self {
            iface: env::var("ETHERNET_IFACE").unwrap_or_else(|_| DEFAULT_ETHERNET_IFACE.to_string()),
            config_path: env::var("ETHERNET_CONFIG_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(DEFAULT_ETHERNET_CONFIG_PATH)),
            network_restart_bin: env::var("NETWORK_RESTART_BIN")
                .unwrap_or_else(|_| DEFAULT_NETWORK_RESTART_BIN.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct EthernetError(pub String);

impl From<std::io::Error> for EthernetError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl std::fmt::Display for EthernetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type Result<T> = std::result::Result<T, EthernetError>;

#[derive(Debug, Clone, Serialize)]
pub struct EthernetStatus {
    pub iface: String,
    pub exists: bool,
    pub operstate: String,
    pub carrier: Option<bool>,
    pub up: bool,
    pub mac_address: Option<String>,
    pub ipv4_addresses: Vec<String>,
    pub primary_ipv4: Option<String>,
    pub netmask: Option<String>,
    pub gateway: Option<String>,
    pub dns_nameservers: Vec<String>,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub config_mode: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EthernetMode {
    Dhcp,
    Static,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetSettings {
    pub mode: EthernetMode,
    pub address: Option<String>,
    pub netmask: Option<String>,
    pub gateway: Option<String>,
    #[serde(default)]
    pub dns_nameservers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EthernetConfigView {
    pub iface: String,
    pub path: String,
    pub settings: EthernetSettings,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EthernetApplyResult {
    pub config: EthernetConfigView,
    pub status: EthernetStatus,
    pub restart_output: String,
}

pub struct EthernetService {
    config: EthernetConfig,
}

impl EthernetService {
    pub fn new(config: EthernetConfig) -> Self {
        Self { config }
    }

    pub fn print_config(&self) {
        println!(
            "以太网配置: iface={}, config_path={}, network_restart_bin={}",
            self.config.iface,
            self.config.config_path.display(),
            self.config.network_restart_bin
        );
    }

    pub async fn status(&self) -> Result<EthernetStatus> {
        let base = PathBuf::from("/sys/class/net").join(&self.config.iface);
        let exists = fs::metadata(&base).await.is_ok();
        let config_mode = self
            .read_config()
            .await
            .ok()
            .map(|config| match config.settings.mode {
                EthernetMode::Dhcp => "dhcp".to_string(),
                EthernetMode::Static => "static".to_string(),
            });

        if !exists {
            return Ok(EthernetStatus {
                iface: self.config.iface.clone(),
                exists: false,
                operstate: "missing".to_string(),
                carrier: None,
                up: false,
                mac_address: None,
                ipv4_addresses: Vec::new(),
                primary_ipv4: None,
                netmask: None,
                gateway: None,
                dns_nameservers: read_dns_nameservers().await,
                rx_packets: 0,
                tx_packets: 0,
                rx_bytes: 0,
                tx_bytes: 0,
                rx_errors: 0,
                tx_errors: 0,
                config_mode,
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
        let mac_address = read_trimmed(base.join("address")).await.ok();
        let ip_info = read_ipv4_info(&self.config.iface).await;
        let gateway = read_default_gateway(&self.config.iface).await;

        Ok(EthernetStatus {
            iface: self.config.iface.clone(),
            exists: true,
            operstate,
            carrier,
            up,
            mac_address,
            ipv4_addresses: ip_info.addresses,
            primary_ipv4: ip_info.primary,
            netmask: ip_info.netmask,
            gateway,
            dns_nameservers: read_dns_nameservers().await,
            rx_packets: read_u64(base.join("statistics/rx_packets")).await,
            tx_packets: read_u64(base.join("statistics/tx_packets")).await,
            rx_bytes: read_u64(base.join("statistics/rx_bytes")).await,
            tx_bytes: read_u64(base.join("statistics/tx_bytes")).await,
            rx_errors: read_u64(base.join("statistics/rx_errors")).await,
            tx_errors: read_u64(base.join("statistics/tx_errors")).await,
            config_mode,
        })
    }

    pub async fn read_config(&self) -> Result<EthernetConfigView> {
        let raw = fs::read_to_string(&self.config.config_path)
            .await
            .unwrap_or_default();
        let settings = parse_ethernet_settings(&raw);

        Ok(EthernetConfigView {
            iface: self.config.iface.clone(),
            path: self.config.config_path.display().to_string(),
            settings,
            raw,
        })
    }

    pub async fn apply_config(&self, settings: EthernetSettings) -> Result<EthernetApplyResult> {
        validate_settings(&settings)?;

        let body = render_config(&self.config.iface, &settings);
        if let Some(parent) = self.config.config_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&self.config.config_path, body).await?;

        let restart_output = self.restart_network().await?;
        let config = self.read_config().await?;
        let status = self.status().await?;

        Ok(EthernetApplyResult {
            config,
            status,
            restart_output,
        })
    }

    async fn restart_network(&self) -> Result<String> {
        let command = Command::new(&self.config.network_restart_bin)
            .arg("restart")
            .output();
        let output = timeout(NETWORK_RESTART_TIMEOUT, command)
            .await
            .map_err(|_| EthernetError("网络服务重启超时".to_string()))??;

        let mut text = String::new();
        text.push_str(&String::from_utf8_lossy(&output.stdout));
        text.push_str(&String::from_utf8_lossy(&output.stderr));

        if output.status.success() {
            Ok(text.trim().to_string())
        } else {
            Err(EthernetError(format!("网络服务重启失败: {}", text.trim())))
        }
    }
}

#[derive(Default)]
struct Ipv4Info {
    addresses: Vec<String>,
    primary: Option<String>,
    netmask: Option<String>,
}

fn validate_settings(settings: &EthernetSettings) -> Result<()> {
    match settings.mode {
        EthernetMode::Dhcp => Ok(()),
        EthernetMode::Static => {
            require_ipv4(settings.address.as_deref(), "静态 IP 地址")?;
            require_ipv4(settings.netmask.as_deref(), "子网掩码")?;
            validate_optional_ipv4(settings.gateway.as_deref(), "网关")?;
            for dns in &settings.dns_nameservers {
                parse_ipv4(dns, "DNS 服务器")?;
            }
            Ok(())
        }
    }
}

fn require_ipv4(value: Option<&str>, label: &str) -> Result<()> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Err(EthernetError(format!("{label}不能为空")));
    };
    parse_ipv4(value, label)
}

fn validate_optional_ipv4(value: Option<&str>, label: &str) -> Result<()> {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        parse_ipv4(value, label)?;
    }
    Ok(())
}

fn parse_ipv4(value: &str, label: &str) -> Result<()> {
    value
        .trim()
        .parse::<Ipv4Addr>()
        .map(|_| ())
        .map_err(|_| EthernetError(format!("{label}格式无效: {value}")))
}

fn render_config(iface: &str, settings: &EthernetSettings) -> String {
    match settings.mode {
        EthernetMode::Dhcp => format!(
            "auto {iface}\niface {iface} inet dhcp\n    pre-up /etc/network/nfs_check\n    pre-up /usr/sbin/eth-mac-prepare {iface}\n    wait-delay 15\n    hostname $(hostname)\n"
        ),
        EthernetMode::Static => {
            let mut body = format!(
                "auto {iface}\niface {iface} inet static\n    pre-up /etc/network/nfs_check\n    pre-up /usr/sbin/eth-mac-prepare {iface}\n    address {}\n    netmask {}\n",
                settings.address.as_deref().unwrap_or_default().trim(),
                settings.netmask.as_deref().unwrap_or_default().trim()
            );
            if let Some(gateway) = settings.gateway.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
                body.push_str("    gateway ");
                body.push_str(gateway);
                body.push('\n');
            }
            let dns_nameservers: Vec<&str> = settings
                .dns_nameservers
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .collect();
            if !dns_nameservers.is_empty() {
                body.push_str("    dns-nameservers ");
                body.push_str(&dns_nameservers.join(" "));
                body.push('\n');
            }
            body
        }
    }
}

fn parse_ethernet_settings(raw: &str) -> EthernetSettings {
    let mut settings = EthernetSettings {
        mode: if raw.contains(" inet static") {
            EthernetMode::Static
        } else {
            EthernetMode::Dhcp
        },
        address: None,
        netmask: None,
        gateway: None,
        dns_nameservers: Vec::new(),
    };

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("address") => settings.address = parts.next().map(str::to_string),
            Some("netmask") => settings.netmask = parts.next().map(str::to_string),
            Some("gateway") => settings.gateway = parts.next().map(str::to_string),
            Some("dns-nameservers") => {
                settings.dns_nameservers = parts.map(str::to_string).collect();
            }
            _ => {}
        }
    }

    settings
}

async fn read_ipv4_info(iface: &str) -> Ipv4Info {
    let output = Command::new("ip")
        .args(["-4", "-o", "addr", "show", "dev", iface])
        .output()
        .await;
    let Ok(output) = output else {
        return Ipv4Info::default();
    };
    if !output.status.success() {
        return Ipv4Info::default();
    }

    let mut info = Ipv4Info::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.split_whitespace();
        while let Some(field) = fields.next() {
            if field == "inet"
                && let Some(cidr) = fields.next()
            {
                info.addresses.push(cidr.to_string());
                if info.primary.is_none() {
                    let (address, prefix) = cidr
                        .split_once('/')
                        .map(|(address, prefix)| (address.to_string(), prefix.parse::<u8>().ok()))
                        .unwrap_or_else(|| (cidr.to_string(), None));
                    info.primary = Some(address);
                    info.netmask = prefix.and_then(prefix_to_netmask);
                }
                break;
            }
        }
    }

    info
}

async fn read_default_gateway(iface: &str) -> Option<String> {
    let output = Command::new("ip")
        .args(["-4", "route", "show", "default", "dev", iface])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tokens = stdout.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == "via" {
            return tokens.next().map(str::to_string);
        }
    }
    None
}

async fn read_dns_nameservers() -> Vec<String> {
    let Ok(raw) = fs::read_to_string("/etc/resolv.conf").await else {
        return Vec::new();
    };

    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                return None;
            }
            let mut parts = line.split_whitespace();
            match (parts.next(), parts.next()) {
                (Some("nameserver"), Some(server)) => Some(server.to_string()),
                _ => None,
            }
        })
        .collect()
}

fn prefix_to_netmask(prefix: u8) -> Option<String> {
    if prefix > 32 {
        return None;
    }
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    Some(Ipv4Addr::from(mask).to_string())
}

async fn read_trimmed(path: PathBuf) -> Result<String> {
    Ok(fs::read_to_string(path).await?.trim().to_string())
}

async fn read_u64(path: PathBuf) -> u64 {
    read_trimmed(path)
        .await
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn parse_iface_up_flag(flags: &str) -> bool {
    let value = flags.strip_prefix("0x").unwrap_or(flags);
    u32::from_str_radix(value, 16)
        .map(|flags| flags & 0x1 == 0x1)
        .unwrap_or(false)
}

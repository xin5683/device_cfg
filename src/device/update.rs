use std::{
    env,
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use reqwest::header::{ACCEPT, HeaderMap, USER_AGENT};
use self_update::update::{Release, ReleaseAsset};
use semver::Version;
use serde::Serialize;

const DEFAULT_REPO_OWNER: &str = "xin5683";
const DEFAULT_REPO_NAME: &str = "device_cfg";
const DEFAULT_BIN_NAME: &str = "device_cfg";
const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
const DEFAULT_MIRRORS: &[&str] = &[
    "https://gh-proxy.org",
    "https://gh.monlor.com",
    "https://ghproxy.monkeyray.net",
    "https://ghfast.top",
    "https://cors.isteed.cc",
    "https://cdn.crashmc.com",
    "https://gh.jasonzeng.dev",
];
const UPDATE_USER_AGENT: &str = concat!("device_cfg/", env!("CARGO_PKG_VERSION"));
static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

#[derive(Clone)]
pub struct UpdateConfig {
    repo_owner: String,
    repo_name: String,
    bin_name: String,
    current_version: String,
    target: String,
    github_api_url: String,
    mirrors: Vec<String>,
}

impl UpdateConfig {
    pub fn from_env() -> Self {
        Self {
            repo_owner: env::var("UPDATE_REPO_OWNER")
                .unwrap_or_else(|_| DEFAULT_REPO_OWNER.to_string()),
            repo_name: env::var("UPDATE_REPO_NAME")
                .unwrap_or_else(|_| DEFAULT_REPO_NAME.to_string()),
            bin_name: env::var("UPDATE_BIN_NAME").unwrap_or_else(|_| DEFAULT_BIN_NAME.to_string()),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            target: env::var("UPDATE_TARGET")
                .unwrap_or_else(|_| self_update::get_target().to_string()),
            github_api_url: env::var("UPDATE_GITHUB_API_URL")
                .unwrap_or_else(|_| DEFAULT_GITHUB_API_URL.to_string()),
            mirrors: env::var("UPDATE_MIRRORS")
                .ok()
                .map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .filter(|mirrors: &Vec<String>| !mirrors.is_empty())
                .unwrap_or_else(|| {
                    DEFAULT_MIRRORS
                        .iter()
                        .map(|value| value.to_string())
                        .collect()
                }),
        }
    }

    pub fn print_config(&self) {
        println!(
            "更新配置: repo={}/{}, target={}, mirrors={}",
            self.repo_owner,
            self.repo_name,
            self.target,
            self.mirrors.join(",")
        );
    }
}

#[derive(Debug)]
pub struct UpdateError(pub String);

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<E> From<E> for UpdateError
where
    E: std::error::Error,
{
    fn from(value: E) -> Self {
        Self(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, UpdateError>;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub target: String,
    pub asset_name: Option<String>,
    pub download_url: Option<String>,
    pub mirror_urls: Vec<String>,
    pub release_url: String,
    pub release_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyUpdateResult {
    pub previous_version: String,
    pub updated_version: String,
    pub target: String,
    pub asset_name: String,
    pub downloaded_from: String,
    pub installed_path: String,
}

pub struct UpdateService {
    config: UpdateConfig,
}

impl UpdateService {
    pub fn new(config: UpdateConfig) -> Self {
        Self { config }
    }

    pub fn print_config(&self) {
        self.config.print_config();
    }

    pub async fn check(&self) -> Result<UpdateInfo> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || {
            let release = fetch_latest_release(&config)?;
            Ok(build_update_info(&config, &release))
        })
        .await
        .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn apply(&self) -> Result<ApplyUpdateResult> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || apply_update(config))
            .await
            .map_err(|err| UpdateError(err.to_string()))?
    }
}

fn apply_update(config: UpdateConfig) -> Result<ApplyUpdateResult> {
    let release = fetch_latest_release(&config)?;
    if !is_newer_version(&config.current_version, &release.version) {
        return Err(UpdateError(format!(
            "当前已是最新版本: v{}",
            config.current_version
        )));
    }

    let asset = release_asset_for(&config, &release)?;
    let mirror_urls = mirror_urls(&config, &asset.download_url);
    let tmp_dir = tempfile::Builder::new()
        .prefix("device_cfg_update")
        .tempdir()?;
    let archive_path = tmp_dir.path().join(&asset.name);
    let downloaded_from = download_with_mirrors(&mirror_urls, &archive_path)?;

    let bin_name_in_archive = archive_bin_name(&config);
    self_update::Extract::from_source(&archive_path)
        .extract_file(tmp_dir.path(), Path::new(&bin_name_in_archive))?;

    let new_exe = tmp_dir.path().join(&bin_name_in_archive);
    let install_path = env::current_exe()?;
    self_update::self_replace::self_replace(&new_exe)?;

    Ok(ApplyUpdateResult {
        previous_version: config.current_version,
        updated_version: release.version,
        target: config.target,
        asset_name: asset.name,
        downloaded_from,
        installed_path: install_path.display().to_string(),
    })
}

fn fetch_latest_release(config: &UpdateConfig) -> Result<Release> {
    let api_url = format!(
        "{}/repos/{}/{}/releases/latest",
        config.github_api_url.trim_end_matches('/'),
        config.repo_owner,
        config.repo_name
    );

    let mut last_error = None;
    for url in mirror_urls(config, &api_url) {
        match download_json(&url) {
            Ok(release) => return release_from_json(release),
            Err(err) => last_error = Some(format!("{}: {}", url, err)),
        }
    }

    Err(UpdateError(format!(
        "无法获取最新 release: {}",
        last_error.unwrap_or_else(|| "没有可用下载地址".to_string())
    )))
}

fn download_json(url: &str) -> Result<serde_json::Value> {
    let tmp_dir = tempfile::Builder::new()
        .prefix("device_cfg_release")
        .tempdir()?;
    let json_path = tmp_dir.path().join("release.json");
    let file = File::create(&json_path)?;
    download_url_to(url, file)?;
    let json = std::fs::read_to_string(json_path)?;
    serde_json::from_str(&json).map_err(UpdateError::from)
}

fn download_with_mirrors(urls: &[String], dest: &Path) -> Result<String> {
    let mut last_error = None;
    for url in urls {
        let file = File::create(dest)?;
        match download_url_to(url, file) {
            Ok(()) => return Ok(url.clone()),
            Err(err) => {
                let _ = std::fs::remove_file(dest);
                last_error = Some(format!("{}: {}", url, err));
            }
        }
    }

    Err(UpdateError(format!(
        "下载更新包失败: {}",
        last_error.unwrap_or_else(|| "没有可用下载地址".to_string())
    )))
}

fn http_client() -> Result<&'static reqwest::blocking::Client> {
    if let Some(client) = HTTP_CLIENT.get() {
        return Ok(client);
    }

    let root_certs = webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .map(|cert| reqwest::Certificate::from_der(cert.as_ref()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let client = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .tls_certs_only(root_certs)
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .build()?;
    let _ = HTTP_CLIENT.set(client);
    Ok(HTTP_CLIENT.get().expect("HTTP client was initialized"))
}

fn download_url_to<T: io::Write>(url: &str, mut dest: T) -> Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, UPDATE_USER_AGENT.parse()?);
    headers.insert(
        ACCEPT,
        "application/vnd.github+json, application/octet-stream, */*".parse()?,
    );

    let mut response = http_client()?
        .get(url)
        .headers(headers)
        .send()
        .map_err(UpdateError::from)?;
    if !response.status().is_success() {
        return Err(UpdateError(format!(
            "HTTP 请求失败: status={} url={}",
            response.status(),
            url
        )));
    }

    io::copy(&mut response, &mut dest).map_err(UpdateError::from)?;
    Ok(())
}

fn release_from_json(json: serde_json::Value) -> Result<Release> {
    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| UpdateError("release 缺少 tag_name".to_string()))?;
    let date = json["created_at"]
        .as_str()
        .ok_or_else(|| UpdateError("release 缺少 created_at".to_string()))?;
    let name = json["name"].as_str().unwrap_or(tag);
    let body = json["body"].as_str().map(ToOwned::to_owned);
    let assets = json["assets"]
        .as_array()
        .ok_or_else(|| UpdateError("release 缺少 assets".to_string()))?
        .iter()
        .map(release_asset_from_json)
        .collect::<Result<Vec<_>>>()?;

    Ok(Release {
        name: name.to_string(),
        version: tag.trim_start_matches('v').to_string(),
        date: date.to_string(),
        body,
        assets,
    })
}

fn release_asset_from_json(asset: &serde_json::Value) -> Result<ReleaseAsset> {
    let name = asset["name"]
        .as_str()
        .ok_or_else(|| UpdateError("release asset 缺少 name".to_string()))?;
    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| UpdateError("release asset 缺少 browser_download_url".to_string()))?;

    Ok(ReleaseAsset {
        name: name.to_string(),
        download_url: download_url.to_string(),
    })
}

fn build_update_info(config: &UpdateConfig, release: &Release) -> UpdateInfo {
    let asset = find_target_asset(config, release);
    let download_url = asset.as_ref().map(|asset| asset.download_url.clone());
    let mirror_urls = download_url
        .as_deref()
        .map(|url| mirror_urls(config, url))
        .unwrap_or_default();

    UpdateInfo {
        current_version: config.current_version.clone(),
        latest_version: release.version.clone(),
        update_available: is_newer_version(&config.current_version, &release.version),
        target: config.target.clone(),
        asset_name: asset.map(|asset| asset.name),
        download_url,
        mirror_urls,
        release_url: format!(
            "https://github.com/{}/{}/releases/tag/v{}",
            config.repo_owner, config.repo_name, release.version
        ),
        release_notes: release.body.clone(),
    }
}

fn release_asset_for(config: &UpdateConfig, release: &Release) -> Result<ReleaseAsset> {
    find_target_asset(config, release).ok_or_else(|| {
        let available_assets = release
            .assets
            .iter()
            .map(|asset| asset.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        UpdateError(format!(
            "未找到匹配 target={} 的 release asset，可用 assets: {}",
            config.target, available_assets
        ))
    })
}

fn find_target_asset(config: &UpdateConfig, release: &Release) -> Option<ReleaseAsset> {
    release
        .assets
        .iter()
        .find(|asset| asset.name.contains(&config.target) && asset.name.contains(&config.bin_name))
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.contains(&config.target))
        })
        .cloned()
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    match (Version::parse(current), Version::parse(latest)) {
        (Ok(current), Ok(latest)) => latest > current,
        _ => latest != current,
    }
}

fn mirror_urls(config: &UpdateConfig, url: &str) -> Vec<String> {
    let mut urls: Vec<String> = config
        .mirrors
        .iter()
        .map(|mirror| format!("{}/{}", mirror.trim_end_matches('/'), url))
        .collect();
    urls.push(url.to_string());
    urls
}

fn archive_bin_name(config: &UpdateConfig) -> String {
    let mut path = PathBuf::from(format!("{}-{}", config.bin_name, config.target));
    if let Some(ext) = env::consts::EXE_EXTENSION.strip_prefix('.') {
        path.set_extension(ext);
    }
    path.to_string_lossy().into_owned()
}

use std::{
    collections::VecDeque,
    env,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use crate::device::update::{
    Result, UpdateError, build_mirror_urls, default_mirrors, download_json, download_with_mirrors,
    is_newer_version, sha256_file,
};

const DEFAULT_REPO_OWNER: &str = "xin5683";
const DEFAULT_REPO_NAME: &str = "udp_gw_builder";
const DEFAULT_ASSET_PREFIX: &str = "XPlaneUDP";
const DEFAULT_BIN_NAME: &str = "XPlaneUDP";
const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
const DEFAULT_INSTALL_DIR: &str = "/opt/device_cfg/udp_gw";
const DEFAULT_LOG_FILE: &str = "udp_gw_builder.log";
const DEFAULT_STATE_FILE: &str = "udp_gw_builder.state.json";
const DEFAULT_LOG_MAX_LINES: usize = 2000;
const RESTART_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const LOG_TRIM_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct DaemonConfig {
    repo_owner: String,
    repo_name: String,
    asset_prefix: String,
    bin_name: String,
    target: String,
    github_api_url: String,
    mirrors: Vec<String>,
    install_dir: PathBuf,
    log_path: PathBuf,
    state_path: PathBuf,
    log_max_lines: usize,
    args: Vec<String>,
    default_auto_start: bool,
}

impl DaemonConfig {
    pub fn from_env() -> Self {
        let install_dir = env::var("DAEMON_INSTALL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_INSTALL_DIR));
        let log_path = env::var("DAEMON_LOG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| install_dir.join(DEFAULT_LOG_FILE));
        let state_path = env::var("DAEMON_STATE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| install_dir.join(DEFAULT_STATE_FILE));

        Self {
            repo_owner: env::var("DAEMON_REPO_OWNER")
                .unwrap_or_else(|_| DEFAULT_REPO_OWNER.to_string()),
            repo_name: env::var("DAEMON_REPO_NAME")
                .unwrap_or_else(|_| DEFAULT_REPO_NAME.to_string()),
            asset_prefix: env::var("DAEMON_ASSET_PREFIX")
                .unwrap_or_else(|_| DEFAULT_ASSET_PREFIX.to_string()),
            bin_name: env::var("DAEMON_BIN_NAME").unwrap_or_else(|_| DEFAULT_BIN_NAME.to_string()),
            target: env::var("DAEMON_TARGET")
                .unwrap_or_else(|_| self_update::get_target().to_string()),
            github_api_url: env::var("DAEMON_GITHUB_API_URL")
                .unwrap_or_else(|_| DEFAULT_GITHUB_API_URL.to_string()),
            mirrors: env::var("DAEMON_MIRRORS")
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
                .unwrap_or_else(default_mirrors),
            install_dir,
            log_path,
            state_path,
            log_max_lines: env::var("DAEMON_LOG_MAX_LINES")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value| *value > 0)
                .unwrap_or(DEFAULT_LOG_MAX_LINES),
            args: env::var("DAEMON_ARGS")
                .ok()
                .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
                .unwrap_or_default(),
            default_auto_start: env::var("DAEMON_AUTO_START")
                .ok()
                .and_then(|value| parse_bool(&value))
                .unwrap_or(false),
        }
    }

    pub fn print_config(&self) {
        println!(
            "守护进程配置: repo={}/{}, asset_prefix={}, bin={}, target={}, install_dir={}, log={}, state={}, auto_start_default={}, log_max_lines={}, mirrors={}",
            self.repo_owner,
            self.repo_name,
            self.asset_prefix,
            self.bin_name,
            self.target,
            self.install_dir.display(),
            self.log_path.display(),
            self.state_path.display(),
            self.default_auto_start,
            self.log_max_lines,
            self.mirrors.join(",")
        );
    }

    fn executable_path(&self) -> PathBuf {
        self.install_dir.join(&self.bin_name)
    }
}

#[derive(Clone)]
pub struct DaemonService {
    config: DaemonConfig,
    child: Arc<Mutex<Option<Child>>>,
}

impl DaemonService {
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            config,
            child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn print_config(&self) {
        self.config.print_config();
    }

    pub async fn status(&self) -> Result<DaemonStatus> {
        let config = self.config.clone();
        let child = Arc::clone(&self.child);
        tokio::task::spawn_blocking(move || daemon_status(&config, &child))
            .await
            .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn check(&self) -> Result<DaemonUpdateInfo> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || {
            let release = fetch_latest_release(&config)?;
            Ok(build_update_info(&config, &release))
        })
        .await
        .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn pull(&self) -> Result<DaemonInstallResult> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || install_latest(config, InstallMode::Pull))
            .await
            .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn upgrade(&self) -> Result<DaemonInstallResult> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || install_latest(config, InstallMode::Upgrade))
            .await
            .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn start(&self) -> Result<DaemonStartResult> {
        let config = self.config.clone();
        let child = Arc::clone(&self.child);
        tokio::task::spawn_blocking(move || start_daemon(&config, &child))
            .await
            .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn start_if_auto_start(&self) -> Result<Option<DaemonStartResult>> {
        let config = self.config.clone();
        let child = Arc::clone(&self.child);
        tokio::task::spawn_blocking(move || {
            if !read_daemon_state(&config)?.auto_start {
                return Ok(None);
            }

            start_daemon(&config, &child).map(Some)
        })
        .await
        .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn restart(&self) -> Result<DaemonStartResult> {
        let config = self.config.clone();
        let child = Arc::clone(&self.child);
        tokio::task::spawn_blocking(move || restart_daemon(&config, &child))
            .await
            .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn set_auto_start(&self, auto_start: bool) -> Result<DaemonStatus> {
        let config = self.config.clone();
        let child = Arc::clone(&self.child);
        tokio::task::spawn_blocking(move || {
            let mut state = read_daemon_state(&config)?;
            state.auto_start = auto_start;
            write_daemon_state(&config, &state)?;
            daemon_status(&config, &child)
        })
        .await
        .map_err(|err| UpdateError(err.to_string()))?
    }

    pub async fn logs(&self, lines: usize) -> Result<DaemonLogInfo> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || read_logs(&config, lines))
            .await
            .map_err(|err| UpdateError(err.to_string()))?
    }

    pub fn log_path(&self) -> PathBuf {
        self.config.log_path.clone()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub latest_known_version: Option<String>,
    pub target: String,
    pub executable_path: String,
    pub install_dir: String,
    pub log_path: String,
    pub state_path: String,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DaemonUpdateInfo {
    pub installed_version: Option<String>,
    pub latest_version: String,
    pub update_available: bool,
    pub target: String,
    pub asset_name: Option<String>,
    pub download_url: Option<String>,
    pub mirror_urls: Vec<String>,
    pub sha256: Option<String>,
    pub release_url: String,
    pub release_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DaemonInstallResult {
    pub previous_version: Option<String>,
    pub installed_version: String,
    pub target: String,
    pub asset_name: String,
    pub sha256: String,
    pub downloaded_from: String,
    pub installed_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DaemonStartResult {
    pub running: bool,
    pub pid: u32,
    pub executable_path: String,
    pub log_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DaemonLogInfo {
    pub path: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
struct DaemonRelease {
    version: String,
    body: Option<String>,
    assets: Vec<DaemonAsset>,
}

#[derive(Debug, Clone)]
struct DaemonAsset {
    name: String,
    download_url: String,
    sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstalledVersion {
    version: String,
    target: String,
    asset_name: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DaemonState {
    auto_start: bool,
    installed: Option<InstalledVersion>,
    runtime: DaemonRuntime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DaemonRuntime {
    pid: Option<u32>,
}

enum InstallMode {
    Pull,
    Upgrade,
}

fn daemon_status(config: &DaemonConfig, child: &Arc<Mutex<Option<Child>>>) -> Result<DaemonStatus> {
    let pid = running_pid(config, child)?;
    let state = read_daemon_state(config)?;
    let version = state.installed;
    Ok(DaemonStatus {
        running: pid.is_some(),
        pid,
        installed: config.executable_path().is_file(),
        installed_version: version.as_ref().map(|value| value.version.clone()),
        latest_known_version: version.map(|value| value.version),
        target: config.target.clone(),
        executable_path: config.executable_path().display().to_string(),
        install_dir: config.install_dir.display().to_string(),
        log_path: config.log_path.display().to_string(),
        state_path: config.state_path.display().to_string(),
        auto_start: state.auto_start,
    })
}

fn fetch_latest_release(config: &DaemonConfig) -> Result<DaemonRelease> {
    let api_url = format!(
        "{}/repos/{}/{}/releases/latest",
        config.github_api_url.trim_end_matches('/'),
        config.repo_owner,
        config.repo_name
    );

    let mut last_error = None;
    for url in build_mirror_urls(&config.mirrors, &api_url) {
        match download_json(&url).and_then(release_from_json) {
            Ok(release) => return Ok(release),
            Err(err) => last_error = Some(format!("{}: {}", url, err)),
        }
    }

    Err(UpdateError(format!(
        "无法获取守护进程最新 release: {}",
        last_error.unwrap_or_else(|| "没有可用下载地址".to_string())
    )))
}

fn release_from_json(json: serde_json::Value) -> Result<DaemonRelease> {
    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| UpdateError("release 缺少 tag_name".to_string()))?;
    let assets = json["assets"]
        .as_array()
        .ok_or_else(|| UpdateError("release 缺少 assets".to_string()))?
        .iter()
        .map(asset_from_json)
        .collect::<Result<Vec<_>>>()?;

    Ok(DaemonRelease {
        version: tag.trim_start_matches('v').to_string(),
        body: json["body"].as_str().map(ToOwned::to_owned),
        assets,
    })
}

fn asset_from_json(asset: &serde_json::Value) -> Result<DaemonAsset> {
    let name = asset["name"]
        .as_str()
        .ok_or_else(|| UpdateError("release asset 缺少 name".to_string()))?;
    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| UpdateError("release asset 缺少 browser_download_url".to_string()))?;
    let sha256 = asset["digest"]
        .as_str()
        .and_then(|value| value.strip_prefix("sha256:"))
        .map(ToOwned::to_owned);

    Ok(DaemonAsset {
        name: name.to_string(),
        download_url: download_url.to_string(),
        sha256,
    })
}

fn build_update_info(config: &DaemonConfig, release: &DaemonRelease) -> DaemonUpdateInfo {
    let asset = find_target_asset(config, release);
    let installed_version = read_installed_version(config).ok().flatten();
    let installed = installed_version
        .as_ref()
        .map(|value| value.version.as_str());
    let update_available = installed
        .map(|version| is_newer_version(version, &release.version))
        .unwrap_or(true);
    let download_url = asset.as_ref().map(|asset| asset.download_url.clone());
    let mirror_urls = download_url
        .as_deref()
        .map(|url| build_mirror_urls(&config.mirrors, url))
        .unwrap_or_default();

    DaemonUpdateInfo {
        installed_version: installed_version.map(|value| value.version),
        latest_version: release.version.clone(),
        update_available,
        target: config.target.clone(),
        asset_name: asset.as_ref().map(|asset| asset.name.clone()),
        download_url,
        mirror_urls,
        sha256: asset.and_then(|asset| asset.sha256),
        release_url: format!(
            "https://github.com/{}/{}/releases/tag/v{}",
            config.repo_owner, config.repo_name, release.version
        ),
        release_notes: release.body.clone(),
    }
}

fn install_latest(config: DaemonConfig, mode: InstallMode) -> Result<DaemonInstallResult> {
    let release = fetch_latest_release(&config)?;
    let mut state = read_daemon_state(&config)?;
    let previous_version = state.installed.as_ref().map(|value| value.version.clone());
    if matches!(mode, InstallMode::Upgrade)
        && previous_version
            .as_deref()
            .is_some_and(|version| !is_newer_version(version, &release.version))
    {
        return Err(UpdateError(format!(
            "守护进程已是最新版本: v{}",
            previous_version.unwrap_or_else(|| release.version.clone())
        )));
    }

    let asset = find_target_asset(&config, &release).ok_or_else(|| {
        let available_assets = release
            .assets
            .iter()
            .map(|asset| asset.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        UpdateError(format!(
            "未找到匹配 target={} 的守护进程 release asset，可用 assets: {}",
            config.target, available_assets
        ))
    })?;
    let expected_sha256 = asset.sha256.clone().ok_or_else(|| {
        UpdateError(format!(
            "守护进程 release asset 缺少 GitHub sha256 digest: {}",
            asset.name
        ))
    })?;

    fs::create_dir_all(&config.install_dir)?;
    let tmp_dir = tempfile::Builder::new()
        .prefix("device_cfg_daemon")
        .tempdir()?;
    let archive_path = tmp_dir.path().join(&asset.name);
    let downloaded_from = download_with_mirrors(
        &build_mirror_urls(&config.mirrors, &asset.download_url),
        &archive_path,
    )?;
    let actual_sha256 = sha256_file(&archive_path)?;
    if !actual_sha256.eq_ignore_ascii_case(&expected_sha256) {
        return Err(UpdateError(format!(
            "守护进程更新包 SHA256 校验失败: expected={} actual={}",
            expected_sha256, actual_sha256
        )));
    }

    let extract_dir = tmp_dir.path().join("extract");
    self_update::Extract::from_source(&archive_path).extract_into(&extract_dir)?;
    let extracted_bin = find_extracted_binary(&extract_dir, &config.bin_name)?;
    install_binary(&extracted_bin, &config.executable_path())?;

    state.installed = Some(InstalledVersion {
        version: release.version.clone(),
        target: config.target.clone(),
        asset_name: asset.name.clone(),
        sha256: actual_sha256.clone(),
    });
    write_daemon_state(&config, &state)?;

    Ok(DaemonInstallResult {
        previous_version,
        installed_version: release.version,
        target: config.target.clone(),
        asset_name: asset.name,
        sha256: actual_sha256,
        downloaded_from,
        installed_path: config.executable_path().display().to_string(),
    })
}

fn find_target_asset(config: &DaemonConfig, release: &DaemonRelease) -> Option<DaemonAsset> {
    daemon_target_candidates(&config.target)
        .iter()
        .find_map(|target| {
            release
                .assets
                .iter()
                .find(|asset| {
                    asset.name.contains(&config.asset_prefix) && asset.name.contains(target)
                })
                .cloned()
        })
}

fn daemon_target_candidates(target: &str) -> Vec<String> {
    let mut targets = vec![target.to_string()];
    match target {
        "aarch64-unknown-linux-gnu" => {
            targets.push("aarch64-unknown-linux-musl".to_string());
        }
        "aarch64-unknown-linux-musl" => {
            targets.push("aarch64-unknown-linux-gnu".to_string());
        }
        "armv7-unknown-linux-gnueabihf" => {
            targets.push("armv7-unknown-linux-musleabihf".to_string());
        }
        "armv7-unknown-linux-musleabihf" => {
            targets.push("armv7-unknown-linux-gnueabihf".to_string());
        }
        _ => {}
    }
    targets
}

fn find_extracted_binary(dir: &Path, bin_name: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    collect_files(dir, &mut candidates)?;

    candidates
        .iter()
        .find(|path| file_name_eq(path, bin_name))
        .or_else(|| {
            candidates
                .iter()
                .find(|path| file_name_starts_with(path, bin_name))
        })
        .or_else(|| candidates.iter().find(|path| is_executable(path)))
        .cloned()
        .ok_or_else(|| UpdateError(format!("解压后未找到守护进程可执行文件: {}", bin_name)))
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn file_name_eq(path: &Path, name: &str) -> bool {
    path.file_name().and_then(|value| value.to_str()) == Some(name)
}

fn file_name_starts_with(path: &Path, name: &str) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with(name))
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn install_binary(source: &Path, dest: &Path) -> Result<()> {
    let tmp_dest = dest.with_extension("new");
    fs::copy(source, &tmp_dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_dest, fs::Permissions::from_mode(0o755))?;
    }
    fs::rename(tmp_dest, dest)?;
    Ok(())
}

fn start_daemon(
    config: &DaemonConfig,
    child_ref: &Arc<Mutex<Option<Child>>>,
) -> Result<DaemonStartResult> {
    if let Some(pid) = running_pid(config, child_ref)? {
        return Ok(DaemonStartResult {
            running: true,
            pid,
            executable_path: config.executable_path().display().to_string(),
            log_path: config.log_path.display().to_string(),
        });
    }

    let executable = config.executable_path();
    if !executable.is_file() {
        return Err(UpdateError(format!(
            "守护进程尚未安装: {}",
            executable.display()
        )));
    }

    fs::create_dir_all(&config.install_dir)?;
    clear_log_file(&config.log_path)?;
    let stdout = open_log_file(&config.log_path)?;
    let stderr = stdout.try_clone()?;
    let child = Command::new(&executable)
        .args(&config.args)
        .current_dir(&config.install_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;
    let pid = child.id();
    let mut state = read_daemon_state(config)?;
    state.runtime.pid = Some(pid);
    write_daemon_state(config, &state)?;
    *child_ref
        .lock()
        .map_err(|_| UpdateError("守护进程状态锁已损坏".to_string()))? = Some(child);
    spawn_log_trimmer(config.clone(), pid);

    Ok(DaemonStartResult {
        running: true,
        pid,
        executable_path: executable.display().to_string(),
        log_path: config.log_path.display().to_string(),
    })
}

fn spawn_log_trimmer(config: DaemonConfig, pid: u32) {
    thread::spawn(move || {
        while process_matches(pid, &config.executable_path()) {
            thread::sleep(LOG_TRIM_INTERVAL);
            let _ = trim_log_file(&config.log_path, config.log_max_lines);
        }
    });
}

fn restart_daemon(
    config: &DaemonConfig,
    child_ref: &Arc<Mutex<Option<Child>>>,
) -> Result<DaemonStartResult> {
    stop_daemon(config, child_ref)?;
    start_daemon(config, child_ref)
}

fn stop_daemon(config: &DaemonConfig, child_ref: &Arc<Mutex<Option<Child>>>) -> Result<()> {
    let mut owned_child = child_ref
        .lock()
        .map_err(|_| UpdateError("守护进程状态锁已损坏".to_string()))?
        .take();

    if let Some(child) = owned_child.as_mut() {
        if child.try_wait()?.is_none() {
            terminate_child(child)?;
        }
        clear_runtime_pid(config)?;
        return Ok(());
    }

    if let Some(pid) = read_daemon_state(config)?.runtime.pid {
        if process_matches(pid, &config.executable_path()) {
            terminate_pid(pid)?;
            wait_for_process_exit(pid, &config.executable_path())?;
        }
    }
    clear_runtime_pid(config)?;
    Ok(())
}

fn terminate_child(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    {
        terminate_pid(child.id())?;
        let deadline = Instant::now() + RESTART_WAIT_TIMEOUT;
        while Instant::now() < deadline {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    if child.try_wait()?.is_none() {
        child.kill()?;
    }
    let _ = child.wait();
    Ok(())
}

fn terminate_pid(pid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()?;
        if !status.success() {
            return Err(UpdateError(format!("发送终止信号失败: pid={pid}")));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
    Ok(())
}

fn wait_for_process_exit(pid: u32, executable: &Path) -> Result<()> {
    let deadline = Instant::now() + RESTART_WAIT_TIMEOUT;
    while Instant::now() < deadline {
        if !process_matches(pid, executable) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .arg("-KILL")
            .arg(pid.to_string())
            .status();
    }
    Ok(())
}

fn open_log_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(UpdateError::from)
}

fn clear_log_file(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    Ok(())
}

fn running_pid(
    config: &DaemonConfig,
    child_ref: &Arc<Mutex<Option<Child>>>,
) -> Result<Option<u32>> {
    let mut child_guard = child_ref
        .lock()
        .map_err(|_| UpdateError("守护进程状态锁已损坏".to_string()))?;
    if let Some(child) = child_guard.as_mut() {
        if child.try_wait()?.is_none() {
            return Ok(Some(child.id()));
        }
        *child_guard = None;
        clear_runtime_pid(config)?;
        return Ok(None);
    }
    drop(child_guard);

    let pid = read_daemon_state(config)?.runtime.pid;
    if pid.is_some_and(|value| process_matches(value, &config.executable_path())) {
        return Ok(pid);
    }
    if pid.is_some() {
        clear_runtime_pid(config)?;
    }
    Ok(None)
}

fn process_matches(pid: u32, executable: &Path) -> bool {
    #[cfg(unix)]
    {
        let proc_exe = Path::new("/proc").join(pid.to_string()).join("exe");
        fs::read_link(proc_exe)
            .map(|path| path == executable)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = executable;
        pid > 0
    }
}

fn read_installed_version(config: &DaemonConfig) -> Result<Option<InstalledVersion>> {
    Ok(read_daemon_state(config)?.installed)
}

fn read_daemon_state(config: &DaemonConfig) -> Result<DaemonState> {
    if !config.state_path.is_file() {
        return Ok(DaemonState {
            auto_start: config.default_auto_start,
            installed: None,
            runtime: DaemonRuntime::default(),
        });
    }

    let text = fs::read_to_string(&config.state_path)?;
    serde_json::from_str(&text).map_err(UpdateError::from)
}

fn write_daemon_state(config: &DaemonConfig, state: &DaemonState) -> Result<()> {
    if let Some(parent) = config.state_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&config.state_path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn clear_runtime_pid(config: &DaemonConfig) -> Result<()> {
    let mut state = read_daemon_state(config)?;
    state.runtime.pid = None;
    write_daemon_state(config, &state)?;
    Ok(())
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

fn read_logs(config: &DaemonConfig, lines: usize) -> Result<DaemonLogInfo> {
    let max_lines = lines.clamp(1, config.log_max_lines.max(1));
    if !config.log_path.is_file() {
        return Ok(DaemonLogInfo {
            path: config.log_path.display().to_string(),
            lines: Vec::new(),
        });
    }

    trim_log_file(&config.log_path, config.log_max_lines)?;
    let file = File::open(&config.log_path)?;
    let reader = BufReader::new(file);
    let mut tail = VecDeque::with_capacity(max_lines);
    for line in reader.lines() {
        if tail.len() == max_lines {
            tail.pop_front();
        }
        tail.push_back(line?);
    }

    Ok(DaemonLogInfo {
        path: config.log_path.display().to_string(),
        lines: tail.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_with_assets(names: &[&str]) -> DaemonRelease {
        DaemonRelease {
            version: "0.2.4".to_string(),
            body: None,
            assets: names
                .iter()
                .map(|name| DaemonAsset {
                    name: (*name).to_string(),
                    download_url: format!("https://example.com/{name}"),
                    sha256: Some("abc123".to_string()),
                })
                .collect(),
        }
    }

    fn config_for_target(target: &str) -> DaemonConfig {
        DaemonConfig {
            repo_owner: DEFAULT_REPO_OWNER.to_string(),
            repo_name: DEFAULT_REPO_NAME.to_string(),
            asset_prefix: DEFAULT_ASSET_PREFIX.to_string(),
            bin_name: DEFAULT_BIN_NAME.to_string(),
            target: target.to_string(),
            github_api_url: DEFAULT_GITHUB_API_URL.to_string(),
            mirrors: Vec::new(),
            install_dir: PathBuf::from("/tmp/device_cfg_test"),
            log_path: PathBuf::from("/tmp/device_cfg_test/test.log"),
            state_path: PathBuf::from("/tmp/device_cfg_test/test.state.json"),
            log_max_lines: DEFAULT_LOG_MAX_LINES,
            args: Vec::new(),
            default_auto_start: false,
        }
    }

    #[test]
    fn matches_gnu_armv7_asset_for_musl_runtime_target() {
        let config = config_for_target("armv7-unknown-linux-musleabihf");
        let release = release_with_assets(&[
            "XPlaneUDP-v0.2.4-aarch64-unknown-linux-gnu.tar.gz",
            "XPlaneUDP-v0.2.4-armv7-unknown-linux-gnueabihf.tar.gz",
        ]);

        let asset = find_target_asset(&config, &release).expect("matching asset");

        assert_eq!(
            asset.name,
            "XPlaneUDP-v0.2.4-armv7-unknown-linux-gnueabihf.tar.gz"
        );
    }

    #[test]
    fn prefers_exact_target_match_before_compatible_target() {
        let config = config_for_target("armv7-unknown-linux-musleabihf");
        let release = release_with_assets(&[
            "XPlaneUDP-v0.2.4-armv7-unknown-linux-gnueabihf.tar.gz",
            "XPlaneUDP-v0.2.4-armv7-unknown-linux-musleabihf.tar.gz",
        ]);

        let asset = find_target_asset(&config, &release).expect("matching asset");

        assert_eq!(
            asset.name,
            "XPlaneUDP-v0.2.4-armv7-unknown-linux-musleabihf.tar.gz"
        );
    }

    #[test]
    fn matches_gnu_aarch64_asset_for_musl_runtime_target() {
        let config = config_for_target("aarch64-unknown-linux-musl");
        let release = release_with_assets(&["XPlaneUDP-v0.2.4-aarch64-unknown-linux-gnu.tar.gz"]);

        let asset = find_target_asset(&config, &release).expect("matching asset");

        assert_eq!(
            asset.name,
            "XPlaneUDP-v0.2.4-aarch64-unknown-linux-gnu.tar.gz"
        );
    }
}

fn trim_log_file(path: &Path, max_lines: usize) -> Result<()> {
    if max_lines == 0 || !path.is_file() {
        return Ok(());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut tail = VecDeque::with_capacity(max_lines);
    let mut exceeded = false;
    for line in reader.lines() {
        if tail.len() == max_lines {
            tail.pop_front();
            exceeded = true;
        }
        tail.push_back(line?);
    }

    if exceeded {
        let mut content = tail.into_iter().collect::<Vec<_>>().join("\n");
        content.push('\n');
        fs::write(path, content)?;
    }

    Ok(())
}

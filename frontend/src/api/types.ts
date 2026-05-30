export type ApiResponse<T> = {
  success: boolean;
  data?: T;
  message?: string;
};

export type NetworkInfo = {
  ssid: string;
  bssid: string;
  signal: number;
  security: string;
  frequency: number;
};

export type WifiStatus = {
  state: string;
  ssid?: string | null;
  bssid?: string | null;
  ip?: string | null;
};

export type SystemInfo = {
  version: string;
};

export type ConnectRequest = {
  ssid: string;
  password?: string;
};

export type UpdateInfo = {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  target: string;
  asset_name?: string | null;
  checksum_asset_name?: string | null;
  download_url?: string | null;
  checksum_download_url?: string | null;
  mirror_urls: string[];
  release_url: string;
  release_notes?: string | null;
};

export type ApplyUpdateResult = {
  previous_version: string;
  updated_version: string;
  target: string;
  asset_name: string;
  sha256: string;
  downloaded_from: string;
  installed_path: string;
};

export type DaemonStatus = {
  running: boolean;
  pid?: number | null;
  installed: boolean;
  installed_version?: string | null;
  latest_known_version?: string | null;
  target: string;
  executable_path: string;
  install_dir: string;
  log_path: string;
  pid_path: string;
  version_path: string;
};

export type DaemonUpdateInfo = {
  installed_version?: string | null;
  latest_version: string;
  update_available: boolean;
  target: string;
  asset_name?: string | null;
  download_url?: string | null;
  mirror_urls: string[];
  sha256?: string | null;
  release_url: string;
  release_notes?: string | null;
};

export type DaemonInstallResult = {
  previous_version?: string | null;
  installed_version: string;
  target: string;
  asset_name: string;
  sha256: string;
  downloaded_from: string;
  installed_path: string;
};

export type DaemonStartResult = {
  running: boolean;
  pid: number;
  executable_path: string;
  log_path: string;
};

export type DaemonLogInfo = {
  path: string;
  lines: string[];
};

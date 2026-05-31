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

export type RebootResult = {
  scheduled: boolean;
  delay_seconds: number;
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
  state_path: string;
  auto_start: boolean;
};

export type DaemonAutoStartRequest = {
  auto_start: boolean;
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

export type CanStatus = {
  iface: string;
  exists: boolean;
  operstate: string;
  carrier?: boolean | null;
  up: boolean;
  bitrate?: number | null;
  load_percent?: number | null;
  rx_packets: number;
  tx_packets: number;
  rx_bytes: number;
  tx_bytes: number;
  rx_errors: number;
  tx_errors: number;
};

export type TimeStatus = {
  board_time: string;
  board_unix_ms: number;
  internet_time?: string | null;
  internet_unix_ms?: number | null;
  internet_connected: boolean;
  ntp_server: string;
  error?: string | null;
};

export type TimeSyncResult = {
  synced: boolean;
  ntp_server: string;
  output: string;
};

export type EthernetMode = "dhcp" | "static";

export type EthernetStatus = {
  iface: string;
  exists: boolean;
  operstate: string;
  carrier?: boolean | null;
  up: boolean;
  mac_address?: string | null;
  ipv4_addresses: string[];
  primary_ipv4?: string | null;
  netmask?: string | null;
  gateway?: string | null;
  dns_nameservers: string[];
  rx_packets: number;
  tx_packets: number;
  rx_bytes: number;
  tx_bytes: number;
  rx_errors: number;
  tx_errors: number;
  config_mode?: string | null;
};

export type EthernetSettings = {
  mode: EthernetMode;
  address?: string | null;
  netmask?: string | null;
  gateway?: string | null;
  dns_nameservers: string[];
};

export type EthernetConfigView = {
  iface: string;
  path: string;
  settings: EthernetSettings;
  raw: string;
};

export type EthernetApplyResult = {
  config: EthernetConfigView;
  status: EthernetStatus;
  restart_output: string;
};

export type DiagnosticStatus = {
  can: CanStatus;
  time: TimeStatus;
  ethernet: EthernetStatus;
};

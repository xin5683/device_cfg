import type {
  ApiResponse,
  ApplyUpdateResult,
  ConnectRequest,
  DaemonAutoStartRequest,
  DaemonInstallResult,
  DaemonLogInfo,
  DaemonStartResult,
  DaemonStatus,
  DaemonUpdateInfo,
  EthernetApplyResult,
  EthernetConfigView,
  EthernetSettings,
  EthernetStatus,
  CanStatus,
  NetworkInfo,
  RebootResult,
  SystemInfo,
  TimeStatus,
  TimeSyncResult,
  UpdateInfo,
  WifiStatus
} from "./types";

async function request<T>(path: string, init?: RequestInit): Promise<ApiResponse<T>> {
  const response = await fetch(path, init);
  return (await response.json()) as ApiResponse<T>;
}

function postJson<TBody>(body?: TBody): RequestInit {
  if (!body) {
    return { method: "POST" };
  }

  return {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body)
  };
}

export const api = {
  systemInfo: () => request<SystemInfo>("/api/system/info"),
  rebootSystem: () => request<RebootResult>("/api/system/reboot", postJson()),
  scanNetworks: () => request<NetworkInfo[]>("/api/scan"),
  wifiStatus: () => request<WifiStatus>("/api/status"),
  connectWifi: (body: ConnectRequest) => request<string>("/api/connect", postJson(body)),
  disconnectWifi: () => request<string>("/api/disconnect", postJson()),
  ethernetStatus: () => request<EthernetStatus>("/api/ethernet/status"),
  ethernetConfig: () => request<EthernetConfigView>("/api/ethernet/config"),
  applyEthernetConfig: (body: EthernetSettings) => request<EthernetApplyResult>("/api/ethernet/config", postJson(body)),
  checkUpdate: () => request<UpdateInfo>("/api/update/check"),
  applyUpdate: () => request<ApplyUpdateResult>("/api/update/apply", postJson()),
  daemonStatus: () => request<DaemonStatus>("/api/daemon/status"),
  checkDaemon: () => request<DaemonUpdateInfo>("/api/daemon/check"),
  pullDaemon: () => request<DaemonInstallResult>("/api/daemon/pull", postJson()),
  upgradeDaemon: () => request<DaemonInstallResult>("/api/daemon/upgrade", postJson()),
  startDaemon: () => request<DaemonStartResult>("/api/daemon/start", postJson()),
  restartDaemon: () => request<DaemonStartResult>("/api/daemon/restart", postJson()),
  setDaemonAutoStart: (body: DaemonAutoStartRequest) => request<DaemonStatus>("/api/daemon/auto-start", postJson(body)),
  daemonLogs: (lines = 120) => request<DaemonLogInfo>(`/api/daemon/logs?lines=${encodeURIComponent(lines)}`),
  canStatus: () => request<CanStatus>("/api/diagnostic/can"),
  timeStatus: () => request<TimeStatus>("/api/diagnostic/time"),
  syncTime: () => request<TimeSyncResult>("/api/diagnostic/time/sync", postJson()),
  diagnosticStatusWsUrl: () => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/diagnostic/status/ws`;
  },
  daemonLogsWsUrl: () => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/daemon/logs/ws`;
  }
};

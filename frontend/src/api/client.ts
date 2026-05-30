import type {
  ApiResponse,
  ApplyUpdateResult,
  ConnectRequest,
  DaemonInstallResult,
  DaemonLogInfo,
  DaemonStartResult,
  DaemonStatus,
  DaemonUpdateInfo,
  NetworkInfo,
  SystemInfo,
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
  scanNetworks: () => request<NetworkInfo[]>("/api/scan"),
  wifiStatus: () => request<WifiStatus>("/api/status"),
  connectWifi: (body: ConnectRequest) => request<string>("/api/connect", postJson(body)),
  disconnectWifi: () => request<string>("/api/disconnect", postJson()),
  checkUpdate: () => request<UpdateInfo>("/api/update/check"),
  applyUpdate: () => request<ApplyUpdateResult>("/api/update/apply", postJson()),
  daemonStatus: () => request<DaemonStatus>("/api/daemon/status"),
  checkDaemon: () => request<DaemonUpdateInfo>("/api/daemon/check"),
  pullDaemon: () => request<DaemonInstallResult>("/api/daemon/pull", postJson()),
  upgradeDaemon: () => request<DaemonInstallResult>("/api/daemon/upgrade", postJson()),
  startDaemon: () => request<DaemonStartResult>("/api/daemon/start", postJson()),
  restartDaemon: () => request<DaemonStartResult>("/api/daemon/restart", postJson()),
  daemonLogs: (lines = 160) => request<DaemonLogInfo>(`/api/daemon/logs?lines=${lines}`)
};

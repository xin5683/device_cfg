export type DetailRow = [label: string, value: string | number | null | undefined];

export function stateLabel(state?: string | null): string {
  if (state === "SCANNING") return "扫描中";
  if (state === "ASSOCIATING") return "关联中";
  if (state === "AUTHENTICATING") return "认证中";
  if (state === "INACTIVE") return "空闲";
  return "未连接";
}

export function statusDescription(state?: string | null, connected = false): string {
  if (connected) return "设备已接入无线网络";
  if (state === "SCANNING") return "正在搜索附近的无线网络";
  if (state === "ASSOCIATING") return "正在建立接入点连接";
  if (state === "AUTHENTICATING") return "正在进行身份认证";
  if (state === "INACTIVE") return "无线接口当前空闲";
  return "设备当前未连接到无线网络";
}

export function compactTarget(target?: string | null): string {
  if (!target) return "-";
  if (target.startsWith("aarch64")) return "aarch64";
  if (target.startsWith("armv7")) return "armv7";
  if (target.startsWith("x86_64")) return "x86_64";
  return target;
}

export function displayValue(value: string | number | null | undefined): string {
  if (value === null || value === undefined || value === "") return "-";
  return String(value);
}

export function joinLines(values?: string[] | null): string {
  return values && values.length > 0 ? values.join("\n") : "-";
}

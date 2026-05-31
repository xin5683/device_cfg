<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount, tick } from "svelte";
  import { AnsiUp } from "ansi_up";
  import { Button, Card } from "flowbite-svelte";
  import { Play, RefreshCw, RotateCw, Upload } from "@lucide/svelte";
  import DetailRows from "../components/DetailRows.svelte";
  import { api } from "../api/client";
  import type { DaemonInstallResult, DaemonStatus, DaemonUpdateInfo } from "../api/types";
  import type { DetailRow } from "../utils/format";
  import { joinLines } from "../utils/format";
  import type { ToastKind } from "../utils/toast";

  const dispatch = createEventDispatcher<{
    toast: { text: string; kind: ToastKind };
    statusChanged: { status: DaemonStatus };
  }>();

  export let active = false;

  const ansiUp = new AnsiUp();

  let title = "正在读取状态";
  let summary = "正在获取守护进程状态。";
  let details: DetailRow[] = [];
  let logsText = "";
  let logsHtml = "暂无日志";
  let busyAction: "check" | "install" | "start" | "restart" | "auto-start" | null = null;
  let status: DaemonStatus | null = null;
  let canInstall = false;
  let installButtonLabel = "升级";
  let logSocket: WebSocket | undefined;
  let reconnectTimer: number | undefined;
  let autoStart = false;
  let logViewport: HTMLPreElement | undefined;
  let followLatestLogs = true;
  let loadingInitialLogs = false;

  $: if (active) {
    startLogStream();
  } else {
    stopLogStream();
  }

  onMount(() => {
    void loadStatus();
  });

  onDestroy(stopLogStream);

  function notify(text: string, kind: ToastKind) {
    dispatch("toast", { text, kind });
  }

  function renderStatus(nextStatus: DaemonStatus) {
    status = nextStatus;
    autoStart = nextStatus.auto_start;
    const installedVersion = nextStatus.installed_version ? `v${nextStatus.installed_version}` : "-";
    const stateText = nextStatus.running ? "运行中" : nextStatus.installed ? "已安装" : "未安装";

    title = `UDP 网关${stateText}`;
    summary = nextStatus.running
      ? "守护进程正在运行。"
      : nextStatus.installed
        ? "守护进程已安装，可以启动。"
        : "守护进程尚未安装，请先升级安装 release 包。";
    details = [
      ["运行状态", stateText],
      ["PID", nextStatus.pid],
      ["安装版本", installedVersion],
      ["目标平台", nextStatus.target],
      ["自启动", nextStatus.auto_start ? "已开启" : "已关闭"],
      ["可执行文件", nextStatus.executable_path],
      ["日志文件", nextStatus.log_path],
      ["状态文件", nextStatus.state_path]
    ];
    dispatch("statusChanged", { status: nextStatus });
  }

  export async function loadStatus() {
    try {
      const result = await api.daemonStatus();
      if (result.success && result.data) {
        renderStatus(result.data);
      } else {
        title = "状态读取失败";
        summary = result.message ?? "无法获取守护进程状态";
      }
    } catch {
      title = "状态读取失败";
      summary = "服务未响应";
    }
  }

  function renderDaemonUpdateInfo(info: DaemonUpdateInfo) {
    const installed = info.installed_version ? `v${info.installed_version}` : "未安装";

    if (info.update_available && info.asset_name && info.sha256) {
      title = info.installed_version ? "发现新版本" : "可升级安装";
      summary = `${installed}，最新 v${info.latest_version}。`;
      installButtonLabel = "升级";
      canInstall = true;
    } else if (info.update_available && !info.asset_name) {
      title = "无匹配架构包";
      summary = `最新版本 v${info.latest_version} 没有匹配 ${info.target} 的 release asset。`;
      installButtonLabel = "升级";
      canInstall = false;
    } else if (info.update_available && !info.sha256) {
      title = "缺少校验摘要";
      summary = `最新版本 v${info.latest_version} 未提供 GitHub sha256 digest。`;
      installButtonLabel = "升级";
      canInstall = false;
    } else {
      title = "已是最新版本";
      summary = `当前 ${installed} 与最新 release 一致。`;
      installButtonLabel = "强制升级";
      canInstall = !!info.asset_name && !!info.sha256;
    }

    details = [
      ["安装版本", installed],
      ["最新版本", `v${info.latest_version}`],
      ["目标平台", info.target],
      ["更新包", info.asset_name],
      ["SHA256", info.sha256],
      ["Release", info.release_url],
      ["镜像顺序", joinLines(info.mirror_urls)]
    ];
  }

  function renderInstallResult(result: DaemonInstallResult) {
    title = "安装完成";
    summary = `已安装 v${result.installed_version}。`;
    canInstall = false;
    details = [
      ["上一版本", result.previous_version ? `v${result.previous_version}` : "-"],
      ["安装版本", `v${result.installed_version}`],
      ["目标平台", result.target],
      ["更新包", result.asset_name],
      ["SHA256", result.sha256],
      ["下载地址", result.downloaded_from],
      ["安装路径", result.installed_path]
    ];
  }

  async function checkDaemon() {
    busyAction = "check";
    canInstall = false;
    title = "正在检查";
    summary = "正在通过镜像站请求 GitHub Release 信息。";

    try {
      const result = await api.checkDaemon();
      if (result.success && result.data) {
        renderDaemonUpdateInfo(result.data);
      } else {
        title = "检查失败";
        summary = result.message ?? "无法获取最新版本信息";
        notify("UDP 网关检查失败", "error");
      }
    } catch {
      title = "检查失败";
      summary = "服务未响应";
      notify("UDP 网关服务未响应", "error");
    } finally {
      busyAction = null;
    }
  }

  async function installDaemon() {
    busyAction = "install";
    title = installButtonLabel === "强制升级" ? "强制升级中" : "升级中";
    summary = "正在下载、校验并安装守护进程。";

    try {
      const result = await api.pullDaemon();
      if (result.success && result.data) {
        renderInstallResult(result.data);
        notify("UDP 网关安装完成", "success");
        await loadStatus();
      } else {
        title = "安装失败";
        summary = result.message ?? "下载或安装失败";
        notify("UDP 网关安装失败", "error");
      }
    } catch {
      title = "安装失败";
      summary = "请求失败";
      notify("UDP 网关请求失败", "error");
    } finally {
      busyAction = null;
    }
  }

  async function startDaemon() {
    busyAction = "start";
    try {
      const result = await api.startDaemon();
      if (result.success) {
        resetLogView();
        notify("UDP 网关已启动", "success");
        await loadStatus();
      } else {
        notify(`UDP 网关启动失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      notify("UDP 网关启动请求失败", "error");
    } finally {
      busyAction = null;
      await loadStatus();
    }
  }

  async function restartDaemon() {
    busyAction = "restart";
    try {
      const result = await api.restartDaemon();
      if (result.success) {
        resetLogView();
        notify("UDP 网关进程已重启", "success");
        await loadStatus();
      } else {
        notify(`UDP 网关进程重启失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      notify("UDP 网关进程重启请求失败", "error");
    } finally {
      busyAction = null;
      await loadStatus();
    }
  }

  async function setAutoStart(enabled: boolean) {
    const previous = autoStart;
    autoStart = enabled;
    busyAction = "auto-start";

    try {
      const result = await api.setDaemonAutoStart({ auto_start: enabled });
      if (result.success && result.data) {
        renderStatus(result.data);
        notify(enabled ? "UDP 网关自启动已开启" : "UDP 网关自启动已关闭", "success");
      } else {
        autoStart = previous;
        notify(`自启动设置失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      autoStart = previous;
      notify("自启动设置请求失败", "error");
    } finally {
      busyAction = null;
    }
  }

  export function loadLogs() {
    followLatestLogs = true;
    void loadInitialLogs();
    scrollLogsToBottom();
    if (!logSocket || logSocket.readyState > WebSocket.OPEN) {
      startLogStream();
    }
  }

  async function loadInitialLogs() {
    if (loadingInitialLogs) return;
    loadingInitialLogs = true;

    try {
      const result = await api.daemonLogs();
      if (result.success && result.data) {
        const text = result.data.lines.length > 0 ? `${result.data.lines.join("\n")}\n` : "暂无日志";
        logsText = result.data.lines.length > 0 ? text : "";
        await renderLogText(text, true);
      } else if (!logsText) {
        await renderLogText(result.message ? `日志读取失败: ${result.message}` : "暂无日志", true);
      }
    } catch {
      if (!logsText) {
        await renderLogText("日志读取失败: 服务未响应", true);
      }
    } finally {
      loadingInitialLogs = false;
    }
  }

  async function appendLogChunk(chunk: string, forceFollow = false) {
    if (!chunk) return;

    const previousScrollTop = logViewport?.scrollTop ?? 0;
    const shouldPinToBottom = forceFollow || followLatestLogs || isLogAtBottom();
    logsText += chunk;
    await renderLogText(logsText || "暂无日志", shouldPinToBottom, previousScrollTop);
  }

  async function renderLogText(text: string, shouldPinToBottom = followLatestLogs, previousScrollTop = logViewport?.scrollTop ?? 0) {
    logsHtml = ansiUp.ansi_to_html(text);
    await tick();

    if (!logViewport) return;
    if (shouldPinToBottom) {
      scrollLogsToBottom();
      return;
    }

    logViewport.scrollTop = previousScrollTop;
    followLatestLogs = isLogAtBottom();
  }

  function resetLogView() {
    followLatestLogs = true;
    logsText = "";
    logsHtml = "暂无日志";
    void tick().then(scrollLogsToBottom);
  }

  function handleLogScroll() {
    followLatestLogs = isLogAtBottom();
  }

  function isLogAtBottom() {
    if (!logViewport) return true;
    return logViewport.scrollHeight - logViewport.scrollTop - logViewport.clientHeight <= 16;
  }

  function scrollLogsToBottom() {
    if (!logViewport) return;
    logViewport.scrollTop = logViewport.scrollHeight;
    followLatestLogs = true;
  }

  function startLogStream() {
    if (logSocket || reconnectTimer) return;
    void loadStatus();
    void loadInitialLogs();
    logSocket = new WebSocket(api.daemonLogsWsUrl());

    logSocket.onmessage = (event) => {
      if (typeof event.data === "string") {
        void appendLogChunk(event.data);
        return;
      }
      if (event.data instanceof Blob) {
        void event.data.text().then((text) => appendLogChunk(text));
      }
    };

    logSocket.onclose = () => {
      logSocket = undefined;
      if (!active || reconnectTimer) return;
      reconnectTimer = window.setTimeout(() => {
        reconnectTimer = undefined;
        startLogStream();
      }, 1000);
    };

    logSocket.onerror = () => {
      logSocket?.close();
    };
  }

  function stopLogStream() {
    if (reconnectTimer) {
      window.clearTimeout(reconnectTimer);
      reconnectTimer = undefined;
    }
    if (!logSocket) return;
    logSocket.close();
    logSocket = undefined;
  }
</script>

<Card class="glass-card max-w-none" size="xl">
  <div class="flex flex-col gap-4 p-5 xl:flex-row xl:items-center xl:justify-between">
    <div>
      <h2 class="text-xl font-semibold tracking-normal text-slate-950 dark:text-white">UDP 网关</h2>
      <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">管理 XPlaneUDP 守护进程</p>
    </div>
    <div class="grid gap-3 sm:flex sm:flex-wrap sm:items-center">
      <button
        type="button"
        class="glass-switch"
        class:glass-switch--checked={autoStart}
        role="switch"
        aria-checked={autoStart}
        aria-label="XPlaneUDP 自启动"
        disabled={!!busyAction}
        onclick={() => setAutoStart(!autoStart)}
      >
        <span class="glass-switch__track" aria-hidden="true">
          <span class="glass-switch__thumb"></span>
        </span>
        <span class="glass-switch__label">自启动</span>
      </button>
      <Button class="glass-button" color="alternative" loading={busyAction === "check"} disabled={!!busyAction} onclick={checkDaemon}>
        <RefreshCw size={16} class="mr-2" />
        检查
      </Button>
      <Button class="glass-button glass-button--primary" color="alternative" loading={busyAction === "install"} disabled={!!busyAction || !canInstall} onclick={installDaemon}>
        <Upload size={16} class="mr-2" />
        {installButtonLabel}
      </Button>
      <Button class="glass-button glass-button--success" color="alternative" loading={busyAction === "start"} disabled={!!busyAction || !status?.installed || !!status?.running} onclick={startDaemon}>
        <Play size={16} class="mr-2" />
        启动
      </Button>
      <Button class="glass-button glass-button--warning" color="alternative" loading={busyAction === "restart"} disabled={!!busyAction || !status?.installed || !status?.running} onclick={restartDaemon}>
        <RotateCw size={16} class="mr-2" />
        重启进程
      </Button>
    </div>
  </div>

  <div class="px-5 pb-5">
    <div class="glass-lens mb-4 p-4">
      <h3 class="relative text-lg font-semibold tracking-normal text-slate-950 dark:text-white">{title}</h3>
      <p class="relative mt-1 text-sm text-slate-700 dark:text-slate-200">{summary}</p>
    </div>

    {#if details.length > 0}
      <DetailRows rows={details} />
    {/if}

    <pre
      bind:this={logViewport}
      class="glass-terminal mt-5 max-h-96 overflow-auto p-4 text-xs leading-5 text-slate-100"
      onscroll={handleLogScroll}
    >{@html logsHtml}</pre>
  </div>
</Card>

<script lang="ts">
  import { createEventDispatcher, onDestroy } from "svelte";
  import { AnsiUp } from "ansi_up";
  import { Button, Card } from "flowbite-svelte";
  import { Download, FileText, Play, RefreshCw, RotateCw, Upload } from "@lucide/svelte";
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
  let logsHtml = "暂无日志";
  let busyAction: "check" | "pull" | "upgrade" | "start" | "restart" | "logs" | null = null;
  let status: DaemonStatus | null = null;
  let canUpgrade = false;
  let logTimer: number | undefined;

  $: if (active) {
    startLogPoll();
  } else {
    stopLogPoll();
  }

  onDestroy(stopLogPoll);

  function notify(text: string, kind: ToastKind) {
    dispatch("toast", { text, kind });
  }

  function renderStatus(nextStatus: DaemonStatus) {
    status = nextStatus;
    const installedVersion = nextStatus.installed_version ? `v${nextStatus.installed_version}` : "-";
    const stateText = nextStatus.running ? "运行中" : nextStatus.installed ? "已安装" : "未安装";

    title = `UDP 网关${stateText}`;
    summary = nextStatus.running
      ? "守护进程正在运行。"
      : nextStatus.installed
        ? "守护进程已安装，可以启动。"
        : "守护进程尚未安装，请先拉取 release 包。";
    details = [
      ["运行状态", stateText],
      ["PID", nextStatus.pid],
      ["安装版本", installedVersion],
      ["目标平台", nextStatus.target],
      ["可执行文件", nextStatus.executable_path],
      ["日志文件", nextStatus.log_path],
      ["版本文件", nextStatus.version_path]
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
      title = info.installed_version ? "发现新版本" : "可拉取程序";
      summary = `${installed}，最新 v${info.latest_version}。`;
      canUpgrade = !!info.installed_version;
    } else if (info.update_available && !info.asset_name) {
      title = "无匹配架构包";
      summary = `最新版本 v${info.latest_version} 没有匹配 ${info.target} 的 release asset。`;
      canUpgrade = false;
    } else if (info.update_available && !info.sha256) {
      title = "缺少校验摘要";
      summary = `最新版本 v${info.latest_version} 未提供 GitHub sha256 digest。`;
      canUpgrade = false;
    } else {
      title = "已是最新版本";
      summary = `当前 ${installed} 与最新 release 一致。`;
      canUpgrade = false;
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
    canUpgrade = false;
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

  async function installDaemon(mode: "pull" | "upgrade") {
    busyAction = mode;
    title = mode === "pull" ? "拉取中" : "升级中";
    summary = "正在下载、校验并安装守护进程。";

    try {
      const result = mode === "pull" ? await api.pullDaemon() : await api.upgradeDaemon();
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
        notify("UDP 网关已启动", "success");
        await loadStatus();
        await loadLogs(false);
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
        notify("UDP 网关已重启", "success");
        await loadStatus();
        await loadLogs(false);
      } else {
        notify(`UDP 网关重启失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      notify("UDP 网关重启请求失败", "error");
    } finally {
      busyAction = null;
      await loadStatus();
    }
  }

  export async function loadLogs(silent = false) {
    if (!silent) busyAction = "logs";

    try {
      const result = await api.daemonLogs(160);
      if (result.success && result.data) {
        renderLogText(result.data.lines.length > 0 ? result.data.lines.join("\n") : "暂无日志");
      } else {
        renderLogText(result.message ?? "日志读取失败");
      }
    } catch {
      renderLogText("日志请求失败");
    } finally {
      if (!silent) busyAction = null;
    }
  }

  function renderLogText(text: string) {
    logsHtml = ansiUp.ansi_to_html(text);
  }

  function startLogPoll() {
    if (logTimer) return;
    void loadStatus();
    void loadLogs(false);
    logTimer = window.setInterval(() => void loadLogs(true), 2000);
  }

  function stopLogPoll() {
    if (!logTimer) return;
    window.clearInterval(logTimer);
    logTimer = undefined;
  }
</script>

<Card class="glass-card max-w-none" size="xl">
  <div class="flex flex-col gap-4 p-5 xl:flex-row xl:items-center xl:justify-between">
    <div>
      <h2 class="text-xl font-semibold tracking-normal text-slate-950 dark:text-white">UDP 网关</h2>
      <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">管理 XPlaneUDP 守护进程</p>
    </div>
    <div class="flex flex-wrap gap-3">
      <Button class="glass-button" color="alternative" loading={busyAction === "check"} disabled={!!busyAction} onclick={checkDaemon}>
        <RefreshCw size={16} class="mr-2" />
        检查
      </Button>
      <Button class="glass-button" color="alternative" loading={busyAction === "pull"} disabled={!!busyAction} onclick={() => installDaemon("pull")}>
        <Download size={16} class="mr-2" />
        拉取
      </Button>
      <Button class="glass-button glass-button--primary" color="alternative" loading={busyAction === "upgrade"} disabled={!!busyAction || !canUpgrade} onclick={() => installDaemon("upgrade")}>
        <Upload size={16} class="mr-2" />
        升级
      </Button>
      <Button class="glass-button glass-button--success" color="alternative" loading={busyAction === "start"} disabled={!!busyAction || !status?.installed || !!status?.running} onclick={startDaemon}>
        <Play size={16} class="mr-2" />
        启动
      </Button>
      <Button class="glass-button" color="alternative" loading={busyAction === "restart"} disabled={!!busyAction || !status?.installed || !status?.running} onclick={restartDaemon}>
        <RotateCw size={16} class="mr-2" />
        重启
      </Button>
      <Button class="glass-button" color="alternative" loading={busyAction === "logs"} disabled={!!busyAction} onclick={() => loadLogs(false)}>
        <FileText size={16} class="mr-2" />
        日志
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

    <pre class="glass-terminal mt-5 max-h-96 overflow-auto p-4 text-xs leading-5 text-slate-100">{@html logsHtml}</pre>
  </div>
</Card>

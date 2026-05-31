<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { scale } from "svelte/transition";
  import { Button, Tabs, TabItem, Toast } from "flowbite-svelte";
  import { Activity, EthernetPort, FileText, Power, Radio, RefreshCw, Server, Wifi, X } from "@lucide/svelte";
  import CompactStatusCard from "./components/CompactStatusCard.svelte";
  import ModalLayer from "./components/ModalLayer.svelte";
  import StatusCard from "./components/StatusCard.svelte";
  import WifiTab from "./tabs/WifiTab.svelte";
  import EthernetTab from "./tabs/EthernetTab.svelte";
  import UpdateTab from "./tabs/UpdateTab.svelte";
  import DaemonTab from "./tabs/DaemonTab.svelte";
  import DiagnosticTab from "./tabs/DiagnosticTab.svelte";
  import { api } from "./api/client";
  import type { ApiResponse, CanStatus, DaemonStatus, DiagnosticStatus, EthernetStatus, TimeStatus, WifiStatus } from "./api/types";
  import type { DetailRow } from "./utils/format";
  import { stateLabel, statusDescription } from "./utils/format";
  import { modalOpen } from "./utils/modal";
  import type { ToastKind, ToastMessage } from "./utils/toast";

  type CompactMetric = [label: string, value: string | number | null | undefined];

  let systemVersion = "加载中";
  let activeTab = "wifi";
  let wifiStatus: WifiStatus | null = null;
  let wifiStatusError: string | null = null;
  let daemonStatus: DaemonStatus | null = null;
  let daemonStatusError: string | null = null;
  let canStatus: CanStatus | null = null;
  let canStatusError: string | null = null;
  let ethernetStatus: EthernetStatus | null = null;
  let ethernetStatusError: string | null = null;
  let timeStatus: TimeStatus | null = null;
  let timeStatusError: string | null = null;
  let rebooting = false;
  let updatePanelOpen = false;
  let rebootConfirmOpen = false;
  let statusPollTimer: number | undefined;
  let diagnosticSocket: WebSocket | undefined;
  let diagnosticReconnectTimer: number | undefined;
  let toastId = 0;
  let toasts: ToastMessage[] = [];

  $: wifiConnected = wifiStatus?.state === "COMPLETED";
  $: wifiStateText = wifiStatusError
    ? "不可用"
    : wifiStatus
      ? wifiConnected
        ? "已连接"
        : stateLabel(wifiStatus.state)
      : "加载中";
  $: wifiMetrics = buildWifiMetrics(wifiStatus, wifiConnected);
  $: daemonStateText = daemonStatusError
    ? "不可用"
    : daemonStatus
      ? daemonStatus.running
        ? "运行中"
        : daemonStatus.installed
          ? "已安装"
          : "未安装"
      : "加载中";
  $: daemonRows = buildDaemonRows(daemonStatus);
  $: canStateText = canStatusError
    ? "不可用"
    : canStatus
      ? canStatus.up
        ? "已启用"
        : canStatus.exists
          ? "未启用"
          : "不存在"
      : "加载中";
  $: canMetrics = buildCanMetrics(canStatus);
  $: ethernetStateText = ethernetStatusError
    ? "不可用"
    : ethernetStatus
      ? ethernetStatus.up
        ? ethernetStatus.carrier === true
          ? "已启用"
          : "未接入"
        : ethernetStatus.exists
          ? "未启用"
          : "不存在"
      : "加载中";
  $: ethernetMetrics = buildEthernetMetrics(ethernetStatus);
  $: timeStateText = timeStatusError
    ? "不可用"
    : timeStatus
      ? timeStatus.internet_connected
        ? "已连接"
        : "无互联网"
      : "加载中";
  $: timeMetrics = buildTimeMetrics(timeStatus);

  function buildWifiMetrics(status: WifiStatus | null, connected: boolean): CompactMetric[] {
    return [
      ["网络", connected && status?.ssid ? status.ssid : "未连接"],
      ["IP", connected && status?.ip ? status.ip : "-"],
      ["BSSID", connected && status?.bssid ? status.bssid : "-"],
      ["阶段", status?.state && status.state !== "DISCONNECTED" ? stateLabel(status.state) : "-"]
    ];
  }

  function buildDaemonRows(status: DaemonStatus | null): DetailRow[] {
    return [
      ["安装版本", status?.installed_version ? `v${status.installed_version}` : "-"],
      ["PID", status?.pid],
      ["自启动", status ? (status.auto_start ? "已开启" : "已关闭") : "-"]
    ];
  }

  function buildCanMetrics(status: CanStatus | null): CompactMetric[] {
    return [
      ["负载", formatCanLoad(status?.load_percent)],
      ["波特率", formatCanBitrate(status?.bitrate)],
      ["RX/TX", status ? `${formatCanCount(status.rx_packets)}/${formatCanCount(status.tx_packets)}` : "-"],
      ["错误", status ? `${formatCanCount(status.rx_errors)}/${formatCanCount(status.tx_errors)}` : "-"]
    ];
  }

  function buildEthernetMetrics(status: EthernetStatus | null): CompactMetric[] {
    const linkActive = status?.exists === true && status.carrier === true;

    return [
      ["Link", status ? (linkActive ? "已连接" : "未连接") : "-"],
      ["IP", linkActive ? status?.primary_ipv4 : null],
      ["方式", formatEthernetMode(status?.config_mode)],
      ["网关", linkActive ? status?.gateway : null]
    ];
  }

  function formatEthernetMode(mode: string | null | undefined) {
    if (mode === "dhcp") return "DHCP";
    if (mode === "static") return "静态地址";
    return "-";
  }

  function formatCanCount(value: number | null | undefined) {
    if (typeof value !== "number" || !Number.isFinite(value)) return "-";
    if (value >= 1_000_000_000) return `${trimMetric(value / 1_000_000_000)}G`;
    if (value >= 1_000_000) return `${trimMetric(value / 1_000_000)}M`;
    if (value >= 1_000) return `${trimMetric(value / 1_000)}K`;
    return `${value}`;
  }

  function trimMetric(value: number) {
    return value.toFixed(2).replace(/\.?0+$/, "");
  }

  function formatCanLoad(loadPercent: number | null | undefined) {
    if (typeof loadPercent !== "number" || !Number.isFinite(loadPercent)) return "--";
    if (loadPercent < 0.1) return "0.0%";
    if (loadPercent < 10) return `${loadPercent.toFixed(1)}%`;
    return `${Math.round(loadPercent)}%`;
  }

  function formatCanBitrate(bitrate: number | null | undefined) {
    if (typeof bitrate !== "number" || !Number.isFinite(bitrate) || bitrate <= 0) return "-";
    if (bitrate >= 1_000_000) return `${bitrate / 1_000_000}M`;
    if (bitrate >= 1_000) return `${Math.round(bitrate / 1_000)}K`;
    return `${bitrate}`;
  }

  function buildTimeMetrics(status: TimeStatus | null): CompactMetric[] {
    return [
      ["板卡时间", status?.board_time],
      ["互联网时间", status?.internet_time],
      ["NTP", status?.ntp_server],
      ["互联网", status ? (status.internet_connected ? "可用" : "不可用") : "-"]
    ];
  }

  function toastColor(kind: ToastKind) {
    if (kind === "success") return "green";
    if (kind === "error") return "red";
    return "blue";
  }

  function showToast(text: string, kind: ToastKind) {
    const id = ++toastId;
    toasts = [...toasts, { id, text, kind }];
    window.setTimeout(() => {
      toasts = toasts.filter((toast) => toast.id !== id);
    }, 3000);
  }

  onMount(() => {
    void loadSystemInfo();
    void loadWifiStatus();
    void loadDaemonStatus();
    startDiagnosticStream();
  });

  onDestroy(() => {
    if (statusPollTimer) {
      window.clearInterval(statusPollTimer);
    }
    stopDiagnosticStream();
  });

  async function loadSystemInfo() {
    try {
      const result = await api.systemInfo();
      systemVersion = result.success && result.data?.version ? `v${result.data.version}` : "-";
    } catch {
      systemVersion = "-";
    }
  }

  async function loadWifiStatus() {
    try {
      const result = await api.wifiStatus();
      if (result.success && result.data) {
        wifiStatus = result.data;
        wifiStatusError = null;
      } else {
        wifiStatusError = `无法获取状态: ${result.message ?? "未知错误"}`;
      }
    } catch {
      wifiStatusError = "服务未响应";
    }
  }

  async function loadDaemonStatus() {
    try {
      const result = await api.daemonStatus();
      if (result.success && result.data) {
        daemonStatus = result.data;
        daemonStatusError = null;
      } else {
        daemonStatusError = `无法获取状态: ${result.message ?? "未知错误"}`;
      }
    } catch {
      daemonStatusError = "服务未响应";
    }
  }

  function startDiagnosticStream() {
    if (diagnosticSocket || diagnosticReconnectTimer) return;

    diagnosticSocket = new WebSocket(api.diagnosticStatusWsUrl());

    diagnosticSocket.onmessage = (event) => {
      if (typeof event.data !== "string") return;

      try {
        const result = JSON.parse(event.data) as ApiResponse<DiagnosticStatus>;
        if (result.success && result.data) {
          canStatus = result.data.can;
          timeStatus = result.data.time;
          ethernetStatus = result.data.ethernet;
          canStatusError = null;
          timeStatusError = null;
          ethernetStatusError = null;
        } else {
          const message = result.message ?? "诊断状态推送异常";
          canStatusError = message;
          timeStatusError = message;
          ethernetStatusError = message;
        }
      } catch {
        canStatusError = "诊断状态解析失败";
        timeStatusError = "诊断状态解析失败";
        ethernetStatusError = "诊断状态解析失败";
      }
    };

    diagnosticSocket.onclose = () => {
      diagnosticSocket = undefined;
      if (diagnosticReconnectTimer) return;
      diagnosticReconnectTimer = window.setTimeout(() => {
        diagnosticReconnectTimer = undefined;
        startDiagnosticStream();
      }, 1500);
    };

    diagnosticSocket.onerror = () => {
      diagnosticSocket?.close();
    };
  }

  function stopDiagnosticStream() {
    if (diagnosticReconnectTimer) {
      window.clearTimeout(diagnosticReconnectTimer);
      diagnosticReconnectTimer = undefined;
    }
    if (!diagnosticSocket) return;
    diagnosticSocket.close();
    diagnosticSocket = undefined;
  }

  function startStatusPoll() {
    if (statusPollTimer) {
      window.clearInterval(statusPollTimer);
    }

    let count = 0;
    statusPollTimer = window.setInterval(() => {
      count += 1;
      void loadWifiStatus();
      if (count >= 10 && statusPollTimer) {
        window.clearInterval(statusPollTimer);
        statusPollTimer = undefined;
      }
    }, 2000);
  }

  async function rebootDevice() {
    rebooting = true;
    try {
      const result = await api.rebootSystem();
      if (result.success) {
        rebootConfirmOpen = false;
        showToast("设备正在重启", "success");
      } else {
        showToast(result.message ?? "重启设备失败", "error");
      }
    } catch {
      showToast("重启请求失败", "error");
    } finally {
      rebooting = false;
    }
  }

  function handleUpdatePanelKeydown(event: KeyboardEvent) {
    if (rebootConfirmOpen && event.key === "Escape" && !rebooting) {
      rebootConfirmOpen = false;
      return;
    }

    if (updatePanelOpen && event.key === "Escape") {
      updatePanelOpen = false;
    }
  }

</script>

<svelte:window onkeydown={handleUpdatePanelKeydown} />

<main class="liquid-app min-h-screen overflow-hidden">
  <div class="liquid-background" class:modal-background-blurred={$modalOpen} aria-hidden="true"></div>

  <div
    class="app-content mx-auto flex w-full max-w-7xl flex-col gap-5 px-4 py-4 sm:gap-6 sm:px-6 sm:py-6 lg:px-8"
    class:modal-background-blurred={$modalOpen}
  >
    <header class="glass-panel sticky top-3 z-30 px-4 py-4 sm:px-5">
      <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex items-center gap-3">
          <span class="liquid-symbol grid h-10 w-10 place-items-center rounded-[16px] text-white">
            <Activity size={22} />
          </span>
          <div>
            <h1 class="text-xl font-semibold tracking-normal text-slate-950 dark:text-white">设备配置中心</h1>
            <p class="text-sm text-slate-600 dark:text-slate-300">嵌入式设备控制台</p>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <span class="version-label w-fit">版本 {systemVersion}</span>
          <Button
            color="alternative"
            size="sm"
            class="glass-button glass-button--primary"
            onclick={() => (updatePanelOpen = true)}
          >
            <RefreshCw size={15} class="mr-2" />
            更新
          </Button>
          <Button
            color="alternative"
            size="sm"
            class="glass-button glass-button--danger"
            loading={rebooting}
            disabled={rebooting}
            onclick={() => (rebootConfirmOpen = true)}
          >
            <Power size={15} class="mr-2" />
            重启设备
          </Button>
        </div>
      </div>
    </header>

    <section class="grid gap-4" aria-label="状态卡片区域">
      <StatusCard
        title="UDP 网关"
        badge="XPlaneUDP"
        summaryTitle="进程状态"
        summary={daemonStatusError ??
          (daemonStatus?.running
            ? `PID ${daemonStatus.pid}`
            : daemonStatus?.installed
              ? "可启动"
              : "请先拉取程序")}
        stateText={daemonStateText}
        stateTone={daemonStatus?.running ? "connected" : daemonStatus ? "disconnected" : "loading"}
        rows={daemonRows}
        layout="horizontal"
      />

      <div class="grid gap-4 lg:grid-cols-2">
        <CompactStatusCard
          title="WiFi 状态"
          badge="wlan1"
          summaryTitle="当前连接状态"
          summary={wifiStatusError ?? statusDescription(wifiStatus?.state, wifiConnected)}
          stateText={wifiStateText}
          stateTone={wifiConnected ? "connected" : wifiStatus ? "disconnected" : "loading"}
          metrics={wifiMetrics}
        />

        <CompactStatusCard
          title="CAN 接口"
          badge={canStatus?.iface ?? "can0"}
          summaryTitle="can0 状态"
          summary={canStatusError ??
            (canStatus
              ? canStatus.exists
                ? `RX ${formatCanCount(canStatus.rx_packets)} / TX ${formatCanCount(canStatus.tx_packets)}`
                : "未发现 can0 接口"
              : "正在读取 CAN 接口状态")}
          stateText={canStateText}
          stateTone={canStatus?.up ? "connected" : canStatus ? "disconnected" : "loading"}
          metrics={canMetrics}
        />
      </div>

      <div class="grid gap-4 lg:grid-cols-2">
        <CompactStatusCard
          title="网口"
          badge={ethernetStatus?.iface ?? "eth0"}
          summaryTitle="eth0 状态"
          summary={ethernetStatusError ??
            (ethernetStatus
              ? ethernetStatus.exists
                ? ethernetStatus.carrier === true
                  ? ethernetStatus.primary_ipv4 ?? "已接入，等待 IP"
                  : "网线未接入"
                : "未发现 eth0 接口"
              : "正在读取网口状态")}
          stateText={ethernetStateText}
          stateTone={ethernetStatus?.up && ethernetStatus?.carrier === true ? "connected" : ethernetStatus ? "disconnected" : "loading"}
          metrics={ethernetMetrics}
        />

        <CompactStatusCard
          title="系统时间"
          badge="ntpd"
          summaryTitle="板卡 / 互联网"
          summary={timeStatusError ??
            (timeStatus
              ? timeStatus.internet_connected
                ? "互联网时间源可用"
                : timeStatus.error ?? "无法连接互联网时间源"
              : "正在读取系统时间")}
          stateText={timeStateText}
          stateTone={timeStatus?.internet_connected ? "connected" : timeStatus ? "disconnected" : "loading"}
          metrics={timeMetrics}
        />
      </div>
    </section>

    <section class="glass-tabs-shell">
      <Tabs bind:selected={activeTab} tabStyle="pill" divider={false} contentClass="hidden">
        <TabItem key="wifi">
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><Wifi size={18} />无线网络</span>
          {/snippet}
        </TabItem>

        <TabItem key="ethernet">
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><EthernetPort size={18} />网口配置</span>
          {/snippet}
        </TabItem>

        <TabItem key="daemon">
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><Server size={18} />UDP 网关</span>
          {/snippet}
        </TabItem>

        <TabItem key="diagnostic">
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><Radio size={18} />诊断工具</span>
          {/snippet}
        </TabItem>

        <TabItem key="logs" disabled>
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><FileText size={18} />系统日志</span>
          {/snippet}
        </TabItem>
      </Tabs>

      <div class="glass-tab-panels">
        <section
          class="glass-tab-panel"
          class:glass-tab-panel--active={activeTab === "wifi"}
          hidden={activeTab !== "wifi"}
          aria-label="无线网络"
        >
          <WifiTab
            on:toast={(event) => showToast(event.detail.text, event.detail.kind)}
            on:statusPoll={startStatusPoll}
            on:statusChanged={loadWifiStatus}
          />
        </section>

        <section
          class="glass-tab-panel"
          class:glass-tab-panel--active={activeTab === "ethernet"}
          hidden={activeTab !== "ethernet"}
          aria-label="网口配置"
        >
          <EthernetTab
            on:toast={(event) => showToast(event.detail.text, event.detail.kind)}
            on:statusChanged={(event) => {
              ethernetStatus = event.detail.status;
              ethernetStatusError = null;
            }}
          />
        </section>

        <section
          class="glass-tab-panel"
          class:glass-tab-panel--active={activeTab === "daemon"}
          hidden={activeTab !== "daemon"}
          aria-label="UDP 网关"
        >
          <DaemonTab
            active={activeTab === "daemon"}
            on:toast={(event) => showToast(event.detail.text, event.detail.kind)}
            on:statusChanged={(event) => {
              daemonStatus = event.detail.status;
              daemonStatusError = null;
            }}
          />
        </section>

        <section
          class="glass-tab-panel"
          class:glass-tab-panel--active={activeTab === "diagnostic"}
          hidden={activeTab !== "diagnostic"}
          aria-label="诊断工具"
        >
          <DiagnosticTab
            {timeStatus}
            on:toast={(event) => showToast(event.detail.text, event.detail.kind)}
            on:timeChanged={(event) => {
              timeStatus = event.detail.status;
              timeStatusError = null;
            }}
          />
        </section>
      </div>
    </section>
  </div>

  {#if updatePanelOpen}
    <ModalLayer closeLabel="关闭更新面板" onClose={() => (updatePanelOpen = false)}>
      <div
        class="glass-card glass-dialog-panel glass-dialog-panel--wide"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-panel-title"
        tabindex="-1"
        transition:scale={{ duration: 220, start: 0.96, opacity: 0 }}
      >
        <div class="relative flex items-start justify-between gap-4 border-b border-white/35 px-5 py-4 dark:border-white/10">
          <div class="min-w-0">
            <h2 id="update-panel-title" class="text-lg font-semibold tracking-normal text-slate-950 dark:text-white">控制台更新</h2>
            <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">从 GitHub Release 获取控制台程序包</p>
          </div>
          <button
            type="button"
            class="glass-icon-button"
            aria-label="关闭更新面板"
            onclick={() => (updatePanelOpen = false)}
          >
            <X size={18} />
          </button>
        </div>
        <UpdateTab on:toast={(event) => showToast(event.detail.text, event.detail.kind)} />
      </div>
    </ModalLayer>
  {/if}

  {#if rebootConfirmOpen}
    <ModalLayer closeLabel="关闭重启确认窗口" disabled={rebooting} onClose={() => (rebootConfirmOpen = false)}>
      <div
        class="glass-card glass-dialog-panel glass-dialog-panel--compact"
        role="dialog"
        aria-modal="true"
        aria-labelledby="reboot-confirm-title"
        tabindex="-1"
        transition:scale={{ duration: 220, start: 0.96, opacity: 0 }}
      >
        <div class="relative flex items-start justify-between gap-4 border-b border-white/35 px-5 py-4 dark:border-white/10">
          <div class="min-w-0">
            <h2 id="reboot-confirm-title" class="text-lg font-semibold tracking-normal text-slate-950 dark:text-white">重启设备</h2>
            <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">设备会立即断开连接并重新启动</p>
          </div>
          <button
            type="button"
            class="glass-icon-button"
            aria-label="关闭重启确认窗口"
            disabled={rebooting}
            onclick={() => (rebootConfirmOpen = false)}
          >
            <X size={18} />
          </button>
        </div>

        <div class="relative p-5">
          <p class="text-sm leading-6 text-slate-700 dark:text-slate-200">确认现在重启设备？</p>
        </div>

        <div class="relative flex flex-col-reverse gap-3 border-t border-white/35 px-5 py-4 sm:flex-row sm:justify-end dark:border-white/10">
          <Button class="glass-button" color="alternative" disabled={rebooting} onclick={() => (rebootConfirmOpen = false)}>取消</Button>
          <Button class="glass-button glass-button--danger" color="alternative" loading={rebooting} disabled={rebooting} onclick={rebootDevice}>
            <Power size={16} class="mr-2" />
            重启设备
          </Button>
        </div>
      </div>
    </ModalLayer>
  {/if}

  {#if toasts.length > 0}
    <div class="fixed right-4 top-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-3">
      {#each toasts as message (message.id)}
        <Toast color={toastColor(message.kind)} class="glass-panel">{message.text}</Toast>
      {/each}
    </div>
  {/if}
</main>

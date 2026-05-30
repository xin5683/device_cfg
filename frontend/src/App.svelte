<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { Badge, Tabs, TabItem, Toast } from "flowbite-svelte";
  import { Activity, FileText, Radio, Search, Server, Wifi } from "@lucide/svelte";
  import StatusCard from "./components/StatusCard.svelte";
  import WifiTab from "./tabs/WifiTab.svelte";
  import UpdateTab from "./tabs/UpdateTab.svelte";
  import DaemonTab from "./tabs/DaemonTab.svelte";
  import { api } from "./api/client";
  import type { DaemonStatus, WifiStatus } from "./api/types";
  import type { DetailRow } from "./utils/format";
  import { compactTarget, stateLabel, statusDescription } from "./utils/format";
  import type { ToastKind, ToastMessage } from "./utils/toast";

  let systemVersion = "加载中";
  let activeTab = "wifi";
  let wifiStatus: WifiStatus | null = null;
  let wifiStatusError: string | null = null;
  let daemonStatus: DaemonStatus | null = null;
  let daemonStatusError: string | null = null;
  let disconnecting = false;
  let startingDaemon = false;
  let statusPollTimer: number | undefined;
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
  $: wifiRows = buildWifiRows(wifiStatus, wifiConnected);
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

  onMount(() => {
    void loadSystemInfo();
    void loadWifiStatus();
    void loadDaemonStatus();
  });

  onDestroy(() => {
    if (statusPollTimer) {
      window.clearInterval(statusPollTimer);
    }
  });

  function buildWifiRows(status: WifiStatus | null, connected: boolean): DetailRow[] {
    if (!status) {
      return [
        ["网络名称", "-"],
        ["IP 地址", "-"],
        ["BSSID", "-"]
      ];
    }

    const rows: DetailRow[] = [
      ["网络名称", connected && status.ssid ? status.ssid : "未连接"],
      ["IP 地址", connected && status.ip ? status.ip : "-"],
      ["BSSID", connected && status.bssid ? status.bssid : "-"]
    ];

    if (!connected && status.state && status.state !== "DISCONNECTED") {
      rows.push(["连接阶段", status.state]);
    }

    return rows;
  }

  function buildDaemonRows(status: DaemonStatus | null): DetailRow[] {
    return [
      ["安装版本", status?.installed_version ? `v${status.installed_version}` : "-"],
      ["PID", status?.pid],
      ["目标架构", compactTarget(status?.target)]
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

  async function disconnectWifi() {
    disconnecting = true;

    try {
      const result = await api.disconnectWifi();
      if (result.success) {
        showToast("已断开连接", "info");
        await loadWifiStatus();
      } else {
        showToast(`断开失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      showToast("断开请求失败", "error");
    } finally {
      disconnecting = false;
    }
  }

  async function startDaemon() {
    startingDaemon = true;

    try {
      const result = await api.startDaemon();
      if (result.success) {
        showToast("UDP 网关已启动", "success");
        await loadDaemonStatus();
      } else {
        showToast(`UDP 网关启动失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      showToast("UDP 网关启动请求失败", "error");
    } finally {
      startingDaemon = false;
      await loadDaemonStatus();
    }
  }
</script>

<main class="min-h-screen bg-gray-100">
  <div class="mx-auto flex w-full max-w-7xl flex-col gap-6 px-4 py-4 sm:px-6 lg:px-8">
    <header class="rounded-lg border border-gray-200 bg-white px-5 py-4 shadow-sm">
      <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex items-center gap-3">
          <span class="grid h-10 w-10 place-items-center rounded-lg bg-blue-700 text-white">
            <Activity size={22} />
          </span>
          <div>
            <h1 class="text-xl font-semibold text-gray-950">设备配置中心</h1>
            <p class="text-sm text-gray-500">嵌入式设备控制台</p>
          </div>
        </div>
        <Badge color="blue" class="w-fit">版本 {systemVersion}</Badge>
      </div>
    </header>

    <section class="grid gap-4 lg:grid-cols-2" aria-label="状态卡片区域">
      <StatusCard
        title="WiFi 状态"
        badge="wlan1"
        summaryTitle="当前连接状态"
        summary={wifiStatusError ?? statusDescription(wifiStatus?.state, wifiConnected)}
        stateText={wifiStateText}
        stateTone={wifiConnected ? "connected" : wifiStatus ? "disconnected" : "loading"}
        rows={wifiRows}
        actionLabel={wifiConnected ? "断开连接" : null}
        actionColor="red"
        actionLoading={disconnecting}
        onAction={disconnectWifi}
      />

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
        actionLabel={daemonStatus?.installed && !daemonStatus.running ? "启动" : null}
        actionColor="primary"
        actionLoading={startingDaemon}
        onAction={startDaemon}
      />
    </section>

    <section class="rounded-lg border border-gray-200 bg-white p-3 shadow-sm">
      <Tabs bind:selected={activeTab} tabStyle="pill" divider={false} contentClass="pt-4">
        <TabItem key="wifi">
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><Wifi size={18} />无线网络</span>
          {/snippet}
          <WifiTab
            on:toast={(event) => showToast(event.detail.text, event.detail.kind)}
            on:statusPoll={startStatusPoll}
            on:statusChanged={loadWifiStatus}
          />
        </TabItem>

        <TabItem key="update">
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><Search size={18} />系统更新</span>
          {/snippet}
          <UpdateTab on:toast={(event) => showToast(event.detail.text, event.detail.kind)} />
        </TabItem>

        <TabItem key="daemon">
          {#snippet titleSlot()}
            <span class="inline-flex items-center gap-2"><Server size={18} />UDP 网关</span>
          {/snippet}
          <DaemonTab
            active={activeTab === "daemon"}
            on:toast={(event) => showToast(event.detail.text, event.detail.kind)}
            on:statusChanged={(event) => {
              daemonStatus = event.detail.status;
              daemonStatusError = null;
            }}
          />
        </TabItem>

        <TabItem key="diagnostic" disabled>
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
    </section>
  </div>

  {#if toasts.length > 0}
    <div class="fixed right-4 top-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-3">
      {#each toasts as message (message.id)}
        <Toast color={toastColor(message.kind)}>{message.text}</Toast>
      {/each}
    </div>
  {/if}
</main>

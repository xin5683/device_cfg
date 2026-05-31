<script lang="ts">
  import { createEventDispatcher, tick } from "svelte";
  import { scale } from "svelte/transition";
  import { Button, Card, Input, Label } from "flowbite-svelte";
  import { Lock, LockOpen, Search, WifiOff, X } from "@lucide/svelte";
  import ModalLayer from "../components/ModalLayer.svelte";
  import SignalBars from "../components/SignalBars.svelte";
  import { api } from "../api/client";
  import type { NetworkInfo } from "../api/types";
  import type { ToastKind } from "../utils/toast";

  const dispatch = createEventDispatcher<{
    toast: { text: string; kind: ToastKind };
    statusPoll: void;
    statusChanged: void;
  }>();

  let networks: NetworkInfo[] = [];
  let scanState: "idle" | "loading" | "done" | "error" = "idle";
  let scanMessage = "点击“扫描网络”开始搜索";
  let selectedNetwork: NetworkInfo | null = null;
  let password = "";
  let connectOpen = false;
  let connecting = false;

  $: isSelectedOpen = selectedNetwork?.security === "开放";

  function notify(text: string, kind: ToastKind) {
    dispatch("toast", { text, kind });
  }

  function setConnectOpen(open: boolean) {
    connectOpen = open;
  }

  async function scan() {
    scanState = "loading";
    scanMessage = "正在扫描 WiFi 网络...";
    networks = [];

    try {
      const result = await api.scanNetworks();
      if (result.success) {
        networks = result.data ?? [];
        scanState = "done";
        scanMessage = networks.length > 0 ? "" : "未发现 WiFi 网络，请确认天线已连接";
      } else {
        scanState = "error";
        scanMessage = `扫描失败: ${result.message ?? "未知错误"}`;
      }
    } catch {
      scanState = "error";
      scanMessage = "扫描请求失败";
    }
  }

  async function openConnect(network: NetworkInfo) {
    selectedNetwork = network;
    password = "";
    setConnectOpen(true);
    await tick();
    document.getElementById("wifi-password-input")?.focus();
  }

  function closeConnect() {
    if (connecting) return;
    setConnectOpen(false);
  }

  function handleConnectKeydown(event: KeyboardEvent) {
    if (connectOpen && event.key === "Escape") {
      closeConnect();
    }
  }

  async function connect() {
    if (!selectedNetwork) return;
    if (!isSelectedOpen && !password) {
      notify("请输入 WiFi 密码", "error");
      return;
    }

    connecting = true;
    const body = isSelectedOpen
      ? { ssid: selectedNetwork.ssid }
      : { ssid: selectedNetwork.ssid, password };

    try {
      const result = await api.connectWifi(body);
      setConnectOpen(false);

      if (result.success) {
        notify(`连接成功！IP: ${result.data ?? ""}`, "success");
        dispatch("statusPoll");
      } else {
        notify(`连接失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      setConnectOpen(false);
      notify("连接请求失败", "error");
    } finally {
      connecting = false;
    }
  }
</script>

<svelte:window onkeydown={handleConnectKeydown} />

<Card class="glass-card max-w-none" size="xl">
  <div class="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h2 class="text-xl font-semibold tracking-normal text-slate-950 dark:text-white">可用网络</h2>
      <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">扫描并连接附近的 WiFi 热点</p>
    </div>
    <Button class="glass-button glass-button--primary w-full sm:w-auto" color="alternative" loading={scanState === "loading"} disabled={scanState === "loading"} onclick={scan}>
      <Search size={16} class="mr-2" />
      扫描网络
    </Button>
  </div>

  <div class="px-5 pb-5">
    {#if networks.length > 0}
      <div class="grid gap-3">
        {#each networks as network}
          <button
            type="button"
            class="glass-list-item grid grid-cols-[auto_1fr] items-center gap-4 p-4 text-left focus:outline-none sm:grid-cols-[auto_1fr_auto]"
            onclick={() => openConnect(network)}
          >
            <SignalBars signal={network.signal} />
            <span class="min-w-0">
              <span class="block truncate text-sm font-semibold text-slate-950 dark:text-white">{network.ssid}</span>
              <span class="mt-1 block text-xs text-slate-600 dark:text-slate-300">{network.signal} dBm · {network.frequency} MHz</span>
            </span>
            <span class="glass-pill col-span-2 inline-flex w-fit items-center gap-1 rounded-full px-2.5 py-1 text-xs font-medium sm:col-span-1">
              {#if network.security === "开放"}
                <LockOpen size={13} />
              {:else}
                <Lock size={13} />
              {/if}
              {network.security}
            </span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="glass-empty flex min-h-56 flex-col items-center justify-center border-dashed p-8 text-center">
        <WifiOff size={44} class="relative text-slate-400 dark:text-slate-500" />
        <p class="relative mt-4 text-sm text-slate-600 dark:text-slate-300">{scanMessage}</p>
      </div>
    {/if}
  </div>
</Card>

{#if connectOpen}
  <ModalLayer closeLabel="关闭连接窗口" onClose={closeConnect}>
    <div
      class="glass-card glass-dialog-panel glass-dialog-panel--compact"
      role="dialog"
      aria-modal="true"
      aria-labelledby="wifi-connect-title"
      tabindex="-1"
      transition:scale={{ duration: 220, start: 0.96, opacity: 0 }}
    >
      <div class="relative flex items-start justify-between gap-4 border-b border-white/35 px-5 py-4 dark:border-white/10">
        <div class="min-w-0">
          <h2 id="wifi-connect-title" class="text-lg font-semibold tracking-normal text-slate-950 dark:text-white">
            {isSelectedOpen ? "连接到开放网络" : "连接到网络"}
          </h2>
          <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">确认无线网络信息后建立连接</p>
        </div>
        <button
          type="button"
          class="glass-icon-button"
          aria-label="关闭连接窗口"
          disabled={connecting}
          onclick={closeConnect}
        >
          <X size={18} />
        </button>
      </div>

      <div class="relative space-y-5 p-5">
        <div class="glass-lens p-4">
          <p class="relative text-sm text-slate-600 dark:text-slate-300">网络名称</p>
          <p class="relative mt-1 break-words text-lg font-semibold text-slate-950 dark:text-white">{selectedNetwork?.ssid ?? "-"}</p>
          <p class="relative mt-2 inline-flex items-center gap-1 text-xs font-medium text-slate-600 dark:text-slate-300">
            {#if isSelectedOpen}
              <LockOpen size={13} />
            {:else}
              <Lock size={13} />
            {/if}
            {selectedNetwork?.security ?? "-"}
          </p>
        </div>

        {#if !isSelectedOpen}
          <div>
            <Label for="wifi-password-input" class="mb-2">密码</Label>
            <Input
              id="wifi-password-input"
              type="password"
              bind:value={password}
              placeholder="输入网络密码"
              autocomplete="off"
              onkeydown={(event) => event.key === "Enter" && connect()}
            />
          </div>
        {/if}
      </div>

      <div class="relative flex flex-col-reverse gap-3 border-t border-white/35 px-5 py-4 sm:flex-row sm:justify-end dark:border-white/10">
        <Button class="glass-button" color="alternative" disabled={connecting} onclick={closeConnect}>取消</Button>
        <Button class="glass-button glass-button--primary" color="alternative" loading={connecting} disabled={connecting} onclick={connect}>连接</Button>
      </div>
    </div>
  </ModalLayer>
{/if}

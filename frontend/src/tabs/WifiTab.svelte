<script lang="ts">
  import { createEventDispatcher, tick } from "svelte";
  import { Button, Card, Input, Label, Modal } from "flowbite-svelte";
  import { Lock, LockOpen, Search, WifiOff } from "@lucide/svelte";
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
    connectOpen = true;
    await tick();
    document.getElementById("wifi-password-input")?.focus();
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
      connectOpen = false;

      if (result.success) {
        notify(`连接成功！IP: ${result.data ?? ""}`, "success");
        dispatch("statusPoll");
      } else {
        notify(`连接失败: ${result.message ?? "未知错误"}`, "error");
      }
    } catch {
      connectOpen = false;
      notify("连接请求失败", "error");
    } finally {
      connecting = false;
    }
  }
</script>

<Card class="max-w-none border border-gray-200 bg-white shadow-sm" size="xl">
  <div class="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h2 class="text-xl font-semibold text-gray-950">可用网络</h2>
      <p class="mt-1 text-sm text-gray-500">扫描并连接附近的 WiFi 热点</p>
    </div>
    <Button color="primary" loading={scanState === "loading"} disabled={scanState === "loading"} onclick={scan}>
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
            class="grid grid-cols-[auto_1fr_auto] items-center gap-4 rounded-lg border border-gray-200 bg-white p-4 text-left transition hover:border-blue-300 hover:bg-blue-50 focus:outline-none focus:ring-4 focus:ring-blue-100"
            onclick={() => openConnect(network)}
          >
            <SignalBars signal={network.signal} />
            <span class="min-w-0">
              <span class="block truncate text-sm font-semibold text-gray-950">{network.ssid}</span>
              <span class="mt-1 block text-xs text-gray-500">{network.signal} dBm · {network.frequency} MHz</span>
            </span>
            <span class="inline-flex items-center gap-1 rounded-full bg-gray-100 px-2.5 py-1 text-xs font-medium text-gray-700">
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
      <div class="flex min-h-56 flex-col items-center justify-center rounded-lg border border-dashed border-gray-300 bg-gray-50 p-8 text-center">
        <WifiOff size={44} class="text-gray-300" />
        <p class="mt-4 text-sm text-gray-600">{scanMessage}</p>
      </div>
    {/if}
  </div>
</Card>

<Modal bind:open={connectOpen} title={isSelectedOpen ? "连接到开放网络" : "连接到网络"} size="sm">
  <div class="space-y-5">
    <div>
      <p class="text-sm text-gray-500">网络名称</p>
      <p class="mt-1 break-words text-lg font-semibold text-gray-950">{selectedNetwork?.ssid ?? "-"}</p>
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

    <div class="flex justify-end gap-3">
      <Button color="alternative" disabled={connecting} onclick={() => (connectOpen = false)}>取消</Button>
      <Button color="primary" loading={connecting} disabled={connecting} onclick={connect}>连接</Button>
    </div>
  </div>
</Modal>

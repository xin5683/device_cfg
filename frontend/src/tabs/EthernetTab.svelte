<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { Button, Card, Input, Label } from "flowbite-svelte";
  import { EthernetPort, RefreshCw, Save } from "@lucide/svelte";
  import DetailRows from "../components/DetailRows.svelte";
  import { api } from "../api/client";
  import type { EthernetMode, EthernetSettings, EthernetStatus } from "../api/types";
  import type { DetailRow } from "../utils/format";
  import type { ToastKind } from "../utils/toast";

  const dispatch = createEventDispatcher<{
    toast: { text: string; kind: ToastKind };
    statusChanged: { status: EthernetStatus };
  }>();

  let loading = true;
  let applying = false;
  let configPath = "/etc/network/interfaces.d/eth0";
  let iface = "eth0";
  let mode: EthernetMode = "dhcp";
  let address = "";
  let netmask = "";
  let gateway = "";
  let dns = "";
  let restartOutput = "";

  $: details = buildPreviewRows(mode, address, netmask, gateway, dns, configPath, iface);
  $: staticMode = mode === "static";

  onMount(() => {
    void loadConfig();
  });

  function notify(text: string, kind: ToastKind) {
    dispatch("toast", { text, kind });
  }

  function buildPreviewRows(
    currentMode: EthernetMode,
    currentAddress: string,
    currentNetmask: string,
    currentGateway: string,
    currentDns: string,
    currentConfigPath: string,
    currentIface: string
  ): DetailRow[] {
    const isStatic = currentMode === "static";
    return [
      ["配置文件", currentConfigPath],
      ["接口", currentIface],
      ["配置方式", currentMode === "dhcp" ? "DHCP" : "静态地址"],
      ["IP 地址", isStatic ? currentAddress : "自动获取"],
      ["子网掩码", isStatic ? currentNetmask : "自动获取"],
      ["网关", isStatic ? currentGateway || "不配置" : "自动获取"],
      ["DNS", isStatic ? currentDns || "不配置" : "自动获取"]
    ];
  }

  async function loadConfig() {
    loading = true;
    restartOutput = "";

    try {
      const result = await api.ethernetConfig();
      if (result.success && result.data) {
        configPath = result.data.path;
        iface = result.data.iface;
        applySettings(result.data.settings);
      } else {
        notify(result.message ?? "读取网口配置失败", "error");
      }
    } catch {
      notify("读取网口配置请求失败", "error");
    } finally {
      loading = false;
    }
  }

  function applySettings(settings: EthernetSettings) {
    mode = settings.mode;
    address = settings.address ?? "";
    netmask = settings.netmask ?? "";
    gateway = settings.gateway ?? "";
    dns = settings.dns_nameservers.join(" ");
  }

  function buildSettings(): EthernetSettings {
    return {
      mode,
      address: staticMode ? address.trim() : null,
      netmask: staticMode ? netmask.trim() : null,
      gateway: staticMode ? gateway.trim() : null,
      dns_nameservers: staticMode
        ? dns
            .split(/[\s,]+/)
            .map((value) => value.trim())
            .filter(Boolean)
        : []
    };
  }

  async function applyConfig() {
    if (staticMode && (!address.trim() || !netmask.trim())) {
      notify("静态地址需要填写 IP 和子网掩码", "error");
      return;
    }

    applying = true;
    restartOutput = "";

    try {
      const result = await api.applyEthernetConfig(buildSettings());
      if (result.success && result.data) {
        configPath = result.data.config.path;
        iface = result.data.config.iface;
        applySettings(result.data.config.settings);
        restartOutput = result.data.restart_output || "网络服务已重启。";
        dispatch("statusChanged", { status: result.data.status });
        notify("网口配置已应用", "success");
      } else {
        notify(result.message ?? "应用网口配置失败", "error");
      }
    } catch {
      notify("应用网口配置请求失败", "error");
    } finally {
      applying = false;
    }
  }
</script>

<Card class="glass-card max-w-none" size="xl">
  <div class="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h2 class="text-xl font-semibold tracking-normal text-slate-950 dark:text-white">网口配置</h2>
      <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">配置 {iface} 的 DHCP 或静态 IPv4 参数</p>
    </div>
    <div class="flex w-full flex-col gap-2 sm:w-auto sm:flex-row">
      <Button class="glass-button w-full sm:w-auto" color="alternative" loading={loading} disabled={loading || applying} onclick={loadConfig}>
        <RefreshCw size={16} class="mr-2" />
        刷新
      </Button>
      <Button class="glass-button glass-button--primary w-full sm:w-auto" color="alternative" loading={applying} disabled={loading || applying} onclick={applyConfig}>
        <Save size={16} class="mr-2" />
        应用配置
      </Button>
    </div>
  </div>

  <div class="grid gap-5 px-5 pb-5 lg:grid-cols-[minmax(0,1.2fr)_minmax(20rem,0.8fr)]">
    <div class="space-y-5">
      <div class="glass-lens p-4">
        <div class="relative flex items-center gap-3">
          <EthernetPort size={20} class="shrink-0 text-slate-600 dark:text-slate-300" />
          <div class="min-w-0">
            <h3 class="text-lg font-semibold tracking-normal text-slate-950 dark:text-white">{iface}</h3>
            <p class="mt-1 break-all text-sm text-slate-600 dark:text-slate-300">{configPath}</p>
          </div>
        </div>
      </div>

      <fieldset class="space-y-3">
        <legend class="mb-2 text-sm font-semibold text-slate-950 dark:text-white">配置方式</legend>
        <div class="grid gap-3 sm:grid-cols-2">
          <label class="glass-list-item flex cursor-pointer items-center gap-3 p-4">
            <input class="h-4 w-4" type="radio" bind:group={mode} value="dhcp" disabled={loading || applying} />
            <span class="min-w-0">
              <span class="block text-sm font-semibold text-slate-950 dark:text-white">DHCP</span>
              <span class="mt-1 block text-xs text-slate-600 dark:text-slate-300">自动获取地址、网关和 DNS</span>
            </span>
          </label>
          <label class="glass-list-item flex cursor-pointer items-center gap-3 p-4">
            <input class="h-4 w-4" type="radio" bind:group={mode} value="static" disabled={loading || applying} />
            <span class="min-w-0">
              <span class="block text-sm font-semibold text-slate-950 dark:text-white">静态地址</span>
              <span class="mt-1 block text-xs text-slate-600 dark:text-slate-300">手动设置 IPv4 网络参数</span>
            </span>
          </label>
        </div>
      </fieldset>

      {#if staticMode}
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <Label for="eth-address" class="mb-2">IP 地址</Label>
            <Input id="eth-address" bind:value={address} placeholder="192.168.1.100" disabled={loading || applying} />
          </div>
          <div>
            <Label for="eth-netmask" class="mb-2">子网掩码</Label>
            <Input id="eth-netmask" bind:value={netmask} placeholder="255.255.255.0" disabled={loading || applying} />
          </div>
          <div>
            <Label for="eth-gateway" class="mb-2">网关</Label>
            <Input id="eth-gateway" bind:value={gateway} placeholder="192.168.1.1" disabled={loading || applying} />
          </div>
          <div>
            <Label for="eth-dns" class="mb-2">DNS</Label>
            <Input id="eth-dns" bind:value={dns} placeholder="8.8.8.8 114.114.114.114" disabled={loading || applying} />
          </div>
        </div>
      {/if}
    </div>

    <div class="space-y-4">
      <DetailRows rows={details} />
      {#if restartOutput}
        <pre class="glass-terminal max-h-56 overflow-auto p-4 text-xs leading-5 text-slate-100">{restartOutput}</pre>
      {/if}
    </div>
  </div>
</Card>

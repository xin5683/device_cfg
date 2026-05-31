<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { Button, Card } from "flowbite-svelte";
  import { Clock, RefreshCw } from "@lucide/svelte";
  import DetailRows from "../components/DetailRows.svelte";
  import { api } from "../api/client";
  import type { TimeStatus } from "../api/types";
  import type { DetailRow } from "../utils/format";
  import type { ToastKind } from "../utils/toast";

  const dispatch = createEventDispatcher<{
    toast: { text: string; kind: ToastKind };
    timeChanged: { status: TimeStatus };
  }>();

  export let timeStatus: TimeStatus | null = null;

  let syncing = false;
  let syncOutput = "";

  $: details = buildRows(timeStatus);

  function buildRows(status: TimeStatus | null): DetailRow[] {
    return [
      ["NTP 服务器", status?.ntp_server],
      ["互联网连接", status ? (status.internet_connected ? "可用" : "不可用") : "-"],
      ["板卡时间", status?.board_time],
      ["互联网时间", status?.internet_time],
      ["状态信息", status?.error]
    ];
  }

  function notify(text: string, kind: ToastKind) {
    dispatch("toast", { text, kind });
  }

  async function syncTime() {
    syncing = true;
    syncOutput = "";

    try {
      const result = await api.syncTime();
      if (result.success && result.data?.synced) {
        syncOutput = result.data.output || "ntpd 已完成时间同步。";
        notify("时间同步完成", "success");
        await refreshTimeStatus();
      } else {
        syncOutput = result.message ?? "ntpd 时间同步失败";
        notify("时间同步失败", "error");
      }
    } catch {
      syncOutput = "同步请求失败";
      notify("时间同步请求失败", "error");
    } finally {
      syncing = false;
    }
  }

  async function refreshTimeStatus() {
    try {
      const result = await api.timeStatus();
      if (result.success && result.data) {
        dispatch("timeChanged", { status: result.data });
      }
    } catch {
      notify("时间状态刷新失败", "error");
    }
  }
</script>

<Card class="glass-card max-w-none" size="xl">
  <div class="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h2 class="text-xl font-semibold tracking-normal text-slate-950 dark:text-white">诊断工具</h2>
      <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">通过 ntpd 手动同步板卡时间</p>
    </div>
    <Button class="glass-button glass-button--primary w-full sm:w-auto" color="alternative" loading={syncing} disabled={syncing} onclick={syncTime}>
      <RefreshCw size={16} class="mr-2" />
      同步时间
    </Button>
  </div>

  <div class="px-5 pb-5">
    <div class="glass-lens mb-4 flex items-start gap-3 p-4">
      <Clock size={20} class="relative mt-0.5 shrink-0 text-slate-600 dark:text-slate-300" />
      <div class="relative min-w-0">
        <h3 class="text-lg font-semibold tracking-normal text-slate-950 dark:text-white">时间同步</h3>
        <p class="mt-1 text-sm text-slate-700 dark:text-slate-200">
          {timeStatus?.internet_connected ? "互联网时间源可用，可执行手动同步。" : "互联网时间源不可用，请先检查网络连接。"}
        </p>
      </div>
    </div>

    <DetailRows rows={details} />

    {#if syncOutput}
      <pre class="glass-terminal mt-5 max-h-56 overflow-auto p-4 text-xs leading-5 text-slate-100">{syncOutput}</pre>
    {/if}
  </div>
</Card>

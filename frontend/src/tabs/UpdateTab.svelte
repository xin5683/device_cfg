<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { Button, Card } from "flowbite-svelte";
  import { Download, RefreshCw } from "@lucide/svelte";
  import DetailRows from "../components/DetailRows.svelte";
  import { api } from "../api/client";
  import type { ApplyUpdateResult, UpdateInfo } from "../api/types";
  import type { DetailRow } from "../utils/format";
  import { joinLines } from "../utils/format";
  import type { ToastKind } from "../utils/toast";

  const dispatch = createEventDispatcher<{ toast: { text: string; kind: ToastKind } }>();

  let title = "尚未检查更新";
  let summary = "点击“检查更新”获取最新 release 信息。";
  let details: DetailRow[] = [];
  let updateInfo: UpdateInfo | null = null;
  let checking = false;
  let applying = false;

  $: canApply =
    !!updateInfo?.update_available && !!updateInfo.asset_name && !!updateInfo.checksum_asset_name && !applying;

  function notify(text: string, kind: ToastKind) {
    dispatch("toast", { text, kind });
  }

  function renderUpdateInfo(info: UpdateInfo) {
    const hasAsset = !!info.asset_name;
    const hasChecksum = !!info.checksum_asset_name;

    if (info.update_available && hasAsset && hasChecksum) {
      title = "发现新版本";
      summary = `当前 v${info.current_version}，可更新到 v${info.latest_version}。`;
    } else if (info.update_available && !hasAsset) {
      title = "发现新版本但无匹配包";
      summary = `最新版本 v${info.latest_version} 没有匹配 ${info.target} 的更新包。`;
    } else if (info.update_available && !hasChecksum) {
      title = "发现新版本但缺少校验文件";
      summary = `最新版本 v${info.latest_version} 未提供 SHA256 校验文件。`;
    } else {
      title = "已是最新版本";
      summary = `当前版本 v${info.current_version} 与最新 release 一致。`;
    }

    details = [
      ["当前版本", `v${info.current_version}`],
      ["最新版本", `v${info.latest_version}`],
      ["目标平台", info.target],
      ["更新包", info.asset_name],
      ["校验文件", info.checksum_asset_name],
      ["Release", info.release_url],
      ["镜像顺序", joinLines(info.mirror_urls)]
    ];
  }

  function renderApplyResult(result: ApplyUpdateResult) {
    title = "更新完成";
    summary = `已从 v${result.previous_version} 更新到 v${result.updated_version}，请重启服务后生效。`;
    details = [
      ["目标平台", result.target],
      ["更新包", result.asset_name],
      ["SHA256", result.sha256],
      ["下载地址", result.downloaded_from],
      ["安装路径", result.installed_path]
    ];
  }

  async function checkUpdate() {
    checking = true;
    updateInfo = null;
    title = "正在检查更新";
    summary = "正在通过镜像站请求 GitHub Release 信息。";
    details = [];

    try {
      const result = await api.checkUpdate();
      if (result.success && result.data) {
        updateInfo = result.data;
        renderUpdateInfo(result.data);
      } else {
        title = "检查更新失败";
        summary = result.message ?? "无法获取最新版本信息";
        notify("检查更新失败", "error");
      }
    } catch {
      title = "检查更新失败";
      summary = "更新服务未响应";
      notify("更新服务未响应", "error");
    } finally {
      checking = false;
    }
  }

  async function applyUpdate() {
    applying = true;
    title = "正在更新";
    summary = "正在下载并替换当前程序，请不要断电。";

    try {
      const result = await api.applyUpdate();
      if (result.success && result.data) {
        renderApplyResult(result.data);
        notify("更新完成，请重启服务", "success");
      } else {
        title = "更新失败";
        summary = result.message ?? "下载或安装更新失败";
        notify("更新失败", "error");
      }
    } catch {
      title = "更新失败";
      summary = "更新请求失败";
      notify("更新请求失败", "error");
    } finally {
      applying = false;
    }
  }
</script>

<Card class="glass-card max-w-none" size="xl">
  <div class="flex flex-col gap-4 p-5 xl:flex-row xl:items-center xl:justify-between">
    <div>
      <h2 class="text-xl font-semibold tracking-normal text-slate-950 dark:text-white">系统更新</h2>
      <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">从 GitHub Release 获取设备固件包</p>
    </div>
    <div class="flex flex-wrap gap-3">
      <Button class="glass-button" color="alternative" loading={checking} disabled={checking || applying} onclick={checkUpdate}>
        <RefreshCw size={16} class="mr-2" />
        检查更新
      </Button>
      <Button class="glass-button glass-button--primary" color="alternative" loading={applying} disabled={!canApply} onclick={applyUpdate}>
        <Download size={16} class="mr-2" />
        立即更新
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
  </div>
</Card>

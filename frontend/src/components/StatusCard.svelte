<script lang="ts">
  import { Badge, Button, Card } from "flowbite-svelte";
  import DetailRows from "./DetailRows.svelte";
  import type { DetailRow } from "../utils/format";

  export let title: string;
  export let badge: string;
  export let summaryTitle: string;
  export let summary: string;
  export let stateText: string;
  export let stateTone: "connected" | "disconnected" | "loading" = "loading";
  export let rows: DetailRow[] = [];
  export let actionLabel: string | null = null;
  export let actionColor: "primary" | "red" | "green" | "blue" | "gray" = "primary";
  export let actionLoading = false;
  export let actionDisabled = false;
  export let onAction: (() => void | Promise<void>) | null = null;
  let stateColor: "green" | "gray" | "yellow" = "yellow";

  $: stateColor = stateTone === "connected" ? "green" : stateTone === "disconnected" ? "gray" : "yellow";

  function actionToneClass(color: typeof actionColor) {
    if (color === "primary" || color === "blue") return "glass-button--primary";
    if (color === "red") return "glass-button--danger";
    if (color === "green") return "glass-button--success";
    return "";
  }
</script>

<Card class="glass-card h-full max-w-none" size="xl">
  <div class="flex h-full flex-col">
    <div class="flex items-start justify-between gap-4 p-5">
      <div>
        <p class="text-sm font-medium text-slate-600 dark:text-slate-300">{title}</p>
        <h2 class="mt-1 text-xl font-semibold tracking-normal text-slate-950 dark:text-white">{summaryTitle}</h2>
      </div>
      <Badge color="blue" class="glass-badge shrink-0">{badge}</Badge>
    </div>

    <div class="flex flex-1 flex-col px-5 pb-5">
      <div class={`glass-lens glass-lens--${stateTone} mb-4 flex items-start justify-between gap-3 p-4`}>
        <p class="relative text-sm text-slate-700 dark:text-slate-200">{summary}</p>
        <Badge color={stateColor} class="glass-status-badge shrink-0">
          <span class={`status-dot status-dot--${stateTone}`}></span>
          {stateText}
        </Badge>
      </div>

      <DetailRows {rows} />

      <div class="mt-auto flex min-h-10 items-end justify-end pt-4">
        {#if actionLabel && onAction}
        <Button
          color="alternative"
          size="sm"
          class={`glass-button ${actionToneClass(actionColor)}`}
          loading={actionLoading}
          disabled={actionDisabled || actionLoading}
          onclick={onAction}
        >
          {actionLabel}
        </Button>
        {/if}
      </div>
    </div>
  </div>
</Card>

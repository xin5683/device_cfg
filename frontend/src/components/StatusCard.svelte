<script lang="ts">
  import { Badge, Card } from "flowbite-svelte";
  import type { DetailRow } from "../utils/format";
  import { displayValue } from "../utils/format";

  export let title: string;
  export let badge: string;
  export let summaryTitle: string;
  export let summary: string;
  export let stateText: string;
  export let stateTone: "connected" | "disconnected" | "loading" = "loading";
  export let rows: DetailRow[] = [];
  export let layout: "default" | "horizontal" = "default";
  let stateColor: "green" | "gray" | "yellow" = "yellow";

  $: stateColor = stateTone === "connected" ? "green" : stateTone === "disconnected" ? "gray" : "yellow";
</script>

<Card class="glass-card h-full max-w-none" size="xl">
  <div
    class={layout === "horizontal"
      ? "flex h-full flex-col gap-3 p-3 sm:flex-row sm:items-center sm:justify-between sm:gap-4 sm:px-4"
      : "flex h-full flex-col"}
  >
    <div class={layout === "horizontal" ? "flex min-w-0 items-center justify-between gap-3 sm:w-48 sm:shrink-0 sm:justify-start" : "flex items-start justify-between gap-4 p-5"}>
      <div class="min-w-0">
        <p class="truncate text-sm font-medium text-slate-600 dark:text-slate-300">{title}</p>
        <h2
          class={layout === "horizontal"
            ? "mt-0.5 truncate text-sm font-semibold tracking-normal text-slate-950 dark:text-white"
            : "mt-1 text-xl font-semibold tracking-normal text-slate-950 dark:text-white"}
        >
          {summaryTitle}
        </h2>
      </div>
      <Badge color="blue" class="glass-badge shrink-0">{badge}</Badge>
    </div>

    {#if layout === "horizontal"}
      <div class="flex min-w-0 items-center gap-3 sm:flex-1">
        <span class={`status-dot status-dot--${stateTone} shrink-0`}></span>
        <div class="min-w-0">
          <div class="text-sm font-semibold text-slate-950 dark:text-white">{stateText}</div>
          <p class="truncate text-sm text-slate-600 dark:text-slate-300">{summary}</p>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-white/50 pt-3 dark:border-white/10 sm:ml-auto sm:max-w-[55%] sm:justify-end sm:border-l sm:border-t-0 sm:pl-4 sm:pt-0">
        {#each rows as [label, value]}
          <div class="min-w-0">
            <span class="mr-1 text-xs font-medium text-slate-500 dark:text-slate-400">{label}</span>
            <span class="font-mono text-sm font-semibold text-slate-950 dark:text-slate-100">
              {displayValue(value)}
            </span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="flex flex-1 flex-col px-5 pb-5">
        <div class={`glass-lens glass-lens--${stateTone} mb-4 flex items-start justify-between gap-3 p-4`}>
          <p class="relative text-sm text-slate-700 dark:text-slate-200">{summary}</p>
          <Badge color={stateColor} class="glass-status-badge shrink-0">
            <span class={`status-dot status-dot--${stateTone}`}></span>
            {stateText}
          </Badge>
        </div>

        <div class="glass-detail-table divide-y divide-white/50 dark:divide-white/10">
          {#each rows as [label, value]}
            <div class="grid gap-2 px-4 py-3 text-sm sm:grid-cols-[140px_1fr]">
              <span class="font-medium text-slate-600 dark:text-slate-300">{label}</span>
              <span class="break-words whitespace-pre-line font-mono text-slate-950 dark:text-slate-100">{displayValue(value)}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</Card>

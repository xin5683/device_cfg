<script lang="ts">
  import { Badge, Card } from "flowbite-svelte";
  import { displayValue } from "../utils/format";

  type MetricValue = string | number | null | undefined;
  type CompactMetric = [string, MetricValue];

  export let title: string;
  export let badge: string;
  export let summaryTitle: string;
  export let summary: string;
  export let stateText: string;
  export let stateTone: "connected" | "disconnected" | "loading" = "loading";
  export let metrics: CompactMetric[] = [];
  export let metricsLayout: "grid" | "list" = "grid";

  let stateColor: "green" | "gray" | "yellow" = "yellow";

  $: stateColor = stateTone === "connected" ? "green" : stateTone === "disconnected" ? "gray" : "yellow";
</script>

<Card class="glass-card h-full max-w-none" size="xl">
  <div class="flex h-full flex-col gap-3 p-4">
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <p class="truncate text-sm font-medium text-slate-600 dark:text-slate-300">{title}</p>
        <h2 class="mt-1 truncate text-base font-semibold tracking-normal text-slate-950 dark:text-white">
          {summaryTitle}
        </h2>
      </div>
      <Badge color="blue" class="glass-badge shrink-0">{badge}</Badge>
    </div>

    <div class={`glass-lens glass-lens--${stateTone} flex items-start justify-between gap-3 p-3`}>
      <p class="relative line-clamp-2 min-w-0 text-sm text-slate-700 dark:text-slate-200">{summary}</p>
      <Badge color={stateColor} class="glass-status-badge shrink-0">
        <span class={`status-dot status-dot--${stateTone}`}></span>
        {stateText}
      </Badge>
    </div>

    {#if metricsLayout === "list"}
      <div class="divide-y divide-white/50 rounded-lg border border-white/50 bg-white/30 shadow-inner dark:divide-white/10 dark:border-white/10 dark:bg-white/10">
        {#each metrics as [label, value]}
          <div class="grid grid-cols-[5rem_minmax(0,1fr)] items-baseline gap-2 px-3 py-2">
            <span class="truncate text-xs font-medium text-slate-500 dark:text-slate-400">{label}</span>
            <span class="min-w-0 break-words text-right font-mono text-sm font-semibold text-slate-950 dark:text-slate-100">
              {displayValue(value)}
            </span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="grid grid-cols-2 gap-2">
        {#each metrics as [label, value]}
          <div class="min-w-0 rounded-lg border border-white/50 bg-white/30 px-3 py-2 shadow-inner dark:border-white/10 dark:bg-white/10">
            <span class="block truncate text-xs font-medium text-slate-500 dark:text-slate-400">{label}</span>
            <span class="mt-1 block break-words font-mono text-sm font-semibold text-slate-950 dark:text-slate-100">
              {displayValue(value)}
            </span>
          </div>
        {/each}
      </div>
    {/if}

  </div>
</Card>

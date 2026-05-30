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
</script>

<Card class="h-full max-w-none border border-gray-200 bg-white shadow-sm" size="xl">
  <div class="flex items-start justify-between gap-4 p-5">
    <div>
      <p class="text-sm font-medium text-gray-500">{title}</p>
      <h2 class="mt-1 text-xl font-semibold text-gray-950">{summaryTitle}</h2>
    </div>
    <Badge color="blue" class="shrink-0">{badge}</Badge>
  </div>

  <div class="px-5 pb-5">
    <div class="mb-4 flex items-start justify-between gap-3 rounded-lg border border-gray-100 bg-gray-50 p-4">
      <p class="text-sm text-gray-600">{summary}</p>
      <Badge color={stateColor}>{stateText}</Badge>
    </div>

    <DetailRows {rows} />

    {#if actionLabel && onAction}
      <div class="mt-4 flex justify-end">
        <Button
          color={actionColor}
          size="sm"
          loading={actionLoading}
          disabled={actionDisabled || actionLoading}
          onclick={onAction}
        >
          {actionLabel}
        </Button>
      </div>
    {/if}
  </div>
</Card>

<script lang="ts">
  import type { DetailRow } from "../utils/format";
  import { displayValue } from "../utils/format";

  export let rows: DetailRow[] = [];
</script>

<div class="glass-detail-table divide-y divide-white/50 dark:divide-white/10">
  {#each rows as [label, value]}
    <div class="grid gap-2 px-4 py-3 text-sm sm:grid-cols-[140px_1fr]">
      <span class="font-medium text-slate-600 dark:text-slate-300">{label}</span>
      <span class="break-words font-mono text-slate-950 whitespace-pre-line dark:text-slate-100">{displayValue(value)}</span>
    </div>
  {/each}
</div>

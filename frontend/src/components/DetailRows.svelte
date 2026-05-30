<script lang="ts">
  import type { DetailRow } from "../utils/format";
  import { displayValue } from "../utils/format";

  export let rows: DetailRow[] = [];
</script>

<div class="divide-y divide-gray-100 rounded-lg border border-gray-100 bg-white">
  {#each rows as [label, value]}
    <div class="grid gap-2 px-4 py-3 text-sm sm:grid-cols-[140px_1fr]">
      <span class="font-medium text-gray-500">{label}</span>
      <span class="break-words font-mono text-gray-900 whitespace-pre-line">{displayValue(value)}</span>
    </div>
  {/each}
</div>

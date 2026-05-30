<script lang="ts">
  export let signal = -90;

  $: level = signal >= -50 ? 4 : signal >= -65 ? 3 : signal >= -75 ? 2 : 1;
</script>

<div class="flex h-8 w-8 items-end justify-center gap-0.5 text-[#007aff] dark:text-[#0a84ff]" aria-label={`信号 ${signal} dBm`}>
  {#each [1, 2, 3, 4] as bar}
    <span
      class:opacity-25={bar > level}
      class="w-1 rounded-full bg-current"
      style={`height: ${bar * 5 + 7}px`}
    ></span>
  {/each}
</div>

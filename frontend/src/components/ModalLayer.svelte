<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { closeModal, openModal } from "../utils/modal";

  export let align: "center" | "top" = "center";
  export let backdropClass = "";
  export let closeLabel = "关闭弹窗";
  export let disabled = false;
  export let onClose: (() => void) | null = null;

  function portal(node: HTMLElement) {
    document.body.appendChild(node);

    return {
      destroy() {
        node.remove();
      }
    };
  }

  onMount(openModal);
  onDestroy(closeModal);

  function close() {
    if (disabled) return;
    onClose?.();
  }
</script>

<div
  class={`glass-modal fixed inset-0 z-40 flex px-4 py-6 ${align === "top" ? "items-start justify-center overflow-y-auto sm:py-10" : "items-center justify-center"}`}
  use:portal
>
  <button
    type="button"
    class={`glass-modal-backdrop ${backdropClass}`}
    aria-label={closeLabel}
    {disabled}
    onclick={close}
    transition:fade={{ duration: 180 }}
  ></button>

  <slot />
</div>

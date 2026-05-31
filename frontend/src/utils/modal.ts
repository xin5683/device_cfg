import { derived, writable } from "svelte/store";

export const modalDepth = writable(0);
export const modalOpen = derived(modalDepth, ($modalDepth) => $modalDepth > 0);

export function openModal() {
  modalDepth.update((value) => value + 1);
}

export function closeModal() {
  modalDepth.update((value) => Math.max(0, value - 1));
}

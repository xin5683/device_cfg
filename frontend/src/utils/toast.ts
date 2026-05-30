export type ToastKind = "success" | "error" | "info";

export type ToastMessage = {
  id: number;
  text: string;
  kind: ToastKind;
};

import { el } from "./dom";

export type ToastKind = "info" | "success" | "error";

let container: HTMLElement | null = null;

function ensureContainer(): HTMLElement {
  if (!container) {
    container = el("div", { class: "toast-container", attrs: { "aria-live": "polite" } });
    document.body.appendChild(container);
  }
  return container;
}

export function showToast(message: string, kind: ToastKind = "info", timeoutMs = 3600): void {
  const root = ensureContainer();
  const toast = el("div", { class: `toast toast-${kind}`, attrs: { role: "status" } }, message);
  root.appendChild(toast);
  // trigger enter transition
  requestAnimationFrame(() => toast.classList.add("toast-show"));

  const dismiss = () => {
    toast.classList.remove("toast-show");
    setTimeout(() => toast.remove(), 200);
  };
  toast.addEventListener("click", dismiss);
  if (timeoutMs > 0) setTimeout(dismiss, timeoutMs);
}

export function toastError(message: string, hint?: string): void {
  showToast(hint ? `${message}\n${hint}` : message, "error", 6000);
}

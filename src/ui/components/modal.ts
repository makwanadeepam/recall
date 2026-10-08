import { el } from "./dom";

export interface ModalHandle {
  close(): void;
  content: HTMLElement;
}

const openStack: ModalHandle[] = [];

export interface ModalOptions {
  title?: string;
  body: Node;
  footer?: Node;
  onClose?: () => void;
  class?: string;
}

export function openModal(opts: ModalOptions): ModalHandle {
  const overlay = el("div", { class: "modal-overlay" });
  const box = el("div", {
    class: "modal " + (opts.class ?? ""),
    attrs: { role: "dialog", "aria-modal": "true" },
  });

  let closed = false;
  const handle: ModalHandle = {
    content: box,
    close() {
      if (closed) return;
      closed = true;
      overlay.remove();
      const i = openStack.indexOf(handle);
      if (i >= 0) openStack.splice(i, 1);
      opts.onClose?.();
    },
  };

  const closeBtn = el(
    "button",
    { class: "icon-btn modal-close", title: "Close (Esc)", on: { click: () => handle.close() } },
    "×"
  );
  const header = el(
    "div",
    { class: "modal-header" },
    el("h2", { class: "modal-title", text: opts.title ?? "" }),
    closeBtn
  );

  const children: Node[] = [header, opts.body];
  if (opts.footer) children.push(opts.footer);
  box.append(...children);

  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) handle.close();
  });

  overlay.appendChild(box);
  document.body.appendChild(overlay);
  openStack.push(handle);

  // Focus the first sensible control.
  const focusable = box.querySelector<HTMLElement>(
    "input, textarea, select, button:not(.modal-close)"
  );
  focusable?.focus();

  return handle;
}

/** Close the top-most modal. Returns true if one was open. */
export function closeTopModal(): boolean {
  const h = openStack[openStack.length - 1];
  if (h) {
    h.close();
    return true;
  }
  return false;
}

export function anyModalOpen(): boolean {
  return openStack.length > 0;
}

export interface ConfirmOptions {
  title?: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

export function confirm(opts: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    let decided = false;
    let handle: ModalHandle;

    const cancel = el(
      "button",
      { class: "btn", on: { click: () => handle.close() } },
      opts.cancelLabel ?? "Cancel"
    );
    const ok = el(
      "button",
      {
        class: "btn " + (opts.danger ? "btn-danger" : "btn-primary"),
        on: {
          click: () => {
            decided = true;
            handle.close();
            resolve(true);
          },
        },
      },
      opts.confirmLabel ?? "Confirm"
    );

    const body = el("div", { class: "modal-body" }, el("p", { class: "modal-message", text: opts.message }));
    const footer = el("div", { class: "modal-footer" }, cancel, ok);

    handle = openModal({
      title: opts.title ?? "Are you sure?",
      body,
      footer,
      onClose: () => {
        if (!decided) resolve(false);
      },
    });
    ok.focus();
  });
}

import { el, type Child } from "./dom";
import { icon } from "./icons";

export type ButtonVariant = "primary" | "ghost" | "danger" | "default";

export interface ButtonOptions {
  variant?: ButtonVariant;
  icon?: string;
  onClick?: () => void;
  title?: string;
  disabled?: boolean;
  class?: string;
  type?: string;
}

export function button(label: string, opts: ButtonOptions = {}): HTMLButtonElement {
  const variantClass = opts.variant && opts.variant !== "default" ? ` btn-${opts.variant}` : "";
  const children: Child[] = [];
  if (opts.icon) children.push(icon(opts.icon, "btn-icon"));
  if (label) children.push(el("span", { text: label }));
  return el(
    "button",
    {
      class: "btn" + variantClass + (opts.class ? ` ${opts.class}` : ""),
      title: opts.title,
      disabled: opts.disabled,
      attrs: { type: opts.type ?? "button" },
      on: opts.onClick ? { click: opts.onClick } : undefined,
    },
    ...children
  );
}

export function iconButton(iconName: string, title: string, onClick: () => void): HTMLButtonElement {
  return el("button", { class: "icon-btn", title, on: { click: onClick } }, icon(iconName));
}

export function pageHeader(title: string, subtitle?: string, actions?: Child[]): HTMLElement {
  const left = el(
    "div",
    { class: "page-header-text" },
    el("h1", { class: "page-title", text: title }),
    subtitle ? el("p", { class: "page-subtitle", text: subtitle }) : null
  );
  return el(
    "header",
    { class: "page-header" },
    left,
    actions && actions.length ? el("div", { class: "page-actions" }, ...actions) : null
  );
}

export function tagChip(name: string, onClick?: () => void): HTMLElement {
  return el(
    onClick ? "button" : "span",
    {
      class: "chip" + (onClick ? " chip-button" : ""),
      on: onClick ? { click: onClick } : undefined,
    },
    icon("tag", "chip-icon"),
    el("span", { text: name })
  );
}

export function emptyState(
  iconName: string,
  title: string,
  message: string,
  action?: HTMLElement
): HTMLElement {
  return el(
    "div",
    { class: "empty-state" },
    el("div", { class: "empty-icon" }, icon(iconName)),
    el("h3", { text: title }),
    el("p", { class: "muted", text: message }),
    action ?? null
  );
}

export function statTile(iconName: string, label: string, value: string | number, accent = ""): HTMLElement {
  return el(
    "div",
    { class: "stat-tile" + (accent ? ` accent-${accent}` : "") },
    el("div", { class: "stat-icon" }, icon(iconName)),
    el(
      "div",
      { class: "stat-body" },
      el("div", { class: "stat-value", text: String(value) }),
      el("div", { class: "stat-label", text: label })
    )
  );
}

export function spinner(label = "Loading…"): HTMLElement {
  return el(
    "div",
    { class: "loading" },
    el("div", { class: "spinner", attrs: { "aria-hidden": "true" } }),
    el("span", { class: "muted", text: label })
  );
}

export function ratingBadge(rating: string): HTMLElement {
  return el("span", { class: `rating-badge rating-${rating}`, text: rating });
}

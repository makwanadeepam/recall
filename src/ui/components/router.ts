import { clear, el } from "./dom";
import { icon } from "./icons";
import { showShortcutsHelp } from "./help";

export type RouteName = "dashboard" | "library" | "knowledge" | "recall" | "chat" | "settings";

export interface Route {
  name: RouteName;
  params: Record<string, unknown>;
}

export type PageRender = (container: HTMLElement, params: Record<string, unknown>) => void | Promise<void>;

const pages = new Map<RouteName, PageRender>();
const navButtons = new Map<RouteName, HTMLElement>();

let contentEl: HTMLElement;
let current: Route = { name: "dashboard", params: {} };
let pageCleanup: (() => void) | null = null;

export function registerPage(name: RouteName, render: PageRender): void {
  pages.set(name, render);
}

/** Register a cleanup callback for the current page (called before the next
 *  page renders). Used e.g. by the recall session to remove its key handler. */
export function onLeave(fn: () => void): void {
  pageCleanup = fn;
}

export function getCurrentRoute(): Route {
  return current;
}

export async function navigate(name: RouteName, params: Record<string, unknown> = {}): Promise<void> {
  current = { name, params };
  for (const [n, btn] of navButtons) btn.classList.toggle("active", n === name);
  await render();
}

async function render(): Promise<void> {
  const fn = pages.get(current.name);
  if (!fn) return;
  if (pageCleanup) {
    pageCleanup();
    pageCleanup = null;
  }
  clear(contentEl);
  contentEl.scrollTop = 0;
  try {
    await fn(contentEl, current.params);
  } catch (e) {
    clear(contentEl);
    contentEl.appendChild(
      el(
        "div",
        { class: "empty-state" },
        el("p", { text: "Something went wrong rendering this page." }),
        el("p", { class: "muted", text: String(e) })
      )
    );
  }
}

/** Re-render the current page (after data changes). */
export function refresh(): Promise<void> {
  return render();
}

function navItem(name: RouteName, label: string, iconName: string): HTMLElement {
  const btn = el(
    "button",
    { class: "nav-item", on: { click: () => navigate(name) } },
    icon(iconName),
    el("span", { text: label })
  );
  navButtons.set(name, btn);
  return btn;
}

export function buildShell(root: HTMLElement): void {
  const brand = el(
    "div",
    { class: "brand" },
    el("span", { class: "brand-mark" }, icon("layers")),
    el("span", { class: "brand-name", text: "Recall" })
  );

  const nav = el(
    "nav",
    { class: "nav" },
    navItem("dashboard", "Dashboard", "home"),
    navItem("library", "Library", "library"),
    navItem("recall", "Recall", "recall"),
    navItem("chat", "Chat", "chat"),
    navItem("settings", "Settings", "settings")
  );

  const footer = el(
    "div",
    { class: "sidebar-footer" },
    el(
      "button",
      { class: "nav-item subtle", title: "Keyboard shortcuts (?)", on: { click: () => showShortcutsHelp() } },
      icon("keyboard"),
      el("span", { text: "Shortcuts" })
    )
  );

  const sidebar = el("aside", { class: "sidebar" }, brand, nav, footer);
  contentEl = el("main", { class: "content", id: "content" });

  clear(root);
  root.appendChild(el("div", { class: "app-shell" }, sidebar, contentEl));
}

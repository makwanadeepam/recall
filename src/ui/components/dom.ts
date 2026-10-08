/**
 * A tiny hyperscript helper. Deliberately framework-free.
 *
 * Security: dynamic/user/AI content is always set via `text` or string children
 * (which become text nodes), never via innerHTML. `html` is available only for
 * trusted, static markup (e.g. inline icon SVGs defined in code).
 */

export type Child = Node | string | number | null | undefined | false;

export interface ElProps {
  class?: string;
  id?: string;
  type?: string;
  name?: string;
  value?: string;
  placeholder?: string;
  title?: string;
  href?: string;
  disabled?: boolean;
  checked?: boolean;
  rows?: number;
  min?: string | number;
  max?: string | number;
  step?: string | number;
  tabindex?: number;
  role?: string;
  text?: string | number;
  /** Trusted static markup only. Never pass user or AI content here. */
  html?: string;
  attrs?: Record<string, string | number | boolean | null | undefined>;
  dataset?: Record<string, string>;
  style?: Partial<CSSStyleDeclaration> | string;
  on?: Record<string, EventListenerOrEventListenerObject>;
}

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: ElProps = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);

  for (const [key, val] of Object.entries(props)) {
    if (val === null || val === undefined || val === false) continue;
    switch (key) {
      case "class":
        node.className = String(val);
        break;
      case "text":
        node.textContent = String(val);
        break;
      case "html":
        node.innerHTML = String(val);
        break;
      case "style":
        if (typeof val === "string") node.setAttribute("style", val);
        else Object.assign(node.style, val);
        break;
      case "dataset":
        Object.assign(node.dataset, val as Record<string, string>);
        break;
      case "attrs":
        for (const [a, av] of Object.entries(val as Record<string, unknown>)) {
          if (av !== null && av !== undefined && av !== false) node.setAttribute(a, String(av));
        }
        break;
      case "on":
        for (const [ev, fn] of Object.entries(val as Record<string, EventListener>)) {
          node.addEventListener(ev, fn);
        }
        break;
      default:
        // value, disabled, placeholder, href, id, title, type, rows, etc.
        (node as unknown as Record<string, unknown>)[key] = val;
    }
  }

  appendChildren(node, children);
  return node;
}

function appendChildren(node: Node, children: Child[]): void {
  for (const child of children) {
    if (child === null || child === undefined || child === false) continue;
    if (child instanceof Node) node.appendChild(child);
    else node.appendChild(document.createTextNode(String(child)));
  }
}

export function frag(...children: Child[]): DocumentFragment {
  const f = document.createDocumentFragment();
  appendChildren(f, children);
  return f;
}

export function clear(node: Node): void {
  while (node.firstChild) node.removeChild(node.firstChild);
}

export function mount(parent: Node, ...children: Child[]): void {
  clear(parent);
  appendChildren(parent, children);
}

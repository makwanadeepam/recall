import { clear, el } from "./dom";
import { icon } from "./icons";

export interface TagInputHandle {
  element: HTMLElement;
  getTags(): string[];
}

/** A small chip-based tag editor. Enter or comma adds a tag; Backspace on an
 *  empty field removes the last; each chip has a remove button. */
export function createTagInput(initial: string[] = []): TagInputHandle {
  let tags = dedupe(initial);
  const wrap = el("div", { class: "tag-input" });
  const field = el("input", {
    class: "tag-input-field",
    type: "text",
    placeholder: "Add tag…",
    attrs: { "aria-label": "Add tag" },
  });

  const addFromField = () => {
    const raw = field.value.replace(/,+$/, "").trim();
    if (raw && !tags.some((t) => t.toLowerCase() === raw.toLowerCase())) {
      tags.push(raw);
    }
    field.value = "";
    render(true);
  };

  field.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      addFromField();
    } else if (e.key === "Backspace" && field.value === "" && tags.length) {
      tags.pop();
      render(true);
    }
  });
  field.addEventListener("blur", addFromField);

  function render(focus = false): void {
    clear(wrap);
    for (const t of tags) {
      wrap.appendChild(
        el(
          "span",
          { class: "chip removable" },
          icon("tag", "chip-icon"),
          el("span", { text: t }),
          el(
            "button",
            {
              class: "chip-x",
              attrs: { type: "button", "aria-label": `Remove ${t}` },
              on: {
                click: () => {
                  tags = tags.filter((x) => x !== t);
                  render(true);
                },
              },
            },
            "×"
          )
        )
      );
    }
    wrap.appendChild(field);
    if (focus) field.focus();
  }

  render();
  return { element: wrap, getTags: () => [...tags] };
}

function dedupe(list: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const t of list) {
    const v = t.trim();
    if (v && !seen.has(v.toLowerCase())) {
      seen.add(v.toLowerCase());
      out.push(v);
    }
  }
  return out;
}

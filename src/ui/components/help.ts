import { el } from "./dom";
import { openModal } from "./modal";

const SHORTCUTS: [string, string][] = [
  ["N", "New knowledge item"],
  ["R", "Start a recall session"],
  ["/", "Focus search (in Library)"],
  ["Space", "Reveal answer (in a session)"],
  ["Enter", "Submit answer (in a session)"],
  ["1 / 2 / 3 / 4", "Rate: Again / Hard / Good / Easy"],
  ["Esc", "Close dialog / end session"],
  ["?", "Show this help"],
];

export function showShortcutsHelp(): void {
  const list = el(
    "dl",
    { class: "shortcuts" },
    ...SHORTCUTS.flatMap(([key, desc]) => [
      el("dt", {}, el("kbd", { text: key })),
      el("dd", { text: desc }),
    ])
  );
  const body = el("div", { class: "modal-body" }, list);
  openModal({ title: "Keyboard shortcuts", body });
}

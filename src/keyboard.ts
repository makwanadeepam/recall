/** Global keyboard shortcuts. Session-specific keys (Space, Enter, 1–4) are
 *  handled inside the recall page while a session is active. */

import { navigate } from "./ui/components/router";
import { anyModalOpen, closeTopModal } from "./ui/components/modal";
import { showShortcutsHelp } from "./ui/components/help";

function isTypingTarget(target: EventTarget | null): boolean {
  const elm = target as HTMLElement | null;
  if (!elm) return false;
  const tag = elm.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || elm.isContentEditable;
}

export function installGlobalKeyboard(): void {
  window.addEventListener("keydown", (e) => {
    // Escape always tries to close a dialog first.
    if (e.key === "Escape") {
      if (closeTopModal()) e.preventDefault();
      return;
    }

    // Don't hijack keys while a modal is open or the user is typing.
    if (anyModalOpen() || isTypingTarget(e.target)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;

    switch (e.key) {
      case "n":
      case "N":
        e.preventDefault();
        navigate("knowledge", { id: "new" });
        break;
      case "r":
      case "R":
        e.preventDefault();
        navigate("recall");
        break;
      case "/":
        e.preventDefault();
        navigate("library").then(() => {
          document.getElementById("library-search")?.focus();
        });
        break;
      case "?":
        e.preventDefault();
        showShortcutsHelp();
        break;
    }
  });
}

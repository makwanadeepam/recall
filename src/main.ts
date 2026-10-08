import "./ui/styles/global.css";

import { api } from "./api";
import { installGlobalKeyboard } from "./keyboard";
import { buildShell, navigate, registerPage } from "./ui/components/router";
import { applyThemeToDom } from "./ui/state/theme";
import { renderDashboard } from "./ui/pages/dashboard";
import { renderLibrary } from "./ui/pages/library";
import { renderKnowledge } from "./ui/pages/knowledge";
import { renderRecall } from "./ui/pages/recall";
import { renderChat } from "./ui/pages/chat";
import { renderSettings } from "./ui/pages/settings";
import { maybeSeed } from "./seed";

async function boot(): Promise<void> {
  const root = document.getElementById("app");
  if (!root) return;

  // Apply the saved theme as early as possible to avoid a flash.
  try {
    const settings = await api.getSettings();
    applyThemeToDom(settings.theme);
  } catch {
    applyThemeToDom("system");
  }

  buildShell(root);
  registerPage("dashboard", renderDashboard);
  registerPage("library", renderLibrary);
  registerPage("knowledge", renderKnowledge);
  registerPage("recall", renderRecall);
  registerPage("chat", renderChat);
  registerPage("settings", renderSettings);

  installGlobalKeyboard();

  // First-run demo content (no-op afterwards).
  await maybeSeed();

  await navigate("dashboard");
  root.removeAttribute("aria-busy");
}

boot();

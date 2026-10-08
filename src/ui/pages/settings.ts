import { api } from "../../api";
import { errorHint, errorMessage } from "../../api/tauri";
import type { AiConfigView, AppSettings, ThemePref } from "../../db/models";
import { el } from "../components/dom";
import { icon } from "../components/icons";
import { confirm } from "../components/modal";
import { navigate } from "../components/router";
import { showToast, toastError } from "../components/toast";
import { button, emptyState, spinner } from "../components/widgets";
import { setTheme } from "../state/theme";

const MODEL_PLACEHOLDER: Record<string, string> = {
  openai: "gpt-4o-mini",
  anthropic: "claude-3-5-haiku-latest",
  gemini: "gemini-2.0-flash",
  openrouter: "openai/gpt-4o-mini",
  ollama: "llama3.2",
  custom: "your-model-name",
};

export async function renderSettings(container: HTMLElement): Promise<void> {
  container.appendChild(spinner());
  let settings: AppSettings;
  try {
    settings = await api.getSettings();
  } catch (e) {
    container.replaceChildren(emptyState("info", "Couldn't load settings", errorMessage(e)));
    return;
  }

  const header = el(
    "header",
    { class: "page-header" },
    el(
      "div",
      { class: "page-header-text" },
      el("h1", { class: "page-title" }, icon("settings", "title-icon"), el("span", { text: "Settings" }))
    )
  );

  container.replaceChildren(
    el(
      "div",
      { class: "page settings" },
      header,
      appearancePanel(settings.theme),
      aiPanel(settings.ai),
      dataPanel(),
      aboutPanel()
    )
  );
}

function appearancePanel(theme: ThemePref): HTMLElement {
  const options: ThemePref[] = ["light", "dark", "system"];
  const seg = el("div", { class: "segmented" });
  const buttons = new Map<ThemePref, HTMLElement>();
  options.forEach((opt) => {
    const b = el(
      "button",
      {
        class: "segment" + (opt === theme ? " active" : ""),
        on: {
          click: () => {
            setTheme(opt);
            for (const [o, btn] of buttons) btn.classList.toggle("active", o === opt);
          },
        },
      },
      el("span", { text: opt[0].toUpperCase() + opt.slice(1) })
    );
    buttons.set(opt, b);
    seg.appendChild(b);
  });

  return panel("Appearance", [field("Theme", seg)]);
}

function aiPanel(ai: AiConfigView): HTMLElement {
  let currentProvider = ai.provider || "openai";
  let hasKey = ai.has_key;

  const enabledToggle = toggle(ai.enabled);
  const confirmToggle = toggle(ai.confirm_before_send);

  const providerSelect = el(
    "select",
    { class: "select" },
    ...ai.providers.map((p) =>
      el("option", { value: p, text: providerLabel(p), attrs: { selected: p === currentProvider } })
    )
  ) as HTMLSelectElement;

  const baseUrlInput = el("input", {
    class: "input",
    type: "text",
    value: ai.base_url,
    placeholder: ai.default_base_url || "https://…",
  }) as HTMLInputElement;

  const modelInput = el("input", {
    class: "input",
    type: "text",
    value: ai.model,
    placeholder: MODEL_PLACEHOLDER[currentProvider] ?? "model name",
  }) as HTMLInputElement;

  const keyInput = el("input", {
    class: "input",
    type: "password",
    placeholder: hasKey ? "•••••••• (a key is saved)" : "Paste API key",
    attrs: { autocomplete: "off", spellcheck: "false" },
  }) as HTMLInputElement;

  const keyStatus = el("span", { class: "key-status muted" });
  const updateKeyStatus = () => {
    keyStatus.replaceChildren(
      icon("lock", "inline-icon"),
      el("span", {
        text: hasKey ? "Stored securely in your OS keychain" : "No key stored",
      })
    );
    keyInput.placeholder = hasKey ? "•••••••• (a key is saved)" : "Paste API key";
  };
  updateKeyStatus();

  const privacyNote = el("p", { class: "privacy-note" });
  const updatePrivacy = () => {
    privacyNote.replaceChildren(
      icon(ai.is_local || currentProvider === "ollama" ? "lock" : "info", "inline-icon"),
      el("span", {
        text:
          currentProvider === "ollama" || isLocalUrl(baseUrlInput.value || ai.default_base_url)
            ? "Local provider: your content stays on this device."
            : `Content you generate/evaluate is sent to ${providerLabel(currentProvider)} only when you use an AI action.`,
      })
    );
  };

  providerSelect.addEventListener("change", async () => {
    currentProvider = providerSelect.value;
    baseUrlInput.value = "";
    baseUrlInput.placeholder = defaultBaseFor(currentProvider);
    modelInput.placeholder = MODEL_PLACEHOLDER[currentProvider] ?? "model name";
    try {
      hasKey = await api.aiHasKey(currentProvider);
    } catch {
      hasKey = false;
    }
    updateKeyStatus();
    updatePrivacy();
  });
  baseUrlInput.addEventListener("input", updatePrivacy);
  updatePrivacy();

  const saveKeyBtn = button("Save key", {
    onClick: async () => {
      const key = keyInput.value.trim();
      if (!key) {
        showToast("Enter a key first.", "error");
        return;
      }
      try {
        await api.aiSetKey(currentProvider, key);
        hasKey = true;
        keyInput.value = "";
        updateKeyStatus();
        showToast("API key saved to your keychain.", "success");
      } catch (e) {
        toastError(errorMessage(e), errorHint(e));
      }
    },
  });

  const removeKeyBtn = button("Remove", {
    variant: "ghost",
    onClick: async () => {
      const ok = await confirm({
        title: "Remove API key?",
        message: `Delete the stored ${providerLabel(currentProvider)} key from your keychain?`,
        confirmLabel: "Remove",
        danger: true,
      });
      if (!ok) return;
      try {
        await api.aiDeleteKey(currentProvider);
        hasKey = false;
        updateKeyStatus();
        showToast("API key removed.", "success");
      } catch (e) {
        toastError(errorMessage(e), errorHint(e));
      }
    },
  });

  const saveBtn = button("Save AI settings", {
    variant: "primary",
    icon: "check",
    onClick: async () => {
      try {
        const updated = await api.setAiConfig({
          provider: currentProvider,
          base_url: baseUrlInput.value.trim(),
          model: modelInput.value.trim(),
          enabled: (enabledToggle.querySelector("input") as HTMLInputElement).checked,
          confirm_before_send: (confirmToggle.querySelector("input") as HTMLInputElement).checked,
        });
        showToast("AI settings saved.", "success");
        updatePrivacyFrom(updated);
      } catch (e) {
        toastError(errorMessage(e), errorHint(e));
      }
    },
  });

  const testBtn = button("Test connection", {
    icon: "sparkles",
    onClick: async () => {
      const span = testBtn.querySelector("span");
      const label = span?.textContent ?? "";
      testBtn.disabled = true;
      if (span) span.textContent = "Testing…";
      try {
        // Save current config first so the test uses what's on screen.
        await api.setAiConfig({
          provider: currentProvider,
          base_url: baseUrlInput.value.trim(),
          model: modelInput.value.trim(),
          enabled: (enabledToggle.querySelector("input") as HTMLInputElement).checked,
          confirm_before_send: (confirmToggle.querySelector("input") as HTMLInputElement).checked,
        });
        await api.aiTestConnection();
        showToast("Connection OK — the provider responded.", "success");
      } catch (e) {
        toastError(errorMessage(e), errorHint(e));
      } finally {
        testBtn.disabled = false;
        if (span) span.textContent = label;
      }
    },
  });

  function updatePrivacyFrom(v: AiConfigView) {
    ai.is_local = v.is_local;
    updatePrivacy();
  }

  return panel(
    "AI provider",
    [
      field("Enable AI features", enabledToggle, "The app works fully without this. AI only assists."),
      field("Provider", providerSelect),
      field("Base URL", baseUrlInput, "Leave blank to use the provider default."),
      field("Model", modelInput),
      field(
        "API key",
        el("div", { class: "key-row" }, keyInput, saveKeyBtn, removeKeyBtn),
        undefined,
        keyStatus
      ),
      field("Confirm before sending content", confirmToggle, "Ask before any content leaves this device."),
      privacyNote,
      el("div", { class: "panel-actions end" }, testBtn, saveBtn),
    ],
    "sparkles"
  );
}

function dataPanel(): HTMLElement {
  const doExport = async (format: "json" | "markdown") => {
    try {
      const res = await api.exportData(format);
      if (res) showToast(`Exported ${res.items} items, ${res.cards} cards to ${res.path}`, "success", 6000);
    } catch (e) {
      toastError(errorMessage(e), errorHint(e));
    }
  };
  const doImport = async () => {
    try {
      const res = await api.importData("auto");
      if (res) {
        showToast(`Imported ${res.items} items, ${res.cards} cards, ${res.reviews} reviews.`, "success", 6000);
      }
    } catch (e) {
      toastError(errorMessage(e), errorHint(e));
    }
  };

  return panel(
    "Your data",
    [
      el("p", { class: "muted", text: "Everything lives in a local SQLite database. Back it up or move it anytime." }),
      field(
        "Export",
        el(
          "div",
          { class: "btn-row" },
          button("Export JSON", { icon: "download", onClick: () => doExport("json") }),
          button("Export Markdown", { icon: "download", onClick: () => doExport("markdown") })
        ),
        "JSON is a complete, versioned backup. Markdown is human-readable."
      ),
      field(
        "Import",
        el("div", { class: "btn-row" }, button("Import file…", { icon: "upload", onClick: doImport })),
        "Supports Recall JSON, Markdown, or plain text (detected by extension)."
      ),
    ],
    "layers"
  );
}

function aboutPanel(): HTMLElement {
  return panel(
    "About",
    [
      el("p", { text: "Recall 0.1.0" }),
      el("p", {
        class: "muted",
        text: "Lightweight, local-first active recall. AI generates and assists; the app manages memory.",
      }),
      el(
        "div",
        { class: "btn-row" },
        button("Open dashboard", { variant: "ghost", onClick: () => navigate("dashboard") })
      ),
    ],
    "info"
  );
}

// ---- small helpers ----

function panel(title: string, children: (HTMLElement | null)[], iconName?: string): HTMLElement {
  return el(
    "section",
    { class: "panel settings-panel" },
    el("h2", { class: "panel-title" }, iconName ? icon(iconName, "panel-icon") : null, el("span", { text: title })),
    el("div", { class: "panel-body" }, ...children.filter(Boolean))
  );
}

function field(label: string, control: HTMLElement, hint?: string, extra?: HTMLElement): HTMLElement {
  return el(
    "div",
    { class: "setting-field" },
    el("label", { class: "field-label", text: label }),
    control,
    hint ? el("p", { class: "field-help muted", text: hint }) : null,
    extra ?? null
  );
}

function toggle(checked: boolean): HTMLElement {
  const input = el("input", { type: "checkbox", checked }) as HTMLInputElement;
  return el("label", { class: "switch" }, input, el("span", { class: "switch-slider" }));
}

function providerLabel(p: string): string {
  const labels: Record<string, string> = {
    openai: "OpenAI",
    anthropic: "Anthropic",
    gemini: "Google Gemini",
    openrouter: "OpenRouter",
    ollama: "Ollama (local)",
    custom: "Custom (OpenAI-compatible)",
  };
  return labels[p] ?? p;
}

function defaultBaseFor(p: string): string {
  const defaults: Record<string, string> = {
    openai: "https://api.openai.com/v1",
    anthropic: "https://api.anthropic.com",
    gemini: "https://generativelanguage.googleapis.com",
    openrouter: "https://openrouter.ai/api/v1",
    ollama: "http://localhost:11434/v1",
    custom: "https://your-endpoint/v1",
  };
  return defaults[p] ?? "https://…";
}

function isLocalUrl(url: string): boolean {
  const u = url.toLowerCase();
  return ["localhost", "127.0.0.1", "0.0.0.0", "::1"].some((h) => u.includes(h));
}

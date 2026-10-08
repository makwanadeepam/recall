import { api } from "../../api";
import { errorHint, errorMessage } from "../../api/tauri";
import type { AiConfigView, ChatTurn, KbSource } from "../../db/models";
import { el } from "../components/dom";
import { icon } from "../components/icons";
import { confirm } from "../components/modal";
import { navigate } from "../components/router";
import { toastError } from "../components/toast";
import { button, emptyState, spinner } from "../components/widgets";

const PROVIDER_LABELS: Record<string, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  gemini: "Google Gemini",
  openrouter: "OpenRouter",
  ollama: "Ollama",
  custom: "your custom provider",
};

export async function renderChat(container: HTMLElement): Promise<void> {
  container.appendChild(spinner());
  let ai: AiConfigView;
  try {
    ai = await api.getAiConfig();
  } catch (e) {
    container.replaceChildren(emptyState("info", "Couldn't open chat", errorMessage(e)));
    return;
  }

  const header = el(
    "header",
    { class: "page-header" },
    el(
      "div",
      { class: "page-header-text" },
      el("h1", { class: "page-title" }, icon("chat", "title-icon"), el("span", { text: "Chat" })),
      el("p", { class: "page-subtitle", text: "Ask questions and get answers grounded in your notes." })
    )
  );

  if (!ai.enabled || !ai.provider) {
    container.replaceChildren(
      el(
        "div",
        { class: "page chat" },
        header,
        emptyState(
          "sparkles",
          "AI isn't set up yet",
          "Enable an AI provider in Settings to chat with your knowledge base. Everything else in Recall works without it.",
          button("Open Settings", { variant: "primary", onClick: () => navigate("settings") })
        )
      )
    );
    return;
  }

  const label = PROVIDER_LABELS[ai.provider] ?? ai.provider;

  const banner = el(
    "div",
    { class: "chat-banner" + (ai.is_local ? " local" : "") },
    icon(ai.is_local ? "lock" : "info", "inline-icon"),
    el("span", {
      text: ai.is_local
        ? `Using a local model (${ai.effective_base_url}). Your notes never leave this device.`
        : `Your question and the matching notes are sent to ${label} to answer.`,
    })
  );

  const log = el("div", { class: "chat-log" });
  log.appendChild(
    assistantMessage(
      "Ask me anything about your notes — I'll answer from your knowledge base and cite the notes I used.",
      []
    )
  );

  const input = el("textarea", {
    class: "textarea chat-input",
    rows: 1,
    placeholder: "Ask about your notes…  (Enter to send, Shift+Enter for a new line)",
  }) as HTMLTextAreaElement;

  const sendBtn = button("Send", { variant: "primary", icon: "send", onClick: () => send() });
  const inputBar = el("div", { class: "chat-input-bar" }, input, sendBtn);

  container.replaceChildren(el("div", { class: "page chat" }, header, banner, log, inputBar));

  const messages: ChatTurn[] = [];
  let sending = false;
  let confirmed = false;

  const autosize = () => {
    input.style.height = "auto";
    input.style.height = Math.min(input.scrollHeight, 160) + "px";
  };
  input.addEventListener("input", autosize);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  });
  input.focus();

  function scrollToBottom(): void {
    container.scrollTop = container.scrollHeight;
  }

  async function send(): Promise<void> {
    const question = input.value.trim();
    if (!question || sending) return;

    // Privacy: confirm once per chat session before content leaves the device.
    if (ai.confirm_before_send && !ai.is_local && !confirmed) {
      const ok = await confirm({
        title: `Send to ${label}?`,
        message: `Your questions and the notes that match them will be sent to ${label}. This applies to the rest of this chat.`,
        confirmLabel: "Send",
      });
      if (!ok) return;
      confirmed = true;
    }

    input.value = "";
    autosize();
    log.appendChild(userMessage(question));

    const priorHistory = messages.slice();
    messages.push({ role: "user", content: question });
    scrollToBottom();

    const pending = pendingMessage();
    log.appendChild(pending.node);
    scrollToBottom();

    sending = true;
    sendBtn.disabled = true;
    try {
      const res = await api.aiKbChat(question, priorHistory);
      pending.resolve(res.answer, res.sources);
      messages.push({ role: "assistant", content: res.answer });
    } catch (e) {
      pending.reject(errorMessage(e), errorHint(e));
      toastError(errorMessage(e), errorHint(e));
    } finally {
      sending = false;
      sendBtn.disabled = false;
      scrollToBottom();
      input.focus();
    }
  }
}

// ---- message rendering ----

function userMessage(text: string): HTMLElement {
  return el("div", { class: "chat-msg user" }, el("div", { class: "bubble", text }));
}

function assistantMessage(text: string, sources: KbSource[]): HTMLElement {
  const msg = el("div", { class: "chat-msg assistant" }, el("div", { class: "bubble", text }));
  if (sources.length) msg.appendChild(sourcesRow(sources));
  return msg;
}

interface Pending {
  node: HTMLElement;
  resolve(answer: string, sources: KbSource[]): void;
  reject(message: string, hint?: string): void;
}

function pendingMessage(): Pending {
  const bubble = el(
    "div",
    { class: "bubble" },
    el("span", { class: "typing" }, el("i", {}), el("i", {}), el("i", {}))
  );
  const msg = el("div", { class: "chat-msg assistant" }, bubble);
  return {
    node: msg,
    resolve(answer, sources) {
      bubble.classList.remove("pending");
      bubble.textContent = answer;
      if (sources.length) msg.appendChild(sourcesRow(sources));
    },
    reject(message, hint) {
      bubble.classList.add("error");
      bubble.textContent = hint ? `${message}\n${hint}` : message;
    },
  };
}

function sourcesRow(sources: KbSource[]): HTMLElement {
  const row = el("div", { class: "chat-sources" }, el("span", { class: "sources-label muted", text: "Sources" }));
  for (const s of sources) {
    row.appendChild(
      el(
        "button",
        { class: "chip chip-button tiny", title: "Open note", on: { click: () => navigate("knowledge", { id: s.id }) } },
        icon("library", "chip-icon"),
        el("span", { text: s.title })
      )
    );
  }
  return row;
}

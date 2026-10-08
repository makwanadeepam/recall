import { api } from "../../api";
import { errorHint, errorMessage } from "../../api/tauri";
import {
  CARD_TYPE_LABELS,
  type AiConfigView,
  type Card,
  type KnowledgeItem,
  type KnowledgeItemInput,
} from "../../db/models";
import { clear, el } from "../components/dom";
import { icon } from "../components/icons";
import { confirm } from "../components/modal";
import { navigate, refresh } from "../components/router";
import { showToast, toastError } from "../components/toast";
import { createTagInput, type TagInputHandle } from "../components/tag-input";
import { openCardEditor } from "../components/card-editor";
import { openGenerateReview } from "../components/ai-generate";
import { button, emptyState, spinner } from "../components/widgets";
import { dueLabel } from "../format";

export async function renderKnowledge(container: HTMLElement, params: Record<string, unknown>): Promise<void> {
  const idParam = params.id;
  if (idParam === "new") {
    renderEditorPage(container, null);
    return;
  }
  const id = Number(idParam);
  container.appendChild(spinner());
  try {
    const [item, cards, aiConfig] = await Promise.all([api.getItem(id), api.listCards(id), api.getAiConfig()]);
    renderDetail(container, item, cards, aiConfig);
  } catch (e) {
    container.replaceChildren(
      emptyState("info", "Couldn't open this item", errorMessage(e), button("Back to library", { onClick: () => navigate("library") }))
    );
  }
}

// ---------------------------------------------------------------------------
// Item editor (shared by "new" and the top of the detail page)
// ---------------------------------------------------------------------------

interface EditorHandle {
  element: HTMLElement;
  values(): KnowledgeItemInput;
  focus(): void;
}

function itemEditor(item: KnowledgeItem | null): EditorHandle {
  const titleInput = el("input", {
    class: "input title-input",
    type: "text",
    placeholder: "Title (e.g. TCP Three-Way Handshake)",
    value: item?.title ?? "",
  }) as HTMLInputElement;

  const contentInput = el("textarea", {
    class: "textarea content-input",
    rows: 12,
    placeholder: "Paste or write the passage / notes you want to remember…",
    value: item?.content ?? "",
  }) as HTMLTextAreaElement;

  const tags: TagInputHandle = createTagInput(item?.tags ?? []);

  const element = el(
    "div",
    { class: "item-editor" },
    titleInput,
    el("div", { class: "field" }, el("label", { class: "field-label", text: "Content" }), contentInput),
    el("div", { class: "field" }, el("label", { class: "field-label", text: "Tags" }), tags.element)
  );

  return {
    element,
    values: () => ({ title: titleInput.value.trim(), content: contentInput.value, tags: tags.getTags() }),
    focus: () => titleInput.focus(),
  };
}

function renderEditorPage(container: HTMLElement, item: KnowledgeItem | null): void {
  const editor = itemEditor(item);

  const save = async () => {
    const values = editor.values();
    if (!values.title) {
      showToast("Please add a title.", "error");
      editor.focus();
      return;
    }
    try {
      const created = await api.createItem(values);
      showToast("Item created.", "success");
      navigate("knowledge", { id: created.id });
    } catch (e) {
      toastError(errorMessage(e), errorHint(e));
    }
  };

  const header = el(
    "header",
    { class: "page-header" },
    el(
      "div",
      { class: "page-header-text" },
      el("h1", { class: "page-title", text: "New knowledge item" })
    ),
    el(
      "div",
      { class: "page-actions" },
      button("Cancel", { onClick: () => navigate("library") }),
      button("Save", { variant: "primary", icon: "check", onClick: save })
    )
  );

  container.replaceChildren(el("div", { class: "page knowledge" }, header, el("div", { class: "panel pad" }, editor.element)));
  editor.focus();
}

// ---------------------------------------------------------------------------
// Detail page (existing item)
// ---------------------------------------------------------------------------

function renderDetail(container: HTMLElement, item: KnowledgeItem, cards: Card[], aiConfig: AiConfigView): void {
  const editor = itemEditor(item);

  const saveItem = async () => {
    const values = editor.values();
    if (!values.title) {
      showToast("Please add a title.", "error");
      editor.focus();
      return;
    }
    try {
      await api.updateItem(item.id, values);
      showToast("Saved.", "success");
      refresh();
    } catch (e) {
      toastError(errorMessage(e), errorHint(e));
    }
  };

  const deleteItem = async () => {
    const ok = await confirm({
      title: "Delete this item?",
      message: `"${item.title}" and its ${cards.length} card${cards.length === 1 ? "" : "s"} will be permanently deleted.`,
      confirmLabel: "Delete",
      danger: true,
    });
    if (!ok) return;
    try {
      await api.deleteItem(item.id);
      showToast("Item deleted.", "success");
      navigate("library");
    } catch (e) {
      toastError(errorMessage(e), errorHint(e));
    }
  };

  const header = el(
    "header",
    { class: "page-header" },
    el(
      "div",
      { class: "page-header-text" },
      el("button", { class: "back-link", on: { click: () => navigate("library") } }, "← Library"),
      el("h1", { class: "page-title", text: item.title })
    ),
    el(
      "div",
      { class: "page-actions" },
      button("Delete", { variant: "danger", icon: "trash", onClick: deleteItem }),
      button("Save", { variant: "primary", icon: "check", onClick: saveItem })
    )
  );

  const cardsPanel = buildCardsPanel(item, cards, aiConfig, editor);

  container.replaceChildren(
    el("div", { class: "page knowledge" }, header, el("div", { class: "panel pad" }, editor.element), cardsPanel)
  );
}

function buildCardsPanel(
  item: KnowledgeItem,
  cards: Card[],
  aiConfig: AiConfigView,
  editor: EditorHandle
): HTMLElement {
  const listEl = el("div", { class: "card-list" });

  const reload = async () => {
    try {
      const fresh = await api.listCards(item.id);
      renderCardList(listEl, fresh, item, reload);
      countLabel.textContent = `${fresh.length} card${fresh.length === 1 ? "" : "s"}`;
    } catch (e) {
      toastError(errorMessage(e));
    }
  };

  const addCard = async () => {
    const saved = await openCardEditor(item.id, null);
    if (saved) reload();
  };

  const generate = async () => {
    const content = editor.values().content ?? "";
    if (!content.trim()) {
      showToast("Add some content above first, then generate questions from it.", "error");
      return;
    }
    if (!aiConfig.enabled || !aiConfig.provider) {
      const goSettings = await confirm({
        title: "AI isn't set up",
        message: "Configure and enable an AI provider in Settings to generate questions. Open Settings now?",
        confirmLabel: "Open Settings",
      });
      if (goSettings) navigate("settings");
      return;
    }
    const proceed = await confirmSend(aiConfig);
    if (!proceed) return;

    const genBtnLabel = genBtn.querySelector("span");
    const original = genBtnLabel?.textContent ?? "";
    genBtn.disabled = true;
    if (genBtnLabel) genBtnLabel.textContent = "Generating…";
    try {
      const questions = await api.aiGenerateQuestions(content, 8);
      const saved = await openGenerateReview(item.id, questions);
      if (saved > 0) reload();
    } catch (e) {
      toastError(errorMessage(e), errorHint(e));
    } finally {
      genBtn.disabled = false;
      if (genBtnLabel) genBtnLabel.textContent = original;
    }
  };

  const countLabel = el("span", { class: "count-badge", text: `${cards.length} card${cards.length === 1 ? "" : "s"}` });
  const genBtn = button("Generate with AI", { icon: "sparkles", onClick: generate });

  const panelHeader = el(
    "div",
    { class: "panel-header" },
    el("div", { class: "panel-title-row" }, el("h2", { class: "panel-title", text: "Cards" }), countLabel),
    el(
      "div",
      { class: "panel-actions" },
      button("Start recall", { icon: "play", disabled: cards.length === 0, onClick: () => navigate("recall", { item_id: item.id }) }),
      genBtn,
      button("Add card", { variant: "primary", icon: "plus", onClick: addCard })
    )
  );

  renderCardList(listEl, cards, item, reload);

  return el("section", { class: "panel cards-panel" }, panelHeader, listEl);
}

function renderCardList(listEl: HTMLElement, cards: Card[], item: KnowledgeItem, reload: () => void): void {
  clear(listEl);
  if (cards.length === 0) {
    listEl.appendChild(
      el("p", { class: "muted pad", text: "No cards yet. Add one manually, or generate a set with AI." })
    );
    return;
  }
  for (const card of cards) listEl.appendChild(cardRow(card, item, reload));
}

function cardRow(card: Card, item: KnowledgeItem, reload: () => void): HTMLElement {
  const edit = async () => {
    const saved = await openCardEditor(item.id, card);
    if (saved) reload();
  };
  const del = async () => {
    const ok = await confirm({
      title: "Delete card?",
      message: "This card and its review history will be permanently deleted.",
      confirmLabel: "Delete",
      danger: true,
    });
    if (!ok) return;
    try {
      await api.deleteCard(card.id);
      showToast("Card deleted.", "success");
      reload();
    } catch (e) {
      toastError(errorMessage(e));
    }
  };

  const meta =
    card.review_count === 0
      ? "new"
      : `${dueLabel(card.due_at)} · ${card.review_count} review${card.review_count === 1 ? "" : "s"}`;

  return el(
    "div",
    { class: "card-row" },
    el(
      "button",
      { class: "card-row-main", on: { click: edit } },
      el("span", { class: `type-badge type-${card.type}`, text: CARD_TYPE_LABELS[card.type] }),
      el("span", { class: "card-q", text: card.question })
    ),
    el("span", { class: "card-row-meta muted", text: meta }),
    el(
      "div",
      { class: "card-row-actions" },
      el("button", { class: "icon-btn", title: "Edit", on: { click: edit } }, icon("edit")),
      el("button", { class: "icon-btn danger", title: "Delete", on: { click: del } }, icon("trash"))
    )
  );
}

/** Privacy confirmation before sending content to an AI provider. */
function confirmSend(aiConfig: AiConfigView): Promise<boolean> {
  if (!aiConfig.confirm_before_send) return Promise.resolve(true);
  if (aiConfig.is_local) {
    return confirm({
      title: "Use local model",
      message: `Your content will be sent to your local model at ${aiConfig.effective_base_url} and will not leave this device.`,
      confirmLabel: "Continue",
    });
  }
  return confirm({
    title: `Send content to ${aiConfig.provider}?`,
    message: `This content will be sent to ${aiConfig.provider} (${aiConfig.effective_base_url}) to generate questions.`,
    confirmLabel: "Send",
  });
}

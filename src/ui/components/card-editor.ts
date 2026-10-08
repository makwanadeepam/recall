import { api } from "../../api";
import { errorHint, errorMessage } from "../../api/tauri";
import { CARD_TYPE_LABELS, type Card, type CardInput, type CardType, type Difficulty } from "../../db/models";
import { el } from "./dom";
import { openModal } from "./modal";
import { toastError, showToast } from "./toast";
import { button } from "./widgets";

const TYPE_HELP: Record<CardType, string> = {
  basic: "Front/back flashcard. You'll see the question, then reveal the answer.",
  free: "You type your answer from memory, then compare with the expected answer.",
  cloze: "Wrap hidden text in {{double braces}}, e.g. TCP uses {{SYN}} then {{ACK}}.",
  mcq: "One correct answer plus a few plausible distractors (one per line).",
  reverse: "In review you'll see the answer and recall the concept (question).",
};

const ANSWER_LABEL: Record<CardType, string> = {
  basic: "Answer",
  free: "Expected answer",
  cloze: "Answer (optional — derived from the braces)",
  mcq: "Correct answer",
  reverse: "Answer / definition",
};

/** Open the create/edit card modal. Resolves true if a card was saved. */
export function openCardEditor(itemId: number, card: Card | null): Promise<boolean> {
  return new Promise((resolve) => {
    let saved = false;
    const initialType = card?.type ?? "basic";

    const typeSelect = el(
      "select",
      { class: "select" },
      ...(Object.keys(CARD_TYPE_LABELS) as CardType[]).map((t) =>
        el("option", { value: t, text: CARD_TYPE_LABELS[t], attrs: { selected: t === initialType } })
      )
    ) as HTMLSelectElement;

    const difficultySelect = el(
      "select",
      { class: "select" },
      ...(["easy", "medium", "hard"] as Difficulty[]).map((d) =>
        el("option", { value: d, text: d[0].toUpperCase() + d.slice(1), attrs: { selected: d === (card?.difficulty ?? "medium") } })
      )
    ) as HTMLSelectElement;

    const questionField = el("textarea", { class: "textarea", rows: 3, value: card?.question ?? "" }) as HTMLTextAreaElement;
    const answerField = el("textarea", { class: "textarea", rows: 3, value: card?.answer ?? "" }) as HTMLTextAreaElement;
    const distractorsField = el("textarea", {
      class: "textarea",
      rows: 3,
      placeholder: "One distractor per line",
      value: (card?.metadata?.distractors ?? []).join("\n"),
    }) as HTMLTextAreaElement;

    const helpLine = el("p", { class: "field-help muted", text: TYPE_HELP[initialType] });
    const answerLabel = el("label", { class: "field-label", text: ANSWER_LABEL[initialType] });
    const distractorsBlock = el(
      "div",
      { class: "field" },
      el("label", { class: "field-label", text: "Distractors" }),
      distractorsField
    );

    const questionLabel = el("label", { class: "field-label", text: initialType === "cloze" ? "Cloze text" : "Question" });

    function syncType(): void {
      const t = typeSelect.value as CardType;
      helpLine.textContent = TYPE_HELP[t];
      answerLabel.textContent = ANSWER_LABEL[t];
      questionLabel.textContent = t === "cloze" ? "Cloze text" : "Question";
      distractorsBlock.style.display = t === "mcq" ? "" : "none";
    }
    typeSelect.addEventListener("change", syncType);

    const body = el(
      "div",
      { class: "modal-body card-editor" },
      el(
        "div",
        { class: "field-row" },
        el("div", { class: "field" }, el("label", { class: "field-label", text: "Type" }), typeSelect),
        el("div", { class: "field" }, el("label", { class: "field-label", text: "Difficulty" }), difficultySelect)
      ),
      helpLine,
      el("div", { class: "field" }, questionLabel, questionField),
      el("div", { class: "field" }, answerLabel, answerField),
      distractorsBlock
    );

    const saveBtn = button(card ? "Save changes" : "Add card", {
      variant: "primary",
      onClick: save,
    });
    const footer = el(
      "div",
      { class: "modal-footer" },
      button("Cancel", { onClick: () => handle.close() }),
      saveBtn
    );

    const handle = openModal({
      title: card ? "Edit card" : "New card",
      body,
      footer,
      class: "modal-wide",
      onClose: () => resolve(saved),
    });

    syncType();
    questionField.focus();

    async function save(): Promise<void> {
      const type = typeSelect.value as CardType;
      const question = questionField.value.trim();
      if (!question) {
        showToast("A question is required.", "error");
        questionField.focus();
        return;
      }
      const metadata: Record<string, unknown> = {};
      if (type === "mcq") {
        const distractors = distractorsField.value
          .split("\n")
          .map((s) => s.trim())
          .filter(Boolean);
        metadata.distractors = distractors;
      }
      const input: CardInput = {
        knowledge_item_id: itemId,
        question,
        answer: answerField.value.trim(),
        type,
        difficulty: difficultySelect.value as Difficulty,
        metadata,
      };
      saveBtn.disabled = true;
      try {
        if (card) await api.updateCard(card.id, input);
        else await api.createCard(input);
        saved = true;
        handle.close();
      } catch (e) {
        saveBtn.disabled = false;
        toastError(errorMessage(e), errorHint(e));
      }
    }
  });
}

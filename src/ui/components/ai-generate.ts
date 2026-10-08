import { api } from "../../api";
import { errorHint, errorMessage } from "../../api/tauri";
import type { CardInput, CardType } from "../../db/models";
import type { GeneratedQuestion } from "../../ai/types";
import { el } from "./dom";
import { openModal } from "./modal";
import { showToast, toastError } from "./toast";
import { button } from "./widgets";

interface Row {
  include: boolean;
  type: CardType;
  question: HTMLTextAreaElement;
  answer: HTMLTextAreaElement;
  distractors: HTMLTextAreaElement;
  checkbox: HTMLInputElement;
}

/** Review, edit and selectively save AI-generated questions. Resolves with the
 *  number of cards actually saved. Nothing is saved automatically. */
export function openGenerateReview(itemId: number, questions: GeneratedQuestion[]): Promise<number> {
  return new Promise((resolve) => {
    let savedCount = 0;
    const rows: Row[] = [];

    const list = el("div", { class: "gen-list" });
    for (const q of questions) list.appendChild(buildRow(q));

    const intro = el("p", {
      class: "muted",
      text: `${questions.length} question${questions.length === 1 ? "" : "s"} generated. Edit, deselect any you don't want, then save.`,
    });
    const body = el("div", { class: "modal-body" }, intro, list);

    const saveBtn = button("", { variant: "primary", onClick: save });
    const footer = el(
      "div",
      { class: "modal-footer" },
      button("Discard", { onClick: () => handle.close() }),
      saveBtn
    );
    const handle = openModal({
      title: "Review generated questions",
      body,
      footer,
      class: "modal-wide",
      onClose: () => resolve(savedCount),
    });
    updateSaveLabel();

    function buildRow(q: GeneratedQuestion): HTMLElement {
      const type: CardType = q.type === "mcq" || q.type === "cloze" ? q.type : "basic";
      const checkbox = el("input", { type: "checkbox", checked: true }) as HTMLInputElement;
      const question = el("textarea", { class: "textarea", rows: 2, value: q.question }) as HTMLTextAreaElement;
      const answer = el("textarea", { class: "textarea", rows: 2, value: q.answer }) as HTMLTextAreaElement;
      const distractors = el("textarea", {
        class: "textarea",
        rows: 2,
        placeholder: "One distractor per line",
        value: q.distractors.join("\n"),
      }) as HTMLTextAreaElement;

      const typeSelect = el(
        "select",
        { class: "select small" },
        ...(["basic", "cloze", "mcq"] as CardType[]).map((t) =>
          el("option", { value: t, text: t, attrs: { selected: t === type } })
        )
      ) as HTMLSelectElement;

      const row: Row = { include: true, type, question, answer, distractors, checkbox };
      const distractorsBlock = el("div", { class: "gen-distractors" }, el("label", { class: "field-label", text: "Distractors" }), distractors);
      distractorsBlock.style.display = type === "mcq" ? "" : "none";

      typeSelect.addEventListener("change", () => {
        row.type = typeSelect.value as CardType;
        distractorsBlock.style.display = row.type === "mcq" ? "" : "none";
      });
      checkbox.addEventListener("change", () => {
        row.include = checkbox.checked;
        rowEl.classList.toggle("excluded", !checkbox.checked);
        updateSaveLabel();
      });

      rows.push(row);
      const rowEl = el(
        "div",
        { class: "gen-row" },
        el("label", { class: "gen-check" }, checkbox),
        el(
          "div",
          { class: "gen-fields" },
          el("div", { class: "gen-head" }, el("span", { class: "field-label", text: "Type" }), typeSelect),
          el("label", { class: "field-label", text: "Question" }),
          question,
          el("label", { class: "field-label", text: "Answer" }),
          answer,
          distractorsBlock
        )
      );
      return rowEl;
    }

    function selectedCount(): number {
      return rows.filter((r) => r.include && r.question.value.trim()).length;
    }

    function updateSaveLabel(): void {
      const n = selectedCount();
      saveBtn.textContent = n ? `Save ${n} card${n === 1 ? "" : "s"}` : "Save selected";
      saveBtn.disabled = n === 0;
    }

    async function save(): Promise<void> {
      saveBtn.disabled = true;
      let ok = 0;
      try {
        for (const r of rows) {
          if (!r.include) continue;
          const question = r.question.value.trim();
          if (!question) continue;
          const metadata: Record<string, unknown> = {};
          if (r.type === "mcq") {
            metadata.distractors = r.distractors.value.split("\n").map((s) => s.trim()).filter(Boolean);
          }
          const input: CardInput = {
            knowledge_item_id: itemId,
            question,
            answer: r.answer.value.trim(),
            type: r.type,
            difficulty: "medium",
            metadata,
          };
          await api.createCard(input);
          ok++;
        }
        savedCount = ok;
        showToast(`Saved ${ok} card${ok === 1 ? "" : "s"}.`, "success");
        handle.close();
      } catch (e) {
        saveBtn.disabled = false;
        toastError(errorMessage(e), errorHint(e));
      }
    }
  });
}

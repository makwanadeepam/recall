import { api } from "../../api";
import { errorHint, errorMessage } from "../../api/tauri";
import type { AiConfigView, DueCard, Rating } from "../../db/models";
import type { AnswerEvaluation } from "../../ai/types";
import { clear, el, type Child } from "../components/dom";
import { anyModalOpen } from "../components/modal";
import { navigate, onLeave } from "../components/router";
import { toastError } from "../components/toast";
import { button, emptyState, spinner } from "../components/widgets";
import {
  formatInterval,
  previewIntervals,
  schedule,
  type SchedulingState,
} from "../../recall/scheduler";

const RATING_ORDER: Rating[] = ["again", "hard", "good", "easy"];
const RATING_LABEL: Record<Rating, string> = { again: "Again", hard: "Hard", good: "Good", easy: "Easy" };

interface Session {
  queue: DueCard[];
  pos: number;
  reviewed: number;
  again: number;
  total: number;
  ai: AiConfigView;
}

// Per-card ephemeral UI state, reset on each new card.
interface CardView {
  revealed: boolean;
  userAnswer: string;
  selected: string | null;
  evaluation: AnswerEvaluation | null;
  options: string[] | null; // cached shuffled MCQ options (stable per card)
}

export async function renderRecall(container: HTMLElement, params: Record<string, unknown>): Promise<void> {
  container.appendChild(spinner("Building your session…"));

  const itemId = typeof params.item_id === "number" ? params.item_id : null;
  const tag = typeof params.tag === "string" ? params.tag : null;

  let queue: DueCard[];
  let ai: AiConfigView;
  try {
    [queue, ai] = await Promise.all([
      api.dueCards({ item_id: itemId, tag, include_new: true, new_limit: 20, limit: 200 }),
      api.getAiConfig(),
    ]);
  } catch (e) {
    container.replaceChildren(emptyState("info", "Couldn't start a session", errorMessage(e)));
    return;
  }

  if (queue.length === 0) {
    renderCaughtUp(container);
    return;
  }

  const session: Session = { queue: [...queue], pos: 0, reviewed: 0, again: 0, total: queue.length, ai };
  let view: CardView = freshView();

  const stage = el("div", { class: "recall-stage" });
  const progressBar = el("div", { class: "progress-fill" });
  const progressText = el("span", { class: "progress-text muted" });

  const topbar = el(
    "div",
    { class: "recall-topbar" },
    el("button", { class: "back-link", on: { click: endSession } }, "✕ End session"),
    el(
      "div",
      { class: "recall-progress" },
      el("div", { class: "progress-track" }, progressBar),
      progressText
    )
  );

  container.replaceChildren(el("div", { class: "page recall" }, topbar, stage));

  const onKey = (e: KeyboardEvent) => handleKey(e);
  window.addEventListener("keydown", onKey);
  onLeave(() => window.removeEventListener("keydown", onKey));

  showCard();

  // ---- rendering ----

  function freshView(): CardView {
    return { revealed: false, userAnswer: "", selected: null, evaluation: null, options: null };
  }

  function updateProgress(): void {
    const done = session.reviewed;
    const remaining = session.queue.length - session.pos;
    const pct = session.queue.length ? Math.round((session.pos / session.queue.length) * 100) : 0;
    progressBar.style.width = `${pct}%`;
    progressText.textContent = `${done} reviewed · ${remaining} left`;
  }

  function current(): DueCard {
    return session.queue[session.pos];
  }

  function showCard(): void {
    view = freshView();
    updateProgress();
    renderStage();
  }

  function renderStage(): void {
    const card = current();
    const source = el(
      "div",
      { class: "card-source" },
      el("span", { class: `type-badge type-${card.type}`, text: card.type }),
      el("span", { class: "muted", text: `from ${card.knowledge_title}` })
    );

    const prompt = buildPrompt(card);
    const children: Child[] = [source, prompt.node];

    if (!view.revealed) {
      children.push(el("div", { class: "stage-actions" }, prompt.revealButton));
    } else {
      children.push(buildReveal(card));
    }

    clear(stage);
    stage.appendChild(el("div", { class: "card-stage" }, ...children));
  }

  interface Prompt {
    node: HTMLElement;
    revealButton: HTMLElement;
  }

  function buildPrompt(card: DueCard): Prompt {
    // Reverse cards prompt with the answer and ask for the concept.
    if (card.type === "reverse") {
      const node = el(
        "div",
        { class: "card-prompt" },
        el("div", { class: "prompt-hint muted", text: "Recall the concept for:" }),
        el("div", { class: "prompt-text", text: card.answer || "(no answer set)" })
      );
      return { node, revealButton: revealBtn() };
    }

    if (card.type === "cloze") {
      const node = el("div", { class: "card-prompt" }, clozeNodes(card.question, false));
      return { node, revealButton: revealBtn() };
    }

    if (card.type === "mcq") {
      if (!view.options) {
        view.options = shuffle([card.answer, ...(card.metadata?.distractors ?? [])].filter(Boolean));
      }
      const optionList = el("div", { class: "mcq-options" });
      view.options.forEach((opt) => {
        const isCorrect = opt === card.answer;
        const isChosen = opt === view.selected;
        let cls = "mcq-option";
        if (view.revealed) {
          cls += " revealed";
          if (isCorrect) cls += " correct";
          else if (isChosen) cls += " incorrect";
        }
        const mark =
          view.revealed && isCorrect ? "✓" : view.revealed && isChosen ? "✗" : "";
        const btn = el(
          "button",
          {
            class: cls,
            disabled: view.revealed,
            on: view.revealed
              ? undefined
              : {
                  click: () => {
                    view.selected = opt;
                    view.revealed = true;
                    renderStage();
                  },
                },
          },
          el("span", { text: opt }),
          mark ? el("span", { class: "mcq-mark", text: mark }) : null
        );
        optionList.appendChild(btn);
      });
      const node = el(
        "div",
        { class: "card-prompt" },
        el("div", { class: "prompt-text", text: card.question }),
        optionList
      );
      // MCQ reveals by choosing; still offer a way to reveal without answering.
      return { node, revealButton: revealBtn("Reveal without answering") };
    }

    if (card.type === "free") {
      const promptText = el("div", { class: "prompt-text", text: card.question });
      if (view.revealed) {
        // After reveal, the typed answer is echoed in the reveal block below.
        return { node: el("div", { class: "card-prompt" }, promptText), revealButton: el("span") };
      }
      const answer = el("textarea", {
        class: "textarea answer-input",
        rows: 4,
        value: view.userAnswer,
        placeholder: "Type your answer from memory… (Enter to submit, Shift+Enter for a new line)",
      }) as HTMLTextAreaElement;
      answer.addEventListener("keydown", (e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          view.userAnswer = answer.value;
          reveal();
        }
      });
      answer.addEventListener("input", () => {
        view.userAnswer = answer.value;
      });
      const submit = button("Submit (Enter)", { variant: "primary", onClick: () => reveal() });
      const node = el("div", { class: "card-prompt" }, promptText, answer);
      requestAnimationFrame(() => answer.focus());
      return { node, revealButton: submit };
    }

    // basic
    const node = el("div", { class: "card-prompt" }, el("div", { class: "prompt-text", text: card.question }));
    return { node, revealButton: revealBtn() };
  }

  function revealBtn(label = "Reveal answer (Space)"): HTMLElement {
    return button(label, { variant: "primary", icon: "recall", onClick: () => reveal() });
  }

  function reveal(): void {
    if (view.revealed) return;
    view.revealed = true;
    renderStage();
  }

  function buildReveal(card: DueCard): HTMLElement {
    const parts: Child[] = [];

    // Show the learner's own answer (free / mcq) for comparison.
    if (card.type === "free" && view.userAnswer.trim()) {
      parts.push(
        el(
          "div",
          { class: "your-answer" },
          el("div", { class: "answer-label muted", text: "Your answer" }),
          el("div", { class: "answer-content", text: view.userAnswer })
        )
      );
    }
    if (card.type === "mcq" && view.selected) {
      const correct = view.selected === card.answer;
      parts.push(
        el(
          "div",
          { class: "your-answer" },
          el("div", { class: "answer-label muted", text: "You chose" }),
          el("div", { class: `answer-content ${correct ? "correct" : "incorrect"}`, text: view.selected })
        )
      );
    }

    // Expected answer.
    let answerContent: HTMLElement;
    if (card.type === "cloze") {
      answerContent = el("div", { class: "answer-content" }, clozeNodes(card.question, true));
    } else if (card.type === "reverse") {
      answerContent = el("div", { class: "answer-content", text: card.question });
    } else {
      answerContent = el("div", { class: "answer-content", text: card.answer || "(no answer provided)" });
    }
    parts.push(
      el(
        "div",
        { class: "answer-reveal" },
        el("div", { class: "answer-label", text: "Answer" }),
        answerContent
      )
    );

    // Optional AI evaluation for free-recall answers.
    if (card.type === "free" && session.ai.enabled && view.userAnswer.trim()) {
      parts.push(buildAiEval(card));
    }

    parts.push(buildRatingButtons(card));

    return el("div", { class: "reveal-block" }, ...parts);
  }

  function buildAiEval(card: DueCard): HTMLElement {
    const wrap = el("div", { class: "ai-eval" });
    if (view.evaluation) {
      wrap.appendChild(renderEvaluation(view.evaluation));
      return wrap;
    }
    const evalBtn = button("Evaluate with AI", { icon: "sparkles" });
    evalBtn.addEventListener("click", async () => {
      evalBtn.disabled = true;
      const span = evalBtn.querySelector("span");
      const label = span?.textContent ?? "";
      if (span) span.textContent = "Evaluating…";
      try {
        const evaluation = await api.aiEvaluateAnswer({
          question: card.type === "reverse" ? card.answer : card.question,
          expected_answer: card.type === "reverse" ? card.question : card.answer,
          user_answer: view.userAnswer,
        });
        view.evaluation = evaluation;
        renderStage();
      } catch (e) {
        evalBtn.disabled = false;
        if (span) span.textContent = label;
        toastError(errorMessage(e), errorHint(e));
      }
    });
    wrap.appendChild(evalBtn);
    return wrap;
  }

  function renderEvaluation(ev: AnswerEvaluation): HTMLElement {
    const children: Child[] = [
      el(
        "div",
        { class: "eval-head" },
        el("span", { class: `eval-badge eval-${ev.correctness}`, text: ev.correctness }),
        el("span", { class: "muted", text: `AI assessment · you decide your rating` })
      ),
    ];
    if (ev.feedback) children.push(el("p", { class: "eval-feedback", text: ev.feedback }));
    if (ev.missing) {
      children.push(
        el("p", { class: "eval-missing" }, el("strong", { text: "Missing: " }), el("span", { text: ev.missing }))
      );
    }
    return el("div", { class: "eval-result" }, ...children);
  }

  function buildRatingButtons(card: DueCard): HTMLElement {
    const state = schedulingStateOf(card);
    const preview = previewIntervals(state);
    const row = el("div", { class: "rating-buttons" });
    RATING_ORDER.forEach((rating, i) => {
      const btn = el(
        "button",
        { class: `btn rating rating-${rating}`, on: { click: () => rate(rating) } },
        el("span", { class: "rating-key", text: String(i + 1) }),
        el("span", { class: "rating-name", text: RATING_LABEL[rating] }),
        el("span", { class: "rating-interval", text: formatInterval(preview[rating]) })
      );
      row.appendChild(btn);
    });
    return row;
  }

  // ---- actions ----

  async function rate(rating: Rating): Promise<void> {
    const card = current();
    const state = schedulingStateOf(card);
    const result = schedule(state, rating, Date.now());
    try {
      await api.recordReview({
        card_id: card.id,
        rating,
        user_answer: view.userAnswer.trim() ? view.userAnswer : view.selected ?? null,
        ai_evaluation: view.evaluation ? JSON.stringify(view.evaluation) : null,
        schedule: result,
      });
    } catch (e) {
      toastError(errorMessage(e), errorHint(e));
      return; // don't advance; let the user retry
    }

    session.reviewed++;
    if (rating === "again") {
      session.again++;
      // Re-queue the card (with updated scheduling) so it comes back this session.
      session.queue.push({ ...card, ...result });
    }
    session.pos++;
    if (session.pos >= session.queue.length) finishSession();
    else showCard();
  }

  function handleKey(e: KeyboardEvent): void {
    if (anyModalOpen()) return;
    if (e.key === "Escape") {
      endSession();
      return;
    }
    const typing = (e.target as HTMLElement)?.tagName === "TEXTAREA";
    if (!view.revealed) {
      if (e.key === " " && !typing) {
        e.preventDefault();
        reveal();
      }
      return;
    }
    // revealed → number keys rate
    const idx = ["1", "2", "3", "4"].indexOf(e.key);
    if (idx >= 0) {
      e.preventDefault();
      rate(RATING_ORDER[idx]);
    }
  }

  function finishSession(): void {
    clear(stage);
    progressBar.style.width = "100%";
    progressText.textContent = `${session.reviewed} reviewed`;
    stage.appendChild(
      el(
        "div",
        { class: "session-complete" },
        el("div", { class: "complete-check" }, "✓"),
        el("h2", { text: "Session complete" }),
        el("p", {
          class: "muted",
          text: `You reviewed ${session.reviewed} card${session.reviewed === 1 ? "" : "s"}${
            session.again ? `, ${session.again} marked Again` : ""
          }.`,
        }),
        el(
          "div",
          { class: "complete-actions" },
          button("Back to dashboard", { variant: "primary", onClick: () => navigate("dashboard") }),
          button("Library", { onClick: () => navigate("library") })
        )
      )
    );
  }

  function endSession(): void {
    navigate("dashboard");
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function schedulingStateOf(card: DueCard): SchedulingState {
  return {
    interval: card.interval,
    ease_factor: card.ease_factor,
    review_count: card.review_count,
    due_at: card.due_at,
  };
}

/** Build DOM nodes for cloze text. When not revealed, hidden spans show a blank;
 *  when revealed, they show the answer highlighted. Uses text nodes only (safe). */
function clozeNodes(text: string, revealed: boolean): HTMLElement {
  const container = el("div", { class: "cloze" });
  const regex = /\{\{(.+?)\}\}/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      container.appendChild(document.createTextNode(text.slice(lastIndex, match.index)));
    }
    if (revealed) {
      container.appendChild(el("span", { class: "cloze-answer", text: match[1] }));
    } else {
      container.appendChild(el("span", { class: "cloze-blank", text: "[ … ]" }));
    }
    lastIndex = regex.lastIndex;
  }
  if (lastIndex < text.length) {
    container.appendChild(document.createTextNode(text.slice(lastIndex)));
  }
  return container;
}

function shuffle<T>(arr: T[]): T[] {
  const a = [...arr];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

function renderCaughtUp(container: HTMLElement): void {
  container.replaceChildren(
    el(
      "div",
      { class: "page recall" },
      el(
        "div",
        { class: "recall-topbar" },
        el("button", { class: "back-link", on: { click: () => navigate("dashboard") } }, "← Dashboard")
      ),
      emptyState(
        "check",
        "All caught up",
        "There are no cards due right now. Add more cards or come back later.",
        button("Go to library", { variant: "primary", onClick: () => navigate("library") })
      )
    )
  );
}

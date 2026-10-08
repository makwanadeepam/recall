/** Shapes returned by the AI subsystem (which lives in Rust). */

import type { Difficulty } from "../db/models";

/** A question produced by AI generation, shown for review/edit before saving. */
export interface GeneratedQuestion {
  question: string;
  answer: string;
  type: "basic" | "cloze" | "mcq";
  difficulty: Difficulty;
  distractors: string[];
}

/** Advisory evaluation of a free-form answer. Never replaces the user's own judgment. */
export interface AnswerEvaluation {
  correctness: "correct" | "partial" | "incorrect" | string;
  score: number;
  missing: string;
  feedback: string;
}

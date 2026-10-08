/** Typed command wrappers. This is the only place the frontend talks to Rust. */

import { invoke } from "./tauri";
import type {
  AiConfigInput,
  AiConfigView,
  AppSettings,
  Card,
  CardInput,
  ChatTurn,
  DashboardStats,
  DueCard,
  DueQuery,
  IoResult,
  KbChatResponse,
  KnowledgeItem,
  KnowledgeItemInput,
  KnowledgeItemSummary,
  ReviewInput,
  ThemePref,
} from "../db/models";
import type { AnswerEvaluation, GeneratedQuestion } from "../ai/types";

export const api = {
  // ---- knowledge ----
  listItems: (search?: string | null, tag?: string | null) =>
    invoke<KnowledgeItemSummary[]>("list_items", { search: search ?? null, tag: tag ?? null }),
  getItem: (id: number) => invoke<KnowledgeItem>("get_item", { id }),
  createItem: (input: KnowledgeItemInput) => invoke<KnowledgeItem>("create_item", { input }),
  updateItem: (id: number, input: KnowledgeItemInput) =>
    invoke<KnowledgeItem>("update_item", { id, input }),
  deleteItem: (id: number) => invoke<void>("delete_item", { id }),

  // ---- cards ----
  listCards: (item: number) => invoke<Card[]>("list_cards", { item }),
  getCard: (id: number) => invoke<Card>("get_card", { id }),
  createCard: (input: CardInput) => invoke<Card>("create_card", { input }),
  updateCard: (id: number, input: CardInput) => invoke<Card>("update_card", { id, input }),
  deleteCard: (id: number) => invoke<void>("delete_card", { id }),

  // ---- reviews / scheduling ----
  dueCards: (query: DueQuery) => invoke<DueCard[]>("due_cards", { query }),
  recordReview: (input: ReviewInput) => invoke<Card>("record_review", { input }),

  // ---- stats ----
  dashboard: () => invoke<DashboardStats>("dashboard_stats"),
  listTags: () => invoke<string[]>("list_tags"),

  // ---- settings ----
  getSettings: () => invoke<AppSettings>("get_settings"),
  setTheme: (theme: ThemePref) => invoke<void>("set_theme", { theme }),
  getAiConfig: () => invoke<AiConfigView>("get_ai_config"),
  setAiConfig: (config: AiConfigInput) => invoke<AiConfigView>("set_ai_config", { config }),
  aiSetKey: (provider: string, key: string) => invoke<void>("ai_set_key", { provider, key }),
  aiDeleteKey: (provider: string) => invoke<void>("ai_delete_key", { provider }),
  aiHasKey: (provider: string) => invoke<boolean>("ai_has_key", { provider }),

  // ---- ai operations ----
  aiTestConnection: () => invoke<string>("ai_test_connection"),
  aiGenerateQuestions: (content: string, count?: number) =>
    invoke<GeneratedQuestion[]>("ai_generate_questions", { content, count: count ?? null }),
  aiEvaluateAnswer: (input: { question: string; expected_answer: string; user_answer: string }) =>
    invoke<AnswerEvaluation>("ai_evaluate_answer", { input }),
  aiGenerateExplanation: (question: string, answer: string) =>
    invoke<string>("ai_generate_explanation", { question, answer }),
  aiKbChat: (question: string, history: ChatTurn[]) =>
    invoke<KbChatResponse>("ai_kb_chat", { question, history }),

  // ---- import / export ----
  exportData: (format: "json" | "markdown") => invoke<IoResult | null>("export_data", { format }),
  importData: (format: "json" | "markdown" | "text" | "auto") =>
    invoke<IoResult | null>("import_data", { format }),
};

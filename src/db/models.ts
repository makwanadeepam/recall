/**
 * TypeScript mirrors of the Rust data models. Field names are snake_case to
 * match the structs exactly across the IPC boundary (no case translation).
 */

export type CardType = "basic" | "free" | "cloze" | "mcq" | "reverse";
export type Difficulty = "easy" | "medium" | "hard";
export type Rating = "again" | "hard" | "good" | "easy";

export interface CardMetadata {
  /** Multiple-choice distractors (wrong options). */
  distractors?: string[];
  [key: string]: unknown;
}

export interface KnowledgeItem {
  id: number;
  title: string;
  content: string;
  created_at: number;
  updated_at: number;
  tags: string[];
}

export interface KnowledgeItemSummary {
  id: number;
  title: string;
  excerpt: string;
  created_at: number;
  updated_at: number;
  tags: string[];
  card_count: number;
  due_count: number;
}

export interface KnowledgeItemInput {
  title: string;
  content?: string;
  tags?: string[];
}

export interface Card {
  id: number;
  knowledge_item_id: number;
  question: string;
  answer: string;
  type: CardType;
  difficulty: Difficulty;
  metadata: CardMetadata;
  created_at: number;
  updated_at: number;
  due_at: number;
  interval: number;
  ease_factor: number;
  review_count: number;
}

export interface DueCard extends Card {
  knowledge_title: string;
}

export interface CardInput {
  knowledge_item_id: number;
  question: string;
  answer?: string;
  type: CardType;
  difficulty?: Difficulty;
  metadata?: CardMetadata;
}

export interface ScheduleUpdate {
  interval: number;
  ease_factor: number;
  due_at: number;
  review_count: number;
}

export interface ReviewInput {
  card_id: number;
  rating: Rating;
  user_answer?: string | null;
  ai_evaluation?: string | null;
  schedule: ScheduleUpdate;
}

export interface DueQuery {
  limit?: number;
  include_new?: boolean;
  new_limit?: number;
  item_id?: number | null;
  tag?: string | null;
}

export interface ActivityEntry {
  reviewed_at: number;
  rating: Rating;
  card_id: number;
  question: string;
  knowledge_title: string;
}

export interface DashboardStats {
  due_now: number;
  due_today: number;
  new_cards: number;
  reviewed_today: number;
  total_cards: number;
  total_items: number;
  mastered: number;
  streak: number;
  recent_activity: ActivityEntry[];
}

export type ThemePref = "light" | "dark" | "system";

export interface AiConfigView {
  provider: string;
  base_url: string;
  effective_base_url: string;
  default_base_url: string;
  model: string;
  enabled: boolean;
  confirm_before_send: boolean;
  has_key: boolean;
  is_local: boolean;
  providers: string[];
}

export interface AiConfigInput {
  provider: string;
  base_url: string;
  model: string;
  enabled: boolean;
  confirm_before_send: boolean;
}

export interface AppSettings {
  theme: ThemePref;
  ai: AiConfigView;
}

export interface IoResult {
  path: string;
  items: number;
  cards: number;
  reviews: number;
}

export interface ChatTurn {
  role: "user" | "assistant";
  content: string;
}

export interface KbSource {
  id: number;
  title: string;
}

export interface KbChatResponse {
  answer: string;
  sources: KbSource[];
}

export const CARD_TYPE_LABELS: Record<CardType, string> = {
  basic: "Basic",
  free: "Free recall",
  cloze: "Cloze",
  mcq: "Multiple choice",
  reverse: "Reverse",
};

/**
 * Spaced-repetition scheduler.
 *
 * This is a deliberately small, understandable SM-2-style algorithm with a
 * light learning phase. It is a PURE module: given a card's current scheduling
 * state, a rating, and the current time, it returns the next scheduling state.
 * It has no dependency on the DB, the UI, or Tauri, so it is trivially testable
 * and can later be swapped for a more sophisticated algorithm (e.g. FSRS)
 * without touching any calling code.
 *
 * Model
 * -----
 * - `interval` is measured in DAYS and may be fractional (e.g. a 10-minute
 *   learning step is ~0.0069 days). A card is "in learning" while its interval
 *   is below one day (or it has never been reviewed).
 * - `ease_factor` scales how fast intervals grow; it drifts down on lapses/hard
 *   answers and up on easy answers, clamped to a sane minimum.
 * - `due_at` is an absolute timestamp in milliseconds since the Unix epoch.
 *
 * The field names match the Rust `ScheduleUpdate` struct so the result can be
 * sent straight back over IPC.
 */

export type Rating = "again" | "hard" | "good" | "easy";

export interface SchedulingState {
  interval: number; // days
  ease_factor: number;
  review_count: number;
  due_at: number; // ms epoch
}

export interface ScheduleResult {
  interval: number; // days
  ease_factor: number;
  review_count: number;
  due_at: number; // ms epoch
}

const DAY_MS = 86_400_000;

// Tunable constants — kept together so the algorithm is easy to reason about.
export const SchedulerConfig = {
  MIN_EASE: 1.3,
  DEFAULT_EASE: 2.5,
  AGAIN_EASE_DELTA: -0.2,
  HARD_EASE_DELTA: -0.15,
  EASY_EASE_DELTA: 0.15,
  HARD_INTERVAL_FACTOR: 1.2,
  EASY_BONUS: 1.3,
  LEARNING_STEP: 10 / 1440, // 10 minutes, in days
  HARD_LEARNING_INTERVAL: 0.5, // half a day
  GRADUATING_INTERVAL: 1, // 1 day (first "good")
  EASY_INTERVAL: 4, // 4 days (first "easy")
  MAX_INTERVAL: 3650, // 10 years
} as const;

function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n));
}

/**
 * Compute the next scheduling state for a card given a review rating.
 */
export function schedule(
  state: SchedulingState,
  rating: Rating,
  now: number = Date.now()
): ScheduleResult {
  const c = SchedulerConfig;
  let ease = state.ease_factor > 0 ? state.ease_factor : c.DEFAULT_EASE;
  const interval = Math.max(0, state.interval);
  const reps = Math.max(0, state.review_count);
  const isLearning = reps === 0 || interval < 1;

  let next: number;
  switch (rating) {
    case "again":
      ease = Math.max(c.MIN_EASE, ease + c.AGAIN_EASE_DELTA);
      next = c.LEARNING_STEP;
      break;
    case "hard":
      ease = Math.max(c.MIN_EASE, ease + c.HARD_EASE_DELTA);
      next = isLearning ? c.HARD_LEARNING_INTERVAL : interval * c.HARD_INTERVAL_FACTOR;
      break;
    case "good":
      next = isLearning ? c.GRADUATING_INTERVAL : interval * ease;
      break;
    case "easy":
      ease = ease + c.EASY_EASE_DELTA;
      next = isLearning ? c.EASY_INTERVAL : interval * ease * c.EASY_BONUS;
      break;
  }

  next = clamp(next, c.LEARNING_STEP, c.MAX_INTERVAL);

  return {
    interval: next,
    ease_factor: ease,
    review_count: reps + 1,
    due_at: now + Math.round(next * DAY_MS),
  };
}

/**
 * Preview the resulting interval (in days) for each rating without committing.
 * Used to label the rating buttons, e.g. "Good · 3d".
 */
export function previewIntervals(state: SchedulingState): Record<Rating, number> {
  const now = 0;
  return {
    again: schedule(state, "again", now).interval,
    hard: schedule(state, "hard", now).interval,
    good: schedule(state, "good", now).interval,
    easy: schedule(state, "easy", now).interval,
  };
}

/** Human-friendly interval label from a day count. */
export function formatInterval(days: number): string {
  if (days < 1) {
    const minutes = Math.round(days * 1440);
    if (minutes < 60) return `${Math.max(1, minutes)}m`;
    return `${Math.round(minutes / 60)}h`;
  }
  if (days < 30) return `${Math.round(days)}d`;
  if (days < 365) return `${Math.round(days / 30)}mo`;
  return `${(days / 365).toFixed(1)}y`;
}

/** The initial scheduling state for a brand-new card. */
export function newCardState(now: number = Date.now()): SchedulingState {
  return {
    interval: 0,
    ease_factor: SchedulerConfig.DEFAULT_EASE,
    review_count: 0,
    due_at: now,
  };
}

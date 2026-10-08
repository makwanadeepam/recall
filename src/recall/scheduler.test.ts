import { describe, it, expect } from "vitest";
import {
  schedule,
  previewIntervals,
  formatInterval,
  newCardState,
  SchedulerConfig,
  type SchedulingState,
} from "./scheduler";

const NOW = 1_700_000_000_000;
const DAY = 86_400_000;

function review(interval: number, ease = 2.5, reps = 3): SchedulingState {
  return { interval, ease_factor: ease, review_count: reps, due_at: NOW };
}

describe("scheduler — new / learning cards", () => {
  it("graduates a new card to 1 day on Good", () => {
    const r = schedule(newCardState(NOW), "good", NOW);
    expect(r.interval).toBe(1);
    expect(r.review_count).toBe(1);
    expect(r.due_at).toBe(NOW + DAY);
  });

  it("sends a new card to 4 days on Easy and raises ease", () => {
    const r = schedule(newCardState(NOW), "easy", NOW);
    expect(r.interval).toBe(4);
    expect(r.ease_factor).toBeCloseTo(2.65, 5);
  });

  it("keeps a new card in learning on Again and lowers ease", () => {
    const r = schedule(newCardState(NOW), "again", NOW);
    expect(r.interval).toBeLessThan(1);
    expect(r.interval).toBeCloseTo(SchedulerConfig.LEARNING_STEP, 6);
    expect(r.ease_factor).toBeCloseTo(2.3, 5);
  });

  it("orders learning outcomes again < hard < good < easy", () => {
    const p = previewIntervals(newCardState(NOW));
    expect(p.again).toBeLessThan(p.hard);
    expect(p.hard).toBeLessThan(p.good);
    expect(p.good).toBeLessThan(p.easy);
  });
});

describe("scheduler — review cards", () => {
  it("multiplies interval by ease on Good", () => {
    const r = schedule(review(10, 2.5), "good", NOW);
    expect(r.interval).toBeCloseTo(25, 5);
  });

  it("applies the hard factor on Hard and lowers ease", () => {
    const r = schedule(review(10, 2.5), "hard", NOW);
    expect(r.interval).toBeCloseTo(12, 5); // 10 * 1.2
    expect(r.ease_factor).toBeCloseTo(2.35, 5);
  });

  it("applies the easy bonus on Easy", () => {
    const r = schedule(review(10, 2.5), "easy", NOW);
    // 10 * (2.5 + 0.15) * 1.3
    expect(r.interval).toBeCloseTo(34.45, 5);
  });

  it("lapses to relearning on Again and reduces ease", () => {
    const r = schedule(review(20, 2.5), "again", NOW);
    expect(r.interval).toBeLessThan(1);
    expect(r.ease_factor).toBeCloseTo(2.3, 5);
  });

  it("orders review outcomes again < hard < good < easy", () => {
    const p = previewIntervals(review(10, 2.5));
    expect(p.again).toBeLessThan(p.hard);
    expect(p.hard).toBeLessThan(p.good);
    expect(p.good).toBeLessThan(p.easy);
  });
});

describe("scheduler — invariants", () => {
  it("never drops ease below the minimum, even after many Agains", () => {
    let s = review(5, 2.5);
    for (let i = 0; i < 20; i++) {
      const r = schedule(s, "again", NOW);
      s = { ...s, ...r };
    }
    expect(s.ease_factor).toBeGreaterThanOrEqual(SchedulerConfig.MIN_EASE);
    expect(s.ease_factor).toBeCloseTo(SchedulerConfig.MIN_EASE, 5);
  });

  it("grows intervals monotonically under repeated Good", () => {
    let s = newCardState(NOW);
    let last = 0;
    for (let i = 0; i < 6; i++) {
      const r = schedule(s, "good", NOW);
      expect(r.interval).toBeGreaterThan(last);
      last = r.interval;
      s = { ...s, ...r };
    }
  });

  it("caps intervals at the configured maximum", () => {
    const r = schedule(review(3000, 3.0), "easy", NOW);
    expect(r.interval).toBeLessThanOrEqual(SchedulerConfig.MAX_INTERVAL);
  });

  it("increments review_count on every rating", () => {
    for (const rating of ["again", "hard", "good", "easy"] as const) {
      expect(schedule(review(5, 2.5, 7), rating, NOW).review_count).toBe(8);
    }
  });
});

describe("formatInterval", () => {
  it("formats sub-day, day, month and year ranges", () => {
    expect(formatInterval(10 / 1440)).toBe("10m");
    expect(formatInterval(2 / 24)).toBe("2h");
    expect(formatInterval(3)).toBe("3d");
    expect(formatInterval(45)).toBe("2mo");
    expect(formatInterval(400)).toBe("1.1y");
  });
});

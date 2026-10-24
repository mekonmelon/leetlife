import { describe, expect, it } from "vitest";
import { confidenceLabel, describeTask, formatCoins, formatMicros, relativeTime, timeLeft } from "../src/api/format";
import type { DailyTask } from "../src/api/types";

const task = (over: Partial<DailyTask>): DailyTask => ({
  id: 1, date: "2026-09-16", slot: "breakfast", kind: { type: "solve_any" },
  title: "t", target: 1, progress: 0, status: "pending", deadline: null,
  reward_xp: 0, reward_coins: 0, completed_at: null, ...over,
});

describe("describeTask", () => {
  it("renders every task kind", () => {
    expect(describeTask({ type: "solve_difficulty", difficulty: "medium" })).toBe("Solve 1 Medium");
    expect(describeTask({ type: "solve_topic", topic: "Graphs" })).toBe("Solve a Graphs problem");
    expect(describeTask({ type: "review", problem_id: 1, slug: "two-sum" })).toBe("Review two-sum");
    expect(describeTask({ type: "solve_any" })).toBe("Solve anything");
  });
});

describe("timeLeft", () => {
  const now = 1_700_000_000;
  it("formats minutes, hours, and overdue", () => {
    expect(timeLeft(task({ deadline: now + 25 * 60 }), now)).toBe("25m left");
    expect(timeLeft(task({ deadline: now + 3 * 3600 + 20 * 60 }), now)).toBe("3h 20m left");
    expect(timeLeft(task({ deadline: now + 2 * 3600 }), now)).toBe("2h left");
    expect(timeLeft(task({ deadline: now - 60 }), now)).toBe("overdue");
  });
  it("is silent without a deadline or once the task is settled", () => {
    expect(timeLeft(task({ deadline: null }), now)).toBeNull();
    expect(timeLeft(task({ deadline: now + 600, status: "done" }), now)).toBeNull();
    expect(timeLeft(task({ deadline: now - 600, status: "missed" }), now)).toBeNull();
  });
});

describe("number formatting", () => {
  it("shortens large coin counts only", () => {
    expect(formatCoins(420)).toBe("420");
    expect(formatCoins(9999)).toBe("9999");
    expect(formatCoins(12500)).toBe("12.5k");
  });
  it("scales latency units", () => {
    expect(formatMicros(820)).toBe("820µs");
    expect(formatMicros(21_400)).toBe("21.4ms");
    expect(formatMicros(1_200_000)).toBe("1.20s");
  });
  it("labels confidence bands", () => {
    expect(confidenceLabel(0.91)).toBe("strong");
    expect(confidenceLabel(0.67)).toBe("solid");
    expect(confidenceLabel(0.41)).toBe("shaky");
    expect(confidenceLabel(0.2)).toBe("weak");
  });
  it("renders relative times", () => {
    const now = 1_700_000_000;
    expect(relativeTime(now - 10, now)).toBe("just now");
    expect(relativeTime(now - 600, now)).toBe("10m ago");
    expect(relativeTime(now - 7200, now)).toBe("2h ago");
    expect(relativeTime(now - 3 * 86400, now)).toBe("3d ago");
  });
});

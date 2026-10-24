import { describe, expect, it, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import App from "../src/App";
import { __setTransport } from "../src/api/client";
import type { Snapshot } from "../src/api/types";

const baseSnapshot: Snapshot = {
  version: 1,
  generated_at: 1_700_000_000,
  date: "2026-09-16",
  weekday: "Wednesday",
  day_number: 24,
  avatar: {
    hunger: 55, energy: 82, happiness: 74, xp: 420, level: 3, coins: 420,
    streak: 12, best_streak: 18, last_active_date: null, started_on: "2026-08-24", updated_at: 0,
  },
  mood: "I haven't eaten yet... one Easy would really hit the spot.",
  xp_into_level: 220,
  xp_for_next_level: 250,
  tasks: [
    { id: 1, date: "2026-09-16", slot: "breakfast", kind: { type: "solve_difficulty", difficulty: "easy" }, title: "Solve 1 Easy", target: 1, progress: 0, status: "pending", deadline: 1_700_003_600, reward_xp: 20, reward_coins: 15, completed_at: null },
    { id: 2, date: "2026-09-16", slot: "training", kind: { type: "solve_topic", topic: "Graphs" }, title: "Train Graphs", target: 1, progress: 0, status: "pending", deadline: null, reward_xp: 25, reward_coins: 20, completed_at: null },
  ],
  inventory: [
    { item: { id: 1, name: "Plant", category: "decor", cost: 60, asset_path: "furniture/plant", description: "", min_level: 1 }, acquired_at: 0, equipped: true },
    { item: { id: 2, name: "Desk", category: "furniture", cost: 120, asset_path: "furniture/desk", description: "", min_level: 1 }, acquired_at: 0, equipped: false },
  ],
  topics: [{ topic: "Graphs", confidence: 0.41, attempts: 9, failures: 3, last_reviewed: null }],
  recommendations: [
    { problem: { id: 200, slug: "number-of-islands", title: "Number of Islands", difficulty: "medium", topic: "Graphs", importance: 1 }, score: 3.4, reason: "Graphs confidence is 41%", is_review: false },
  ],
  room_level: 0,
  due_reviews: 1,
  last_reward: null,
};

let snapshot: Snapshot;
const calls: { cmd: string; args?: Record<string, unknown> }[] = [];

function transport() {
  return vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    calls.push({ cmd, args });
    switch (cmd) {
      case "get_snapshot":
      case "refresh_snapshot":
        return snapshot;
      case "log_attempt": {
        const done = { ...snapshot.tasks[0], status: "done" as const, progress: 1 };
        snapshot = { ...snapshot, version: snapshot.version + 1, tasks: [done, snapshot.tasks[1]], mood: "Feeling sharp. What's next?" };
        return { attempt_id: 1, problem: snapshot.recommendations[0].problem, reward: { xp: 30, coins: 30, hunger: 40, energy: 0, happiness: 3 }, completed_tasks: [done], level_up: null, streak: 13, is_review: false, next_review_at: 0, achievements: [], snapshot };
      }
      case "get_shop":
        return [
          { item: { id: 3, name: "Bed", category: "furniture", cost: 150, asset_path: "furniture/bed", description: "For naps.", min_level: 1 }, owned: false, affordable: true, unlocked: true },
          { item: { id: 9, name: "Cat", category: "pet", cost: 400, asset_path: "pets/cat", description: "Sits on keys.", min_level: 8 }, owned: false, affordable: true, unlocked: false },
        ];
      case "purchase_item":
        throw new Error("not enough coins: this costs 150 and you have 20");
      case "get_stats":
        return {
          total_solved: 87, total_attempts: 112,
          solved_by_difficulty: [["easy", 41]],
          last_14_days: [{ date: "2026-09-16", solved: 2, failed: 0 }],
          topics: snapshot.topics, due_reviews: [], recent_events: [], achievements: [{ key: "streak_7", title: "One week streak", unlocked_at: 0 }],
        };
      case "get_metrics":
        return [{ name: "ui.open", count: 42, mean_us: 21000, p50_us: 19000, p95_us: 38000, p99_us: 51000, max_us: 74000 }];
      case "get_recommendations": {
        // The backend honours the filter; the breakfast task asks for an Easy.
        const a = (args?.args ?? {}) as { difficulty?: string };
        const pool = [
          { problem: { id: 1, slug: "two-sum", title: "Two Sum", difficulty: "easy", topic: "Arrays", importance: 1 }, score: 2, reason: "", is_review: false },
          { problem: snapshot.recommendations[0].problem, score: 3, reason: "", is_review: false },
        ];
        return a.difficulty ? pool.filter((r) => r.problem.difficulty === a.difficulty) : pool;
      }
      default:
        return undefined;
    }
  });
}

beforeEach(() => {
  cleanup();
  calls.length = 0;
  snapshot = structuredClone(baseSnapshot);
  __setTransport(transport() as never, async () => () => {});
});

describe("App", () => {
  it("renders the house, the day header and today's tasks", async () => {
    render(<App />);
    expect(await screen.findByText("Wednesday — Day 24")).toBeInTheDocument();
    expect(screen.getByTestId("house")).toBeInTheDocument();
    expect(screen.getByTestId("mood")).toHaveTextContent("I haven't eaten yet");
    expect(screen.getByText("Solve 1 Easy")).toBeInTheDocument();
    expect(screen.getByText("Train Graphs")).toBeInTheDocument();
    expect(screen.getByText("💰 420")).toBeInTheDocument();
    expect(screen.getByText("🔥 12")).toBeInTheDocument();
  });

  it("only draws furniture that is owned and equipped", async () => {
    render(<App />);
    await screen.findByTestId("house");
    expect(screen.getByTestId("furniture-furniture/plant")).toBeInTheDocument();
    expect(screen.queryByTestId("furniture-furniture/desk")).not.toBeInTheDocument();
    expect(screen.queryByTestId("furniture-pets/cat")).not.toBeInTheDocument();
  });

  it("reports first paint back to the backend for the open-latency metric", async () => {
    render(<App />);
    await screen.findByTestId("house");
    await waitFor(() => expect(calls.some((c) => c.cmd === "ui_rendered")).toBe(true));
  });

  it("completing a task logs an attempt and shows the reward", async () => {
    render(<App />);
    const button = (await screen.findAllByRole("button", { name: /Mark Solve 1 Easy complete/ }))[0];
    fireEvent.click(button);

    await waitFor(() => expect(screen.getByTestId("toast")).toHaveTextContent("+30 XP, +30 coins"));
    expect(screen.getByTestId("mood")).toHaveTextContent("Feeling sharp");
  });

  it("completing a task logs a problem that actually satisfies it", async () => {
    render(<App />);
    const button = (await screen.findAllByRole("button", { name: /Mark Solve 1 Easy complete/ }))[0];
    fireEvent.click(button);
    await waitFor(() => expect(calls.some((c) => c.cmd === "log_attempt")).toBe(true));

    // The only cached recommendation is a Medium, so it asks the backend for an
    // Easy rather than logging something that would leave the task pending.
    expect(calls.find((c) => c.cmd === "get_recommendations")?.args?.args).toMatchObject({ difficulty: "easy" });
    expect(calls.find((c) => c.cmd === "log_attempt")?.args?.input).toMatchObject({ result: "solved", slug: "two-sum" });
  });

  it("surfaces a backend refusal as a readable message", async () => {
    render(<App />);
    await screen.findByTestId("house");
    fireEvent.click(screen.getByRole("tab", { name: "Shop" }));

    const buy = await screen.findByRole("button", { name: "Buy" });
    fireEvent.click(buy);
    await waitFor(() => expect(screen.getByTestId("toast")).toHaveTextContent("not enough coins: this costs 150 and you have 20"));
  });

  it("locks shop items above the avatar's level", async () => {
    render(<App />);
    await screen.findByTestId("house");
    fireEvent.click(screen.getByRole("tab", { name: "Shop" }));
    expect(await screen.findByRole("button", { name: "Level 8 required" })).toBeDisabled();
  });

  it("shows analytics and the latency table on the stats tab", async () => {
    render(<App />);
    await screen.findByTestId("house");
    fireEvent.click(screen.getByRole("tab", { name: "Stats" }));

    expect(await screen.findByTestId("activity-chart")).toBeInTheDocument();
    expect(screen.getByTestId("topic-bars")).toHaveTextContent("Graphs");
    expect(screen.getByTestId("achievements")).toHaveTextContent("One week streak");
    const table = screen.getByTestId("metrics-table");
    expect(table).toHaveTextContent("Tray click → painted");
    expect(table).toHaveTextContent("38.0ms");
  });
});

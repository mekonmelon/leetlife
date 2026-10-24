// Pure display helpers. Kept out of components so they can be unit-tested.
import type { DailyTask, Difficulty, Slot, TaskKind } from "./types";

export const SLOT_LABEL: Record<Slot, string> = {
  breakfast: "Breakfast",
  training: "Training",
  work: "Work",
  evening: "Evening",
};

export const SLOT_ICON: Record<Slot, string> = {
  breakfast: "tea",
  training: "leaf",
  work: "star",
  evening: "moon",
};

export const DIFFICULTY_LABEL: Record<Difficulty, string> = {
  easy: "Easy",
  medium: "Medium",
  hard: "Hard",
};

export function describeTask(kind: TaskKind): string {
  switch (kind.type) {
    case "solve_difficulty":
      return `Solve 1 ${DIFFICULTY_LABEL[kind.difficulty]}`;
    case "solve_topic":
      return `Solve a ${kind.topic} problem`;
    case "review":
      return `Review ${kind.slug}`;
    case "solve_any":
      return "Solve anything";
  }
}

/** "3h 20m left", "25m left", "overdue" — or null when there is no deadline. */
export function timeLeft(task: DailyTask, nowSeconds: number): string | null {
  if (task.deadline === null) return null;
  if (task.status !== "pending") return null;
  const secs = task.deadline - nowSeconds;
  if (secs <= 0) return "overdue";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m left`;
  const hours = Math.floor(mins / 60);
  const rem = mins % 60;
  return rem === 0 ? `${hours}h left` : `${hours}h ${rem}m left`;
}

export function formatCoins(n: number): string {
  return n >= 10000 ? `${(n / 1000).toFixed(1)}k` : String(n);
}

/** Microseconds to a short human string: 820µs, 21.4ms, 1.20s */
export function formatMicros(us: number): string {
  if (us < 1000) return `${Math.round(us)}µs`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(1)}ms`;
  return `${(us / 1_000_000).toFixed(2)}s`;
}

export function confidenceLabel(confidence: number): string {
  if (confidence >= 0.85) return "strong";
  if (confidence >= 0.6) return "solid";
  if (confidence >= 0.4) return "shaky";
  return "weak";
}

export function relativeTime(ts: number, nowSeconds: number): string {
  const d = nowSeconds - ts;
  if (d < 60) return "just now";
  if (d < 3600) return `${Math.floor(d / 60)}m ago`;
  if (d < 86400) return `${Math.floor(d / 3600)}h ago`;
  return `${Math.floor(d / 86400)}d ago`;
}

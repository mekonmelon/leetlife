// Browser-only stand-in for the Rust backend, so `npm run dev` renders a real
// house without Tauri. It is intentionally shallow: the real rules live in Rust.
import type { AttemptOutcome, LogAttemptInput, MetricSummary, Recommendation, SettingsView, ShopItem, Snapshot, Stats, ProblemMeta } from "./types";

const CATALOG: ProblemMeta[] = [
  { id: 1, slug: "two-sum", title: "Two Sum", difficulty: "easy", topic: "Arrays", importance: 1 },
  { id: 200, slug: "number-of-islands", title: "Number of Islands", difficulty: "medium", topic: "Graphs", importance: 1 },
  { id: 322, slug: "coin-change", title: "Coin Change", difficulty: "medium", topic: "Dynamic Programming", importance: 1 },
  { id: 146, slug: "lru-cache", title: "LRU Cache", difficulty: "medium", topic: "Linked List", importance: 1 },
  { id: 42, slug: "trapping-rain-water", title: "Trapping Rain Water", difficulty: "hard", topic: "Two Pointers", importance: 0.8 },
];

const ITEMS = [
  { id: 1, name: "Plant", category: "decor", cost: 60, asset_path: "furniture/plant", description: "A small green friend.", min_level: 1 },
  { id: 2, name: "Desk", category: "furniture", cost: 120, asset_path: "furniture/desk", description: "Somewhere to put the keyboard.", min_level: 1 },
  { id: 3, name: "Bed", category: "furniture", cost: 150, asset_path: "furniture/bed", description: "For power naps between Hards.", min_level: 1 },
  { id: 9, name: "Cat", category: "pet", cost: 400, asset_path: "pets/cat", description: "Sits on the keyboard.", min_level: 3 },
];

let snapshot: Snapshot = {
  version: 1,
  generated_at: Math.floor(Date.now() / 1000),
  date: new Date().toISOString().slice(0, 10),
  weekday: new Date().toLocaleDateString("en-US", { weekday: "long" }),
  day_number: 24,
  avatar: {
    hunger: 55, energy: 82, happiness: 74, xp: 420, level: 3, coins: 420,
    streak: 12, best_streak: 18, last_active_date: null,
    started_on: "2026-08-26", updated_at: Math.floor(Date.now() / 1000),
  },
  mood: "I haven't eaten yet... one Easy would really hit the spot.",
  xp_into_level: 220,
  xp_for_next_level: 250,
  tasks: [
    { id: 1, date: "", slot: "breakfast", kind: { type: "solve_difficulty", difficulty: "easy" }, title: "Solve 1 Easy", target: 1, progress: 0, status: "pending", deadline: null, reward_xp: 20, reward_coins: 15, completed_at: null },
    { id: 2, date: "", slot: "training", kind: { type: "solve_topic", topic: "Graphs" }, title: "Train Graphs", target: 1, progress: 0, status: "pending", deadline: null, reward_xp: 25, reward_coins: 20, completed_at: null },
    { id: 3, date: "", slot: "work", kind: { type: "solve_difficulty", difficulty: "medium" }, title: "Solve 1 Medium", target: 1, progress: 0, status: "pending", deadline: null, reward_xp: 30, reward_coins: 30, completed_at: null },
    { id: 4, date: "", slot: "evening", kind: { type: "review", problem_id: 322, slug: "coin-change" }, title: "Review Coin Change", target: 1, progress: 0, status: "pending", deadline: null, reward_xp: 20, reward_coins: 20, completed_at: null },
  ],
  inventory: [
    { item: ITEMS[0], acquired_at: 0, equipped: true },
    { item: ITEMS[1], acquired_at: 0, equipped: true },
  ],
  topics: [
    { topic: "Graphs", confidence: 0.41, attempts: 9, failures: 3, last_reviewed: Math.floor(Date.now() / 1000) - 17 * 86400 },
    { topic: "Dynamic Programming", confidence: 0.58, attempts: 14, failures: 4, last_reviewed: null },
    { topic: "Trees", confidence: 0.67, attempts: 11, failures: 2, last_reviewed: null },
    { topic: "Binary Search", confidence: 0.83, attempts: 7, failures: 1, last_reviewed: null },
    { topic: "Arrays", confidence: 0.91, attempts: 22, failures: 1, last_reviewed: null },
  ],
  recommendations: [
    { problem: CATALOG[1], score: 3.4, reason: "Graphs confidence is 41%, last Graphs practice was 17 days ago", is_review: false },
    { problem: CATALOG[2], score: 2.7, reason: "due for review, interview classic", is_review: true },
    { problem: CATALOG[3], score: 2.1, reason: "interview classic", is_review: false },
  ],
  room_level: 0,
  due_reviews: 1,
  last_reward: null,
};

function bump(): Snapshot {
  snapshot = { ...snapshot, version: snapshot.version + 1 };
  return snapshot;
}

const handlers = new Map<string, ((p: unknown) => void)[]>();

export const mockListen = async (event: string, handler: (p: unknown) => void) => {
  const list = handlers.get(event) ?? [];
  list.push(handler);
  handlers.set(event, list);
  return () => handlers.set(event, (handlers.get(event) ?? []).filter((h) => h !== handler));
};

function emit(event: string, payload: unknown) {
  for (const h of handlers.get(event) ?? []) h(payload);
}

export const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "get_snapshot":
    case "refresh_snapshot":
      return snapshot as T;

    case "log_attempt": {
      const input = args?.input as LogAttemptInput;
      const problem = CATALOG.find((p) => p.slug === input.slug) ?? CATALOG[0];
      const solved = input.result === "solved";
      const xp = solved ? { easy: 10, medium: 25, hard: 60 }[problem.difficulty] : 3;
      const coins = solved ? { easy: 15, medium: 30, hard: 70 }[problem.difficulty] : 0;
      const task = snapshot.tasks.find(
        (t) => t.status === "pending" && solved &&
          ((t.kind.type === "solve_difficulty" && t.kind.difficulty === problem.difficulty) ||
            (t.kind.type === "solve_topic" && t.kind.topic === problem.topic) ||
            (t.kind.type === "review" && t.kind.problem_id === problem.id) ||
            t.kind.type === "solve_any"),
      );
      const reward = { xp: xp + (task?.reward_xp ?? 0), coins: coins + (task?.reward_coins ?? 0), hunger: task?.slot === "breakfast" ? 40 : 0, energy: 0, happiness: 3 };
      snapshot = {
        ...snapshot,
        avatar: {
          ...snapshot.avatar,
          xp: snapshot.avatar.xp + reward.xp,
          coins: snapshot.avatar.coins + reward.coins,
          hunger: Math.min(100, snapshot.avatar.hunger + reward.hunger),
        },
        xp_into_level: snapshot.xp_into_level + reward.xp,
        tasks: snapshot.tasks.map((t) => (t.id === task?.id ? { ...t, status: "done" as const, progress: t.target } : t)),
        mood: solved ? "Feeling sharp. What's next?" : "Could use a win today.",
        last_reward: { headline: task ? `${task.slot} done: ${task.title}` : `Solved ${problem.title}`, xp: reward.xp, coins: reward.coins, at: Math.floor(Date.now() / 1000) },
      };
      const outcome: AttemptOutcome = {
        attempt_id: Math.floor(Math.random() * 1e6),
        problem, reward,
        completed_tasks: task ? [snapshot.tasks.find((t) => t.id === task.id)!] : [],
        level_up: null, streak: snapshot.avatar.streak, is_review: false,
        next_review_at: Math.floor(Date.now() / 1000) + 86400,
        achievements: [], snapshot: bump(),
      };
      emit("state-changed", snapshot);
      return outcome as T;
    }

    case "purchase_item": {
      const id = args?.itemId as number;
      const item = ITEMS.find((i) => i.id === id)!;
      if (snapshot.avatar.coins < item.cost) throw new Error(`not enough coins: this costs ${item.cost} and you have ${snapshot.avatar.coins}`);
      snapshot = {
        ...snapshot,
        avatar: { ...snapshot.avatar, coins: snapshot.avatar.coins - item.cost },
        inventory: [...snapshot.inventory, { item, acquired_at: Date.now() / 1000, equipped: true }],
      };
      emit("state-changed", snapshot);
      return bump() as T;
    }

    case "set_equipped": {
      const id = args?.itemId as number;
      snapshot = { ...snapshot, inventory: snapshot.inventory.map((o) => (o.item.id === id ? { ...o, equipped: args?.equipped as boolean } : o)) };
      return bump() as T;
    }

    case "get_shop":
      return ITEMS.map((item) => ({
        item,
        owned: snapshot.inventory.some((o) => o.item.id === item.id),
        affordable: snapshot.avatar.coins >= item.cost,
        unlocked: snapshot.avatar.level >= item.min_level,
      })) as ShopItem[] as T;

    case "get_stats":
      return {
        total_solved: 87, total_attempts: 112,
        solved_by_difficulty: [["easy", 41], ["hard", 7], ["medium", 39]],
        last_14_days: Array.from({ length: 14 }, (_, i) => {
          const d = new Date(Date.now() - (13 - i) * 86400000);
          return { date: d.toISOString().slice(0, 10), solved: [0, 1, 2, 3, 1, 0, 2][i % 7], failed: i % 5 === 0 ? 1 : 0 };
        }),
        topics: snapshot.topics,
        due_reviews: [CATALOG[2]],
        recent_events: [
          { id: 3, kind: "problem_solved", payload: "{}", created_at: Math.floor(Date.now() / 1000) - 600 },
          { id: 2, kind: "daily_goal_completed", payload: "{}", created_at: Math.floor(Date.now() / 1000) - 640 },
          { id: 1, kind: "daily_plan_created", payload: "{}", created_at: Math.floor(Date.now() / 1000) - 7200 },
        ],
        achievements: [{ key: "streak_7", title: "One week streak", unlocked_at: Math.floor(Date.now() / 1000) - 400000 }],
      } as Stats as T;

    case "get_recommendations":
      return snapshot.recommendations as Recommendation[] as T;

    case "search_problems": {
      const q = String(args?.query ?? "").toLowerCase();
      return CATALOG.filter((p) => p.title.toLowerCase().includes(q) || p.slug.includes(q) || p.topic.toLowerCase().includes(q)) as T;
    }

    case "list_topics":
      return ["Arrays", "Graphs", "Trees", "Dynamic Programming", "Two Pointers", "Linked List"] as T;

    case "get_recent_attempts":
      return [] as T;

    case "get_metrics":
      return [
        { name: "ui.open", count: 42, mean_us: 21000, p50_us: 19000, p95_us: 38000, p99_us: 51000, max_us: 74000 },
        { name: "ipc.get_snapshot", count: 42, mean_us: 380, p50_us: 340, p95_us: 720, p99_us: 980, max_us: 1400 },
        { name: "db.attempt_tx", count: 18, mean_us: 4200, p50_us: 3900, p95_us: 7600, p99_us: 9100, max_us: 11000 },
        { name: "recommend.generate", count: 61, mean_us: 900, p50_us: 850, p95_us: 1500, p99_us: 1900, max_us: 2600 },
      ] as MetricSummary[] as T;

    case "get_settings":
      return { values: { leetcode_username: "" }, autostart: false, ingest_port: 47831 } as SettingsView as T;

    case "sync_now":
      return { provider: "manual", fetched: 0, imported: 0, error: null } as T;

    default:
      return undefined as T;
  }
};

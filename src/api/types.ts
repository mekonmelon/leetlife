// Mirrors the Rust types in src-tauri/src/{models,game,cache,recommendation}.rs.
// Serde uses snake_case on the wire, so these match field-for-field.

export type Difficulty = "easy" | "medium" | "hard";
export type AttemptResultKind = "solved" | "failed";
export type Slot = "breakfast" | "training" | "work" | "evening";
export type TaskStatus = "pending" | "done" | "missed";

export type TaskKind =
  | { type: "solve_difficulty"; difficulty: Difficulty }
  | { type: "solve_topic"; topic: string }
  | { type: "review"; problem_id: number; slug: string }
  | { type: "solve_any" };

export interface DailyTask {
  id: number;
  date: string;
  slot: Slot;
  kind: TaskKind;
  title: string;
  target: number;
  progress: number;
  status: TaskStatus;
  deadline: number | null;
  reward_xp: number;
  reward_coins: number;
  completed_at: number | null;
}

export interface AvatarState {
  hunger: number;
  energy: number;
  happiness: number;
  xp: number;
  level: number;
  coins: number;
  streak: number;
  best_streak: number;
  last_active_date: string | null;
  started_on: string;
  updated_at: number;
}

export interface ItemDef {
  id: number;
  name: string;
  category: string;
  cost: number;
  asset_path: string;
  description: string;
  min_level: number;
}

export interface OwnedItem {
  item: ItemDef;
  acquired_at: number;
  equipped: boolean;
}

export interface ShopItem {
  item: ItemDef;
  owned: boolean;
  affordable: boolean;
  unlocked: boolean;
}

export interface ProblemMeta {
  id: number;
  slug: string;
  title: string;
  difficulty: Difficulty;
  topic: string;
  importance: number;
}

export interface TopicScore {
  topic: string;
  confidence: number;
  attempts: number;
  failures: number;
  last_reviewed: number | null;
}

export interface Recommendation {
  problem: ProblemMeta;
  score: number;
  reason: string;
  is_review: boolean;
}

export interface RewardSummary {
  headline: string;
  xp: number;
  coins: number;
  at: number;
}

export type RoomId = "hearth" | "studio" | "loft";
export interface Placement { room: RoomId; x: number; y: number; }
export interface Flower { species: string; planted_at: number; watered_at: number | null; }
export interface HomeLayout { placements: Record<string, Placement>; wallpapers: Partial<Record<RoomId, string>>; garden?: Record<string, Flower>; }

export interface Snapshot {
  version: number;
  generated_at: number;
  date: string;
  weekday: string;
  day_number: number;
  avatar: AvatarState;
  mood: string;
  xp_into_level: number;
  xp_for_next_level: number;
  tasks: DailyTask[];
  inventory: OwnedItem[];
  topics: TopicScore[];
  recommendations: Recommendation[];
  room_level: number;
  due_reviews: number;
  last_reward: RewardSummary | null;
  home?: HomeLayout;
}

export interface DayCount {
  date: string;
  solved: number;
  failed: number;
}

export interface EventRow {
  id: number;
  kind: string;
  payload: string;
  created_at: number;
}

export interface Achievement {
  key: string;
  title: string;
  unlocked_at: number;
}

export interface Stats {
  total_solved: number;
  total_attempts: number;
  solved_by_difficulty: [string, number][];
  last_14_days: DayCount[];
  topics: TopicScore[];
  due_reviews: ProblemMeta[];
  recent_events: EventRow[];
  achievements: Achievement[];
}

export interface AttemptRow {
  id: number;
  problem_id: number;
  difficulty: string;
  result: string;
  duration_ms: number | null;
  attempted_at: number;
  topic: string;
  source: string;
}

export interface Reward {
  xp: number;
  coins: number;
  hunger: number;
  energy: number;
  happiness: number;
}

export interface AttemptOutcome {
  attempt_id: number;
  problem: ProblemMeta;
  reward: Reward;
  completed_tasks: DailyTask[];
  level_up: number | null;
  streak: number;
  is_review: boolean;
  next_review_at: number;
  achievements: string[];
  snapshot: Snapshot;
}

export interface MetricSummary {
  name: string;
  count: number;
  mean_us: number;
  p50_us: number;
  p95_us: number;
  p99_us: number;
  max_us: number;
}

export interface SettingsView {
  values: Record<string, string>;
  autostart: boolean;
  ingest_port: number;
  data_path?: string;
}

export interface SyncReport {
  provider: string;
  fetched: number;
  imported: number;
  error: string | null;
}

export interface LogAttemptInput {
  problem_id?: number | null;
  slug?: string | null;
  title?: string | null;
  difficulty?: Difficulty | null;
  topic?: string | null;
  result: AttemptResultKind;
  duration_ms?: number | null;
  at?: number | null;
  source?: string | null;
  external_id?: string | null;
}

export type AppEvent =
  | { type: "problem_solved"; slug: string; title: string; difficulty: Difficulty; topic: string; is_review: boolean }
  | { type: "daily_goal_completed"; task_id: number; slot: Slot; title: string }
  | { type: "level_up"; level: number }
  | { type: "task_due"; task_id: number; slot: Slot; title: string; minutes_left: number }
  | { type: "achievement_unlocked"; key: string; title: string }
  | { type: string; [k: string]: unknown };

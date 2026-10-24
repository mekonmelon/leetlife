-- LeetLife schema. Timestamps are unix seconds (INTEGER); dates are 'YYYY-MM-DD' (TEXT, local).
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS problems (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    slug         TEXT NOT NULL UNIQUE,
    title        TEXT NOT NULL,
    difficulty   TEXT NOT NULL CHECK (difficulty IN ('easy','medium','hard')),
    topic        TEXT NOT NULL,
    importance   REAL NOT NULL DEFAULT 0.5,
    leetcode_id  INTEGER
);

CREATE TABLE IF NOT EXISTS attempts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    problem_id    INTEGER NOT NULL REFERENCES problems(id),
    difficulty    TEXT NOT NULL,
    result        TEXT NOT NULL CHECK (result IN ('solved','failed')),
    duration_ms   INTEGER,
    attempted_at  INTEGER NOT NULL,
    topic         TEXT NOT NULL,
    source        TEXT NOT NULL DEFAULT 'manual',
    external_id   TEXT UNIQUE
);
CREATE INDEX IF NOT EXISTS idx_attempts_time ON attempts(attempted_at);
CREATE INDEX IF NOT EXISTS idx_attempts_problem ON attempts(problem_id);

CREATE TABLE IF NOT EXISTS topic_scores (
    topic          TEXT PRIMARY KEY,
    confidence     REAL NOT NULL DEFAULT 0.5,
    attempts       INTEGER NOT NULL DEFAULT 0,
    failures       INTEGER NOT NULL DEFAULT 0,
    last_reviewed  INTEGER
);

CREATE TABLE IF NOT EXISTS review_schedule (
    problem_id     INTEGER PRIMARY KEY REFERENCES problems(id),
    interval_idx   INTEGER NOT NULL DEFAULT 0,
    ease           REAL NOT NULL DEFAULT 2.5,
    due_at         INTEGER NOT NULL,
    last_reviewed  INTEGER NOT NULL,
    reviews        INTEGER NOT NULL DEFAULT 0,
    lapses         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_review_due ON review_schedule(due_at);

CREATE TABLE IF NOT EXISTS daily_tasks (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    date         TEXT NOT NULL,
    slot         TEXT NOT NULL,          -- breakfast | training | work | evening
    kind         TEXT NOT NULL,          -- JSON: {"type":"solve_difficulty","difficulty":"easy"} ...
    title        TEXT NOT NULL,
    target       INTEGER NOT NULL DEFAULT 1,
    progress     INTEGER NOT NULL DEFAULT 0,
    status       TEXT NOT NULL DEFAULT 'pending',   -- pending | done | missed
    deadline     INTEGER,
    reward_xp    INTEGER NOT NULL DEFAULT 0,
    reward_coins INTEGER NOT NULL DEFAULT 0,
    completed_at INTEGER,
    UNIQUE(date, slot)
);

CREATE TABLE IF NOT EXISTS avatar_state (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    hunger           INTEGER NOT NULL DEFAULT 70,
    energy           INTEGER NOT NULL DEFAULT 80,
    happiness        INTEGER NOT NULL DEFAULT 70,
    xp               INTEGER NOT NULL DEFAULT 0,
    level            INTEGER NOT NULL DEFAULT 1,
    coins            INTEGER NOT NULL DEFAULT 100,
    streak           INTEGER NOT NULL DEFAULT 0,
    best_streak      INTEGER NOT NULL DEFAULT 0,
    last_active_date TEXT,
    started_on       TEXT NOT NULL,
    updated_at       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,   -- furniture | tech | pet | decor | room
    cost        INTEGER NOT NULL,
    asset_path  TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    min_level   INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS inventory (
    item_id     INTEGER PRIMARY KEY REFERENCES items(id),
    acquired_at INTEGER NOT NULL,
    equipped    INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS rooms (
    room_id       TEXT PRIMARY KEY,
    upgrade_level INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    kind       TEXT NOT NULL,
    payload    TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_time ON events(created_at);

CREATE TABLE IF NOT EXISTS achievements (
    key         TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    unlocked_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS scheduled_jobs (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    kind    TEXT NOT NULL,      -- JSON
    run_at  INTEGER NOT NULL,
    status  TEXT NOT NULL DEFAULT 'pending'   -- pending | done | cancelled
);
CREATE INDEX IF NOT EXISTS idx_jobs_due ON scheduled_jobs(status, run_at);

CREATE TABLE IF NOT EXISTS metrics (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    count       INTEGER NOT NULL,
    mean_us     REAL NOT NULL,
    p50_us      REAL NOT NULL,
    p95_us      REAL NOT NULL,
    p99_us      REAL NOT NULL,
    max_us      REAL NOT NULL,
    recorded_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

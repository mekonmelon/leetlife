# LeetLife

A macOS menu-bar life sim for LeetCode practice. Your avatar lives in a house.
It gets fed when you solve an Easy before 10 AM, earns coins you spend on
furniture, and gets quietly disappointed when you skip a day.

Underneath the game it is a local-first, event-driven desktop system: a Rust
core that owns all state, scheduling and persistence, and a React UI that only
renders snapshots it is handed.

```
              macOS menu bar
                    │ click
                    ▼
         ┌──────────────────────┐
         │  React + TypeScript  │   house · tasks · shop · stats · settings
         └──────────┬───────────┘
                    │ Tauri IPC (typed commands, pushed events)
                    ▼
         ┌──────────────────────┐
         │      Rust core       │
         │  engine (reducers)   │
         │  events · scheduler  │
         │  recommendation      │
         │  game · activity     │
         │  cache · metrics     │
         └──────────┬───────────┘
                    ▼
                  SQLite
```

## The one design rule

**React renders the world; Rust owns the truth.**

The UI never decides whether the avatar ate breakfast. It asks the backend for a
snapshot. The backend decides that the solve satisfied the breakfast task,
applies the reward, writes the event, and hands back new state.

## How a solve flows through the system

Logging one attempt is a single SQLite transaction:

```
BEGIN
  insert attempt
  update topic confidence      (EMA, weighted by difficulty)
  advance the spaced-repetition ladder
  advance at most one daily task, pay its bonus
  apply XP / coins / hunger / energy / happiness, maybe level up
  touch the streak
  unlock any achievements
  append every resulting event to the events table   ← outbox
COMMIT
                    │ only after commit
                    ▼
  cancel that task's deadline jobs · refresh the snapshot cache
  publish events on the bus
                    │
    ┌───────────────┼────────────────┐
    ▼               ▼                ▼
notifications    UI push         (further subscribers)
```

Events are only published **after** the commit. Nothing announces a reward that
a rollback could take back.

## What is in the Rust core

| Module | What it does |
| --- | --- |
| `engine/` | The only thing that mutates state. One transactional reducer per action. |
| `events/` | `AppEvent` bus: bounded `mpsc` in, `broadcast` fan-out to subscribers. |
| `scheduler/` | Min-heap of jobs keyed by `run_at`. Sleeps until the next one instead of polling. |
| `recommendation/` | Problem scoring and an SM-2 style spaced-repetition ladder. |
| `game/` | Pure functions over `AvatarState`: rewards, levels, streaks, mood copy. |
| `storage/` | SQLx pool, migrations, seed catalogue, and every SQL statement. |
| `activity/` | `ActivityProvider` trait, LeetCode polling, loopback HTTP ingest. |
| `cache/` | The snapshot the menu-bar window reads. No SQLite on the click path. |
| `metrics/` | Bounded-reservoir latency series with p50/p95/p99. |
| `notifications/` | Which events are worth interrupting you for, and what they say. |
| `app/`, `commands/` | The Tauri shell: tray, window, IPC surface. Feature-gated. |

### Recommendations

Each problem is scored:

```
priority = 2.0·weakness + 1.0·staleness + 0.8·failure_rate
         + 1.0·importance + 1.5·review_due − 1.5·recent_exposure
```

Topic confidence is an exponential moving average whose rate depends on
difficulty, so one Hard says more than one Easy. A problem you solved but that
is not due yet scores *negative* on the review term: it is known right now.

Every recommendation carries the reason it was chosen, built from the same
terms, so the UI can say:

> **Number of Islands** — Graphs confidence is 41%, last Graphs practice was 17 days ago

### Spaced repetition

Intervals follow a 1 / 3 / 7 / 14 / 30 / 60 / 120-day ladder scaled by an ease
factor. A success steps up the ladder and nudges ease up; a failure drops two
rungs, lowers ease, and puts the problem back tomorrow.

### The daily plan

At 08:00 the scheduler builds four slots, each with its own deadline:

| Slot | Deadline | What it asks for |
| --- | --- | --- |
| Breakfast | 10:00 | One Easy. This is what feeds the avatar. |
| Training | 14:00 | Your weakest *practised* topic. |
| Work | 19:00 | One Medium, or a Hard at the weekend. |
| Evening | end of day | The problem most due for review. |

One attempt advances **at most one** slot, the earliest pending one it
satisfies. Solving a single Easy feeds you breakfast; it does not quietly clear
the whole day.

Missing a day is forgiven once. The streak only resets after two.

## Concurrency

Four Tokio tasks run behind the window, all cancelled together on quit:

| Worker | Job |
| --- | --- |
| Event dispatcher | Drains the bounded queue, fans out to subscribers, drains again on shutdown. |
| Scheduler | Sleeps until the next job is due. Wakes on any new job. |
| Notification worker | Turns events into native macOS notifications. |
| Ingest endpoint | Loopback HTTP for the optional browser extension. |

Writers serialise on one async mutex, because SQLite has one writer. That turns
"database is locked" retries into plain queueing. A test drives eight concurrent
solves and asserts the avatar's XP equals the sum of the rewards handed back.

## Latency instrumentation

Every significant operation is timed into a bounded reservoir (2048 most recent
samples plus running count/sum/max), so percentiles stay cheap and memory is
fixed no matter how long the app runs. Summaries flush to SQLite every 10
minutes, and the Stats tab shows them live.

Timed: tray click to first paint, each IPC command, the attempt transaction,
snapshot loads, recommendation generation, event dispatch, and notifications.

The `ui.open` figure is a real end-to-end measurement, not a guess: the tray
handler starts the timer, and the webview reports back on its first painted
frame.

## Running it

```bash
npm install
npm run tauri dev        # menu-bar app
npm run dev              # just the UI in a browser, backed by a mock
```

Tests and benchmarks:

```bash
npm test                                    # Vitest
cd src-tauri && cargo test --no-default-features   # core: 48 tests, no Tauri
cd src-tauri && cargo bench --no-default-features  # Criterion
```

The core builds without the desktop shell (`--no-default-features`). That keeps
Tauri out of the logic, and makes tests and benchmarks fast to compile.

## Getting your activity in

Three options, in increasing order of automation:

1. **By hand.** The Solve tab. Always works.
2. **Username polling.** Put your LeetCode username in Settings. LeetLife polls
   your public accepted submissions every 15 minutes.
3. **Browser extension.** `extension/` holds an unpacked Chrome extension that
   POSTs accepted submissions to `127.0.0.1:47831`. The app never touches your
   cookies or scrapes a page.

All three land in the same place: `ActivityProvider` normalises them into an
`ExternalAttempt`, deduplicated on `external_id`.

## Layout

```
src/                    React UI
  api/                  typed IPC client, shared types, browser mock
  house/  avatar/       the room and the character, as inline SVG
  components/ pages/    meters, tasks, toast; house/solve/shop/stats/settings
  analytics/            dependency-free charts
src-tauri/
  src/                  the Rust core (table above)
  migrations/           SQLite schema
  tests/                end-to-end tests against a real database
  benches/              Criterion
extension/              optional Chrome bridge
```

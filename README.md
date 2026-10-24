# LeetLife

A macOS menu-bar life sim for LeetCode practice. Your avatar lives in a house.
A Susuwatari soot sprite lives in a pixel-art cutaway house. Practice problems,
earn XP and coins, decorate your home, and watch your little world grow.
All progress saves on your Mac in SQLite. No account, server, cloud database,
or paid services are required. Manual logging works offline.

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

At local midnight (or first launch), the scheduler builds four slots, each with its own deadline:

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

## Open it on your Mac

**Double-click `LeetLife.command` in Finder**, or run:

```bash
npm start
```

The launcher finds your installed Node and Rust tools, builds the app on its
first run, and opens it. Once built, you can open the standalone application at
`src-tauri/target/release/bundle/macos/LeetLife.app`, or move that app to your
Applications folder. The built app needs neither Node nor Rust nor a running
terminal. This build is for personal use on your Mac; distribution to other
people would need a separate signing/notarization setup.

Open LeetLife from Finder to bring up its window, even when it is already
running. Click the animated spirit in the menu bar for a quick check-in with needs,
XP, coins, streak, and today’s next quest. Choose **Visit the cottage** for the
full house. The check-in tucks away when you click outside. Drag the full window by its top strip to move it, resize it from its edges,
or click the dash to minimize. Reopening preserves its current position.
Press Escape to tuck away either window. Settings has an optional **Open LeetLife
when I log in** switch. Normal launches open the popup; login launches stay quiet.

### Your Ghibli house

The window opens directly into one original pixel-art cutaway house. Susuwatari
walks along the floor, climbs each stair, makes tea, reads, sweeps, and sleeps in
the loft. Each visit starts at a random activity. Click a spot to send it there;
at night it eventually returns to bed. Reduced Motion pauses autonomous walking.

The small dock opens **Log problem**, **Shop**, **Decorate**, **Quests**,
**Journal**, and **Settings**. Keys 1–6 open the same actions. Escape closes a
menu, finishes decorating, or tucks away the game. The top strip drags the window;
the upper-right controls minimize or hide it. The menu bar still offers a quick
check-in and a shortcut back to the game.

Use **Decorate**, choose a part of the house and an owned treasure, then click a
tile to place it. **Place here** also places it at a default position. Wallpaper
changes affect the chosen wall; old linen is free. Existing ownership and saved
placements carry over into the pixel house. The shop includes pixel versions of
Haku, Jiji, the Totoros, and other familiar companions.

At **level 3**, the **Backyard** gate opens to a separate garden. Choose one of
six beds and plant a daisy, poppy, or moonflower for 8 coins. Water it once, then
let it bloom over 30 minutes from planting; flowers grow while the app is closed.
Picking a mature flower earns 12 coins and frees the bed. Unlocks, spending,
watering, and picking are validated and saved together by Rust. See the
[artwork notes](assets/ARTWORK.md) for the pixel renderer sources.

### Your save file

Progress is stored in:

```text
~/Library/Application Support/dev.leetlife.app/leetlife.db
```

Your attempts, XP, coins, daily quests, furniture, review schedule, and settings
persist across launches. Closing the app or rebuilding it does not reset them.
In **Settings → Back up my world**, SQLite creates a consistent standalone copy
in the save folder's `backups/` directory and reveals it in Finder. Keep a copy
elsewhere if you'd like protection from losing the Mac itself. **Show save
folder** opens the location without changing your save.

To restore a backup: quit LeetLife completely, move the existing `leetlife.db`
and any `leetlife.db-wal` / `leetlife.db-shm` files together to a safe folder,
copy your chosen backup into the save folder as `leetlife.db`, then reopen the
app. Never replace a database while LeetLife is running.

### Development

Build prerequisites: macOS Command Line Tools, Node 22.12+ (Node 24 recommended;
`.nvmrc` is included), and Rust. The launcher handles tools installed through
nvm and rustup even when Finder or the terminal hasn't loaded their shell setup.
See [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/) for setup.

```bash
npm ci
npm run desktop         # native app, hot reload
npm run build:mac       # standalone .app, no DMG or developer account needed
npm start               # open the most recently built .app
npm run dev             # browser demo with clearly labelled sample data
```

After changing the source, use `npm run build:mac` to update the standalone app.
The browser demo is for previewing the interface; its sample progress resets
on refresh. Use the Mac app for your actual saved game.

```bash
npm test
npm run build
export PATH="$HOME/.cargo/bin:$PATH"
cd src-tauri
cargo test --no-default-features --locked
cargo check --all-targets --locked
```

The tests include disk-backed save/reopen, WAL-safe backups, midnight rollover,
long absences, concurrent writes, task rewards, and external-submission deduplication.

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
  game/  house/         pixel scenes, tile navigation, garden and decorating
  avatar/              Susuwatari portrait for the menu bar check-in
  components/ pages/    meters, tasks, toast; house/solve/shop/stats/settings
  analytics/            dependency-free charts
src-tauri/
  src/                  the Rust core (table above)
  migrations/           SQLite schema
  tests/                end-to-end tests against a real database
  benches/              Criterion
extension/              optional Chrome bridge
```

Native window regression check (after building and installing):

```bash
python3 scripts/test_macos_open.py
```

This checks a normal launch, restarts the real Mac app as a quiet login launch,
then checks that opening it again restores the same process's visible window. It leaves the game open
and does not modify your practice history.

// The single place the UI talks to Rust. Outside Tauri (plain `vite dev`, or
// tests) it falls back to an in-memory mock so the house still renders.
import type {
  AttemptOutcome, AppEvent, LogAttemptInput, MetricSummary, ProblemMeta,
  Recommendation, SettingsView, ShopItem, Snapshot, Stats, SyncReport, AttemptRow,
} from "./types";

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type Listen = (event: string, handler: (payload: unknown) => void) => Promise<() => void>;

export const isTauri = (): boolean =>
  typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

let invokeImpl: Invoke | null = null;
let listenImpl: Listen | null = null;

async function getInvoke(): Promise<Invoke> {
  if (invokeImpl) return invokeImpl;
  if (isTauri()) {
    const core = await import("@tauri-apps/api/core");
    invokeImpl = core.invoke as Invoke;
  } else {
    const { mockInvoke } = await import("./mock");
    invokeImpl = mockInvoke;
  }
  return invokeImpl;
}

async function getListen(): Promise<Listen> {
  if (listenImpl) return listenImpl;
  if (isTauri()) {
    const ev = await import("@tauri-apps/api/event");
    listenImpl = ((name, handler) =>
      ev.listen(name, (e) => handler(e.payload))) as Listen;
  } else {
    const { mockListen } = await import("./mock");
    listenImpl = mockListen;
  }
  return listenImpl;
}

/** Test seam: point the client at a stub. */
export function __setTransport(invoke: Invoke | null, listen?: Listen | null): void {
  invokeImpl = invoke;
  if (listen !== undefined) listenImpl = listen;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const invoke = await getInvoke();
  return invoke<T>(cmd, args);
}

export const api = {
  getSnapshot: () => call<Snapshot>("get_snapshot"),
  refreshSnapshot: () => call<Snapshot>("refresh_snapshot"),
  logAttempt: (input: LogAttemptInput) => call<AttemptOutcome>("log_attempt", { input }),
  purchaseItem: (itemId: number) => call<Snapshot>("purchase_item", { itemId }),
  setEquipped: (itemId: number, equipped: boolean) => call<Snapshot>("set_equipped", { itemId, equipped }),
  getShop: () => call<ShopItem[]>("get_shop"),
  getStats: () => call<Stats>("get_stats"),
  getRecommendations: (args?: { difficulty?: string; topic?: string; only_due?: boolean; limit?: number }) =>
    call<Recommendation[]>("get_recommendations", { args: args ?? null }),
  searchProblems: (query: string, limit?: number) => call<ProblemMeta[]>("search_problems", { query, limit }),
  listTopics: () => call<string[]>("list_topics"),
  getRecentAttempts: (limit?: number) => call<AttemptRow[]>("get_recent_attempts", { limit }),
  getMetrics: () => call<MetricSummary[]>("get_metrics"),
  getSettings: () => call<SettingsView>("get_settings"),
  setSetting: (key: string, value: string) => call<void>("set_setting", { key, value }),
  setAutostart: (enabled: boolean) => call<boolean>("set_autostart", { enabled }),
  syncNow: () => call<SyncReport>("sync_now"),
  uiRendered: (renderMs?: number) => call<void>("ui_rendered", { renderMs: renderMs ?? null }),
  hideWindow: () => call<void>("hide_window"),
  quitApp: () => call<void>("quit_app"),
};

/** Subscribes to a backend event; resolves to an unlisten function. */
export async function on(event: "state-changed" | "app-event" | "window-shown" | "nudge", handler: (payload: unknown) => void): Promise<() => void> {
  const listen = await getListen();
  return listen(event, handler);
}

export type { AppEvent };

// Owns the one piece of frontend state: the latest snapshot. The backend pushes
// `state-changed` after every committed write, so this never polls.
import { useCallback, useEffect, useRef, useState } from "react";
import { api, isTauri, on } from "./api/client";
import type { Snapshot } from "./api/types";

export interface LeetLifeState {
  snapshot: Snapshot | null;
  error: string | null;
  notice: string | null;
  now: number;
  justRewarded: boolean;
  applySnapshot: (s: Snapshot, message?: string) => void;
  setError: (m: string | null) => void;
  setNotice: (m: string | null) => void;
  reload: () => Promise<void>;
}

/** Strips the `Error: ` / anyhow wrapper Tauri adds so toasts read like prose. */
export function cleanError(raw: unknown): string {
  const s = String(raw);
  return s.replace(/^Error:\s*/, "").replace(/^"|"$/g, "");
}

export function useLeetLife(): LeetLifeState {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setErrorState] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const [justRewarded, setJustRewarded] = useState(false);
  const rewardTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const firstPaint = useRef(true);

  const setError = useCallback((m: string | null) => setErrorState(m === null ? null : cleanError(m)), []);

  const applySnapshot = useCallback((s: Snapshot, message?: string) => {
    setSnapshot((prev) => (prev && prev.version > s.version ? prev : s));
    if (message) {
      setNotice(message);
      setJustRewarded(true);
      if (rewardTimer.current) clearTimeout(rewardTimer.current);
      rewardTimer.current = setTimeout(() => setJustRewarded(false), 2400);
    }
  }, []);

  const reload = useCallback(async () => {
    try {
      applySnapshot(await api.refreshSnapshot());
    } catch (e) {
      setError(String(e));
    }
  }, [setError, applySnapshot]);

  // Initial load, then report first-paint latency back to Rust.
  useEffect(() => {
    const started = performance.now();
    let live = true;
    (async () => {
      try {
        const s = await api.getSnapshot();
        if (!live) return;
        setSnapshot(s);
        requestAnimationFrame(() => {
          if (firstPaint.current) {
            firstPaint.current = false;
            void api.uiRendered(performance.now() - started).catch(() => {});
          }
        });
      } catch (e) {
        if (live) setError(String(e));
      }
    })();
    return () => {
      live = false;
    };
  }, [setError]);

  // Backend pushes.
  useEffect(() => {
    const unsubs: (() => void)[] = [];
    let live = true;
    (async () => {
      const a = await on("state-changed", (p) => applySnapshot(p as Snapshot));
      const b = await on("window-shown", () => void reload());
      if (!live) {
        a();
        b();
        return;
      }
      unsubs.push(a, b);
    })().catch((e) => { if (live) setError(String(e)); });
    return () => {
      live = false;
      for (const u of unsubs) u();
    };
  }, [applySnapshot, reload, setError]);

  // Deadlines are shown as "25m left", so the clock ticks once a minute.
  useEffect(() => {
    const t = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 30_000);
    return () => clearInterval(t);
  }, []);

  // Keep an open popup current when a study session crosses local midnight.
  useEffect(() => {
    if (!isTauri() || !snapshot) return;
    const date = new Date(now * 1000);
    const today = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    if (snapshot.date !== today) void reload();
  }, [now, snapshot?.date, reload]);

  useEffect(() => () => {
    if (rewardTimer.current) clearTimeout(rewardTimer.current);
  }, []);

  return { snapshot, error, notice, now, justRewarded, applySnapshot, setError, setNotice, reload };
}

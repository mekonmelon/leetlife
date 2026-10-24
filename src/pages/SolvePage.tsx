import { useEffect, useMemo, useState } from "react";
import { api } from "../api/client";
import { DIFFICULTY_LABEL } from "../api/format";
import type { Difficulty, ProblemMeta, Snapshot } from "../api/types";

export function SolvePage({
  onChanged,
  onError,
}: {
  onChanged: (s: Snapshot, message?: string) => void;
  onError: (message: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ProblemMeta[]>([]);
  const [selected, setSelected] = useState<ProblemMeta | null>(null);
  const [minutes, setMinutes] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    const t = setTimeout(async () => {
      if (query.trim().length < 2) {
        setResults([]);
        return;
      }
      try {
        const r = await api.searchProblems(query.trim(), 8);
        if (live) setResults(r);
      } catch (e) {
        onError(String(e));
      }
    }, 160);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [query, onError]);

  const durationMs = useMemo(() => {
    const m = Number(minutes);
    return Number.isFinite(m) && m > 0 ? Math.round(m * 60_000) : null;
  }, [minutes]);

  async function log(result: "solved" | "failed") {
    if (!selected) return;
    setBusy(true);
    try {
      const out = await api.logAttempt({ result, slug: selected.slug, duration_ms: durationMs });
      const bits = [`+${out.reward.xp} XP`, `+${out.reward.coins} coins`];
      if (out.level_up) bits.push(`level ${out.level_up}!`);
      if (out.achievements.length) bits.push(out.achievements.join(", "));
      onChanged(out.snapshot, bits.join(" · "));
      setSelected(null);
      setQuery("");
      setMinutes("");
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page">
      <h2>Log an attempt</h2>
      <input
        className="search"
        placeholder="Search a problem…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        aria-label="Search a problem"
      />

      {selected ? (
        <div className="selected" data-testid="selected-problem">
          <div className="rec-main">
            <span className="rec-title">{selected.title}</span>
            <span className="pill" data-difficulty={selected.difficulty}>
              {DIFFICULTY_LABEL[selected.difficulty as Difficulty]}
            </span>
          </div>
          <p className="rec-reason">{selected.topic}</p>
          <label className="field">
            Minutes spent (optional)
            <input inputMode="numeric" value={minutes} onChange={(e) => setMinutes(e.target.value)} placeholder="25" />
          </label>
          <div className="row">
            <button className="primary" onClick={() => log("solved")} disabled={busy}>
              Solved
            </button>
            <button onClick={() => log("failed")} disabled={busy}>
              Couldn't get it
            </button>
            <button className="ghost" onClick={() => setSelected(null)} disabled={busy}>
              Back
            </button>
          </div>
        </div>
      ) : (
        <ul className="results" data-testid="results">
          {results.map((p) => (
            <li key={p.id}>
              <button onClick={() => setSelected(p)}>
                <span>{p.title}</span>
                <span className="pill" data-difficulty={p.difficulty}>
                  {DIFFICULTY_LABEL[p.difficulty as Difficulty]}
                </span>
              </button>
            </li>
          ))}
          {query.trim().length >= 2 && results.length === 0 && <li className="empty">No matches.</li>}
        </ul>
      )}
    </div>
  );
}

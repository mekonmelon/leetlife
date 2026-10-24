import { Icon } from "../components/Icon";
import { useEffect, useMemo, useState } from "react";
import { api } from "../api/client";
import { DIFFICULTY_LABEL } from "../api/format";
import type { Difficulty, ProblemMeta, Snapshot } from "../api/types";

function problemSlug(query: string) {
  const value = query.trim();
  const match = value.match(/^https?:\/\/(?:www\.)?leetcode\.com\/problems\/([a-z0-9-]+)/i);
  return match?.[1] ?? value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}
export function SolvePage({ onChanged, onError }: { onChanged: (s: Snapshot, message?: string) => void; onError: (message: string) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ProblemMeta[]>([]);
  const [selected, setSelected] = useState<ProblemMeta | null>(null);
  const [minutes, setMinutes] = useState("");
  const [busy, setBusy] = useState(false);
  const [searching, setSearching] = useState(false);
  const [difficulty, setDifficulty] = useState<Difficulty>("easy");
  const [topic, setTopic] = useState("General");

  useEffect(() => {
    let live = true;
    setResults([]);
    setSearching(query.trim().length >= 2);
    const timer = setTimeout(async () => {
      if (query.trim().length < 2) return;
      try { const r = await api.searchProblems(query.includes("leetcode.com/") ? problemSlug(query) : query.trim(), 10); if (live) setResults(r); }
      catch (e) { if (live) onError(String(e)); }
      finally { if (live) setSearching(false); }
    }, 180);
    return () => { live = false; clearTimeout(timer); };
  }, [query, onError]);

  const durationMs = useMemo(() => {
    const m = Number(minutes);
    return Number.isFinite(m) && m > 0 ? Math.round(m * 60_000) : null;
  }, [minutes]);

  async function log(result: "solved" | "failed") {
    if (!selected) return;
    setBusy(true);
    try {
      const out = await api.logAttempt({ result, slug: selected.slug, title: selected.title, difficulty: selected.id === -1 ? difficulty : selected.difficulty, topic: selected.id === -1 ? topic.trim() || "General" : selected.topic, duration_ms: durationMs });
      const bits = [`+${out.reward.xp} XP`, `+${out.reward.coins} coins`];
      if (out.level_up) bits.push(`level ${out.level_up}!`);
      if (out.achievements.length) bits.push(out.achievements.join(", "));
      onChanged(out.snapshot, bits.join(" · "));
      setSelected(null); setQuery(""); setMinutes("");
    } catch (e) { onError(String(e)); } finally { setBusy(false); }
  }
  return (
    <div className="page">
      <h1 className="page-title">Plant a little progress.</h1>
      <p className="page-subtitle">A solve, a retry, a small step. It all counts.</p>
      {!selected && <input className="search" placeholder="Search a problem or paste a LeetCode link…" value={query} onChange={e => setQuery(e.target.value)} aria-label="Search a problem" />}
      {selected ? <div className="selected" data-testid="selected-problem">
        <div className="rec-main"><span className="rec-title">{selected.title}</span><span className="pill" data-difficulty={selected.id === -1 ? difficulty : selected.difficulty}>{DIFFICULTY_LABEL[selected.id === -1 ? difficulty : selected.difficulty]}</span></div>
        <button className="ghost" onClick={() => void api.openProblem(selected.slug).catch(e => onError(String(e)))}>Open on LeetCode ↗</button>
        {selected.id === -1 ? <><label className="field">Difficulty<select value={difficulty} onChange={e => setDifficulty(e.target.value as Difficulty)}><option value="easy">Easy</option><option value="medium">Medium</option><option value="hard">Hard</option></select></label><label className="field">Topic<input value={topic} onChange={e => setTopic(e.target.value)} placeholder="e.g. Arrays" /></label></> : <p className="rec-reason">{selected.topic}</p>}
        <label className="field">Minutes spent (optional)<input type="number" min="1" max="1440" value={minutes} onChange={e => setMinutes(e.target.value)} placeholder="25" /></label>
        <div className="row"><button className="primary" onClick={() => void log("solved")} disabled={busy}>{busy ? "Saving…" : "I solved it!"}</button><button onClick={() => void log("failed")} disabled={busy}>I gave it a try</button><button className="ghost" onClick={() => setSelected(null)} disabled={busy}>Back</button></div>
        <p className="hint">Log an attempt after practicing. Even a tricky one earns a little XP.</p>
      </div> : <>
        <ul className="results" data-testid="results">{results.map(p => <li key={p.id}><button onClick={() => setSelected(p)}><span>{p.title}</span><span className="pill" data-difficulty={p.difficulty}>{DIFFICULTY_LABEL[p.difficulty]}</span></button></li>)}</ul>
        {searching && <p className="empty">Looking through the problem garden…</p>}
        {query.trim().length >= 2 && !searching && results.length === 0 && <div className="intro-card"><h2>A new seed for your collection</h2><p>This problem isn't in your local collection yet. Add it with its difficulty and topic.</p><button disabled={!problemSlug(query)} onClick={() => setSelected({id: -1, slug: problemSlug(query), title: problemSlug(query).split("-").map(w => w.charAt(0).toUpperCase()+w.slice(1)).join(" "), difficulty, topic, importance: .5})}>Add this problem</button></div>}
        {query.trim().length < 2 && <div className="intro-card"><span className="intro-sprout"><Icon name="leaf" size={34}/></span><h2>Your effort makes this place grow.</h2><p>Find the problem you practiced, then log a solve or an attempt. Earn coins to decorate your room, build your streak, and keep your buddy happy.</p><p>Start with a classic: try searching “Two Sum”.</p></div>}
      </>}
    </div>
  );
}

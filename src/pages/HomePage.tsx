import { Icon } from "../components/Icon";
import { useState } from "react";
import { Meters, XpBar } from "../components/Meters";
import { TaskList } from "../components/TaskList";
import { DIFFICULTY_LABEL } from "../api/format";
import { api } from "../api/client";
import type { DailyTask, Snapshot } from "../api/types";

export function HomePage({
  snapshot,
  now,
  justRewarded: _justRewarded,
  onChanged,
  onError,
  onSolve,
  onShop: _onShop,
}: {
  snapshot: Snapshot;
  now: number;
  justRewarded: boolean;
  onChanged: (s: Snapshot, message?: string) => void;
  onError: (message: string) => void;
  onSolve: () => void;
  onShop: () => void;
}) {
  const [busyId, setBusyId] = useState<number | null>(null);
  const [busySlug, setBusySlug] = useState<string | null>(null);

  /** Finds a problem that actually satisfies `task`, so "Done" never logs
   *  something that leaves the task pending. Falls back to asking the backend
   *  for a filtered recommendation when the cached three do not match. */
  async function slugFor(task: DailyTask): Promise<string | null> {
    const kind = task.kind;
    if (kind.type === "review") return kind.slug;

    const matches = (r: { problem: { difficulty: string; topic: string } }) =>
      kind.type === "solve_difficulty"
        ? r.problem.difficulty === kind.difficulty
        : kind.type === "solve_topic"
          ? r.problem.topic === kind.topic
          : true;

    const cached = snapshot.recommendations.find(matches);
    if (cached) return cached.problem.slug;

    const fetched = await api.getRecommendations({
      difficulty: kind.type === "solve_difficulty" ? kind.difficulty : undefined,
      topic: kind.type === "solve_topic" ? kind.topic : undefined,
      limit: 1,
    });
    return fetched[0]?.problem.slug ?? null;
  }

  async function completeTask(task: DailyTask) {
    setBusyId(task.id);
    let slug: string | null = null;
    try {
      slug = await slugFor(task);
    } catch (e) {
      onError(String(e));
      setBusyId(null);
      return;
    }
    if (!slug) {
      onError("No matching problem left in the catalogue — log it from the Solve tab.");
      setBusyId(null);
      return;
    }
    try {
      const out = await api.logAttempt({ result: "solved", slug });
      onChanged(out.snapshot, `+${out.reward.xp} XP, +${out.reward.coins} coins`);
    } catch (e) {
      onError(String(e));
    } finally {
      setBusyId(null);
    }
  }

  async function logSolved(slug: string) {
    setBusySlug(slug);
    try {
      const out = await api.logAttempt({ result: "solved", slug });
      onChanged(out.snapshot, `+${out.reward.xp} XP, +${out.reward.coins} coins`);
    } catch (e) {
      onError(String(e));
    } finally {
      setBusySlug(null);
    }
  }

  return (
    <div className="page home-page">
      <div><span className="eyebrow">ONE SMALL QUEST AT A TIME</span><h1>Your quest book</h1></div>
      <button className="primary practice-button" onClick={onSolve}><span className="practice-title"><Icon name="leaf" size={16}/>Log your practice</span><span>Every little effort grows a little world <Icon name="arrow" size={13}/></span></button>
      <div className="buddy-card"><div className="buddy-head"><div><strong>Susuwatari, your little soot sprite</strong><span>{snapshot.avatar.level < 3 ? "Small steps, big adventures." : "Growing right alongside you."}</span></div><span className="level-pill">LV. {snapshot.avatar.level}</span></div><Meters avatar={snapshot.avatar} /><XpBar into={snapshot.xp_into_level} needed={snapshot.xp_for_next_level} level={snapshot.avatar.level} /></div>

      <section><div className="section-heading"><h2>Little quests</h2><span>{snapshot.tasks.filter(t => t.status === "done").length} / {snapshot.tasks.length} complete</span></div><TaskList tasks={snapshot.tasks} now={now} onComplete={completeTask} busyId={busyId} /></section>

      {snapshot.recommendations.length > 0 && (
        <section className="recs" data-testid="recommendations">
          <div className="section-heading"><h2>A seed for your next session</h2><span>picked for you ✧</span></div>
          {snapshot.recommendations.slice(0, 2).map((rec) => (
            <div className="rec" key={rec.problem.id}>
              <div className="rec-main">
                <a
                  className="rec-title"
                  href={`https://leetcode.com/problems/${rec.problem.slug}/`}
                  target="_blank"
                  rel="noreferrer"
                  onClick={(e) => { e.preventDefault(); void api.openProblem(rec.problem.slug).catch((err) => onError(String(err))); }}
                >
                  {rec.problem.title}
                </a>
                <span className="pill" data-difficulty={rec.problem.difficulty}>
                  {DIFFICULTY_LABEL[rec.problem.difficulty]}
                </span>
                {rec.is_review && <span className="pill pill-review">review</span>}
              </div>
              <p className="rec-reason">{rec.reason}</p>
              <button className="rec-log" onClick={() => logSolved(rec.problem.slug)} disabled={busySlug === rec.problem.slug}>
                {busySlug === rec.problem.slug ? "…" : "I solved it"}
              </button>
            </div>
          ))}
        </section>
      )}
    </div>
  );
}

import { useEffect, useState } from "react";
import { ActivityChart, TopicBars } from "../analytics/Charts";
import { formatMicros, relativeTime } from "../api/format";
import { api } from "../api/client";
import type { MetricSummary, Stats } from "../api/types";

const METRIC_LABELS: Record<string, string> = {
  "ui.open": "Tray click → painted",
  "ipc.get_snapshot": "Snapshot IPC",
  "db.attempt_tx": "Attempt transaction",
  "recommend.generate": "Recommendation",
  "event.dispatch": "Event dispatch",
  "cache.refresh": "Cache refresh",
};

export function StatsPage({ now, onError }: { now: number; onError: (m: string) => void }) {
  const [stats, setStats] = useState<Stats | null>(null);
  const [metrics, setMetrics] = useState<MetricSummary[]>([]);

  useEffect(() => {
    let live = true;
    (async () => {
      try {
        const [s, m] = await Promise.all([api.getStats(), api.getMetrics()]);
        if (!live) return;
        setStats(s);
        setMetrics(m);
      } catch (e) {
        onError(String(e));
      }
    })();
    return () => {
      live = false;
    };
  }, [onError]);

  if (!stats) return <div className="page"><p className="empty">Loading…</p></div>;

  const shownMetrics = metrics.filter((m) => m.name in METRIC_LABELS).sort((a, b) => b.p95_us - a.p95_us);

  return (
    <div className="page">
      <section>
        <h2>Last 14 days</h2>
        <ActivityChart days={stats.last_14_days} />
        <div className="stat-row">
          <span><strong>{stats.total_solved}</strong> problems solved</span>
          <span><strong>{stats.total_attempts}</strong> attempts</span>
        </div>
        <div className="stat-row">
          {stats.solved_by_difficulty.map(([d, n]) => (
            <span key={d}>
              <span className="pill" data-difficulty={d}>{d}</span> {n}
            </span>
          ))}
        </div>
      </section>

      <section>
        <h2>Weakest topics</h2>
        <TopicBars topics={stats.topics} />
      </section>

      {stats.due_reviews.length > 0 && (
        <section>
          <h2>Due for review</h2>
          <ul className="plain">
            {stats.due_reviews.slice(0, 5).map((p) => (
              <li key={p.id}>
                {p.title} <span className="pill" data-difficulty={p.difficulty}>{p.difficulty}</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      {stats.achievements.length > 0 && (
        <section>
          <h2>Achievements</h2>
          <ul className="plain" data-testid="achievements">
            {stats.achievements.map((a) => (
              <li key={a.key}>🏆 {a.title}</li>
            ))}
          </ul>
        </section>
      )}

      {shownMetrics.length > 0 && (
        <section>
          <h2>Latency</h2>
          <table className="metrics" data-testid="metrics-table">
            <thead>
              <tr><th>Operation</th><th>n</th><th>p50</th><th>p95</th><th>p99</th></tr>
            </thead>
            <tbody>
              {shownMetrics.map((m) => (
                <tr key={m.name}>
                  <td>{METRIC_LABELS[m.name]}</td>
                  <td>{m.count}</td>
                  <td>{formatMicros(m.p50_us)}</td>
                  <td>{formatMicros(m.p95_us)}</td>
                  <td>{formatMicros(m.p99_us)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      <section>
        <h2>Recent activity</h2>
        <ul className="events">
          {stats.recent_events.slice(0, 8).map((e) => (
            <li key={e.id}>
              <span className="event-kind">{e.kind.replace(/_/g, " ")}</span>
              <span className="event-time">{relativeTime(e.created_at, now)}</span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

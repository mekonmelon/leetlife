// Small dependency-free charts. Both are plain SVG so they stay crisp in a
// 380px popup and cost nothing to render.
import { confidenceLabel } from "../api/format";
import type { DayCount, TopicScore } from "../api/types";

export function ActivityChart({ days }: { days: DayCount[] }) {
  const max = Math.max(1, ...days.map((d) => d.solved + d.failed));
  const W = 320;
  const H = 72;
  const gap = 3;
  const barW = (W - gap * (days.length - 1)) / days.length;

  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H + 14}`} role="img" aria-label="Attempts over the last 14 days" data-testid="activity-chart">
      {days.map((d, i) => {
        const x = i * (barW + gap);
        const solvedH = (d.solved / max) * H;
        const failedH = (d.failed / max) * H;
        return (
          <g key={d.date}>
            <title>{`${d.date}: ${d.solved} solved, ${d.failed} failed`}</title>
            <rect x={x} y={H - failedH} width={barW} height={failedH} fill="#7d5a6b" rx="1.5" />
            <rect x={x} y={H - failedH - solvedH} width={barW} height={solvedH} fill="#6ec7a8" rx="1.5" />
            {i % 3 === 0 && (
              <text x={x + barW / 2} y={H + 11} textAnchor="middle" fontSize="7" fill="#8b93a7">
                {d.date.slice(8)}
              </text>
            )}
          </g>
        );
      })}
      <line x1="0" y1={H} x2={W} y2={H} stroke="#59627f" strokeWidth="0.6" />
    </svg>
  );
}

export function TopicBars({ topics }: { topics: TopicScore[] }) {
  const shown = [...topics].sort((a, b) => a.confidence - b.confidence).slice(0, 6);
  if (shown.length === 0) return <p className="empty">Solve a few problems to build a topic profile.</p>;

  return (
    <ul className="topics" data-testid="topic-bars">
      {shown.map((t) => {
        const pct = Math.round(t.confidence * 100);
        return (
          <li key={t.topic} title={`${t.attempts} attempts, ${t.failures} failed`}>
            <span className="topic-name">{t.topic}</span>
            <div className="topic-track">
              <div className="topic-fill" data-level={confidenceLabel(t.confidence)} style={{ width: `${pct}%` }} />
            </div>
            <span className="topic-pct">{pct}%</span>
          </li>
        );
      })}
    </ul>
  );
}

import { SLOT_ICON, SLOT_LABEL, timeLeft } from "../api/format";
import type { DailyTask } from "../api/types";

export function TaskList({
  tasks,
  now,
  onComplete,
  busyId,
}: {
  tasks: DailyTask[];
  now: number;
  onComplete: (task: DailyTask) => void;
  busyId: number | null;
}) {
  if (tasks.length === 0) return <p className="empty">No plan yet today.</p>;

  return (
    <ul className="tasks" data-testid="tasks">
      {tasks.map((task) => {
        const left = timeLeft(task, now);
        return (
          <li key={task.id} className="task" data-status={task.status} data-slot={task.slot}>
            <div className="task-head">
              <span className="task-slot">
                <span aria-hidden>{SLOT_ICON[task.slot]}</span> {SLOT_LABEL[task.slot]}
              </span>
              {left && <span className="task-left" data-overdue={left === "overdue" ? "true" : undefined}>{left}</span>}
            </div>
            <div className="task-body">
              <span className="task-title">{task.title}</span>
              <span className="task-progress">
                [{task.progress}/{task.target}]
              </span>
              {task.status === "pending" ? (
                <button
                  className="task-done"
                  onClick={() => onComplete(task)}
                  disabled={busyId === task.id}
                  aria-label={`Mark ${task.title} complete`}
                >
                  {busyId === task.id ? "…" : "Done"}
                </button>
              ) : (
                <span className="task-badge" data-status={task.status}>
                  {task.status === "done" ? "✓" : "missed"}
                </span>
              )}
            </div>
            <div className="task-reward">
              +{task.reward_xp} XP · +{task.reward_coins} coins
            </div>
          </li>
        );
      })}
    </ul>
  );
}

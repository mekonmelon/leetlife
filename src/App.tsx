import { useCallback, useState } from "react";
import { HomePage } from "./pages/HomePage";
import { SolvePage } from "./pages/SolvePage";
import { ShopPage } from "./pages/ShopPage";
import { StatsPage } from "./pages/StatsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { Toast } from "./components/Toast";
import { useLeetLife } from "./useLeetLife";
import { formatCoins } from "./api/format";

const TABS = [
  { id: "home", label: "House" },
  { id: "solve", label: "Solve" },
  { id: "shop", label: "Shop" },
  { id: "stats", label: "Stats" },
  { id: "settings", label: "⚙" },
] as const;

type TabId = (typeof TABS)[number]["id"];

export default function App() {
  const { snapshot, error, notice, now, justRewarded, applySnapshot, setError, setNotice, reload } = useLeetLife();
  const [tab, setTab] = useState<TabId>("home");

  const onError = useCallback((m: string) => setError(m), [setError]);
  const onNotice = useCallback((m: string) => setNotice(m), [setNotice]);

  if (!snapshot) {
    return (
      <div className="app loading">
        <p className="empty">{error ?? "Waking up…"}</p>
        {error && <button onClick={() => { setError(null); void reload(); }}>Retry</button>}
      </div>
    );
  }

  return (
    <div className="app">
      <header className="topbar">
        <div className="topbar-left">
          <span className="day">
            {snapshot.weekday} — Day {snapshot.day_number}
          </span>
        </div>
        <div className="topbar-right">
          <span title="coins">💰 {formatCoins(snapshot.avatar.coins)}</span>
          <span title={`best ${snapshot.avatar.best_streak} days`}>🔥 {snapshot.avatar.streak}</span>
          {snapshot.due_reviews > 0 && <span title="reviews due">🔁 {snapshot.due_reviews}</span>}
        </div>
      </header>

      <nav className="tabs" role="tablist">
        {TABS.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            className={tab === t.id ? "tab active" : "tab"}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <main className="content">
        {tab === "home" && (
          <HomePage snapshot={snapshot} now={now} justRewarded={justRewarded} onChanged={applySnapshot} onError={onError} />
        )}
        {tab === "solve" && <SolvePage onChanged={applySnapshot} onError={onError} />}
        {tab === "shop" && <ShopPage snapshot={snapshot} onChanged={applySnapshot} onError={onError} />}
        {tab === "stats" && <StatsPage now={now} onError={onError} />}
        {tab === "settings" && <SettingsPage onError={onError} onNotice={onNotice} />}
      </main>

      <Toast message={error ?? notice} onDismiss={() => (error ? setError(null) : setNotice(null))} />
    </div>
  );
}

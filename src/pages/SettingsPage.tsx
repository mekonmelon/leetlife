import { useEffect, useState } from "react";
import { api } from "../api/client";
import type { SettingsView } from "../api/types";

export function SettingsPage({ onError, onNotice }: { onError: (m: string) => void; onNotice: (m: string) => void }) {
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [username, setUsername] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const s = await api.getSettings();
        setSettings(s);
        setUsername(s.values.leetcode_username ?? "");
      } catch (e) {
        onError(String(e));
      }
    })();
  }, [onError]);

  if (!settings) return <div className="page"><p className="empty">Loading…</p></div>;

  async function saveUsername() {
    setBusy(true);
    try {
      await api.setSetting("leetcode_username", username.trim());
      onNotice(username.trim() ? `Syncing ${username.trim()}'s accepted submissions` : "LeetCode sync off");
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function toggleAutostart() {
    try {
      const enabled = await api.setAutostart(!settings!.autostart);
      setSettings({ ...settings!, autostart: enabled });
    } catch (e) {
      onError(String(e));
    }
  }

  async function syncNow() {
    setBusy(true);
    try {
      const r = await api.syncNow();
      onNotice(r.error ? `Sync failed: ${r.error}` : `${r.provider}: imported ${r.imported} of ${r.fetched}`);
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page">
      <h2>Settings</h2>

      <label className="field">
        LeetCode username
        <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="leave blank for manual logging" />
      </label>
      <div className="row">
        <button className="primary" onClick={saveUsername} disabled={busy}>Save</button>
        <button onClick={syncNow} disabled={busy}>Sync now</button>
      </div>
      <p className="hint">
        Polls your public accepted submissions every 15 minutes. A browser extension can also POST to
        {" "}<code>http://127.0.0.1:{settings.ingest_port}/attempt</code>.
      </p>

      <label className="check">
        <input type="checkbox" checked={settings.autostart} onChange={toggleAutostart} />
        Start LeetLife when I log in
      </label>

      <div className="row">
        <button className="ghost" onClick={() => api.hideWindow()}>Close window</button>
        <button className="ghost danger" onClick={() => api.quitApp()}>Quit LeetLife</button>
      </div>
    </div>
  );
}

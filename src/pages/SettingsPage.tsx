import { Icon } from "../components/Icon";
import { useEffect, useState } from "react";
import { api, isTauri } from "../api/client";
import type { SettingsView } from "../api/types";

export function SettingsPage({ onError, onNotice }: { onError: (m: string) => void; onNotice: (m: string) => void }) {
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [username, setUsername] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let live = true;
    api.getSettings().then(s => { if (live) { setSettings(s); setUsername(s.values.leetcode_username ?? ""); } }).catch(e => { if (live) onError(String(e)); });
    return () => { live = false; };
  }, [onError]);

  async function saveUsername() {
    setBusy(true);
    try {
      await api.setSetting("leetcode_username", username.trim());
      onNotice(username.trim() ? "Username saved. Use Sync now to bring in your recent solves." : "Manual logging is ready. Make yourself at home.");
    } catch (e) { onError(String(e)); } finally { setBusy(false); }
  }
  async function toggleAutostart() {
    if (!settings) return;
    setBusy(true);
    try { const autostart = await api.setAutostart(!settings.autostart); setSettings({ ...settings, autostart }); }
    catch (e) { onError(String(e)); } finally { setBusy(false); }
  }
  async function syncNow() {
    setBusy(true);
    try {
      const r = await api.syncNow();
      onNotice(r.error ? `Couldn't sync: ${r.error}` : r.provider === "manual" ? "Save your LeetCode username to turn on sync." : `All caught up! ${r.imported} new ${r.imported === 1 ? "solve" : "solves"} imported.`);
    } catch (e) { onError(String(e)); } finally { setBusy(false); }
  }
  async function backup() {
    setBusy(true);
    try { await api.backupDatabase(); onNotice("A safe little copy of your world. Backup opened in Finder."); }
    catch (e) { onError(String(e)); } finally { setBusy(false); }
  }
  if (!settings) return <div className="page"><p className="empty">Getting things ready…</p></div>;
  return (
    <div className="page">
      <h1 className="page-title">Make yourself at home.</h1>
      <p className="page-subtitle">Little preferences for your little world.</p>
      <section className="settings-card">
        <span className="local-badge"><Icon name="leaf" size={13}/> LOCAL & FREE, ALWAYS</span>
        <h3>Your progress lives on your Mac</h3>
        <p>Your buddy, coins, furniture, and practice history save automatically. No account, subscription, or cloud setup needed.</p>
        {isTauri() ? <><div className="row"><button onClick={backup} disabled={busy}>Back up my world</button><button className="ghost" onClick={() => void api.showSaveFolder().catch(e => onError(String(e)))}>Show save folder ↗</button></div><span className="storage-path">{settings.data_path}</span></> : <p>This browser is a demo with sample progress. Open the Mac app for your real, saved game.</p>}
      </section>
      <section className="settings-card">
        <h3>Bring your LeetCode along</h3>
        <p>Optional: sync your recent public accepted submissions every 15 minutes. You can always log practice by hand.</p>
        <label className="field">LeetCode username<input value={username} onChange={e => setUsername(e.target.value)} placeholder="Your username (optional)" autoCapitalize="none" autoCorrect="off" spellCheck={false} /></label>
        <div className="row"><button className="primary" onClick={saveUsername} disabled={busy || !isTauri()}>Save username</button><button onClick={syncNow} disabled={busy || !isTauri()}>Sync now</button></div>
      </section>
      {isTauri() && <section className="settings-card"><h3>A little company, every day</h3><label className="check"><input type="checkbox" checked={settings.autostart} disabled={busy} onChange={toggleAutostart} />Open LeetLife when I log in</label><p>Susuwatari sleeps in your menu bar. Click for a check-in, then choose Visit the cottage for the full house. Drag the top of the house window to move it; use the little dash to minimize.</p></section>}
      {isTauri() && <div className="row"><button className="ghost" onClick={() => void api.hideWindow().catch(e => onError(String(e)))}>Tuck away</button><button className="ghost danger" onClick={() => void api.quitApp().catch(e => onError(String(e)))}>Quit LeetLife</button></div>}
      <p className="hint" style={{textAlign:"center"}}>Made for small steps and softer study days. ♡</p>
    </div>
  );
}

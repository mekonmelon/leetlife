// Forwards accepted submissions to the LeetLife ingest endpoint on loopback.
const DEFAULT_PORT = 47831;

async function port() {
  const { leetlifePort } = await chrome.storage.local.get("leetlifePort");
  return leetlifePort || DEFAULT_PORT;
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg?.kind !== "accepted") return;
  (async () => {
    try {
      const res = await fetch(`http://127.0.0.1:${await port()}/attempt`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(msg.attempt),
      });
      sendResponse({ ok: res.ok, status: res.status });
    } catch (e) {
      // LeetLife is closed. The user can still log the attempt by hand.
      sendResponse({ ok: false, error: String(e) });
    }
  })();
  return true; // keep the channel open for the async response
});

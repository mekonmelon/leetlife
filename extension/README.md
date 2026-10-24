# LeetLife Bridge

An optional Chrome extension. It notices when a LeetCode submission is accepted
and posts it to the LeetLife app on `http://127.0.0.1:47831/attempt`.

You do not need it. LeetLife can poll your public accepted submissions if you
enter your username in Settings, and you can always log attempts by hand.

## Install

1. Open `chrome://extensions`.
2. Turn on **Developer mode**.
3. Choose **Load unpacked** and select this folder.

If you changed the ingest port in LeetLife, set it here too:

```js
chrome.storage.local.set({ leetlifePort: 47831 });
```

## Why a loopback POST instead of scraping

The desktop app never has to open a browser, hold your cookies, or poll pages.
The extension already sits where the event happens, so it just announces it.
LeetLife deduplicates on `external_id`, so reporting the same submission twice
is harmless.

// Watches the submission result area. When LeetCode renders "Accepted", we hand
// the problem slug to the service worker, which posts it to LeetLife.
// Detection is deliberately loose: LeetCode's DOM changes often, and the worst
// case is a missed attempt the user can still log by hand.

const SEEN_TTL_MS = 60_000;
const seen = new Map();

function slugFromUrl() {
  const m = location.pathname.match(/\/problems\/([^/]+)/);
  return m ? m[1] : null;
}

function titleFromDom() {
  const el = document.querySelector('[data-cy="question-title"], a[href^="/problems/"] .text-title-large, div.text-title-large a');
  return el?.textContent?.trim() || null;
}

function difficultyFromDom() {
  const text = document.body.innerText;
  for (const d of ["Easy", "Medium", "Hard"]) {
    if (new RegExp(`\\b${d}\\b`).test(text.slice(0, 4000))) return d.toLowerCase();
  }
  return null;
}

function recentlyReported(slug) {
  const last = seen.get(slug);
  if (last && Date.now() - last < SEEN_TTL_MS) return true;
  seen.set(slug, Date.now());
  return false;
}

function report() {
  const slug = slugFromUrl();
  if (!slug || recentlyReported(slug)) return;
  chrome.runtime.sendMessage({
    kind: "accepted",
    attempt: {
      external_id: `ext:${slug}:${Date.now()}`,
      slug,
      title: titleFromDom() || slug,
      difficulty: difficultyFromDom(),
      result: "solved",
      attempted_at: Math.floor(Date.now() / 1000),
      source: "extension",
    },
  });
}

const observer = new MutationObserver(() => {
  const marker = document.querySelector('[data-e2e-locator="submission-result"], span.text-green-s, .text-success');
  if (marker && /accepted/i.test(marker.textContent || "")) report();
});

observer.observe(document.body, { childList: true, subtree: true, characterData: true });

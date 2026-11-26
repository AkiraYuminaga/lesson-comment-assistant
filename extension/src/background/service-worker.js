// Background service worker:
// - registers the panel on the school-system site chosen in the options page
// - proxies backend calls (requests from here aren't bound by the page's CORS rules)
// - serves the panel's HTML/CSS so no file has to be web-accessible to the host page

const DEFAULTS = { siteUrl: "", backendUrl: "http://127.0.0.1:8787" };
const SCRIPT_ID = "sca-panel";
const TIMEOUT_MS = 60000;

async function getSettings() {
  const stored = await chrome.storage.local.get(["siteUrl", "backendUrl"]);
  return { ...DEFAULTS, ...stored };
}

/** "https://school.example.com/path" -> "https://school.example.com/*" */
function originPattern(url) {
  const u = new URL(url);
  return `${u.protocol}//${u.hostname}/*`;
}

/** (Re)register the content scripts for the configured site. */
async function syncContentScript() {
  const { siteUrl } = await getSettings();
  const existing = await chrome.scripting.getRegisteredContentScripts({ ids: [SCRIPT_ID] });
  if (existing.length) await chrome.scripting.unregisterContentScripts({ ids: [SCRIPT_ID] });
  if (!siteUrl) return { registered: false, reason: "no-site" };

  const pattern = originPattern(siteUrl);
  if (!(await chrome.permissions.contains({ origins: [pattern] }))) {
    return { registered: false, reason: "no-permission" };
  }
  await chrome.scripting.registerContentScripts([
    {
      id: SCRIPT_ID,
      matches: [pattern],
      js: ["src/content/site-config.js", "src/content/content.js"],
      runAt: "document_idle"
    }
  ]);
  return { registered: true, pattern };
}

chrome.runtime.onInstalled.addListener(async () => {
  await syncContentScript();
  const { siteUrl } = await getSettings();
  if (!siteUrl) chrome.runtime.openOptionsPage();
});
chrome.runtime.onStartup.addListener(syncContentScript);
chrome.permissions.onRemoved.addListener(syncContentScript);
chrome.action.onClicked.addListener(() => chrome.runtime.openOptionsPage());

async function backendFetch(path, init = {}) {
  const { backendUrl } = await getSettings();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);
  try {
    const r = await fetch(new URL(path, backendUrl), { ...init, signal: controller.signal });
    if (!r.ok) return { ok: false, error: `API error: ${r.status}`, status: r.status };
    return { ok: true, data: await r.json() };
  } catch (err) {
    return { ok: false, error: err?.name === "AbortError" ? "timeout" : String(err) };
  } finally {
    clearTimeout(timer);
  }
}

async function loadAssets() {
  const read = async (p) => (await fetch(chrome.runtime.getURL(p))).text();
  const [html, css] = await Promise.all([read("src/content/panel.html"), read("src/content/panel.css")]);
  return { ok: true, html, css };
}

const handlers = {
  "sca:assets": loadAssets,
  "sca:courses": () => backendFetch("/v1/courses"),
  "sca:health": () => backendFetch("/health"),
  "sca:sync": syncContentScript,
  "sca:generate": async (msg) => {
    const res = await backendFetch("/v1/comments/generate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(msg.payload)
    });
    return res.ok ? { ok: true, comment: res.data.comment || "" } : res;
  }
};

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  const handler = sender.id === chrome.runtime.id && handlers[msg?.type];
  if (!handler) return;
  handler(msg)
    .then(sendResponse)
    .catch((err) => sendResponse({ ok: false, error: String(err) }));
  return true; // keep the channel open for the async response
});

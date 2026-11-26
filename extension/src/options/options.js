const DEFAULT_BACKEND = "http://127.0.0.1:8787";
const form = document.getElementById("form");
const siteInput = document.getElementById("siteUrl");
const backendInput = document.getElementById("backendUrl");
const statusEl = document.getElementById("status");

function setStatus(text, kind = "") {
  statusEl.textContent = text;
  statusEl.className = kind;
}

/** "https://school.example.com/path" -> "https://school.example.com/*" */
function originPattern(url) {
  const u = new URL(url);
  return `${u.protocol}//${u.hostname}/*`;
}

const isLocal = (url) => ["127.0.0.1", "localhost"].includes(new URL(url).hostname);

function parseUrl(value, label) {
  try {
    const u = new URL(value.trim());
    if (!/^https?:$/.test(u.protocol)) throw new Error();
    return u.origin;
  } catch {
    throw new Error(`${label}格式不正確，請以 http:// 或 https:// 開頭。`);
  }
}

chrome.storage.local.get(["siteUrl", "backendUrl"]).then(({ siteUrl, backendUrl }) => {
  siteInput.value = siteUrl || "";
  backendInput.value = backendUrl || DEFAULT_BACKEND;
});

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  let siteUrl, backendUrl;
  try {
    siteUrl = parseUrl(siteInput.value, "學務系統網址");
    backendUrl = parseUrl(backendInput.value || DEFAULT_BACKEND, "後端網址");
  } catch (err) {
    setStatus(err.message, "err");
    return;
  }

  // Ask only for the sites actually used (localhost is already granted by the manifest).
  const origins = [originPattern(siteUrl)];
  if (!isLocal(backendUrl)) origins.push(originPattern(backendUrl));
  const granted = await chrome.permissions.request({ origins });
  if (!granted) {
    setStatus("沒有取得網站存取權限，面板無法顯示。請再按一次儲存並選擇「允許」。", "err");
    return;
  }

  await chrome.storage.local.set({ siteUrl, backendUrl });
  siteInput.value = siteUrl;
  backendInput.value = backendUrl;
  const res = await chrome.runtime.sendMessage({ type: "sca:sync" });
  if (res?.registered) setStatus("已儲存！請重新整理學務系統的頁面。", "ok");
  else setStatus(`儲存了設定，但面板沒有啟用（${res?.reason || res?.error || "未知原因"}）。`, "err");
});

document.getElementById("test").addEventListener("click", async () => {
  setStatus("測試中…");
  const res = await chrome.runtime.sendMessage({ type: "sca:health" });
  if (res?.ok) setStatus(`後端連線正常（版本 ${res.data.version}）。`, "ok");
  else setStatus("連不到後端：請確認後端已啟動，且網址正確（先按儲存再測試）。", "err");
});

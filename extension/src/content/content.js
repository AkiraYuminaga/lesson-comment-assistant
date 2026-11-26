(async () => {
  const HOST_ID = "sca-lite-host";
  if (document.getElementById(HOST_ID)) return;
  const SITE = globalThis.SCA_SITE || {};

  // The panel lives in a shadow root so the host page's CSS (Bootstrap etc.) can't restyle it
  // and our CSS can't leak into the page. HTML/CSS come from the service worker.
  const host = document.createElement("div");
  host.id = HOST_ID;
  document.body.appendChild(host);
  const shadow = host.attachShadow({ mode: "open" });

  const send = (type, extra = {}) => chrome.runtime.sendMessage({ type, ...extra });

  const root = document.createElement("div");
  root.id = "sca-lite-root";
  try {
    const assets = await send("sca:assets");
    if (!assets?.ok) throw new Error(assets?.error || "no assets");
    const style = document.createElement("style");
    style.textContent = assets.css;
    root.innerHTML = assets.html;
    shadow.append(style, root);
  } catch (err) {
    host.remove();
    console.error("SCA template load failed:", err);
    return;
  }

  const $ = (sel) => root.querySelector(sel);
  const panel = $("#sca-panel");
  const fab = $("#sca-fab");
  const statusEl = $("#sca-status");
  const preview = $("#sca-preview");
  const previewText = $("#sca-preview-text");
  const countEl = $("#sca-count");
  const courseSelect = $("#sca-course");
  const lessonSelect = $("#sca-lesson");
  const performanceEl = $("#sca-performance");
  const generateBtn = $("#sca-generate");

  if (!panel || !fab || !statusEl || !preview || !previewText || !courseSelect || !lessonSelect || !performanceEl || !generateBtn) {
    host.remove();
    console.error("SCA template is missing required elements.");
    return;
  }

  const escapeHtml = (t) =>
    String(t).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
  const getValue = (sel) => String($(sel)?.value || "").trim();
  const checked = (sel) => Boolean($(sel)?.checked);

  // ---------- Look & feel from site-config.js ----------
  const ui = SITE.ui || {};
  if (ui.title) $("#sca-title").textContent = ui.title;
  if (ui.subtitle) $("#sca-subtitle").textContent = ui.subtitle;
  if (ui.logo) $("#sca-logo").textContent = ui.logo;
  if (ui.accent) root.style.setProperty("--sca-accent", ui.accent);

  // ---------- Small persistence helpers (per-browser conveniences only) ----------
  const STORE_KEY = "sca-lite-prefs";
  function loadPrefs() {
    try {
      return JSON.parse(localStorage.getItem(STORE_KEY) || "{}");
    } catch {
      return {};
    }
  }
  function savePrefs(patch) {
    try {
      localStorage.setItem(STORE_KEY, JSON.stringify({ ...loadPrefs(), ...patch }));
    } catch {
      /* storage blocked: ignore */
    }
  }
  const prefs = loadPrefs();

  // ---------- Status ----------
  function setStatus(text, kind = "") {
    statusEl.textContent = text;
    statusEl.classList.toggle("is-success", kind === "success");
    statusEl.classList.toggle("is-error", kind === "error");
  }

  // ---------- Progress buttons ----------
  const progressOptions = SITE.progressOptions?.length ? SITE.progressOptions : ["進度正常", "進度稍慢", "需要協助"];
  $("#sca-progress").style.gridTemplateColumns = `repeat(${progressOptions.length}, 1fr)`;
  $("#sca-progress").innerHTML = progressOptions
    .map(
      (p, i) =>
        `<label><input type="radio" name="sca-progress" value="${escapeHtml(p)}"${i === 0 ? " checked" : ""}><span>${escapeHtml(p)}</span></label>`
    )
    .join("");
  const getProgress = () => root.querySelector('input[name="sca-progress"]:checked')?.value || progressOptions[0];

  // ---------- Courses (from the backend's courses.json) ----------
  // Mirrors the backend lookup: names are compared lowercase without spaces/punctuation, and a
  // series alias prefix ("機器人" -> "robot") is normalised to the series id.
  let catalogue = { series: [], courses: [] };
  const compact = (s) => (String(s).toLowerCase().match(/[\p{L}\p{N}]/gu) || []).join("");

  function normalize(s) {
    const input = compact(s);
    for (const series of catalogue.series) {
      const names = [...series.aliases, series.id].map(compact).filter(Boolean).sort((a, b) => b.length - a.length);
      const alias = names.find((a) => input.startsWith(a));
      if (alias) return compact(series.id) + input.slice(alias.length);
    }
    return input;
  }

  const courseNames = (c) => [c.id, c.name, ...(c.aliases || [])].map(normalize);
  const inSeriesRange = (c, n) => c.series && n >= c.first_lesson && n < c.first_lesson + c.chapters.length;

  function findCourse(text) {
    const input = normalize(text);
    if (!input) return null;
    const exact = catalogue.courses.find((c) => courseNames(c).includes(input));
    if (exact) return exact;
    const prefixed = catalogue.courses.filter((c) => input.startsWith(normalize(c.id))).sort((a, b) => b.id.length - a.id.length)[0];
    if (prefixed) return prefixed;
    const partial = catalogue.courses.filter((c) => courseNames(c).some((n) => n.startsWith(input)));
    return partial.length === 1 ? partial[0] : null;
  }

  /** Course text + lesson number from the page -> { course, n } using the panel's numbering. */
  function resolveLesson(text, n) {
    const named = findCourse(text);
    const inSeries = (series) => catalogue.courses.find((c) => (!series || c.series === series) && inSeriesRange(c, n));
    if (named) {
      if (inSeriesRange(named, n)) return { course: named, n };
      if (n <= named.chapters.length) return { course: named, n: named.series ? named.first_lesson + n - 1 : n };
      const other = named.series && inSeries(named.series);
      return other ? { course: other, n } : null;
    }
    const series = catalogue.series.find((s) => normalize(text).startsWith(compact(s.id)));
    const other = series && inSeries(series.id);
    return other ? { course: other, n } : null;
  }

  /** The captured text may carry a label in front ("Learning Course Python 進階"): try the
   *  last word, then the last two, ... and use the first that names a course. */
  function resolveTrailing(text, n) {
    const words = text.trim().split(/\s+/);
    for (let k = 1; k <= words.length; k++) {
      const hit = resolveLesson(words.slice(-k).join(" "), n);
      if (hit) return hit;
    }
    return null;
  }

  /** Lesson menu for a course: series courses use their global numbers (e.g. L16-L30). */
  function renderLessons(course, selected = "") {
    if (!course) {
      lessonSelect.innerHTML = '<option value="">先選擇課程</option>';
      return;
    }
    const start = course.series ? course.first_lesson : 1;
    lessonSelect.innerHTML =
      '<option value="">選擇課次</option>' +
      course.chapters
        .map((title, i) => {
          const value = `L${start + i}`;
          return `<option value="${value}">${value}${title ? `　${escapeHtml(title)}` : ""}</option>`;
        })
        .join("");
    if (selected) lessonSelect.value = selected;
  }

  const currentCourse = () => catalogue.courses.find((c) => c.id === courseSelect.value) || null;

  function setCourseAndLesson(courseId, lesson) {
    courseSelect.value = courseId;
    renderLessons(currentCourse(), lesson);
    savePrefs({ course: courseSelect.value, lesson: lessonSelect.value });
  }

  courseSelect.addEventListener("change", () => {
    renderLessons(currentCourse());
    savePrefs({ course: courseSelect.value, lesson: "" });
  });
  lessonSelect.addEventListener("change", () => savePrefs({ lesson: lessonSelect.value }));

  async function loadCourses() {
    const res = await send("sca:courses");
    if (!res?.ok || !res.data?.courses?.length) {
      courseSelect.innerHTML = '<option value="">無法載入課程（後端未啟動？）</option>';
      renderLessons(null);
      setStatus("連不到後端，請確認後端已啟動。", "error");
      return false;
    }
    catalogue = { series: res.data.series || [], courses: res.data.courses };
    courseSelect.innerHTML =
      '<option value="">選擇課程</option>' +
      catalogue.courses
        .map((c) => {
          const range = c.series ? `（L${c.first_lesson}–L${c.first_lesson + c.chapters.length - 1}）` : "";
          return `<option value="${escapeHtml(c.id)}">${escapeHtml(c.name)}${range}</option>`;
        })
        .join("");
    setCourseAndLesson(prefs.course || "", prefs.lesson || "");
    return true;
  }

  // ---------- Keyword picker ----------
  const TAG_GROUPS = SITE.tagGroups || [];
  const DEFAULT_QUICK = SITE.quickTags || [];
  const WARN_TAGS = new Set(TAG_GROUPS.filter((g) => g.tone === "warn").flatMap((g) => g.tags));
  const COMPLETION_TAGS = new Set((SITE.completion || []).map((c) => c.tag));
  const QUICK_COUNT = 8;

  let selectedTags = [];
  let customTags = [];
  let tagUsage = {};

  // Custom tags and usage counts persist in chrome.storage so they survive reloads and site-data clears.
  const store = {
    async get() {
      try {
        return await chrome.storage.local.get(["customTags", "tagUsage"]);
      } catch {
        return {};
      }
    },
    set(data) {
      try {
        chrome.storage.local.set(data);
      } catch {
        /* storage unavailable: keep in memory only */
      }
    }
  };

  const selectedEl = $("#sca-selected");
  const quickEl = $("#sca-quick");
  const libraryEl = $("#sca-library");
  const tagsToggle = $("#sca-tags-toggle");

  const chip = (tag, { removable = false } = {}) =>
    `<button type="button" class="sca-chip${selectedTags.includes(tag) ? " is-on" : ""}${WARN_TAGS.has(tag) ? " tone-warn" : ""}" data-tag="${escapeHtml(tag)}">${escapeHtml(tag)}${
      removable ? `<span class="sca-chip-del" data-del="${escapeHtml(tag)}" title="刪除這個關鍵詞">×</span>` : ""
    }</button>`;

  function quickTags() {
    const ranked = Object.entries(tagUsage).sort((a, b) => b[1] - a[1]).map(([t]) => t);
    return [...new Set([...ranked, ...DEFAULT_QUICK])].slice(0, QUICK_COUNT);
  }

  function renderTags() {
    selectedEl.innerHTML = selectedTags.length
      ? selectedTags
          .map((t) => `<span class="sca-pill">${escapeHtml(t)}<button type="button" data-remove="${escapeHtml(t)}" aria-label="移除 ${escapeHtml(t)}">×</button></span>`)
          .join("")
      : '<span class="sca-selected-empty">尚未選擇，點下方關鍵詞加入</span>';

    quickEl.innerHTML = quickTags().map((t) => chip(t)).join("");

    libraryEl.innerHTML =
      TAG_GROUPS.map(
        (g) => `<div class="sca-group"><p class="sca-group-title">${escapeHtml(g.title)}</p><div class="sca-chips">${g.tags.map((t) => chip(t)).join("")}</div></div>`
      ).join("") +
      `<div class="sca-group"><p class="sca-group-title">我的關鍵詞</p><div class="sca-chips">${
        customTags.length ? customTags.map((t) => chip(t, { removable: true })).join("") : '<span class="sca-selected-empty">還沒有自訂關鍵詞</span>'
      }</div><div class="sca-add"><input id="sca-new-tag" maxlength="12" placeholder="新增關鍵詞，例如：很有創意" autocomplete="off"><button id="sca-add-tag" type="button">新增</button></div></div>`;
  }

  function toggleTag(tag) {
    selectedTags = selectedTags.includes(tag) ? selectedTags.filter((t) => t !== tag) : [...selectedTags, tag];
    renderTags();
  }

  function addTag(tag) {
    if (!selectedTags.includes(tag)) selectedTags = [...selectedTags, tag];
    renderTags();
  }

  function clearTags() {
    selectedTags = [];
    renderTags();
  }

  function addCustomTag() {
    const input = $("#sca-new-tag");
    const tag = (input?.value || "").replace(/[、,，]/g, "").trim();
    if (!tag) return;
    const known = TAG_GROUPS.some((g) => g.tags.includes(tag)) || customTags.includes(tag);
    if (!known) {
      customTags = [...customTags, tag];
      store.set({ customTags });
    }
    addTag(tag);
    $("#sca-new-tag")?.focus();
  }

  function recordUsage() {
    selectedTags.forEach((t) => (tagUsage[t] = (tagUsage[t] || 0) + 1));
    store.set({ tagUsage });
  }

  // One delegated handler covers the quick row, the library and the selected pills.
  root.addEventListener("click", (e) => {
    const del = e.target.closest("[data-del]");
    if (del) {
      e.stopPropagation();
      const tag = del.dataset.del;
      customTags = customTags.filter((t) => t !== tag);
      selectedTags = selectedTags.filter((t) => t !== tag);
      store.set({ customTags });
      renderTags();
      return;
    }
    const remove = e.target.closest("[data-remove]");
    if (remove) return toggleTag(remove.dataset.remove);
    const tagBtn = e.target.closest(".sca-chip[data-tag]");
    if (tagBtn) return toggleTag(tagBtn.dataset.tag);
    if (e.target.closest("#sca-add-tag")) addCustomTag();
  });
  root.addEventListener("keydown", (e) => {
    if (e.target.id === "sca-new-tag" && e.key === "Enter" && !e.isComposing) {
      e.preventDefault();
      addCustomTag();
    }
  });

  function setLibraryOpen(open) {
    libraryEl.classList.toggle("sca-hidden", !open);
    quickEl.classList.toggle("sca-hidden", open);
    tagsToggle.textContent = open ? "收起" : "展開全部關鍵詞";
    tagsToggle.setAttribute("aria-expanded", String(open));
    savePrefs({ libraryOpen: open });
  }
  tagsToggle.addEventListener("click", () => setLibraryOpen(libraryEl.classList.contains("sca-hidden")));

  const getPerformance = () => [...selectedTags, getValue("#sca-performance")].filter(Boolean).join("、");

  renderTags();
  setLibraryOpen(Boolean(prefs.libraryOpen));
  store.get().then((data) => {
    customTags = Array.isArray(data.customTags) ? data.customTags : [];
    tagUsage = data.tagUsage && typeof data.tagUsage === "object" ? data.tagUsage : {};
    renderTags();
  });

  // ---------- Collapse / reopen ----------
  function setCollapsed(collapsed) {
    panel.classList.toggle("sca-hidden", collapsed);
    fab.classList.toggle("sca-hidden", !collapsed);
    savePrefs({ collapsed });
  }
  $("#sca-lite-close").addEventListener("click", () => setCollapsed(true));
  fab.addEventListener("click", () => setCollapsed(false));
  if (prefs.collapsed) setCollapsed(true);

  // ---------- Host page: the open record dialog ----------
  // A class page may keep one record dialog per student hidden in the DOM, so everything below
  // works on the *visible* feedback field only.
  const toRegex = (pattern, flags = "i") => {
    try {
      return pattern ? new RegExp(pattern, flags) : null;
    } catch (err) {
      console.error("SCA: invalid pattern in site-config.js:", pattern, err);
      return null;
    }
  };
  const NAME_RE = toRegex(SITE.studentName);
  const LESSON_RE = toRegex(SITE.lesson, "gi");
  const COMPLETION = (SITE.completion || []).map((c) => ({ re: toRegex(c.match), tag: c.tag })).filter((c) => c.re);
  const isVisible = (el) => Boolean(el && el.getClientRects().length && getComputedStyle(el).visibility !== "hidden");

  function findBestTextarea() {
    if (!SITE.feedbackField) return null;
    return [...document.querySelectorAll(SITE.feedbackField)].find(isVisible) || null;
  }

  function setNativeValue(element, value) {
    const descriptor = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(element), "value");
    if (descriptor && descriptor.set) descriptor.set.call(element, value);
    else element.value = value;
    element.dispatchEvent(new Event("input", { bubbles: true }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
  }

  function findOpenRecord() {
    const textarea = findBestTextarea();
    if (!textarea) return null;
    // Prefer the whole dialog (its title holds the student name) over an inner <form>.
    const box = (SITE.dialog && textarea.closest(SITE.dialog)) || textarea.closest("form") || textarea.parentElement;
    return { textarea, box };
  }

  // Texts the user actually sees in the dialog: chosen option of selects, values of inputs, plain text.
  function visibleTexts(box) {
    const texts = [];
    box.querySelectorAll("select").forEach((sel) => texts.push(sel.selectedOptions[0]?.textContent || ""));
    box.querySelectorAll("input:not([type=hidden]):not([type=checkbox]):not([type=radio])").forEach((i) => texts.push(i.value || ""));
    const clone = box.cloneNode(true);
    clone.querySelectorAll("select, textarea, script, style").forEach((n) => n.remove());
    texts.push(clone.textContent || "");
    return texts.map((t) => t.replace(/\s+/g, " ").trim()).filter(Boolean);
  }

  function givenName(fullName) {
    const name = fullName.trim();
    return SITE.stripSurname && /^[一-鿿]{3}$/.test(name) ? name.slice(1) : name;
  }

  function readPageInfo() {
    const record = findOpenRecord();
    if (!record) return {};
    const info = {};
    const texts = visibleTexts(record.box);

    const titleEl = SITE.dialogTitle && record.box.querySelector(SITE.dialogTitle);
    const nameMatch = NAME_RE && (titleEl?.textContent || texts.join(" ")).match(NAME_RE);
    if (nameMatch?.[1]) {
      info.fullName = nameMatch[1];
      info.student = givenName(nameMatch[1]);
    }

    // First "<course> Lesson <n>" whose course text is in the catalogue wins.
    if (LESSON_RE) {
      outer: for (const text of texts) {
        for (const m of text.matchAll(LESSON_RE)) {
          const hit = resolveTrailing(m[1], parseInt(m[2], 10));
          if (hit) {
            info.course = hit.course.id;
            info.lesson = `L${hit.n}`;
            break outer;
          }
        }
      }
    }

    const done = COMPLETION.find((c) => texts.some((t) => c.re.test(t)));
    if (done) info.completion = done.tag;
    return info;
  }

  let lastRecordKey = "";
  function applyPageInfo(info, { force = false } = {}) {
    if (!info.student && !info.course) return false;
    const key = [info.fullName, info.course, info.lesson].join("|");
    if (!force && key === lastRecordKey) return false;
    const newStudent = key !== lastRecordKey;
    lastRecordKey = key;

    if (newStudent) {
      // Different student: start from a clean slate so nothing carries over.
      clearTags();
      performanceEl.value = "";
      $("#sca-issue").value = "";
      $("#sca-last-undone").checked = false;
      previewText.value = "";
      preview.classList.add("sca-hidden");
    }
    if (info.student) $("#sca-student").value = info.student;
    if (info.course) setCourseAndLesson(info.course, info.lesson || "");
    if (info.completion) {
      // Keep only the tag matching the page, so switching status doesn't leave two selected.
      selectedTags = selectedTags.filter((t) => !COMPLETION_TAGS.has(t));
      addTag(info.completion);
    }
    return true;
  }

  const autofillBtn = $("#sca-autofill");
  autofillBtn.addEventListener("click", () => {
    if (applyPageInfo(readPageInfo(), { force: true })) setStatus("已從頁面帶入學生資訊。", "success");
    else setStatus("沒有找到打開中的課堂紀錄。", "error");
  });

  // Watch for a record dialog opening and auto-fill from it.
  let scanTimer = 0;
  function scanPage() {
    const info = readPageInfo();
    const found = Boolean(info.student || info.course);
    autofillBtn.classList.toggle("sca-hidden", !found);
    if (found && applyPageInfo(info)) {
      if (panel.classList.contains("sca-hidden")) setCollapsed(false);
      setStatus(`已帶入 ${info.fullName || info.student} 的資料，點選學習表現後即可生成。`, "success");
    }
  }

  // ---------- Backend ----------
  async function apiGenerate(payload) {
    const res = await send("sca:generate", { payload });
    if (!res) throw new Error("No response from background");
    if (!res.ok) {
      const err = new Error(res.error || "API error");
      err.status = res.status;
      throw err;
    }
    return res.comment || "";
  }

  function describeError(err) {
    if (err?.message === "timeout") return "後端回應逾時，請稍後再試。";
    if (err?.status === 429) return "請求太頻繁，請稍等一下再試。";
    if (err?.status === 503) return "AI 服務目前忙碌中，請稍後再試。";
    if (err?.status === 400 || err?.status === 422) return "輸入內容格式有誤（可能太長或含不允許的字元）。";
    if (err?.status) return `後端回應錯誤（${err.status}）。`;
    return "後端未啟動或回應失敗。";
  }

  // Prevent double submission while a request is in flight
  let busy = false;
  const actionButtons = ["#sca-generate", "#sca-paste-direct", "#sca-regenerate"].map($).filter(Boolean);
  async function withLock(fn) {
    if (busy) return;
    busy = true;
    actionButtons.forEach((b) => (b.disabled = true));
    generateBtn.classList.add("is-loading");
    try {
      await fn();
    } finally {
      busy = false;
      actionButtons.forEach((b) => (b.disabled = false));
      generateBtn.classList.remove("is-loading");
    }
  }

  function validate() {
    if (!getValue("#sca-student")) return "請先填寫學生姓名。";
    if (!getValue("#sca-course")) return "請先選擇課程。";
    if (!getValue("#sca-lesson")) return "請先選擇課次。";
    return "";
  }

  async function buildComment() {
    return apiGenerate({
      student: getValue("#sca-student"),
      course: getValue("#sca-course"),
      lesson: getValue("#sca-lesson"),
      progress: getProgress(),
      performance: getPerformance(),
      issue: getValue("#sca-issue"),
      lastUndone: checked("#sca-last-undone"),
      polish: checked("#sca-polish")
    });
  }

  function updateCount() {
    countEl.textContent = `${previewText.value.trim().length} 字`;
  }
  previewText.addEventListener("input", updateCount);

  async function generate(onText) {
    const problem = validate();
    if (problem) {
      setStatus(problem, "error");
      return;
    }
    setStatus("生成中，約需 10 秒…");
    try {
      const text = await buildComment();
      if (!text) {
        setStatus("後端沒有回傳內容。", "error");
        return;
      }
      recordUsage();
      onText(text);
    } catch (err) {
      console.error("SCA API generate failed:", err);
      setStatus(describeError(err), "error");
    }
  }

  function showPreview(text) {
    previewText.value = text;
    updateCount();
    preview.classList.remove("sca-hidden");
    preview.scrollIntoView({ block: "nearest", behavior: "smooth" });
    setStatus("已生成，可以修改後再貼上。", "success");
  }

  function pasteText(text) {
    const target = findBestTextarea();
    if (!target) {
      setStatus("找不到可貼上的評語欄位。", "error");
      return;
    }
    setNativeValue(target, text.trim());
    target.focus();
    setStatus("已貼到評語欄位。", "success");
  }

  generateBtn.addEventListener("click", () => withLock(() => generate(showPreview)));
  $("#sca-regenerate").addEventListener("click", () => withLock(() => generate(showPreview)));
  $("#sca-paste-direct").addEventListener("click", () => withLock(() => generate(pasteText)));
  $("#sca-confirm").addEventListener("click", () => pasteText(previewText.value));
  $("#sca-copy").addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(previewText.value.trim());
      setStatus("已複製到剪貼簿。", "success");
    } catch {
      previewText.select();
      setStatus("無法自動複製，已幫你選取文字，請按 Ctrl/⌘ + C。", "error");
    }
  });
  $("#sca-preview-close").addEventListener("click", () => preview.classList.add("sca-hidden"));

  // ---------- Start ----------
  // Courses first: page auto-fill needs the catalogue to recognise course names.
  await loadCourses();
  new MutationObserver((mutations) => {
    if (mutations.every((m) => host.contains(m.target))) return;
    clearTimeout(scanTimer);
    scanTimer = setTimeout(scanPage, 250);
  }).observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class", "style"] });
  scanPage();
})();

# 課堂評語助手 lesson-comment-assistant

在學務系統填寫課堂紀錄時，用 AI（Google Gemini）一鍵生成學生的聯絡簿評語，預覽、修改後直接貼進評語欄位。

這是一個**模板**：程式本身通用，每位老師只要放進自己的課程內容、評語風格和學務系統網址就能使用。

- 打開學生的課堂紀錄視窗，面板會**自動帶入**學生姓名、課程、課次、完成狀態
- 點選學習表現關鍵詞（可自訂、會記住常用詞）→ 生成 → 預覽修改 → 貼上
- 評語只根據「課程內容 + 老師選的關鍵詞」撰寫，不會自己編造課堂情節

```
學務系統頁面
  └─ 擴充功能面板 ──▶ 擴充功能背景程式 ──HTTP──▶ 後端（你的電腦） ──▶ Gemini API
                                                   └─ config/：課程、評語規則、風格
```

---

## 你需要自己準備的資訊

| 項目 | 放在哪裡 | 說明 |
|---|---|---|
| Gemini API Key | `backend/.env` 的 `GEMINI_API_KEY` | 到 [Google AI Studio](https://aistudio.google.com/apikey) 免費申請 |
| HMAC 密鑰 | `backend/.env` 的 `HMAC_SECRET` | 執行 `openssl rand -hex 32` 產生一組 |
| 你的課程內容 | `backend/config/courses.json` | 每一課的章節名稱與介紹，評語會依此描述課堂內容 |
| 評語規則（可選） | `backend/config/prompt.md` | 語氣、字數、結尾固定句等；不放就用範例 |
| 文字風格（可選） | `backend/config/style.json` | 禁用詞、替換詞、字數下限；不放就用範例 |
| 學務系統網址 | 擴充功能的「設定」頁 | 例如 `https://school.example.com` |

> `courses.json`、`prompt.md`、`style.json`、`.env` 都已列在 `.gitignore`，**不會被上傳**。
> repo 裡只有 `*.example.*` 範例檔，缺少自己的版本時會自動改用範例。

---

## 安裝

### 需求

- [Rust](https://rustup.rs/)（安裝後有 `cargo` 指令）
- Google Chrome

### 1. 下載

```bash
git clone https://github.com/<你的帳號>/lesson-comment-assistant.git
cd lesson-comment-assistant
```

### 2. 設定後端

```bash
cd backend
cp .env.example .env                                  # 填入 GEMINI_API_KEY、HMAC_SECRET
cp config/courses.example.json config/courses.json   # 改成你的課程（見下方格式）
cp config/prompt.example.md config/prompt.md         # 可選：調整評語規則
cargo run --release
```

看到 `Listening on: http://127.0.0.1:8787` 就是啟動成功。可以打開 <http://127.0.0.1:8787/v1/courses> 確認課程有讀到。

### 3. 安裝擴充功能

1. 打開 `chrome://extensions`，開啟右上角「開發人員模式」
2. 按「載入未封裝項目」，選擇 `extension/` 資料夾
3. 會自動打開設定頁：填入**學務系統網址**，按「儲存」並在 Chrome 詢問時按「允許」
4. 按「測試後端連線」確認後端有啟動
5. 重新整理學務系統頁面，右下角就會出現評語面板

之後要改網址：在 `chrome://extensions` 找到「課堂評語助手」→「詳細資料」→「擴充功能選項」，或直接點工具列上的擴充功能圖示。

---

## 課程格式 `backend/config/courses.json`

```json
{
  "series": [
    { "id": "python", "aliases": ["Python", "py"] }
  ],
  "courses": [
    {
      "id": "Scratch1",
      "name": "Scratch 入門",
      "aliases": ["Scratch"],
      "lessons": [
        "【認識 Scratch】，認識舞台、角色與積木區……",
        "【貓咪抓老鼠】，學習「重複執行」與「如果…那麼」積木……"
      ]
    },
    {
      "id": "Python1", "name": "Python 初階", "series": "python", "first_lesson": 1,
      "lessons": ["【第一行程式】……", "【猜數字遊戲】……", "【九九乘法表】……"]
    },
    {
      "id": "Python2", "name": "Python 進階", "series": "python", "first_lesson": 4,
      "lessons": ["【購物清單】……", "【單字卡】……", "【小算盤】……"]
    }
  ]
}
```

| 欄位 | 說明 |
|---|---|
| `id` | 課程代號，不重複即可 |
| `name` | 顯示名稱，也會出現在評語裡；**建議和學務系統上的課程名稱一樣**，自動帶入才認得到 |
| `aliases` | 其他叫法，例如學務系統上的寫法、簡稱 |
| `lessons` | 每課一段介紹，**開頭用【】寫章節名稱**；評語只會引用這裡寫到的內容，寫得越具體評語越準 |
| `series` + `first_lesson` | （可選）同一系列的多個課程**連續編號**，例如初階 L1–L15、進階 L16–L30。輸入 L20 會自動對到進階第 5 課 |
| 最外層 `series` | 系列代號與別名，例如 `{"id":"robot","aliases":["機器人","rb"]}`，讓「機器人2」「rb 2」都能對到 |

改完重新啟動後端（`Ctrl+C` 後再 `cargo run --release`）即可，不用重新編譯擴充功能。

---

## 常見調整

| 想做的事 | 改哪裡 |
|---|---|
| 新增 / 修改課程內容 | `backend/config/courses.json`（重啟後端） |
| 評語語氣、字數、結尾句 | `backend/config/prompt.md`（重啟後端） |
| 禁用詞、替換詞、名字最多出現幾次 | `backend/config/style.json`（重啟後端） |
| 換 Gemini 模型、每分鐘請求上限 | `backend/.env` 的 `GEMINI_MODEL`、`RATE_LIMIT_PER_MINUTE` |
| 學習表現關鍵詞、進度選項、面板標題與主色 | `extension/src/content/site-config.js`（到 `chrome://extensions` 重新載入） |
| 學務系統的欄位位置不一樣 | `extension/src/content/site-config.js` 的選擇器與規則（見下方） |

### 對接其他學務系統

`extension/src/content/site-config.js` 決定擴充功能怎麼讀寫頁面，預設值適用於「每位學生的課堂紀錄是一個彈出視窗」的系統：

| 設定 | 預設 | 用途 |
|---|---|---|
| `feedbackField` | `textarea#lesson_report_feedback` | 評語要貼進的欄位（只會用目前看得到的那一個） |
| `dialog` / `dialogTitle` | `.modal` / `.modal-title` | 課堂紀錄視窗與標題 |
| `studentName` | `([^\s>]+)\s*Lesson Record` | 從標題抓學生姓名（第 1 組） |
| `stripSurname` | `true` | 三個字的中文名去掉姓（王小明 → 小明） |
| `lesson` | `<課程名稱> Lesson <數字>` | 從視窗內容抓課程與課次 |
| `completion` | Completed in full → 完成 | 完成狀態文字對應要自動選的關鍵詞 |

找選擇器的方法：在學務系統頁面對欄位按右鍵 →「檢查」，看它的 `id`、`name` 或 `class`。

---

## 專案結構

```
lesson-comment-assistant/
├── backend/                         # Rust API 伺服器（Axum），預設 127.0.0.1:8787
│   ├── config/                      # 每位老師自己的設定（*.example.* 為範例）
│   │   ├── courses.example.json
│   │   ├── prompt.example.md
│   │   └── style.example.json
│   ├── src/
│   │   ├── profile.rs               # 讀取 config/ 的課程、評語規則、風格
│   │   ├── data/courses.rs          # 課程查詢（別名、連續編號）
│   │   ├── prompts/builder.rs       # 組合送給 AI 的內容
│   │   ├── services/                # Gemini client（含重試）、評語生成與檢查
│   │   ├── api/                     # 路由：/health、/v1/courses、/v1/comments/generate
│   │   ├── security/                # CORS、限流、HMAC 簽章、輸入驗證
│   │   └── middleware/              # 請求日誌、安全 headers
│   └── .env.example
├── extension/                       # Chrome 擴充功能（Manifest V3）
│   ├── manifest.json
│   └── src/
│       ├── background/service-worker.js   # 註冊面板、代理後端 API
│       ├── options/                       # 設定頁（學務系統網址、後端網址）
│       └── content/
│           ├── site-config.js       # ← 對接學務系統的設定
│           ├── content.js           # 面板邏輯、自動帶入、貼上
│           ├── panel.html / panel.css
└── scripts/check-extension.py       # 檢查 manifest 與版本號
```

## 隱私與安全

- 學生資料只會送到**你自己電腦上的後端**，再由後端送給 Gemini 生成評語，不經過其他伺服器
- 擴充功能只在你於設定頁指定的網站上執行
- API Key 只存在 `backend/.env`，不會出現在網址或 log 中

## 開發

```bash
cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && python3 scripts/check-extension.py
```

### 發佈新版本

1. 同步修改 `backend/Cargo.toml` 與 `extension/manifest.json` 的版本號（CI 會檢查兩者一致）
2. push 到 `main` 等 CI 通過，再打 tag：`git tag v1.0.1 && git push origin v1.0.1`
3. GitHub Actions 會自動打包 macOS 後端執行檔與擴充功能 zip，發佈到 Releases

## License

MIT

# 課堂評語助手後端 (Rust)

課堂評語助手的本機 API 伺服器。專案總覽與安裝步驟請見根目錄 [README](../README.md)。

## API

### `GET /health`
```json
{ "status": "ok", "version": "1.0.0" }
```

### `GET /v1/courses`
目前載入的課程清單（擴充功能用來產生選單）：
```json
{
  "series": [{ "id": "python", "aliases": ["Python", "py"] }],
  "courses": [
    { "id": "Python1", "name": "Python 初階", "aliases": [], "series": "python",
      "first_lesson": 1, "chapters": ["第一行程式", "猜數字遊戲", "九九乘法表"] }
  ]
}
```

### `POST /v1/comments/generate`

Request（欄位同時接受 `lastUndone` / `last_undone`）：
```json
{
  "student": "小明",
  "course": "Scratch1",
  "lesson": "L3",
  "progress": "已完成並驗收",
  "performance": "專注、主動提問、完成、驗收",
  "issue": "迴圈可以用在哪裡？",
  "lastUndone": false,
  "polish": true
}
```

Response：
```json
{ "comment": "……", "signature": "<HMAC-SHA256 hex>" }
```

| 狀態碼 | 意義 |
|---|---|
| 400 | 欄位驗證失敗（過長、課程名稱含不允許字元） |
| 413 | Request body 超過 32 KB |
| 422 | JSON 格式或欄位型別錯誤 |
| 429 | 超過每分鐘請求上限 |
| 503 | Gemini 忙碌（已自動重試 3 次仍失敗） |
| 500 | 其他內部錯誤（細節只寫在伺服器 log） |

## 安全機制

- API Key 以 `secrecy` 包裝，透過 `x-goog-api-key` header 傳送，不會出現在 URL 或 log
- 全域限流（`RATE_LIMIT_PER_MINUTE`）、32 KB body 上限、Gemini 連線 10s / 總計 60s timeout
- CORS 白名單 + Chrome Private Network Access header
- CSP、X-Frame-Options、nosniff 等安全 headers
- 回應附 HMAC-SHA256 簽章

## 環境變數

見 `.env.example`。必填：`GEMINI_API_KEY`、`HMAC_SECRET`。

| 變數 | 預設 | 說明 |
|---|---|---|
| `CONFIG_DIR` | `config` | 放 `courses.json`、`prompt.md`、`style.json` 的資料夾 |
| `GEMINI_MODEL` | `gemini-3.1-flash-lite` | 使用的 Gemini 模型 |
| `RATE_LIMIT_PER_MINUTE` | `30` | 每分鐘請求上限 |
| `SERVER_HOST` / `SERVER_PORT` | `127.0.0.1` / `8787` | 監聽位址 |

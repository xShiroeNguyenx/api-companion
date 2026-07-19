# Changelog

Mọi thay đổi đáng chú ý của API Companion được ghi ở đây.
Định dạng theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/);
project tuân [Semantic Versioning](https://semver.org/lang/vi/) với quy ước `0.<milestone>.<patch>` trong Phase 1 (xem [PLAN.md §9](./PLAN.md#9-cross-cutting-workstreams)).

## [Unreleased]

_(chưa có)_

## [0.4.1] — 2026-07-19

### Added
- **Nhân bản request** (Duplicate) từ menu ngữ cảnh trên cây collection — tạo nhanh bản sao `"<tên> copy"` cùng thư mục và mở ngay vào tab.
- **Menu kebab `⋯`** trên mỗi node: request → Mở / Nhân bản / Run / Export / Xoá; collection & folder → Thêm request / Thêm folder / Run tất cả / Export / Xoá.

### Changed
- **Nút Xoá chuyển vào trong menu kebab** (đặt cuối, tách bằng vạch ngăn, hover đỏ) thay cho nút `×` ngay cạnh tên → giảm bấm nhầm.
- **Icon app mới**: rounded-square gradient chàm→tím + dấu ngoặc `{ }` và spark ✦ (API + AI), đọc rõ cả ở 32px.

### Fixed
- **Layout thanh công cụ**: nút `cURL` và `</>` (Generate code) bị cắt/tràn chữ do ô icon rộng cố định 34px → chuyển sang co giãn theo nội dung (`min-width` + padding).

## [0.4.0] — 2026-07-18 — **Public Alpha đầu tiên** 🎉

Bản phát hành công khai đầu tiên: HTTP client AI-first (M0–M3) + Ops Workspace SSH/DB (P2-M1) + hệ workspace đa vùng. **Toàn bộ build sạch, 81 test Rust pass.** Windows-first (macOS/Linux best-effort).

> ⚠️ Alpha: build Windows chưa ký số → SmartScreen sẽ cảnh báo (bấm _More info → Run anyway_). Tính năng AI cần bạn tự nhập API key (BYOK).

### HTTP core & app shell (M0)
- Engine HTTP tự dựng trên **hyper** (không reqwest): mọi method, query/headers, body (raw/JSON/form-urlencoded/multipart file/binary), timeout, redirect policy, cookie jar cơ bản, **hủy request đang chạy**.
- **ExchangeRecord** đầy đủ: timing waterfall (DNS/TCP/TLS/TTFB/download), TLS version + cipher + cert chain, raw header order, remote addr, HTTP version.
- Response viewer: pretty JSON + search, raw, headers, timeline, image/TLS preview.
- **History (SQLite)**: lưu request + full response, restore về tab.
- App shell 3-cột, tab system, **command palette (Ctrl+K)**, dark/light theme.
- Auth cơ bản: Bearer / Basic / API key (header hoặc query).
- **Import/export cURL**.

### Collections, Environments & Variables (M1)
- Collections + folder lồng nhau lưu **file TOML git-friendly** (một request = một file).
- Environments + switcher nhanh; biến `{{var}}` resolve theo scope **global < collection < environment**; cảnh báo biến chưa resolve.
- **Inherit auth/headers** từ collection.
- **Secret vào OS keychain**, không bao giờ nằm plaintext trong file.
- **Import Postman v2.1** (collection + environment; qua paste/file/folder/API key).
- Global search request trong command palette.

### AI-first (M2)
- **AI provider BYOK**: Claude (mặc định) + OpenAI + Gemini + Ollama — settings nhập key (lưu keychain) + test connection.
- **AI Generate Request** từ ngôn ngữ tự nhiên (có context biến + collection, preview Insert / Insert&Send / Refine).
- **AI Explain API** (side panel Markdown).
- **Secret scrubber bắt buộc** trước mọi payload gửi AI (có unit test khẳng định secret không lọt).

### Smart & Self-Testing (M3)
- **Smart variables**: `{{uuid.v4/v7}}`, `{{today+7:YYYY-MM-DD}}`, `{{faker.*}}`, `{{jwt(token).exp}}`, `{{otp(secret)}}`, `{{randomInt(a,b)}}`, dynamic kiểu Postman `{{$guid}}`…
- **Declarative assertion runner** (status/jsonpath/header/response-time/body) chạy tự động sau Send; tab Tests báo pass/fail.
- **AI Diagnose "Why 4xx/5xx?"**: chip tự hiện khi lỗi → rule-based tức thì + AI, kèm evidence + Apply fix.
- **AI Generate Test Cases** (valid/invalid/boundary/sqli/xss/unicode…) → thêm assertions / run now, có fallback tĩnh.
- **Run collection/folder** (nút ▶) + báo cáo pass/fail.

### Ops Workspace — SSH + Database (P2-M1)
- **Connection manager** SSH/DB (file TOML git-friendly, secret vào keychain) + test connection.
- **DB query runner** (Postgres/MySQL/SQLite) **enforce read-only** — chỉ SELECT/EXPLAIN, chặn DML/DDL ở tầng parse trước khi chạm DB.
- **SSH command runner** (tail/grep log) qua ssh binary hệ thống. Panel 🛠 Ops.

### Workspace đa vùng + 3 feature (lấy cảm hứng Hoppscotch)
- **Multi-workspace registry** (bảng SQLite `workspaces`, migration v4): quản lý nhiều workspace hạng nhất (personal/shared, màu nhãn), một workspace active, dedup theo path, invariant đúng-một-active. Switcher + **WorkspaceManager** (đổi tên/màu/kind/gỡ; gỡ KHÔNG xoá file) + tích hợp command palette.
- **Namespace secret theo workspace**: secret env định danh `(workspace_id, env, key)` → hết đụng độ khi hai workspace trùng tên env. Migrate lazy read-through (đọc scoped → fallback legacy + copy-forward, không mất secret cũ, rollback-safe).
- **Persist & restore tabs** theo từng workspace: đóng/mở app hoặc đổi workspace vẫn giữ nguyên tab đang mở (kể cả request chưa lưu).
- **Code generation** đa ngôn ngữ: sinh snippet cho cURL, HTTP raw, JavaScript (fetch/axios), Node (node-fetch), Python (requests/httpx), Go (net/http), PHP (cURL), Rust (reqwest) — nút `</>` trên toolbar + command palette.

### Chia sẻ & Import/Export
- Export **native bundle** (`.apic.json`) giữ assertions/smart-vars (1 collection hoặc cả workspace) + **Postman v2.1** (interop).
- Import auto-detect bundle vs Postman; **bulk import** qua Postman API key / folder / nhiều file.
- **Shared workspace** = đặt thư mục TOML trên OneDrive/Dropbox/network drive → cả team cùng mở (bất đồng bộ; secret riêng mỗi máy trong keychain).

### Nền tảng kỹ thuật
- **Tauri v2** (Rust core + React/TypeScript, Zustand). WebView không bao giờ tự gọi network — mọi request qua Rust core.
- Rust workspace: 16 crate module hoá (ipc-types, http-engine, storage, workspace, secrets, ai-core, smart-vars, assertions, diagnose, ops-db, ops-ssh, bundle, codegen, curl-tools, postman-import, apitest).
- Storage SQLite (rusqlite bundled): history + settings + registry workspace. Files TOML là source-of-truth cho những gì cần git.

### Đã biết còn hạn chế (Known limitations)
- Build Windows **chưa ký số** (SmartScreen cảnh báo) — sẽ mua cert trước beta.
- AI **non-streaming** (chờ full response); chưa có streaming SSE.
- Ops SSH dùng `ssh` binary hệ thống (password cần `sshpass`); DB/SSH từ xa cần server thật để verify end-to-end.
- Codegen multipart/binary hiện để chú thích (chưa sinh code phần file).
- macOS/Linux: build best-effort, chưa test kỹ.
- Chưa có: GraphQL/WebSocket/gRPC (M4/M6), diff engine (M5), git panel, drag-drop node, rename giữ tên folder gốc. Xem [ICEBOX.md](./ICEBOX.md).

[Unreleased]: https://github.com/OWNER/api-companion/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/OWNER/api-companion/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/OWNER/api-companion/releases/tag/v0.4.0

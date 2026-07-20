# API Companion — Kế hoạch phát triển tổng thể

> **"Everything about APIs"** — không chỉ là Send Request.
> Desktop app AI-first thay thế Postman, tích hợp Ops Workspace (SSH/Docker/K8s/DB) và Plugin Marketplace.

| | |
|---|---|
| **Nền tảng** | Tauri v2 desktop app (Windows-first, cross-platform sau) |
| **AI** | Multi-provider BYOK — Claude (mặc định) + OpenAI + Gemini + Ollama |
| **Quy mô** | Solo dev + AI-assisted coding (Claude Code) |
| **Ngày lập plan** | 2026-07-13 |
| **Phiên bản hiện tại** | **`0.4.3` — Public Alpha** (cập nhật 2026-07-20) |

---

## Trạng thái thực thi (cập nhật 2026-07-18)

**Đã ship trong `v0.4.0` (Public Alpha):**

- ✅ **Phase 1 · M0–M3**: HTTP core + app shell → Collections/Env/Variables (daily driver) → AI identity (BYOK Generate/Explain) → Smart variables + assertion runner + AI Diagnose + Generate Test Cases.
- ✅ **Phase 2 · P2-M1**: Ops Workspace — SSH command runner + DB query runner (read-only enforced).
- ✅ **Bonus (ngoài roadmap gốc)**: multi-workspace registry (personal/shared/team) + namespace secret theo workspace + persist/restore tabs per-workspace + code generation đa ngôn ngữ; Postman bulk import; export native bundle + Postman; **team workspace MySQL** (v0.4.2 — mirror + 3-way sync, crate `workspace-sync`).
- 📊 95 test Rust pass; frontend typecheck + bundle sạch. Xem [CHANGELOG.md](./CHANGELOG.md).

**Chưa làm (theo roadmap):** M4 GraphQL/WebSocket · M5 Diff Engine · M6 gRPC · M7 Plugin SDK · Phase 2 P2-M2…M5 · toàn bộ Phase 3. **1.0.0 = hết Phase 1 (M7).**

---

## Mục lục

1. [Vision & Định vị](#1-vision--định-vị)
2. [Quyết định nền tảng](#2-quyết-định-nền-tảng)
3. [Kiến trúc kỹ thuật](#3-kiến-trúc-kỹ-thuật)
4. [Cấu trúc monorepo](#4-cấu-trúc-monorepo)
5. [Roadmap Phase 1 — MVP (M0 → M7)](#5-roadmap-phase-1--mvp)
6. [Spec chi tiết 4 tính năng AI](#6-spec-chi-tiết-4-tính-năng-ai)
7. [Phase 2 — Ops Workspace](#7-phase-2--ops-workspace)
8. [Phase 3 — Platform](#8-phase-3--platform)
9. [Cross-cutting workstreams](#9-cross-cutting-workstreams)
10. [Risk register](#10-risk-register)
11. [Success metrics](#11-success-metrics)
12. [Open decisions](#12-open-decisions)
13. [Bước tiếp theo ngay sau PLAN.md](#13-bước-tiếp-theo-ngay-sau-planmd)
14. [Phụ lục: Bảng ánh xạ 35 tính năng → milestone](#14-phụ-lục-bảng-ánh-xạ-35-tính-năng--milestone)

---

## 1. Vision & Định vị

### Triết lý

Postman và các đối thủ (Bruno, Insomnia, Hoppscotch) đều xoay quanh **"Send Request"**. API Companion xoay quanh **"Everything about APIs"**: hiểu API, sinh request bằng ngôn ngữ tự nhiên, chẩn đoán lỗi, kiểm chứng dữ liệu trong DB, đọc log qua SSH, điều tra sự cố tự động — toàn bộ vòng đời làm việc với API của một backend/DevOps engineer trong **một** ứng dụng.

### 3 differentiator chiến lược

1. **AI Investigation Agent** — không chỉ gửi request mà tự điều tra nguyên nhân lỗi: gọi API, tail log qua SSH, query DB, so sánh staging, xem git history → kết luận root-cause kèm evidence chain. Chưa HTTP client nào có.
2. **Ops Workspace** — SSH, Docker, Kubernetes, Database, log ngay trong app, gắn theo environment. Đúng quy trình xử lý sự cố của backend/DevOps: không cần mở Navicat, terminal, k9s.
3. **Plugin Marketplace** — nền tảng mở để cộng đồng viết connector (AWS, Stripe, Kafka, Redis...) thay vì chỉ là HTTP client đóng.

### Chiến lược cạnh tranh

**Không đấu ở "HTTP client tốt hơn"** — Bruno/Hoppscotch đã miễn phí và tốt. Đấu ở tổ hợp AI diagnose + Ops Workspace + Investigation Agent mà chưa ai có. AI identity phải xuất hiện ngay từ alpha (M2–M3) để định vị sản phẩm. Hạ rào di cư bằng Postman collection import một chiều.

---

## 2. Quyết định nền tảng

### Quyết định sản phẩm (đã chốt)

| Quyết định | Lựa chọn | Lý do |
|---|---|---|
| Nền tảng | **Tauri v2** | Nhẹ (~10MB vs ~150MB Electron), Rust core truy cập native SSH/Docker/DB/gRPC, an toàn |
| AI | **Multi-provider BYOK** | User tự nhập key (Claude mặc định, OpenAI/Gemini/Ollama). Không lo chi phí server; Ollama = privacy mode cho môi trường cấm gửi data ra ngoài |
| Quy mô | Solo + AI-coding | Kiến trúc module hoá triệt để: mỗi crate một AI agent own được, contract rõ ràng |

### Quyết định kỹ thuật then chốt

| Điểm | Chốt | Lý do ngắn gọn |
|---|---|---|
| HTTP engine | **hyper 1.x tự dựng** (không reqwest): `hyper` + `hyper-util` + `tower` + `tokio-rustls` (rustls 0.23) + `hickory-resolver` | reqwest che mất connection internals. Timing waterfall (DNS/TCP/TLS/TTFB) và TLS cert chain là tính năng lõi của Postman-replacement, không phải nice-to-have |
| Frontend | **React 19 + TypeScript**, Zustand 5 + TanStack Query 5, Tailwind CSS 4 + shadcn/ui, **CodeMirror 6**, TanStack Virtual, Apache ECharts, react-resizable-panels, react-hook-form + zod | Corpus AI lớn nhất → AI agents sinh code chính xác nhất; CodeMirror nhẹ hơn Monaco nhiều lần khi mở 10+ tab |
| File format | **TOML qua `toml_edit`** (giữ comment/formatting → git diff sạch); **một request = một file**; body lớn tách sidecar `.body.json`; `schemaVersion` + migration từ M1 | Không custom DSL (chi phí tooling), không YAML (serde_yaml unmaintained), không JSON (diff noisy) |
| Storage runtime | **SQLite qua `rusqlite` bundled** (sync, chạy trong `spawn_blocking`): history (body nén `zstd`, FTS5 search), timeline analytics, cookie jar, schema cache, benchmark/monitoring results | Files = source of truth cho những gì cần git; SQLite = mọi thứ sinh ra khi chạy |
| Secrets | **OS keychain qua `keyring` 3.x** (Windows Credential Manager / macOS Keychain / Secret Service). File env chỉ khai báo `api_key = { type = "secret" }` — resolve lúc runtime | Secret không bao giờ nằm plaintext trên đĩa; Phase 2 mở rộng qua trait `SecretResolver` |
| AI layer | **100% trong Rust core**: trait `AiProvider`, thin client tự viết cho 4 provider (không SDK bên thứ ba), unified message model kiểu Anthropic (content blocks + tool_use), streaming qua `tauri::ipc::Channel`, **secret scrubber bắt buộc** trước mọi AI call | API key không bao giờ vào WebView; agent loop Phase 3 gọi tools sống ở Rust không tốn IPC round-trip |
| JS runtime | **QuickJS nhúng (`rquickjs`)** — deny-by-default, mọi capability là host function do Rust inject. Dùng chung cho: user scripts (pre/post request), Plugin SDK, Mock Server conditional logic. Custom UI panel = sandboxed iframe + postMessage | Một implementation, ba tính năng. WASM giết ecosystem (bắt compile), external process quá nặng cho hooks đồng bộ |
| IPC | **`tauri-specta` v2** generate TypeScript bindings từ commands + events; streaming dùng `tauri::ipc::Channel<T>` | Frontend và Rust không bao giờ lệch kiểu; AI agents hai bên làm việc trên cùng contract sinh tự động |
| gRPC | **`tonic` 0.13 + `prost-reflect` (DynamicMessage) + `protox`** (compile .proto thuần Rust, không ship protoc); server reflection + fallback import .proto; descriptors cache SQLite | MVP: unary + server-streaming; bidi optional |
| WebSocket / GraphQL / SSE | `tokio-tungstenite`; GraphQL chạy trên http-engine (introspection cache SQLite, subscriptions qua graphql-ws); SSE qua `eventsource-stream` (dùng chung cho AI streaming) | |
| Ops crates (P2) | SSH: **`russh`** (pure Rust async; ssh2 là binding C khổ trên Windows); Docker: **`bollard`** (hỗ trợ Windows named pipe); K8s: **`kube` + `k8s-openapi`**; DB: **`sqlx` 0.8** (Postgres/MySQL trước), Redis: `fred` | AI `db_query` enforce read-only qua `sqlparser` classification |
| Cookie jar | `cookie_store`, persist SQLite | Phục vụ Cookie Explorer P2 |
| Smart variables | Crate `workspace`, function registry: `uuid` (v7), `fake`, `jiff` (date math), `jsonwebtoken` | |

### Quy tắc vàng kiến trúc

> **WebView không bao giờ tự gọi network.** Mọi request đi qua http-engine ở Rust core — tránh CORS, tránh lộ secret, và mọi exchange đều có metadata đầy đủ. Frontend chỉ là "màn hình" của Rust core.

---

## 3. Kiến trúc kỹ thuật

### 3.1. Process model (Tauri v2)

```
┌─────────────────────────────────────────────────────┐
│  WebView (React) — UI thuần túy, KHÔNG network I/O   │
│  render, editor, state hiển thị, virtualized lists   │
└──────────────────────┬──────────────────────────────┘
        IPC: tauri commands + events + Channel (typed qua tauri-specta)
┌──────────────────────┴──────────────────────────────┐
│  Rust Core (main process) — TOÀN BỘ logic nghiệp vụ  │
│  http-engine, grpc, ws, storage, secrets, AI,        │
│  plugin host, ops (ssh/docker/k8s/db), agent loop    │
└──────────────────────┬──────────────────────────────┘
┌──────────────────────┴──────────────────────────────┐
│  Sidecars (tối thiểu) — external-process plugins      │
│  (Phase 3) và binary tùy chọn (vd. kubectl auth)      │
└─────────────────────────────────────────────────────┘
```

### 3.2. HTTP engine & ExchangeRecord

Mỗi request trả về một `ExchangeRecord` với:

- **Timings**: `dns_ms`, `tcp_connect_ms`, `tls_handshake_ms`, `request_write_ms`, `ttfb_ms`, `download_ms`, `total_ms` — đo bằng cách wrap connector từng tầng (tower layers).
- **TLS**: version, cipher suite, cert chain (parse bằng `x509-parser`), ALPN negotiated.
- **Raw**: header order + case gốc (`http1_preserve_header_case`), raw body bytes trước decompress (decompress thủ công bằng `async-compression`), `remote_addr`, HTTP version.

HTTP/3: hoãn — thêm sau bằng `quinn + h3` sau lớp trait `Transport`.

### 3.3. Data layer — hybrid Files + SQLite

**Files (workspace directory, user tự `git init`):**

```
my-workspace/
├─ workspace.toml
├─ collections/orders/
│  ├─ collection.toml           # metadata, auth kế thừa, thứ tự
│  ├─ create-order.toml         # method, url, headers, assertions
│  └─ create-order.body.json    # body lớn tách sidecar → diff đẹp
├─ environments/
│  ├─ staging.toml              # giá trị thường + khai báo secret BY NAME ONLY
│  └─ prod.toml
├─ connections/                 # Phase 2: ssh/db/k8s configs (không chứa password)
└─ flows/                       # Phase 3: Visual API Flow definitions
```

**SQLite (app-data dir):** history, timeline analytics, benchmark/monitoring results, cookie jar, GraphQL/gRPC schema cache, UI state.

**Retention cho API Timeline (2 tầng):** metadata rows (url, status, timings, sizes) giữ **vô hạn** (rất nhỏ); bodies giữ 30 ngày hoặc cap 1GB (config được), prune bodies nhưng giữ metadata → Timeline vẫn vẽ analytics dài hạn.

### 3.4. AI layer

```rust
// crates/ai-core
trait AiProvider: Send + Sync {
    fn id(&self) -> ProviderId; // anthropic | openai | gemini | ollama
    async fn chat_stream(&self, req: ChatRequest) -> Result<BoxStream<AiEvent>>;
    fn capabilities(&self) -> Caps; // tool_use, vision, prompt_caching, max_context
}
```

**Context assembly (`ContextBuilder`)** — pack theo thứ tự **ổn định** để tận dụng prompt caching:

```
[system instructions] → [collection summary: tên/method/path]
→ [schema slice liên quan — path-matching, KHÔNG cần embeddings ở MVP]
→ [environment đã redact secrets]
→ [request hiện tại + response cuối (headers + N KB đầu + JSON shape summary)]
```

Anthropic: `cache_control` breakpoints sau phần stable; OpenAI/Gemini hưởng lợi tự động từ stable prefix.

**Secret scrubber (bắt buộc, có unit test):** match giá trị secrets đã biết + heuristic entropy cho Authorization headers; JWT → decode claims nhưng cắt signature; response body cap ~8–16KB. Toggle trong UI + panel "xem payload đã gửi cho AI" (transparency = trust).

**Agent loop (Phase 3):**

```rust
// crates/ai-agent
trait AgentTool { fn name(); fn schema() -> JsonSchema; async fn run(input) -> ToolResult; }
```

Loop chuẩn: model → tool_use → **permission gate** (tool nhạy cảm như `ssh_exec` phải được user approve qua UI) → execute → tool_result → lặp. Có max-iterations, token budget, `CancellationToken`. Tools: `http_call`, `read_logs`, `db_query` (read-only enforce bằng `sqlparser`), `k8s_logs`, `git_log`, `compare_env`, `read_collection`.

### 3.5. Plugin SDK

- **Runtime:** QuickJS (`rquickjs`) cho logic + sandboxed iframe cho custom panels.
- **Manifest `plugin.toml`:** `id`, `version`, `entry`, `permissions = { net = ["api.example.com"], workspace = "read", ui_panels = true }` — user duyệt lúc install.
- **API surface MVP:** `onPreRequest(ctx)`, `onPostResponse(ctx)`, `registerVariableFunction()`, `registerCodegen()`, custom panel qua postMessage bridge, `registerAiTool()` (đầu tư trước cho Investigation Agent).
- **Limits:** memory cap + interrupt handler (timeout). Plugin **không bao giờ** thấy secret values.
- **npm package `@api-companion/plugin-sdk`:** TypeScript types + local test harness.
- Phase 3 Marketplace: registry = git repo index, plugin ký ed25519 (kiểu minisign).

### 3.6. Ops integrations liên kết với Environments (thiết kế quan trọng)

File `connections/*.toml` định nghĩa connection (host, user, secret refs); mỗi environment tham chiếu theo id:

```toml
# environments/prod.toml
[ops]
ssh = "prod-bastion"
db  = "prod-mysql-replica"
k8s = "prod-cluster/namespace-api"
```

→ Đổi env staging↔prod là đổi luôn toàn bộ ops targets. Đây là nền để Investigation Agent tự biết "log/DB của môi trường đang test nằm ở đâu".

### 3.7. Cross-cutting kỹ thuật

- **Error handling:** mỗi crate có error enum riêng (`thiserror`); app layer convert về `AppError { code, message, details }` serializable — không để `anyhow` vượt IPC boundary; frontend map `code` → message i18n.
- **i18n:** UI **English mặc định** (`i18next + react-i18next` từ ngày đầu), locale `vi` là ngôn ngữ thứ hai ship sớm.
- **Auto-update:** `tauri-plugin-updater` + artifact ký, GitHub Releases làm update server; Windows đóng gói NSIS.
- **Telemetry:** opt-in, **Aptabase** (privacy-first, làm riêng cho Tauri); crash reporting opt-in qua sentry Rust SDK.

---

## 4. Cấu trúc monorepo

pnpm workspaces + cargo workspace, task runner: `just`.

```
API-companion/
├─ apps/
│  └─ desktop/
│     ├─ src/                  # React app (UI thuần, gọi bindings)
│     └─ src-tauri/            # CHỈ wiring: tauri commands ↔ crates, setup, menu
├─ crates/
│  ├─ ipc-types/               # DTOs dùng chung + specta derive — "hợp đồng" trung tâm
│  ├─ http-engine/             # hyper engine, timings, TLS capture, cookie jar
│  ├─ protocols/               # tonic gRPC, tokio-tungstenite WS, GraphQL, SSE
│  ├─ workspace/               # TOML file format, collections, env, smart variables
│  ├─ storage/                 # rusqlite, migrations, history, retention, FTS5
│  ├─ secrets/                 # keyring + trait SecretResolver (Vault/AWS... Phase 2)
│  ├─ ai-core/                 # trait AiProvider, 4 thin clients, ContextBuilder, scrubber
│  ├─ ai-agent/                # AgentTool registry + agentic loop (Phase 3)
│  ├─ plugin-host/             # rquickjs runtime, manifest, permission enforcement
│  ├─ diff-engine/             # JSON diff + API diff (headers/latency/status)
│  ├─ ops-ssh/ ops-docker/ ops-k8s/ ops-db/    # Phase 2, mỗi crate một AI agent own được
│  ├─ mock-server/             # axum-based (Phase 2)
│  └─ proxy-recorder/          # hudsucker MITM + rcgen CA (Phase 2)
├─ packages/
│  ├─ bindings/                # TS generated từ tauri-specta — KHÔNG viết tay
│  ├─ ui/                      # shadcn components dùng chung
│  ├─ editor/                  # CodeMirror 6 presets (json, graphql, headers)
│  └─ plugin-sdk/              # @api-companion/plugin-sdk (npm, types + harness)
├─ docs/adr/                   # Architecture Decision Records — AI agents đọc trước khi code
├─ tooling/                    # justfile, scripts, CI helpers
└─ Cargo.toml / pnpm-workspace.yaml
```

**Quy tắc cho AI coding agents:** mỗi crate có `README.md` mô tả contract + public API trait-first; crate không import lẫn nhau ngoài `ipc-types` và các trait crate (`secrets`, `ai-core`) → một agent own trọn một crate, test độc lập, không đụng crate khác.

---

## 5. Roadmap Phase 1 — MVP

### Nguyên tắc re-sequence (không xoá feature nào của danh sách gốc)

1. **"Daily driver" phải đến trước tuần 6–8** (M1): HTTP + Collections + Environments + Variables + History = ngưỡng "tôi mở app này thay vì Postman".
2. **AI Generate Request là identity feature → M2, không phải cuối Phase 1.** AI infra BYOK là nền cho mọi AI feature sau — đầu tư sớm sinh lãi kép.
3. **gRPC nặng nhất, tần suất dùng hàng ngày thấp nhất → M6.** WebSocket nhẹ hơn nhiều nên đi trước (M4).
4. **Plugin SDK foundation ≠ Plugin SDK public.** Từ M0 kiến trúc nội bộ phải "plugin-shaped" (protocol handler, AI action, panel đều qua registry nội bộ); SDK expose ra ngoài là M7, nhãn `experimental`.
5. **Scope ẩn:** AI Generate Test Cases cần **declarative assertion runner** để chạy test → thêm vào M3.

Quy ước size (solo + AI-assisted): **S** ≈ 2–4 ngày, **M** ≈ 1–2 tuần, **L** ≈ 2–4 tuần, **XL** > 4 tuần.

---

### ✅ M0 — "Hello, Request" (HTTP core + app shell) — Size: L — HOÀN TẤT (v0.4.0)

**Goal:** Mở app, gõ URL, bấm Send, thấy response đẹp hơn Postman. Toàn bộ skeleton kỹ thuật (Tauri IPC, state, SQLite) dựng đúng ngay từ đầu.

**Features:**
- **App shell:** layout 3 cột (sidebar / request editor / response pane), tab system, dark/light theme, command palette (Ctrl+K — móc neo cho AI sau này).
- **HTTP engine (Rust, hyper):** mọi method, query params, headers, body (raw/JSON/form-urlencoded/multipart file), timeout, redirect policy, cookie jar cơ bản, hủy request đang chạy.
- **Response viewer:** pretty-print JSON có folding + search, raw view, headers, status/size/timing breakdown (DNS/TCP/TLS/TTFB từ ExchangeRecord), render HTML/image preview.
- **History (SQLite):** snapshot mọi request đã gửi (request + response meta + body có cap), search + restore về tab.
- **Auth cơ bản:** Bearer token, Basic, API key (header/query).
- **curl import/export:** paste curl → request; request → copy as curl. (Rẻ, giá trị dogfood cực cao.)

**Data model essentials:**
```
Request: { id, name, method, url, params[], headers[],
           body: {type: none|raw|json|form|multipart, content},
           auth: {type: inherit|none|bearer|basic|apikey|oauth2, ...},
           description, assertions[] (từ M3), protocol: http (mở rộng sau) }
HistoryEntry (SQLite): { id, requestSnapshot, responseStatus, responseHeaders,
                         responseBodyRef (cap ~1MB), timings, sentAt }
```

**Definition of Done:**
- Gửi GET/POST/PUT/DELETE tới API thật (kèm multipart upload), overhead < 100ms so với curl.
- Đóng app mở lại → history còn nguyên, restore một entry về tab hoạt động đúng.
- Paste một câu curl phức tạp (headers + JSON body) → request dựng đúng 100%.
- Hủy một request treo 30s mà UI không đơ.

**Unblocks:** tất cả.

---

### ✅ M1 — "Daily Driver" (Collections / Environments / Variables) — Size: L — HOÀN TẤT (v0.4.0)

**Goal:** Đủ để bỏ Postman ở công việc hàng ngày với REST API. **Dogfood nghiêm túc từ cuối M1.**

**Features:**
- **Collections:** cây folder lồng nhau, drag-drop; lưu dạng file TOML git-friendly (một request = một file).
- **Environments:** nhiều environment (local/staging/prod), switch nhanh từ topbar; biến flag `secret` → giá trị vào OS keychain, không bao giờ vào file.
- **Variables cơ bản:** `{{var}}` resolve trong URL/headers/body/auth; scope: environment > collection > global; highlight biến chưa resolve.
- **Collection-level auth & headers inherit:** request kế thừa từ collection, override được.
- **Postman collection import (v2.1):** collections + environments — "cầu di cư" quan trọng nhất.
- **Search toàn cục:** tìm request theo tên/URL/method qua command palette.

**Data model essentials:**
```
Collection: { id, name, description, folders[] (tree), auth mặc định,
              headers mặc định, variables[] }
Environment: { id, name, variables: [{key, value, isSecret, description}], ops (P2) }
```

**Definition of Done:**
- Import một Postman collection thật đang dùng ở công ty → chạy được ngay không sửa tay.
- Đổi environment → cùng request trỏ sang host khác; secret không bao giờ xuất hiện plaintext trong file trên đĩa.
- `git init` trong workspace, sửa 1 request → `git diff` đọc được, không phải một dòng JSON 5000 ký tự.
- Tự dùng app cho công việc thật 5 ngày liên tục không phải mở Postman vì thiếu tính năng HTTP cơ bản.

**Unblocks:** M2 (AI cần context collection), M5 (API Diff cần environments), dogfood loop.

---

### ✅ M2 — "AI Identity" (BYOK infra + Generate Request + Explain API) — Size: L — HOÀN TẤT (v0.4.0)

**Goal:** Sản phẩm chính thức là "AI-first". Người dùng mô tả bằng tiếng người, app dựng request.

**Features:**
- **AI provider layer (BYOK):** abstraction thống nhất Claude/OpenAI/Gemini/Ollama; settings UI nhập key + test connection + chọn model; streaming; structured output qua tool-calling/JSON mode; **redaction layer** — mọi payload qua bộ lọc mask secret (chỉ gửi tên biến, không gửi giá trị). Ollama = "privacy mode".
- **AI Generate Request** (spec §6.1).
- **AI Explain API** (spec §6.2).
- **Prompt template registry:** mọi prompt là template có version, lưu trong repo — test và tune được.

**Definition of Done:**
- Chưa nhập API key → mọi entry point AI hiển thị hướng dẫn setup, không crash, không dead-button.
- Gõ "tạo request đăng nhập vào {{base_url}} với email password, lấy JWT" → request đúng method/URL/body, chèn vào collection trong < 10s.
- Cùng flow chạy được với cả 4 provider.
- Kiểm chứng bằng proxy/log: không có secret value nào rời máy.

**Unblocks:** M3 (Diagnose, Test Cases dùng chung AI infra), P3-M3 Investigation Agent.

---

### ✅ M3 — "Smart & Self-Testing" — Size: L → **Public Alpha** — HOÀN TẤT (v0.4.0)

**Goal:** Hoàn thiện bộ tứ AI identity + hệ biến thông minh không đối thủ nào có đủ.

**Features:**
- **Smart variables:**
  - `{{uuid.v4}}` / `{{uuid.v7}}`, `{{timestamp}}`, `{{today+7}}` (date arithmetic, format tùy chọn `{{today+7:YYYY-MM-DD}}`)
  - `{{faker.name}}` / `{{faker.email}}` / `{{faker.phone}}` / `{{faker.*}}` (crate `fake` phía Rust)
  - `{{jwt.exp}}` / `{{jwt.claims.sub}}` — decode JWT từ biến nguồn (cú pháp cuối chốt khi implement)
  - `{{otp}}` — TOTP từ secret trong keychain
  - `{{random.image}}` và các generator khác qua function registry (mở rộng được bằng plugin từ M7)
  - Panel "variable preview": hover thấy giá trị sẽ resolve trước khi gửi.
- **Declarative assertion runner:** mỗi request có assertions khai báo (status == 200, jsonpath `$.data.id` exists, header chứa X, response time < 500ms, body match schema). Chạy 1 request hoặc cả folder ("Run collection") → báo cáo pass/fail. **Chủ ý không làm JS scripting engine ở đây** — scripting đầy đủ dùng QuickJS ở M7.
- **AI Diagnose error response** (spec §6.3) + rule-based fallback (~10 rule tĩnh chạy instant).
- **AI Generate Test Cases** (spec §6.4) — output là assertions + request mutations chạy được trên runner.

**Definition of Done:**
- Request dùng `{{uuid.v7}}` + `{{today+7}}` + `{{otp}}` gửi thành công với giá trị đúng (verify bằng echo server).
- Nhận 403 từ API thật → bấm "Why 403?" → chẩn đoán có căn cứ (chỉ ra header/token cụ thể) + nút Apply fix hoạt động.
- Generate test cases cho một endpoint → chọn 10 case → "Run" → bảng pass/fail đúng.
- Alpha build chạy trên máy Windows sạch (ký số hoặc kèm hướng dẫn SmartScreen).

**Unblocks:** public alpha + feedback loop; runner là nền cho Benchmark và Monitoring (P2).

---

### M4 — "Beyond REST" (GraphQL + WebSocket) — Size: M–L

**Goal:** Hai protocol phổ biến kế tiếp, tận dụng lại toàn bộ UI/variables/AI đã có.

**Features:**
- **GraphQL Studio:** request type riêng — query editor syntax highlight (cm6-graphql), schema introspection từ endpoint, **autocomplete field từ schema**, variables pane (JSON), errors chuẩn GraphQL, history. AI Generate Request hiểu GraphQL (sinh query từ NL dựa trên introspected schema — điểm ăn tiền). AI Explain cho GraphQL.
- **WebSocket Studio:** connect/disconnect, gửi message (text/JSON), message log 2 chiều timestamp + **filter**, replay message, **saved messages** trong request. Smart variables hoạt động trong message.

**Definition of Done:**
- Introspect một GraphQL endpoint thật → autocomplete hoạt động → chạy query có variables.
- "Sinh query lấy 10 user mới nhất kèm email" → AI trả query hợp lệ với schema đã introspect.
- Kết nối WebSocket echo server, gửi/nhận 100 messages, log không nghẽn UI.

**Unblocks:** M6 (pattern "protocol handler thứ N" chứng minh 2 lần), P3-M4 MQTT/Kafka (tái dùng streaming message log UI).

---

### M5 — "Diff Engine" (JSON Diff + API Diff) — Size: M

**Goal:** Trả lời "prod và staging khác gì nhau?" trong một cú click.

**Features:**
- **JSON Diff:** so sánh 2 JSON bất kỳ (paste, 2 history entries, 2 responses) — semantic diff (không quan tâm thứ tự key), highlight added/removed/changed, ignore paths tùy chọn (vd `$.timestamp`).
- **API Diff (prod vs staging):** chọn 1 request + 2 environments → gửi song song → diff status/headers/cookies/latency/body cạnh nhau; lưu "diff profile" (paths cần ignore) vào request.
- **AI summarize diff:** nút "Summarize differences" → 3 dòng ngôn ngữ tự nhiên (rẻ, cộng dồn identity).

**Definition of Done:**
- Diff 2 response 500KB < 1s; thứ tự key khác nhau không báo khác biệt.
- Chạy cùng 1 request trên staging vs prod → thấy chính xác field khác nhau; field ignore không gây nhiễu.

**Unblocks:** P3-M2 Schema Evolution + Contract Breaking Detector (tái dùng diff engine trực tiếp).

---

### M6 — "Heavy Metal" (gRPC) — Size: L–XL

**Goal:** gRPC client hoàn chỉnh, GUI đẹp hơn Postman — hạng mục nặng nhất Phase 1, làm cuối khi hạ tầng đã vững.

**Features:**
- **Proto management:** import file/folder `.proto` (kèm import paths) qua `protox`, hiển thị services/methods; **server reflection** để khỏi cần proto file; descriptors cache SQLite.
- **Unary call:** form nhập message JSON (map từ proto schema qua `prost-reflect`), metadata, TLS/plaintext, deadline.
- **Streaming:** server-streaming + client-streaming với message log UI tái dùng từ WebSocket; bidirectional mức "hoạt động được", polish sau.
- **AI trên gRPC:** Explain service từ proto; Generate Request sinh message JSON từ NL + proto schema.

**Definition of Done:**
- Gọi unary call tới gRPC server thật qua reflection, không cần proto file.
- Import bộ proto nhiều file import lẫn nhau → parse không lỗi.
- Server-streaming hiển thị message real-time và cancel được.

**Điểm cắt định trước:** nếu M6 vượt 4 tuần → cắt bidirectional streaming khỏi DoD, ship phần còn lại (unary + reflection đã phủ 80% use case).

---

### M7 — "Extensible" (Plugin SDK foundation, experimental) — Size: L → **1.0 sau M7**

**Goal:** Nền tảng plugin — đủ để chính mình viết 2 plugin đầu tiên, chưa hứa API ổn định với bên ngoài.

**Features:**
- **Plugin runtime:** QuickJS (`rquickjs`) — cùng runtime với user scripts pre/post request (ship luôn user scripting ở milestone này).
- **Extension points v0:** (1) custom smart variable, (2) request/response hooks, (3) custom panel (sandboxed iframe), (4) **AI tool** (đăng ký tool cho AI dùng — đầu tư trước cho Investigation Agent).
- **Permission model:** plugin khai báo quyền trong `plugin.toml` (network domains, workspace read, ui) — user duyệt khi cài.
- **2 plugin mẫu chính chủ:** "HMAC signature variable" và "export responses to CSV" — proof SDK dùng được thật.
- Load plugin từ local folder; Marketplace để P3-M5. npm `@api-companion/plugin-sdk` + local test harness + docs tử tế (SDK không docs là SDK chết).

**Definition of Done:**
- Viết plugin custom variable < 50 dòng theo docs, load, dùng trong request thành công.
- Plugin không có quyền network bị chặn khi cố fetch.
- API đánh dấu `experimental` rõ ràng trong docs và type definitions.

**Unblocks:** P3-M5 Marketplace, P3-M3 Investigation Agent (plugin-tool interface), cộng đồng contributor.

---

## 6. Spec chi tiết 4 tính năng AI

**Hạ tầng chung:** mọi AI call qua provider layer (M2); mọi context qua **redaction pipeline**: secret values → `«masked:var_name»`; JWT → decode claims (exp/iss/sub), cắt signature; Authorization header → giữ scheme, mask credential; response body cap ~8–16KB (ưu tiên phần đầu + schema tóm tắt). Structured output qua tool-calling (Claude/OpenAI/Gemini) hoặc JSON-mode + retry parse (Ollama).

### 6.1. AI Generate Request từ ngôn ngữ tự nhiên

- **Vị trí UI:** (1) Command palette Ctrl+K → "Generate request…"; (2) **prompt bar ngay trên tab trống mới** ("Describe the request you want…") — entry point chính, người dùng mới thấy trong 5 giây đầu; (3) chuột phải folder → "Generate request here".
- **Context gửi model:** (a) prompt của user; (b) **tên** biến của environment active + collection variables (không giá trị, trừ non-secret `base_url`); (c) tóm tắt collection: tên + method + URL của ~30 request gần vị trí đích (model học convention: prefix path, header đặc thù, auth style); (d) schema OpenAPI/GraphQL liên quan nếu có (truy hồi keyword match); (e) system prompt yêu cầu ưu tiên `{{variables}}` sẵn có thay vì hardcode.
- **Output (tool-calling):** `{ name, method, url, params[], headers[], body: {type, content}, auth: {type,...}, notes, confidence: high|medium|low }`. GraphQL/gRPC (M4/M6): thêm `protocol` + `query`/`message`.
- **Accept/edit:** kết quả là **preview card** — chưa phải request thật; biến chưa tồn tại highlight vàng + nút "create variable". Ba hành động: **Insert** (thành draft trong tab), **Insert & Send**, **Refine** (sửa prompt, giữ context hội thoại). Mọi field edit được trực tiếp trước Insert. **Không bao giờ tự gửi request mà không có click của user.**
- **Fallback không có key:** prompt bar hiện "Set up AI (2 phút)" + deep-link settings; bên dưới là lối thoát không-AI: "Paste curl" + template gallery (REST CRUD skeleton). Không dead-end.

### 6.2. AI Explain API

- **Vị trí UI:** nút "Explain" trên toolbar request editor + command palette + context menu trong sidebar. Kết quả mở **side panel** bên phải (không che response).
- **Context:** request definition đầy đủ (redacted), response gần nhất từ history (status + headers + body cap), description hiện có, tên các request "anh em" cùng folder (suy ra ngữ cảnh nghiệp vụ).
- **Output:** Markdown streaming, cấu trúc ép trong prompt: **Mục đích** (1–2 câu) → **Parameters/Body** (bảng: field, ý nghĩa, bắt buộc?) → **Auth yêu cầu** → **Response structure** → **Lưu ý/edge cases/possible errors**.
- **Accept/edit:** nút **"Save as description"** (ghi Markdown vào description của request → thành docs vĩnh viễn, git-tracked, và là nguồn cho Auto Documentation P3) + **Copy**. Follow-up chat trong panel giữ context.
- **Fallback:** nút hiện tooltip "Requires AI setup" + link settings; panel vẫn hiển thị description thủ công hiện có.

### 6.3. AI Diagnose error response ("Why 403?")

- **Vị trí UI:** khi status ≥ 400 hoặc network error, **suggestion chip xuất hiện tự động trong response pane**: `⚠ Why 403? → Diagnose`. Đây là moment ma thuật của sản phẩm — không cần tìm menu. Cộng nút Diagnose thường trực trong response toolbar.
- **Context:** (a) request đã gửi (redacted; JWT decode claims để model thấy `exp` quá hạn); (b) response đầy đủ (status, headers, body cap); (c) auth config (type + tên biến nguồn); (d) **lần gọi thành công gần nhất tới cùng endpoint từ history + diff hai request** — tín hiệu chẩn đoán mạnh nhất ("lần trước có header X, lần này không"); (e) environment active + timing.
- **Output (tool-calling):** `{ hypotheses: [{cause, evidence: string[], confidence, fix: {description, patch?: {headers|auth|url|body delta}}}], summary }`. Render danh sách xếp hạng: mỗi hypothesis là card có evidence trích dẫn cụ thể ("Token exp = 2026-07-12T09:00, đã quá hạn 3 giờ") + nút **"Apply fix"** khi fix biểu diễn được thành patch.
- **Accept/edit:** "Apply fix" áp patch vào **request draft** (không ghi đè saved request cho tới khi Save), highlight thay đổi; "Apply & Resend" chạy lại ngay. Fix không patch được (vd "kiểm tra quyền IAM") → checklist hướng dẫn.
- **Fallback:** chip vẫn xuất hiện, dẫn tới **rule-based mini-diagnosis** không cần AI (~10 rule tĩnh: JWT hết hạn decode local, thiếu Content-Type, CORS hint, redirect chain...). Vừa là fallback vừa là lớp "instant" chạy trước — hiện ngay, AI bổ sung sau.

### 6.4. AI Generate Test Cases

- **Vị trí UI:** nút "Generate tests" trong tab Assertions của request editor. Dialog chọn categories: `☑ valid ☑ invalid ☑ boundary ☑ SQL injection ☑ XSS ☑ unicode/emoji ☑ long string ☑ special chars ☑ duplicate` + số lượng mỗi loại + ô ghi chú tự do ("field email phải đúng RFC").
- **Context:** request definition (redacted), sample response thành công gần nhất (model biết shape của success), JSON schema của body nếu suy ra được, categories đã chọn. SQLi/XSS: system prompt nêu rõ mục đích security testing trên API của chính user — payload mức kiểm thử tiêu chuẩn (OWASP-style strings), không sinh exploit chain.
- **Output (tool-calling):** `{ tests: [{name, category, requestDelta: {body?|params?|headers? mutations}, expectedAssertions: [{type, target, operator, value}], rationale}] }` — mỗi assertion map được vào declarative assertion runner của M3.
- **Accept/edit:** bảng checklist: tên / category / thay đổi gì / expect gì — expand xem chi tiết, sửa assertion inline, bỏ tick dòng không muốn. Nút **"Add N tests to collection"** tạo folder `Tests/<request-name>` chứa request con + assertions. Nút **"Run now"** chạy ngay → bảng pass/fail; case fail có nút "Diagnose" (nối vào §6.3 — flywheel giữa các AI feature).
- **Fallback:** dialog vẫn hoạt động với **static mutation templates** built-in (bộ payload SQLi/XSS/boundary/unicode chuẩn kiểu big-list-of-naughty-strings áp máy móc vào từng field). Kém thông minh hơn nhưng vẫn hữu dụng — và là baseline so chất lượng AI.

---

## 7. Phase 2 — Ops Workspace

> Chủ đề: **"debug xong một API mà không rời app"**. Bắt đầu sau khi Phase 1 đạt 1.0.

### ✅ P2-M1 — Ops Connections Core (SSH + Database) — XL ⭐ quan trọng nhất Phase 2 — HOÀN TẤT (v0.4.0)

**Goal:** sau khi gọi API, tail/grep log qua SSH và query DB verify data ngay trong app.
**Scope:** SSH connection manager (`russh`: key/password/agent), terminal (xterm.js + PTY channel) + tail/grep panel gắn theo workspace ("POST /payment → tail catalina.out + nginx + php-fpm"); DB clients (`sqlx`: PostgreSQL/MySQL trước, SQLite; SQL Server/Mongo đợt sau); **post-request verify:** đính SQL query vào request, tự chạy sau response và assert kết quả (check order/payment/history/log không cần mở Navicat).
**Dependencies:** assertion runner (M3), secret model (M1), connections ↔ environments (§3.6). Milestone này cấp "tay chân" cho Investigation Agent P3-M3.

### P2-M2 — Container & Cluster (Docker + Kubernetes) — L

**Goal:** xem containers/pods, logs, exec, events, metrics cạnh request đang debug.
**Scope:** Docker qua `bollard` (list/logs/exec/stats/restart/network/volumes); K8s qua `kube` + kubeconfig (pods/logs/events/describe/restart, CPU/Memory metrics, port-forward).
**Dependencies:** pattern connection manager từ P2-M1.

### P2-M3 — Secret Managers — L

**Goal:** environment variables trỏ tới secret backend thay vì lưu local.
**Scope:** providers sau trait `SecretResolver` chung: **Vault + AWS Secrets Manager trước** (phổ biến nhất với team backend), rồi Azure Key Vault / GCP / Bitwarden / 1Password / KeePass — mỗi provider là "thêm một adapter", ship dần được; syntax `source = "vault:kv/data/app#api_key"`; cache + TTL.
**Dependencies:** secret model M1.

### P2-M4 — Traffic Tools (Proxy Recorder + HAR + Replay + Mock Server) — XL

**Goal:** bắt, mổ xẻ, phát lại và giả lập traffic.
**Scope:** **Proxy Recorder** (`hudsucker` MITM + `rcgen` CA per-machine, private key trong keychain, guided install flow, **per-domain allowlist** — chỉ MITM domain user chọn) record browser traffic → convert thành requests (giống Charles); **HAR import + AI analyzer** (waterfall, slow calls, caching, duplicate); **Replay** một request/session N lần (1000 lần); **Mock Server 2.0** (axum): conditional logic dùng QuickJS (`if user==1 → 200 else → 403`), response template, delay, sequence.
**Dependencies:** http-engine M0, collection model M1, plugin runtime M7 (điều kiện Mock Server — tránh phát minh thêm DSL).

### P2-M5 — Quality & Observability — L

**Goal:** đo và canh gác.
**Scope:** **Benchmark** runner (concurrent load từ Rust core: 100/500/1000/5000 requests, TPS/Latency/P95/P99, biểu đồ ECharts); **API Monitor** (schedule chạy request/collection định kỳ + alert **Discord/Slack/Telegram/Email** khi 500/slow/timeout/assertion fail — xem Open decision §12 về chạy nền); **Cookie Explorer** (Session/Expires/Secure/SameSite, từ cookie jar SQLite); **JWT Explorer** (decode/verify/refresh/generate, timeline hết hạn); **OAuth Playground** (authorization code + PKCE + OIDC, browser popup, hiển thị từng bước trao đổi token, refresh); **API Timeline + Performance Insights** (analytics từ history SQLite: "GET user 521 lần, POST login 88 lần, DELETE never used"; Average/Median/P95/Trend theo từng API).
**Dependencies:** runner M3, history M0.

---

## 8. Phase 3 — Platform

### P3-M1 — Git & Spec Sync — L

**Goal:** collection là "code" đúng nghĩa.
**Scope:** **Git integration** (panel status/commit/diff/branch/pull-push cho workspace collections, conflict UX — file TOML từ M1 làm nền); **OpenAPI import + Live Sync** (watch spec URL/file, backend đổi Swagger → đề xuất update requests); **Auto Documentation** từ collections + descriptions (tích luỹ từ AI Explain "Save as description") → export **Markdown/HTML/PDF/Swagger UI** (Confluence/Notion qua plugin sau).
**Dependencies:** file format git-friendly M1.

### P3-M2 — Contract Intelligence — XL

**Goal:** app hiểu API như một hợp đồng tiến hoá theo thời gian.
**Scope:** **Schema Evolution** (chụp schema response theo thời gian từ history → timeline: "customer_name removed, address added" — không cần đọc Swagger); **Contract Breaking Detector** (diff engine M5 + quy tắc semver hoá: removed field / type change String→Number / required mới → báo đỏ); **Local API Discovery** (scanner đọc source Spring/Laravel/Express/Nest/Django/ASP.NET tìm route definitions → sinh collection; mỗi framework một adapter ship dần; AI đọc code làm fallback scanner cho framework chưa có adapter).
**Dependencies:** M5 diff engine, P3-M1 spec model.

### P3-M3 — AI Investigation Agent — XL ⭐ killer feature số 1

**Goal:** "Investigate POST /checkout" → agent tự gọi lại API, tail log SSH, query DB, xem git history, so sánh staging → kết luận "Possible cause: Redis timeout" kèm evidence chain.
**Scope:** agentic loop trên provider layer (tool-calling nhiều bước, max-iterations + token budget + cost estimator hiển thị trước khi chạy); tool registry = plugin-tool interface M7 + connections P2-M1/M2 (`http_call`, `read_logs`, `db_query`, `k8s_logs`, `git_log`, `compare_env`, `read_collection`); **permission gate từng bước** (user duyệt "cho phép agent chạy SQL này?"; mặc định mọi tool read-only — `sqlparser` enforce, không bao giờ tự chạy lệnh ghi); **investigation report** có timeline hành động + trích dẫn log/data làm bằng chứng; lưu thành investigation history.
**Dependencies:** hầu như toàn bộ Phase 2 — lý do Ops Workspace phải xong trước.

### P3-M4 — Visual Flow & Async Protocols — XL

**Goal:** chaining trực quan + phủ nốt async messaging.
**Scope:** **Visual API Flow** (flowchart editor: node = request, edge = mapping response→variable, condition/branch/loop cơ bản; chạy flow với live status từng node — Login→Get User→Get Cart→Checkout→Payment→Notification); **MQTT/Kafka/RabbitMQ GUI** (connect, publish/subscribe/consume, message log tái dùng UI WebSocket/gRPC streaming).
**Dependencies:** runner M3, variables engine, streaming UI M4/M6.

### P3-M5 — Plugin Marketplace — L–XL

**Goal:** hệ sinh thái.
**Scope:** registry (bắt đầu bằng **git repo index — đừng xây backend service vội**), browse/install/update trong app, **ký ed25519 + checksum**, permission review UI khi cài, **stabilize Plugin SDK experimental → v1** (semver commitment), docs + template repo cho plugin author; thêm backend WASM cho plugin compute-heavy nếu cần.
**Dependencies:** M7 SDK đã có ≥ 6 tháng sử dụng thực tế + feedback alpha users.

---

## 9. Cross-cutting workstreams

### Testing

- **Rust core (mọi milestone):** unit tests per crate (`cargo-nextest`); integration tests chạy với **test server nội bộ** (axum echo/scenario server nằm trong repo — CI không cần network, không phụ thuộc httpbin).
- **Smart variables & diff (M3, M5):** golden-file tests — bảng input → expected output.
- **AI features (M2–M3), hai tầng:** (1) **Deterministic:** test redaction pipeline (**bắt buộc** — có test khẳng định secret không lọt vào payload), test parse structured output với fixtures ghi sẵn từ model thật; (2) **Eval nhẹ:** ~20 prompt scenarios chạy thủ công trước mỗi release khi đổi prompt template (không tự động hoá LLM eval khi solo — tốn hơn lợi).
- **UI:** Vitest + Testing Library cho logic (stores, parsers), mock layer bindings; **e2e:** Playwright chạy frontend với IPC mock làm lớp chính (~10 smoke flows: send request, switch env, AI generate với mock provider, run tests) + 1 smoke test tauri-driver trên CI Windows (WebDriver còn thô).
- **Dogfood là tầng test số 1:** từ M1, dùng app cho công việc thật mỗi ngày; bug daily-driver sửa trước feature mới.

### Docs

README + docs site tối giản (để đến alpha M3); mỗi milestone kết thúc bằng changelog + **GIF demo** (là marketing asset luôn). Plugin SDK (M7) là hạng mục duy nhất cần docs "tử tế" ngay. `docs/adr/` ghi Architecture Decision Records — AI coding agents đọc trước khi code.

### Release & packaging

| Mốc | Kênh |
|---|---|
| M1 | Internal build, dogfood cá nhân |
| M2 | Private alpha (5–10 dev quen, Discord/Telegram feedback) |
| **M3 → `v0.4.0`** | ✅ **Public alpha ĐÃ SHIP** — GitHub Releases, Windows trước (chưa ký số, chấp nhận SmartScreen); macOS/Linux "best effort" từ CI matrix. Xem [docs/RELEASE.md](./docs/RELEASE.md) |
| M5 | Beta + mua code signing (updater đã bật sớm từ `0.4.2`) |
| M7 | **1.0** |

Cadence: mỗi milestone = một tagged release + patch release cho bug daily-driver. Code signing Windows: chấp nhận SmartScreen warning ở alpha, mua cert (hoặc Azure Trusted Signing) trước beta. CI build matrix 3 OS từ M1 để phát hiện lệch sớm.

### Versioning

- App: `0.<milestone>.<patch>` trong Phase 1 (M3 → 0.3.x); **1.0.0 = hết Phase 1**; Phase 2 = 1.x minors, Phase 3 = 2.x.
  - **Hiện tại: `0.4.2`** — public alpha M3 cộng thêm P2-M1 (Ops) + workspace registry đã vượt M3 nên bump minor lên `.4`; `0.4.1` là patch daily-driver (menu kebab + Nhân bản, fix layout); `0.4.2` thêm **team workspace MySQL** + icon full-bleed + fix chiều cao button modal. M4 sẽ là `0.5.0`.
- Plugin SDK: version độc lập, `0.x-experimental` cho tới P3-M5.
- File formats: `schemaVersion` riêng ngay từ M1 + migration code — đổi format sau alpha mà không migrate được là mất user.

---

## 10. Risk register

### Rủi ro delivery

| # | Rủi ro | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| D1 | **Scope creep Phase 1** — danh sách feature vốn là 3 sản phẩm gộp | Cao | Cao | Kỷ luật milestone: không mở M(n+1) khi DoD M(n) chưa xanh; feature mới → ghi `ICEBOX.md`, không làm ngay; M6/M7 có điểm cắt định trước |
| D2 | **Solo burnout** — 8 milestones liên tục | Trung bình | Cao | Milestone 2–4 tuần có "moment ship được" (dopamine loop); public alpha sớm M3 để có feedback người thật; polish week sau milestone L/XL; dogfood để công việc chính và side project cộng hưởng |
| D3 | **gRPC phức tạp hơn dự tính** | Cao | Trung bình | Đã đẩy xuống M6; DoD chia bậc (unary+reflection bắt buộc, bidi optional); spike 2 ngày với tonic/prost-reflect trước khi commit scope; vượt 4 tuần → ship unary, còn lại thành 1.1 |
| D4 | **Chi phí AI API khi dev/test** | Trung bình | Trung bình | Dev bằng Ollama local + fixtures ghi sẵn (CI không gọi API thật); prompt caching; cap context; BYOK = user tự trả chi phí runtime |
| D5 | **Cạnh tranh** — Bruno/Hoppscotch/Insomnia sẵn có, Postman đang thêm AI | Cao | Trung bình | Không đấu "HTTP client tốt hơn" — đấu tổ hợp AI diagnose + Ops + Investigation Agent; ship AI identity M2–M3; Postman import hạ rào di cư; theo dõi Postman AI mỗi quý |
| D6 | **Windows-first quirks** — WebView2 drift, SmartScreen, keychain 3 OS | Trung bình | Trung bình | Windows là primary (dogfood tại đó); CI matrix 3 OS từ M1; abstraction keychain/paths từ đầu; mua code signing trước beta |
| D7 | **Plugin SDK freeze API quá sớm** → nợ tương thích vĩnh viễn | Trung bình | Cao | M7 gắn nhãn experimental; nội bộ dùng chính extension points từ sớm để API "chín"; chỉ stabilize ở P3-M5 sau ≥ 6 tháng feedback |
| D8 | **Secret leakage lên LLM** — một lần lộ token là chết uy tín sản phẩm AI-first | Trung bình | Cao | Redaction pipeline có unit test bắt buộc, mọi AI call qua một cửa duy nhất; privacy mode (Ollama-only); panel "xem payload đã gửi cho AI"; không bao giờ gửi env values, chỉ tên biến |

### Rủi ro kỹ thuật

| # | Rủi ro | Mitigation |
|---|---|---|
| T1 | **gRPC reflection + dynamic messages** — nhiều server tắt reflection, proto3/edition edge cases | `prost-reflect` + `protox` (không ship protoc); fallback import .proto; MVP unary + server-streaming; cache descriptors SQLite |
| T2 | **Proxy Recorder cert trust (MITM)** — cài root CA vào trust store là điểm nhạy cảm nhất security/UX | `hudsucker` + `rcgen` CA per-machine; guided install flow; per-domain allowlist; private key CA trong keychain; đã đẩy sang Phase 2 |
| T3 | **Plugin sandbox escape** — plugin bên thứ ba trong app có keychain access | QuickJS không có ambient authority, capability = host function enforce ở Rust; memory/time limits; plugin không bao giờ thấy secret values; marketplace ký ed25519 |
| T4 | **WebView2 Windows: render response lớn + version drift** | Virtualize toàn bộ (TanStack Virtual); cap render body (>2MB → raw viewer phân trang từ Rust); CodeMirror thay Monaco; CI test Evergreen + fixed-version runtime |
| T5 | **AI cost/latency/chất lượng lệch giữa BYOK providers** — agent có thể đốt hàng trăm nghìn tokens | Prompt caching (stable prefix); context budget cứng + cost estimator trước khi chạy agent; model routing (Explain dùng model nhỏ, agent dùng flagship); max-iterations + token ceiling; Ollama fallback offline; mọi AI feature degrade gracefully khi không có key |

---

## 11. Success metrics

### Phase 1 (MVP)

- **"Tôi không mở Postman ở công ty nữa"** — 20 ngày làm việc liên tục chỉ dùng API Companion (bản thân + ≥ 3 alpha user xác nhận tương tự).
- Time-to-first-response của người dùng mới < 30 giây từ lúc mở app.
- AI Generate Request: ≥ 60% kết quả được Insert không phải sửa method/URL.
- AI Diagnose: trong 10 lỗi 4xx/5xx thật khi dogfood, ≥ 7 lần chẩn đoán đúng hướng root-cause.
- Import Postman collection thật của ≥ 5 người khác nhau chạy được không sửa tay.
- Public alpha (sau M3): ≥ 100 downloads, ≥ 10 người quay lại dùng tuần thứ 2.

### Phase 2 (Ops Workspace)

- **Một phiên debug hoàn chỉnh (gọi API → tail log SSH → query DB verify) không rời app** — với ≥ 5 bug thật.
- Bớt được ≥ 3 tool trong workflow cá nhân (terminal SSH riêng, DBeaver/Navicat, k9s) cho tác vụ debug API.
- Mock Server thay một service thật trong dev ≥ 1 dự án; Monitoring bắt ≥ 1 sự cố thật trước user cuối.
- Retention alpha→beta: ≥ 30% user alpha còn active.

### Phase 3 (Platform)

- **AI Investigation Agent tìm đúng root-cause của ≥ 3 sự cố production thật** với evidence chain con người xác nhận đúng — bài test tồn tại của killer feature.
- ≥ 5 plugin từ tác giả bên ngoài trên Marketplace trong 3 tháng đầu.
- Local API Discovery sinh collection dùng được từ ≥ 3 framework khác nhau trên codebase thật.
- Auto Documentation được ít nhất một team dùng làm docs chính thức thay Swagger UI thủ công.

---

## 12. Open decisions

| Điểm | Trạng thái | Deadline quyết định |
|---|---|---|
| **Monitoring chạy nền khi app đóng** — tray process riêng vs chỉ chạy khi app mở | Mở. Nghiêng về "chỉ khi app mở" ở P2-M5 đầu tiên, tray process là enhancement | Trước P2-M5 |
| **HTTP/3 (QUIC)** | Hoãn — thêm sau qua trait `Transport` (`quinn + h3`) | Sau 1.0 |
| **Thứ tự hỗ trợ macOS/Linux chính thức** | Windows primary; mac/linux best-effort CI build từ M1, hỗ trợ chính thức khi có user thật yêu cầu | Trước beta (M5) |
| **Cú pháp cuối của `{{jwt(token_var).exp}}`** | Chốt khi implement M3 | M3 |
| **SQL Server / MongoDB drivers** (`tiberius`, `mongodb`) | Sau Postgres/MySQL, theo nhu cầu user | P2-M1+ |

---

## 13. Bước tiếp theo ngay sau PLAN.md

1. **Scaffold monorepo:** `pnpm-workspace.yaml`, `Cargo.toml` workspace, `justfile`, CI matrix 3 OS (GitHub Actions), `create-tauri-app` base trong `apps/desktop`.
2. **Tạo 5 file nền móng contract** (mọi module sau xây trên đây):
   - `crates/ipc-types/src/lib.rs` — toàn bộ DTOs + specta derive; hợp đồng trung tâm giữa mọi crate và frontend
   - `crates/http-engine/src/lib.rs` — engine hyper + `ExchangeRecord` (timings/TLS); mọi protocol và AI client xây trên đây
   - `crates/workspace/src/format.rs` — TOML file format cho collections/environments + smart variable resolver
   - `crates/ai-core/src/provider.rs` — trait `AiProvider` + unified message model
   - `apps/desktop/src-tauri/src/commands.rs` — wiring tauri commands ↔ crates + tauri-specta export bindings
3. **ADR đầu tiên** trong `docs/adr/`: 0001-tauri-v2, 0002-hyper-not-reqwest, 0003-toml-file-format, 0004-quickjs-runtime, 0005-ai-in-rust-core.
4. **Bắt đầu M0** theo Definition of Done ở §5.
5. Tạo `ICEBOX.md` — nơi ghi mọi ý tưởng mới nảy ra để không phá kỷ luật milestone.

---

## 14. Phụ lục: Bảng ánh xạ 35 tính năng → milestone

| # | Tính năng (danh sách gốc) | Milestone |
|---|---|---|
| 1 | AI First (Generate Request, Explain, Why 403) | M2, M3 |
| 2 | AI Generate Test Case | M3 |
| 3 | AI Fix Response (đọc 500 → NullPointerException) | M3 (Diagnose) |
| 4 | API Timeline (Yesterday/Today/Last Week, call counts) | P2-M5 |
| 5 | Auto Documentation (Swagger/MD/HTML/PDF) | P3-M1 |
| 6 | Visual Flow (Login→...→Notification flowchart) | P3-M4 |
| 7 | Database Integration (call API → query DB verify) | P2-M1 |
| 8 | SSH Integration (tail log, grep) | P2-M1 |
| 9 | Kubernetes Integration | P2-M2 |
| 10 | Docker Integration | P2-M2 |
| 11 | Smart Variables ({{jwt.exp}}, {{uuid.v7}}, {{faker.*}}, {{otp}}) | M3 |
| 12 | Secret Manager (AWS/Vault/Azure/GCP/Bitwarden/1Password/KeePass) | P2-M3 |
| 13 | Request Recorder (browser proxy, giống Charles) | P2-M4 |
| 14 | API Diff (prod vs staging) | M5 |
| 15 | JSON Compare | M5 |
| 16 | Schema Evolution | P3-M2 |
| 17 | Contract Breaking Detector | P3-M2 |
| 18 | Replay (1000 lần) | P2-M4 |
| 19 | Mock Server 2.0 (conditional) | P2-M4 |
| 20 | API Benchmark (TPS/P95/P99) | P2-M5 |
| 21 | API Monitor (alert Discord/Slack/Telegram/Email) | P2-M5 |
| 22 | HAR Analyzer | P2-M4 |
| 23 | Cookie Explorer | P2-M5 |
| 24 | JWT Explorer | P2-M5 |
| 25 | OAuth Playground (PKCE/OIDC) | P2-M5 |
| 26 | gRPC | M6 |
| 27 | GraphQL Studio | M4 |
| 28 | WebSocket Studio | M4 |
| 29 | MQTT / Kafka / RabbitMQ GUI | P3-M4 |
| 30 | AI Agent (Investigation) | P3-M3 |
| 31 | Git Integration (collection commit/review/diff/merge) | P3-M1 |
| 32 | Local API Discovery (scan Spring/Laravel/Express/...) | P3-M2 |
| 33 | OpenAPI Live Sync | P3-M1 |
| 34 | Performance Insights (Average/Median/P95/Trend) | P2-M5 |
| 35 | Plugin Marketplace | M7 (SDK) + P3-M5 (Marketplace) |

**Tính năng bổ sung ngoài danh sách gốc** (đề xuất trong quá trình plan): curl import/export (M0), Postman collection import v2.1 (M1), command palette Ctrl+K (M0), declarative assertion runner (M3), rule-based mini-diagnosis fallback (M3), AI summarize diff (M5), user scripts pre/post request qua QuickJS (M7), SSE support, connections ↔ environments binding (§3.6 — nền cho Investigation Agent).

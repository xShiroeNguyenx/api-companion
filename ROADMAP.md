# Roadmap — API Companion

> Tổng quan lộ trình. Chi tiết kỹ thuật từng milestone: xem [PLAN.md](./PLAN.md). Lịch sử thay đổi: [CHANGELOG.md](./CHANGELOG.md).

**Hiện tại: `v0.4.3` — Public Alpha** · Windows-first · 95 test Rust pass.

Quy ước version: `0.<milestone>.<patch>` trong Phase 1; **`1.0.0` = hết Phase 1 (M7)**; Phase 2 = `1.x`, Phase 3 = `2.x`.

---

## Phase 1 — MVP (HTTP client AI-first)

| Milestone | Nội dung | Trạng thái |
|---|---|---|
| **M0** | Hello Request — HTTP core (hyper) + app shell, timing/TLS, history, cURL | ✅ `v0.4.0` |
| **M1** | Daily Driver — Collections/Environments/Variables (TOML), inherit, secret keychain, Postman import | ✅ `v0.4.0` |
| **M2** | AI Identity — BYOK (Claude/OpenAI/Gemini/Ollama), Generate Request, Explain API, secret scrubber | ✅ `v0.4.0` |
| **M3** | Smart & Self-Testing — smart variables, assertion runner, AI Diagnose 4xx/5xx, Generate Test Cases | ✅ `v0.4.0` |
| **M4** | Beyond REST — GraphQL + WebSocket | ⏳ `0.5.0` |
| **M5** | Diff Engine — JSON Diff + API Diff · beta + code signing | ⏳ |
| **M6** | Heavy Metal — gRPC (tonic + reflection) | ⏳ |
| **M7** | Extensible — Plugin SDK foundation (experimental) → **1.0** | ⏳ |

## Phase 2 — Ops Workspace

| Milestone | Nội dung | Trạng thái |
|---|---|---|
| **P2-M1** ⭐ | Ops Connections Core — SSH command runner + DB query (read-only enforced) | ✅ `v0.4.0` |
| **P2-M2** | Container & Cluster — Docker + Kubernetes | ⏳ |
| **P2-M3** | Secret Managers | ⏳ |
| **P2-M4** | Traffic Tools — Proxy Recorder + HAR + Replay + Mock Server | ⏳ |
| **P2-M5** | Quality & Observability | ⏳ |

## Phase 3 — Platform

| Milestone | Nội dung | Trạng thái |
|---|---|---|
| **P3-M1** | Git & Spec Sync | ⏳ |
| **P3-M2** | Contract Intelligence | ⏳ |
| **P3-M3** ⭐ | AI Investigation Agent — killer feature số 1 | ⏳ |
| **P3-M4** | Visual Flow & Async Protocols | ⏳ |
| **P3-M5** | Plugin Marketplace | ⏳ |

---

## Ngoài roadmap gốc (bonus đã ship)

Những thứ vượt phạm vi M0–M3 nhưng đã có trong alpha:

- **Multi-workspace registry** (personal/shared/team) + namespace secret theo workspace + persist/restore tabs.
- **🗄 Team workspace (MySQL)** (`v0.4.2`) — cả team dùng chung một workspace qua MySQL server tự dựng, đồng bộ 3 chiều theo từng file. Con đường giữa "shared folder" và "sync server realtime".
- **🚀 Auto-update** (`v0.4.2`) — check/tải/cài bản mới trong app, artifact ký minisign.
- **Code generation** đa ngôn ngữ (cURL/HTTP/JS/Python/Go/PHP/Rust); export native bundle + Postman.

## Nguyên tắc

- **Kỷ luật milestone**: không mở M(n+1) khi Definition-of-Done của M(n) chưa xanh. Ý tưởng mới → ghi [ICEBOX.md](./ICEBOX.md), không làm ngay.
- **Local-first**: files TOML là source-of-truth cho những gì cần git; SQLite cho dữ liệu runtime. Không có server bắt buộc, không account.
- **Mỗi milestone** kết thúc bằng một tagged release + CHANGELOG.

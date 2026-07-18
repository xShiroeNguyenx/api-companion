# ADR 0005 — AI chạy 100% trong Rust core, multi-provider BYOK

**Trạng thái:** Accepted (dự kiến hiện thực M2) · **Ngày:** 2026-07-13

## Quyết định
Mọi AI call thực thi trong Rust core sau trait `AiProvider`; thin client tự viết cho Claude (mặc định)/OpenAI/Gemini/Ollama (không SDK bên thứ ba). Streaming đẩy lên UI qua `tauri::ipc::Channel`.

## Lý do
- API key nằm trong keychain, KHÔNG bao giờ vào WebView.
- Agent loop Phase 3 gọi tools sống ở Rust (http_call, ssh_exec, db_query) — loop ở core không tốn IPC round-trip.
- Thin client tận dụng luôn timing/telemetry của http-engine, tránh dependency churn.

## Hệ quả
- **Secret scrubber bắt buộc** (có unit test) trước mọi payload gửi provider — chỉ gửi tên biến, không gửi giá trị.
- Context assembly theo prefix ổn định để tận dụng prompt caching.
- Không có key → mọi AI feature degrade gracefully (fallback rule-based / template).

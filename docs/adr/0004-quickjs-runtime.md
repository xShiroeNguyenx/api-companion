# ADR 0004 — QuickJS (rquickjs) cho plugin & scripting

**Trạng thái:** Accepted (dự kiến hiện thực M7) · **Ngày:** 2026-07-13

## Quyết định
Dùng **QuickJS nhúng (`rquickjs`)** làm runtime cho: user scripts (pre/post request), Plugin SDK, và Mock Server conditional logic. Custom UI panel = sandboxed iframe + postMessage.

## Lý do
- Deny-by-default thật sự: không network, không fs — mọi capability là host function do Rust inject và enforce.
- User Postman quen viết JS → giữ ecosystem (WASM bắt compile sẽ giết cộng đồng).
- Một runtime, ba tính năng.

## Hệ quả
- Plugin không bao giờ thấy secret values.
- Memory cap + interrupt (timeout) qua QuickJS.
- Phase 3: thêm WASM backend cho plugin compute-heavy nếu cần.

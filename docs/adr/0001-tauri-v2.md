# ADR 0001 — Chọn Tauri v2 làm nền tảng

**Trạng thái:** Accepted · **Ngày:** 2026-07-13

## Bối cảnh
Cần một desktop app cross-platform (Windows-first) truy cập được native SSH/Docker/DB/gRPC cho Ops Workspace, nhẹ, và dev nhanh bởi solo + AI-assisted.

## Quyết định
Dùng **Tauri v2**: Rust core + WebView (React). Toàn bộ logic nghiệp vụ nằm ở Rust core; WebView chỉ là UI.

## Lý do
- Nhẹ (~10MB) so với Electron (~150MB); RAM thấp.
- Rust core truy cập native crates (russh, bollard, kube, sqlx) — nền cho Phase 2/3.
- Bảo mật: WebView không có network I/O, secret không vào frontend.

## Hệ quả
- Cần MSVC toolchain trên Windows để build.
- Ecosystem plugin phức tạp hơn → chọn QuickJS cho plugin (ADR 0004).
- WebView2 version drift → virtualize + cap render (xem PLAN.md risk T4).

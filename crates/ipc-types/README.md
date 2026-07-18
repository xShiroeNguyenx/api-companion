# crate: ipc-types

**Contract trung tâm.** Mọi crate Rust và frontend TypeScript nói chuyện qua các type ở đây.

## Public API
- `RequestSpec` — định nghĩa một request (method, url, query, headers, body, auth, timeout, redirects).
- `ExchangeRecord` — kết quả thực thi (response, `Timings`, `TlsInfo`, redirects, error).
- `Environment` / `EnvVar` — biến môi trường (secret chỉ khai báo tên).
- `AppError` + `ErrorCode` — lỗi serializable duy nhất qua IPC.

## Quy tắc
- **Không** thêm logic nghiệp vụ vào crate này — chỉ DTO thuần + helper constructor.
- Đổi type = đổi contract → cập nhật cả engine lẫn frontend bindings.
- Feature `specta` (bật khi wiring Tauri) sinh TypeScript bindings tự động.

## Phụ thuộc
Chỉ `serde`, `serde_json`, `thiserror`, (optional) `specta`. **Không** import crate khác trong workspace.

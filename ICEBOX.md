# ICEBOX

Nơi ghi mọi ý tưởng nảy ra trong quá trình làm — để **không phá kỷ luật milestone**.
Không code những mục ở đây cho tới khi được kéo vào một milestone chính thức trong PLAN.md.

## Hoãn (không chặn M0, làm khi cần)

- **tauri-specta auto-bindings**: thay `apps/desktop/src/types.ts` (viết tay) bằng bindings sinh tự động. Bật feature `specta` của `ipc-types`, thêm `tauri-specta` vào src-tauri, export `bindings.ts` lúc dev. Cần pin version specta khớp.
- **Insecure-TLS mode** (`verify_tls=false`): viết custom `ServerCertVerifier` cho rustls để test self-signed (engine hiện trả Unsupported).
- **HTTP/2 và HTTP/3** (quinn+h3) sau lớp trait `Transport`.
- **Streaming download progress** (hiện collect toàn bộ rồi mới trả) + cap render body >2MB.
- **History body prune** (30 ngày / 1GB) — hiện lưu full record không giới hạn.

## Hoãn của M1 (không chặn, làm khi cần)

- **Collection-level variable editor UI**: hiện sửa được env vars; biến collection/global chỉ đọc từ file (chưa có UI sửa). Resolver đã hỗ trợ đủ 3 scope.
- **Rename/drag-drop** node trong collection tree (hiện có tạo/xoá/mở).
- **Folder giữ tên gốc** (hiện folder hiển thị theo slug thư mục; collection giữ tên trong collection.toml).
- **Import Postman environment** (hiện chỉ import collection v2.1).
- **toml_edit giữ comment** khi ghi lại file (hiện dùng crate `toml`, mất comment khi save).

## Hoãn của M2 (không chặn)

- **AI streaming** (SSE) — hiện non-streaming (chờ full response). Explain sẽ mượt hơn nếu stream token qua `tauri::ipc::Channel`.
- **Tool-calling native** (thay JSON-mode) cho Anthropic/OpenAI — tăng độ tin cậy structured output.
- **Prompt caching** (Anthropic cache_control) để giảm chi phí context lặp lại.
- **Model routing** (task rẻ dùng model nhỏ) + cost estimator trước khi chạy.
- **Rich Markdown render** cho Explain (hiện mini-renderer: heading/bullet/code/bold).
- **AI conversation/Refine đa lượt** giữ lịch sử (hiện Refine = sinh lại từ prompt sửa).

## Hoãn của P2-M1 (không chặn)

- **Connection ↔ Environment binding**: env chọn sẵn DB/SSH connection (ADR §3.6) — hiện chọn connection thủ công trong Ops panel. Nền cho Investigation Agent (Phase 3).
- **Post-request DB verify**: đính SQL vào request, tự chạy sau response và assert (call API → verify DB).
- **ops-ssh pure-Rust (russh)** + **PTY streaming / tail -f live**: hiện dùng ssh binary hệ thống (password cần sshpass). russh cho self-contained password auth + stream log real-time.
- **DB timestamp/decimal stringify**: sqlx Any chưa map datetime/decimal → hiện hiện "<?>"; cần cột kiểu đó thì cast ::text trong SELECT.
- **SQL Server / MongoDB / Redis** drivers (sau Postgres/MySQL/SQLite).

## Đã hoàn thành trong P2-M1 (Ops Workspace)

- ✅ Connection manager SSH/DB (file TOML, secret keychain) + test connection
- ✅ ops-db: read-only guard (sqlparser chặn INSERT/UPDATE/DELETE/DROP...) + query Postgres/MySQL/SQLite
- ✅ ops-ssh: chạy lệnh qua ssh hệ thống (tail/grep log)
- ✅ Ops panel: query runner (bảng kết quả) + SSH command runner

## Hoãn của M3 (không chặn)

- **Variable preview hover**: hover `{{...}}` hiện giá trị resolve (hiện chỉ có chip đếm biến chưa resolve).
- **Assertion regex/schema match**: hiện có eq/ne/contains/exists/lt/gt; chưa có match regex hay JSON schema.
- **Test cases → tạo folder Tests/ với request con**: hiện "Add assertions vào request" + "Run now"; chưa tự tạo request biến thể lưu vào collection.
- **AI Diagnose dùng history diff** (so request lỗi với lần thành công gần nhất) — hiện chỉ dùng request+response hiện tại.
- **jwt/otp**: cú pháp `{{jwt(var).claim}}`, `{{otp(var)}}` cố định; chưa hỗ trợ nhiều thuật toán TOTP.

## Đã hoàn thành trong M3

- ✅ smart-vars: uuid v4/v7, today±N + format, faker.*, jwt(var).claim, otp(var), randomInt, Postman `$` dynamics
- ✅ Assertion runner (status/jsonpath/header/response-time/body) + tab Tests tự chạy sau Send
- ✅ AI Diagnose (rule-based tức thì + AI) + chip "Why 4xx?" + Apply fix
- ✅ AI Generate Test Cases (nhóm categories, add assertions/run now, fallback tĩnh)
- ✅ Run collection/folder (nút ▶) + báo cáo pass/fail

## Đã hoàn thành (v0.4.2): Team workspace MySQL — cả team dùng chung MỘT workspace

Con đường giữa "shared folder" và "sync server realtime kiểu Hoppscotch" (vẫn hoãn vì trái local-first): team tự dựng MySQL, app **mirror + 3-way sync** — không cần account/server app riêng.

- ✅ Crate `workspace-sync`: nội dung workspace vẫn là TOML trong thư mục cache local (mọi tính năng file-based dùng nguyên vẹn); sync 3 chiều local/remote/base theo từng file, tombstone cho xoá, conflict → server thắng + bản local giữ thành `*-conflict-*.toml` và đẩy lên cho cả team; `workspace.toml.active_environment` là lựa chọn cá nhân (không sync).
- ✅ An toàn DB hệ thống: chỉ `CREATE DATABASE/TABLE IF NOT EXISTS` với database MỚI (`apic_workspace` mặc định, validate tên chống injection), 2 bảng `apic_files`/`apic_meta`, mọi SQL qualified — idempotent khi người thứ 2+ join, không ghi đè.
- ✅ Tương thích MySQL cũ: không ép ENGINE (MyISAM-only OK, hết lỗi 1286), PK `path_hash` CHAR(64) (né giới hạn key 767/1000 bytes). Password ở OS keychain (scope workspace id). Registry migration v5 (`remote_json`), kind `team`.
- ✅ 3 command (`team_ws_test/add/sync`) + modal kết nối + auto-sync (mở app/switch/sau mutation debounce 1.5s/poll 30s) + nút Sync trong switcher.

Hoãn: sync realtime (hiện poll 30s — đủ cho team nhỏ); chọn engine/charset tùy ý; nén content; xoá database khi gỡ workspace (hiện giữ nguyên — an toàn trước).

## Đã hoàn thành: Multi-workspace registry + 3 feature (lấy cảm hứng Hoppscotch)

Nâng cấp workspace từ "một folder tại một thời điểm" lên **registry đa-workspace hạng nhất** (thích nghi local-first — bỏ team/GraphQL của Hoppscotch vì trái ADR 0001). Làm tuần tự 6 phase, mỗi phase build+test xanh.

- ✅ **Registry** bảng SQLite `workspaces` (migration v4): mỗi workspace = `{id (uuid), name, path, kind personal|shared, color, is_active, created_at, last_opened_at}`; unique(path) + invariant đúng-một-active (partial index + transaction). CRUD trong `crates/storage`, `normalize_root` trong `crates/workspace`.
- ✅ **Boot seed/migrate**: `resolve_boot_workspace` seed từ legacy `workspace.path` (hoặc default "Personal"), fallback offline giữ metadata (`available=false`). 5 command mới (`list/add/set_active/update/remove_workspace`) + `set_workspace` cũ cài lại trên registry (back-compat).
- ✅ **Switcher chỉn chu** (`WorkspaceSwitcher` viết lại từ registry, không còn localStorage) + **`WorkspaceManager`** modal (đổi tên/màu/kind/gỡ; gỡ KHÔNG xoá file) + tích hợp **command palette** (Switch/Add/Manage). Recent localStorage cũ được `migrateRecents` đẩy vào registry một lần.
- ✅ **[Feature 1] Namespace secret theo workspace**: `secrets::*_scoped` (account = `{workspace_id}\u{1f}{env}\u{1f}{key}`) + `get_scoped_or_legacy` (fallback legacy + copy-forward, KHÔNG xoá legacy → rollback-safe). Wire env (send/preview/run/editor), connection (ops), postman import, AI context. AI key / Postman key giữ app-global có chủ đích.
- ✅ **[Feature 2] Persist & restore tabs per-workspace**: `save/load_tab_session` (SQLite settings key `session.tabs.<id>`); store auto-save debounce + hydrate lúc boot/đổi workspace → mỗi workspace nhớ tab đang mở (kể cả request chưa lưu).
- ✅ **[Feature 3] Code generation đa ngôn ngữ**: crate `codegen` (cURL/HTTP raw/fetch/axios/node-fetch/Python requests+httpx/Go/PHP/Rust reqwest) + command `generate_code`/`list_codegen_targets` + `CodegenModal` (dropdown ngôn ngữ + copy) + nút `</>` trên toolbar + palette.

Hoãn: file lock / cảnh báo conflict khi 2 người sửa cùng lúc; git panel (P3-M1); codegen cho multipart/binary (hiện chú thích); sync `workspace.toml.name` khi rename (hiện registry-only); bulk-migrate secret legacy→scoped (hiện lazy read-through).

## Đã hoàn thành: Shared workspace folder (team dùng chung) — ĐÃ nâng cấp thành registry ở trên

- ✅ Chọn/mở thư mục workspace bất kỳ (`set_workspace`) + nhớ lựa chọn (SQLite settings), fallback default nếu drive offline
- ✅ WorkspaceSwitcher: hiện tên workspace, mở folder khác, recent list
- ✅ Team share = đặt folder trên OneDrive/Dropbox/network drive → mọi người cùng mở (bất đồng bộ, secret riêng keychain)

Hoãn: file lock / cảnh báo conflict khi 2 người sửa cùng lúc; git panel (P3-M1) cho merge/history.

## Đã hoàn thành: Export / Share (team share kiểu Postman)

- ✅ Export **native bundle** (.apic.json) — đầy đủ, giữ assertions/smart-vars; export 1 collection HOẶC cả workspace + environment (secret để trống)
- ✅ Export **Postman v2.1** (.postman_collection.json) — interop với Postman/Insomnia (bỏ assertions)
- ✅ Import auto-detect: luồng Import sẵn có (paste/file/folder/API) tự nhận bundle vs Postman
- ✅ crate `bundle` (format native) + `postman-import::to_postman_*` (exporter)

Hoãn: export kèm file Postman environment riêng (hiện native bundle đã gồm env); nén .zip nhiều file.

## Đã hoàn thành: Bulk Postman import

- ✅ Import qua **Postman API key** (kéo mọi collection + environment của mọi workspace)
- ✅ Import **folder export** (quét đệ quy *.json) và **nhiều file**
- ✅ Parser **Postman environment** (secret → keychain) + `parse_any` tự phân loại
- ✅ Hiển thị summary (số collection/environment/request + lỗi)

## Đã hoàn thành trong M2

- ✅ Provider BYOK 4 nhà (Claude/OpenAI/Gemini/Ollama) qua http-engine, non-streaming, JSON-mode
- ✅ Secret scrubber bắt buộc (test: secret không lọt payload) + redact Authorization/Cookie
- ✅ AI Generate Request (context biến + request lân cận) + preview Insert/Insert&Send/Refine
- ✅ AI Explain API (side panel Markdown tiếng Việt)
- ✅ AI settings (provider/model/key keychain + test connection), fallback khi chưa có key

## Đã hoàn thành trong M1

- ✅ Collections + folders lưu TOML (một request một file), tree scan
- ✅ Environments + switcher + active env trong workspace.toml
- ✅ Variables {{var}} resolver 3 scope + unresolved warning + inherit auth/headers
- ✅ Secret vào OS keychain (keyring), file chỉ giữ tên
- ✅ Postman v2.1 import (folder/request/body/auth/headers/query)
- ✅ Save request vào collection (Ctrl+S), global search trong palette

## Đã hoàn thành trong M0

- ✅ Hủy request đang chạy (CancellationToken + cancel_request + nút Cancel)
- ✅ Multipart / binary body UI (file picker qua plugin-dialog)
- ✅ Lưu full response vào history + restore cả response (storage v2)
- ✅ Import/export cURL (crate curl-tools)
- ✅ Tab system + command palette Ctrl+K
- ✅ Response viewer: pretty/raw toggle, search, image preview

## Ý tưởng chờ (Phase sau)

- _(thêm khi có)_

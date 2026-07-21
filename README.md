# API Companion

**Tiếng Việt** · [English](./README.en.md) · [日本語](./README.ja.md)

> **"Everything about APIs"** — desktop app AI-first thay thế Postman.
> Tauri v2 (Rust core + React) · Multi-provider AI (BYOK) · Ops Workspace · git-friendly.

[![CI](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/ci.yml/badge.svg)](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/ci.yml)
[![Release](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/release.yml/badge.svg)](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/release.yml)

**Phiên bản: `0.4.4` — Public Alpha** · Windows-first · MIT License · 96 test pass

🌐 **[Trang giới thiệu](https://xShiroeNguyenx.github.io/api-companion/)** · ⬇ **[Tải bản mới nhất](https://github.com/xShiroeNguyenx/api-companion/releases/latest)** · 🗺 **[Roadmap](./ROADMAP.md)** · 📋 **[Changelog](./CHANGELOG.md)**

Không chỉ "Send Request": API Companion xoay quanh **toàn bộ vòng đời làm việc với API** — hiểu API, sinh request bằng ngôn ngữ tự nhiên, chẩn đoán lỗi, kiểm chứng dữ liệu trong DB, đọc log qua SSH — trong **một** ứng dụng nhẹ (~10MB).

> ⚠️ **Alpha:** đây là bản phát hành công khai đầu tiên. Bản Windows **chưa ký số** nên SmartScreen sẽ cảnh báo (bấm *More info → Run anyway*). Tính năng AI theo mô hình **BYOK** — bạn tự nhập API key. Xem [known limitations](./CHANGELOG.md#đã-biết-còn-hạn-chế-known-limitations).

---

## Vì sao chọn API Companion

Không đấu ở "HTTP client tốt hơn" (Bruno/Hoppscotch/Insomnia đã miễn phí và tốt). API Companion đấu ở tổ hợp **3 differentiator**:

1. **AI-first** — Generate Request từ ngôn ngữ tự nhiên, Explain API, Diagnose "Why 4xx/5xx?" với evidence + Apply fix, Generate Test Cases. Đa provider BYOK (Claude/OpenAI/Gemini/Ollama), secret không bao giờ rời máy.
2. **Ops Workspace** — SSH tail/grep log + query DB (read-only enforced) ngay trong app, gắn theo workspace. Không cần mở Navicat/terminal riêng.
3. **Local-first & git-friendly** — collections/environments là file **TOML** (một request một file), diff sạch, `git init` là chia sẻ được cho cả team.

---

## Tính năng chính (v0.4.4)

### HTTP & giao thức
- Engine **hyper** tự dựng: mọi method, body (raw/JSON/form/multipart/binary), timeout, redirect, cookie jar, **hủy request**.
- **Timing waterfall** (DNS/TCP/TLS/TTFB/download) + **TLS cert chain** + raw headers.
- Response viewer: pretty JSON + search, raw, image/TLS preview.
- **History** (SQLite) restore cả response. Import/export **cURL**.

### Collections, Environments & Variables
- Collections + folder lồng nhau, lưu **TOML git-friendly** (một request = một file).
- Environments + switcher; biến `{{var}}` scope **global < collection < env**; cảnh báo biến chưa resolve; inherit auth/headers.
- **Smart variables**: `{{uuid.v7}}`, `{{today+7:YYYY-MM-DD}}`, `{{faker.*}}`, `{{jwt(token).exp}}`, `{{otp(secret)}}`, `{{randomInt(a,b)}}`, dynamic kiểu Postman.
- **Secret vào OS keychain** — không bao giờ plaintext trong file.
- **Import Postman v2.1** (paste/file/folder/API key).

### AI (BYOK)
- **Generate Request** từ mô tả tiếng người (preview Insert/Insert&Send/Refine).
- **Explain API** (Markdown side panel).
- **Diagnose "Why 4xx/5xx?"** — rule-based tức thì + AI, evidence + Apply fix.
- **Generate Test Cases** (valid/invalid/boundary/sqli/xss/unicode…).
- **Secret scrubber bắt buộc** trước mọi payload gửi AI (có unit test).

### Testing
- **Declarative assertion runner** (status/jsonpath/header/response-time/body) chạy tự động sau Send.
- **Run collection/folder** → báo cáo pass/fail.

### Ops Workspace (SSH + Database)
- Connection manager SSH/DB (TOML + secret keychain) + test connection.
- **DB query runner** read-only enforced (Postgres/MySQL/SQLite) — chặn DML/DDL ở tầng parse.
- **SSH command runner** (tail/grep log).

### Cập nhật & phân phối
- **🚀 Auto-update**: app tự báo khi có version mới → một chạm "Cập nhật & khởi động lại" (artifact ký minisign, verify trong app; không gặp lại SmartScreen sau lần cài đầu). Có từ v0.4.2.

### Workspace đa vùng + tiện ích
- **Multi-workspace registry**: nhiều workspace hạng nhất (personal/shared/team, màu nhãn), switcher + manager + command palette.
- **🗄 Team workspace (MySQL)**: team tự dựng MySQL server → mỗi thành viên chỉ nhập thông tin kết nối là dùng chung MỘT workspace. Đồng bộ 3 chiều theo từng file (tự động + nút Sync), conflict giữ cả hai bản, password trong OS keychain. Setup chỉ tạo database MỚI riêng — không đụng database khác trên server; chạy được cả MySQL cũ (MyISAM).
- **Namespace secret theo workspace** (hết đụng độ env trùng tên; migrate an toàn không mất secret cũ).
- **Persist & restore tabs** theo từng workspace.
- **Code generation** đa ngôn ngữ: cURL, HTTP raw, JS fetch/axios, node-fetch, Python requests/httpx, Go, PHP, Rust reqwest.
- **Chia sẻ team**: Team workspace qua MySQL (ở trên), hoặc đặt thư mục TOML trên OneDrive/Google Drive/Dropbox/network drive → cả team cùng mở. Export native bundle (`.apic.json`) / Postman v2.1.

---

## Cài đặt

### Tải bản build sẵn (khuyến nghị)
1. Tải installer Windows (`.msi` hoặc `.exe` NSIS) mới nhất từ **[GitHub Releases](https://github.com/xShiroeNguyenx/api-companion/releases)**.
2. Chạy installer. Nếu SmartScreen cảnh báo (do chưa ký số): *More info → Run anyway*.
3. Cần **WebView2 Runtime** (Windows 11 có sẵn; Windows 10 installer sẽ tự nhắc).
4. Từ v0.4.2: các bản sau **tự update trong app** — không cần tải lại installer.

### Build từ source
Xem [docs/RELEASE.md](./docs/RELEASE.md) cho hướng dẫn build & đóng gói đầy đủ. Tóm tắt:

```bash
# Yêu cầu: Rust ≥ 1.80 + target msvc, MSVC Build Tools, Node ≥ 18, pnpm ≥ 9
git clone <repo> && cd API-companion
pnpm install
pnpm --filter api-companion-desktop tauri build   # ra installer trong target/release/bundle/
```

---

## Bắt đầu nhanh (dev)

```bash
# Build & test toàn bộ Rust core
cargo build && cargo test

# Chạy app desktop chế độ dev (hot-reload)
pnpm dev            # = pnpm --filter api-companion-desktop tauri dev

# Smoke-test HTTP engine với URL thật
cargo run -p apitest -- https://example.com
```

Mẹo dùng nhanh: `Ctrl+K` mở command palette · `Ctrl+T` tab mới · `Ctrl+S` lưu request · `Ctrl+Enter` gửi.

---

## Yêu cầu môi trường (Windows)

- Rust ≥ 1.80 (đã test 1.95) + target `x86_64-pc-windows-msvc`
- MSVC Build Tools (VC.Tools.x86.x64) — cần cho link
- Node ≥ 18 + pnpm ≥ 9
- WebView2 Runtime (Windows 11 có sẵn)
- Tauri CLI: `cargo install tauri-cli --version "^2"` (hoặc dùng `pnpm tauri`)

macOS/Linux: build best-effort từ CI matrix, chưa test kỹ ở alpha.

---

## Kiến trúc & trạng thái crate

Rust core module hoá triệt để (mỗi crate một contract trait-first, test độc lập). **95 test pass.**

| Crate / thành phần | Trạng thái |
|---|---|
| `crates/ipc-types` — hợp đồng dữ liệu trung tâm | ✅ 3 test |
| `crates/http-engine` — engine hyper (timing/TLS/redirect/decompress/cancel) | ✅ 5 test + verify endpoint thật |
| `crates/storage` — SQLite history + settings + registry workspace (v5 remote) | ✅ 6 test |
| `crates/curl-tools` — import/export cURL | ✅ 7 test |
| `crates/workspace` — TOML collections/env + resolver + inherit + normalize_root | ✅ 14 test |
| `crates/workspace-sync` — team workspace MySQL (mirror + 3-way sync) | ✅ 13 test |
| `crates/postman-import` — Postman v2.1 collection + environment | ✅ 5 test |
| `crates/secrets` — OS keychain (keyring) + scoped theo workspace | ✅ 1 test |
| `crates/ai-core` — provider BYOK + scrubber + prompts | ✅ 7 test |
| `crates/smart-vars` — {{uuid.v7}}/{{today+7}}/{{faker.*}}/{{jwt}}/{{otp}} | ✅ 9 test |
| `crates/assertions` — declarative runner (status/jsonpath/header/time/body) | ✅ 7 test |
| `crates/diagnose` — rule-based error diagnosis | ✅ 4 test |
| `crates/ops-db` — query DB read-only (sqlparser guard + sqlx) | ✅ 4 test |
| `crates/ops-ssh` — chạy lệnh qua ssh binary hệ thống | ✅ 1 test |
| `crates/bundle` — format share native (export/import) | ✅ 2 test |
| `crates/codegen` — sinh code request đa ngôn ngữ (fetch/python/go/php/rust…) | ✅ 6 test |
| `apps/desktop/src-tauri` — Tauri shell + 57 commands | ✅ 3 test |
| `apps/desktop` — React frontend | ✅ typecheck + bundle sạch |

**Quy tắc vàng:** WebView không bao giờ tự gọi network — mọi request đi qua Rust core (tránh CORS, không lộ secret, metadata đầy đủ).

```
crates/     Rust core — mỗi crate một module độc lập, contract trait-first
apps/       Ứng dụng desktop (Tauri + React)
docs/adr/   Architecture Decision Records
docs/RELEASE.md  Hướng dẫn build & đóng gói release
```

---

## Roadmap

Alpha hiện tại hoàn tất **Phase 1 M0–M3** (HTTP core → daily driver → AI identity → smart & self-testing) + **Phase 2 P2-M1** (Ops SSH/DB) + hệ workspace đa vùng, kèm bonus team workspace MySQL và auto-update.

**Tiếp theo:** M4 GraphQL + WebSocket → M5 Diff Engine → M6 gRPC → M7 Plugin SDK (→ 1.0). Bảng đầy đủ 3 giai đoạn: **[ROADMAP.md](./ROADMAP.md)**.

Chi tiết vision, kiến trúc: **[PLAN.md](./PLAN.md)** · Ý tưởng hoãn/đã làm: **[ICEBOX.md](./ICEBOX.md)** · Lịch sử thay đổi: **[CHANGELOG.md](./CHANGELOG.md)**.

---

## Đóng góp & kỷ luật phát triển

- Không mở milestone mới khi Definition of Done của milestone hiện tại chưa xanh (xem [PLAN.md §5](./PLAN.md)).
- Ý tưởng mới nảy ra → ghi vào [ICEBOX.md](./ICEBOX.md), không làm ngay.
- Mỗi crate có contract rõ ràng để AI coding agent own độc lập; test độc lập từng crate.
- Trước mỗi release: `cargo test` + `pnpm --filter api-companion-desktop build` phải xanh; cập nhật CHANGELOG.

## License

[MIT](./LICENSE) © API Companion.

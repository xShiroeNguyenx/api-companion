# API Companion

[Tiếng Việt](./README.md) · **English** · [日本語](./README.ja.md)

> **"Everything about APIs"** — an AI-first desktop app to replace Postman.
> Tauri v2 (Rust core + React) · Multi-provider AI (BYOK) · Ops Workspace · git-friendly.

[![CI](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/ci.yml/badge.svg)](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/ci.yml)
[![Release](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/release.yml/badge.svg)](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/release.yml)

**Version: `0.4.4` — Public Alpha** · Windows-first · MIT License · 96 tests passing

🌐 **[Landing page](https://xShiroeNguyenx.github.io/api-companion/)** · ⬇ **[Download latest](https://github.com/xShiroeNguyenx/api-companion/releases/latest)** · 🗺 **[Roadmap](./ROADMAP.md)** · 📋 **[Changelog](./CHANGELOG.md)**

More than "Send Request": API Companion is built around the **entire API workflow** — understand an API, generate requests from natural language, diagnose errors, verify data in the DB, read logs over SSH — all in **one** lightweight app (~10MB).

> ⚠️ **Alpha:** this is the first public release. The Windows build is **not code-signed**, so SmartScreen will warn you (click *More info → Run anyway*). AI features follow the **BYOK** model — you provide your own API key. See [known limitations](./CHANGELOG.md).

---

## Why API Companion

It doesn't compete on being "a better HTTP client" (Bruno/Hoppscotch/Insomnia are already free and good). API Companion competes on a combination of **3 differentiators**:

1. **AI-first** — Generate Request from natural language, Explain API, Diagnose "Why 4xx/5xx?" with evidence + Apply fix, Generate Test Cases. Multi-provider BYOK (Claude/OpenAI/Gemini/Ollama); secrets never leave your machine.
2. **Ops Workspace** — SSH tail/grep logs + query the DB (read-only enforced) right inside the app, bound to the workspace. No need to open Navicat or a separate terminal.
3. **Local-first & git-friendly** — collections/environments are **TOML** files (one request per file), clean diffs; `git init` is all it takes to share with the whole team.

---

## Key features (v0.4.4)

### HTTP & protocols
- Custom **hyper** engine: any method, body (raw/JSON/form/multipart/binary), timeout, redirects, cookie jar, **request cancellation**.
- **Timing waterfall** (DNS/TCP/TLS/TTFB/download) + **TLS cert chain** + raw headers.
- Response viewer: pretty JSON + search, raw, image/TLS preview.
- **History** (SQLite) restores the full response too. **cURL** import/export.

### Collections, Environments & Variables
- Nested collections + folders, stored as **git-friendly TOML** (one request = one file).
- Environments + switcher; `{{var}}` scoping **global < collection < env**; warnings for unresolved variables; inherited auth/headers.
- **Smart variables**: `{{uuid.v7}}`, `{{today+7:YYYY-MM-DD}}`, `{{faker.*}}`, `{{jwt(token).exp}}`, `{{otp(secret)}}`, `{{randomInt(a,b)}}`, Postman-style dynamics.
- **Secrets go to the OS keychain** — never plaintext in files.
- **Postman v2.1 import** (paste/file/folder/API key).

### AI (BYOK)
- **Generate Request** from a plain-language description (preview Insert/Insert&Send/Refine).
- **Explain API** (Markdown side panel).
- **Diagnose "Why 4xx/5xx?"** — instant rule-based + AI, with evidence + Apply fix.
- **Generate Test Cases** (valid/invalid/boundary/sqli/xss/unicode…).
- **Mandatory secret scrubber** before any payload is sent to AI (with unit tests).

### Testing
- **Declarative assertion runner** (status/jsonpath/header/response-time/body) runs automatically after Send.
- **Run collection/folder** → pass/fail report.

### Ops Workspace (SSH + Database)
- Connection manager for SSH/DB (TOML + keychain secrets) + test connection.
- **DB query runner** with read-only enforced (Postgres/MySQL/SQLite) — blocks DML/DDL at the parse layer.
- **SSH command runner** (tail/grep logs).

### Updates & distribution
- **🚀 Auto-update**: the app notifies you when a new version is available → one-click "Update & restart" (artifacts signed with minisign, verified in-app; no more SmartScreen warning after the first install). Available since v0.4.2.

### Multi-region workspace + utilities
- **Multi-workspace registry**: multiple first-class workspaces (personal/shared/team, color labels), switcher + manager + command palette.
- **🗄 Team workspace (MySQL)**: the team runs its own MySQL server → each member just enters the connection details to share ONE workspace. Three-way per-file sync (automatic + a Sync button), conflicts keep both copies, password in the OS keychain. Setup only creates a NEW dedicated database — it never touches other databases on the server; works even on old MySQL (MyISAM).
- **Per-workspace secret namespacing** (no more clashes when env names collide; safe migration without losing old secrets).
- **Persist & restore tabs** per workspace.
- **Code generation** in many languages: cURL, raw HTTP, JS fetch/axios, node-fetch, Python requests/httpx, Go, PHP, Rust reqwest.
- **Team sharing**: Team workspace via MySQL (above), or put the TOML folder on OneDrive/Google Drive/Dropbox/network drive → the whole team opens it. Export a native bundle (`.apic.json`) / Postman v2.1.

---

## Installation

### Download a prebuilt binary (recommended)
1. Download the latest Windows installer (`.msi` or NSIS `.exe`) from **[GitHub Releases](https://github.com/xShiroeNguyenx/api-companion/releases)**.
2. Run the installer. If SmartScreen warns (because it isn't signed): *More info → Run anyway*.
3. Requires the **WebView2 Runtime** (built into Windows 11; the Windows 10 installer will prompt you).
4. From v0.4.2: later versions **update in-app** — no need to re-download the installer.

### Build from source
See [docs/RELEASE.md](./docs/RELEASE.md) for the full build & packaging guide. Summary:

```bash
# Requires: Rust ≥ 1.80 + msvc target, MSVC Build Tools, Node ≥ 18, pnpm ≥ 9
git clone <repo> && cd API-companion
pnpm install
pnpm --filter api-companion-desktop tauri build   # installer lands in target/release/bundle/
```

---

## Quick start (dev)

```bash
# Build & test the whole Rust core
cargo build && cargo test

# Run the desktop app in dev mode (hot-reload)
pnpm dev            # = pnpm --filter api-companion-desktop tauri dev

# Smoke-test the HTTP engine against a real URL
cargo run -p apitest -- https://example.com
```

Handy shortcuts: `Ctrl+K` command palette · `Ctrl+T` new tab · `Ctrl+S` save request · `Ctrl+Enter` send.

---

## Environment requirements (Windows)

- Rust ≥ 1.80 (tested on 1.95) + the `x86_64-pc-windows-msvc` target
- MSVC Build Tools (VC.Tools.x86.x64) — needed for linking
- Node ≥ 18 + pnpm ≥ 9
- WebView2 Runtime (built into Windows 11)
- Tauri CLI: `cargo install tauri-cli --version "^2"` (or use `pnpm tauri`)

macOS/Linux: best-effort builds from the CI matrix, not thoroughly tested in alpha.

---

## Architecture & crate status

The Rust core is heavily modularized (each crate is a trait-first contract, tested independently). **95 tests passing.**

| Crate / component | Status |
|---|---|
| `crates/ipc-types` — central data contract | ✅ 3 tests |
| `crates/http-engine` — hyper engine (timing/TLS/redirect/decompress/cancel) | ✅ 5 tests + real endpoint verify |
| `crates/storage` — SQLite history + settings + workspace registry (v5 remote) | ✅ 6 tests |
| `crates/curl-tools` — cURL import/export | ✅ 7 tests |
| `crates/workspace` — TOML collections/env + resolver + inherit + normalize_root | ✅ 14 tests |
| `crates/workspace-sync` — team workspace MySQL (mirror + 3-way sync) | ✅ 13 tests |
| `crates/postman-import` — Postman v2.1 collection + environment | ✅ 5 tests |
| `crates/secrets` — OS keychain (keyring) + per-workspace scoping | ✅ 1 test |
| `crates/ai-core` — BYOK provider + scrubber + prompts | ✅ 7 tests |
| `crates/smart-vars` — {{uuid.v7}}/{{today+7}}/{{faker.*}}/{{jwt}}/{{otp}} | ✅ 9 tests |
| `crates/assertions` — declarative runner (status/jsonpath/header/time/body) | ✅ 7 tests |
| `crates/diagnose` — rule-based error diagnosis | ✅ 4 tests |
| `crates/ops-db` — read-only DB query (sqlparser guard + sqlx) | ✅ 4 tests |
| `crates/ops-ssh` — run commands via the system ssh binary | ✅ 1 test |
| `crates/bundle` — native share format (export/import) | ✅ 2 tests |
| `crates/codegen` — multi-language request codegen (fetch/python/go/php/rust…) | ✅ 6 tests |
| `apps/desktop/src-tauri` — Tauri shell + 57 commands | ✅ 3 tests |
| `apps/desktop` — React frontend | ✅ typecheck + clean bundle |

**Golden rule:** the WebView never makes network calls itself — every request goes through the Rust core (avoids CORS, never leaks secrets, full metadata).

```
crates/     Rust core — each crate an independent module, trait-first contracts
apps/       Desktop app (Tauri + React)
docs/adr/   Architecture Decision Records
docs/RELEASE.md  Build & release packaging guide
```

---

## Roadmap

The current alpha completes **Phase 1 M0–M3** (HTTP core → daily driver → AI identity → smart & self-testing) + **Phase 2 P2-M1** (Ops SSH/DB) + the multi-region workspace system, plus bonus team workspace MySQL and auto-update.

**Next:** M4 GraphQL + WebSocket → M5 Diff Engine → M6 gRPC → M7 Plugin SDK (→ 1.0). Full three-phase table: **[ROADMAP.md](./ROADMAP.md)**.

Full vision & architecture: **[PLAN.md](./PLAN.md)** · Deferred/done ideas: **[ICEBOX.md](./ICEBOX.md)** · Change history: **[CHANGELOG.md](./CHANGELOG.md)**.

---

## Contributing & development discipline

- Don't open a new milestone until the current milestone's Definition of Done is green (see [PLAN.md §5](./PLAN.md)).
- New ideas → write them into [ICEBOX.md](./ICEBOX.md), don't build them immediately.
- Each crate has a clear contract so an AI coding agent can own it independently; each crate is tested independently.
- Before every release: `cargo test` + `pnpm --filter api-companion-desktop build` must be green; update the CHANGELOG.

## License

[MIT](./LICENSE) © API Companion.

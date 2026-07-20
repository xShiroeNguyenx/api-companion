# API Companion

[Tiếng Việt](./README.md) · [English](./README.en.md) · **日本語**

> **「Everything about APIs」** — Postman を置き換える AI ファーストのデスクトップアプリ。
> Tauri v2（Rust コア + React）· マルチプロバイダ AI（BYOK）· Ops ワークスペース · git フレンドリー。

[![CI](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/ci.yml/badge.svg)](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/ci.yml)
[![Release](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/release.yml/badge.svg)](https://github.com/xShiroeNguyenx/api-companion/actions/workflows/release.yml)

**バージョン: `0.4.3` — Public Alpha** · Windows 優先 · MIT ライセンス · テスト 95 件パス

🌐 **[紹介ページ](https://xShiroeNguyenx.github.io/api-companion/)** · ⬇ **[最新版をダウンロード](https://github.com/xShiroeNguyenx/api-companion/releases/latest)** · 🗺 **[ロードマップ](./ROADMAP.md)** · 📋 **[変更履歴](./CHANGELOG.md)**

単なる「Send Request」ではありません。API Companion は **API 作業のライフサイクル全体** を中心に据えています — API を理解し、自然言語からリクエストを生成し、エラーを診断し、DB のデータを検証し、SSH でログを読む — それらすべてを **1 つ** の軽量アプリ（約 10MB）で。

> ⚠️ **アルファ版:** これは最初の公開リリースです。Windows ビルドは **コード署名されていない** ため、SmartScreen が警告を表示します（*詳細情報 → 実行* をクリック）。AI 機能は **BYOK** 方式で、API キーはご自身で入力します。[既知の制限](./CHANGELOG.md) を参照してください。

---

## なぜ API Companion なのか

「より良い HTTP クライアント」で勝負するわけではありません（Bruno/Hoppscotch/Insomnia はすでに無料で優秀です）。API Companion は **3 つの差別化要素** の組み合わせで勝負します:

1. **AI ファースト** — 自然言語からの Generate Request、Explain API、根拠 + 修正適用付きの「なぜ 4xx/5xx か？」の Diagnose、Generate Test Cases。マルチプロバイダ BYOK（Claude/OpenAI/Gemini/Ollama）で、シークレットは端末から出ません。
2. **Ops ワークスペース** — SSH での tail/grep ログ + DB クエリ（読み取り専用を強制）をアプリ内で直接、ワークスペースに紐づけて実行。Navicat や別のターミナルを開く必要はありません。
3. **ローカルファースト & git フレンドリー** — collections/environments は **TOML** ファイル（1 リクエスト = 1 ファイル）で差分がきれい。`git init` するだけでチーム全体と共有できます。

---

## 主な機能（v0.4.3）

### HTTP & プロトコル
- 自作の **hyper** エンジン: あらゆるメソッド、ボディ（raw/JSON/form/multipart/binary）、タイムアウト、リダイレクト、cookie jar、**リクエストのキャンセル**。
- **タイミングウォーターフォール**（DNS/TCP/TLS/TTFB/download）+ **TLS 証明書チェーン** + 生ヘッダー。
- レスポンスビューア: 整形 JSON + 検索、raw、画像/TLS プレビュー。
- **履歴**（SQLite）はレスポンス全体も復元。**cURL** のインポート/エクスポート。

### コレクション、環境 & 変数
- ネストしたコレクション + フォルダを **git フレンドリーな TOML** で保存（1 リクエスト = 1 ファイル）。
- 環境 + スイッチャー。`{{var}}` のスコープは **global < collection < env**。未解決の変数を警告。auth/headers の継承。
- **スマート変数**: `{{uuid.v7}}`、`{{today+7:YYYY-MM-DD}}`、`{{faker.*}}`、`{{jwt(token).exp}}`、`{{otp(secret)}}`、`{{randomInt(a,b)}}`、Postman 風のダイナミック変数。
- **シークレットは OS キーチェーンへ** — ファイルに平文で残しません。
- **Postman v2.1 インポート**（貼り付け/ファイル/フォルダ/API キー）。

### AI（BYOK）
- 平易な説明からの **Generate Request**（Insert/Insert&Send/Refine のプレビュー）。
- **Explain API**（Markdown サイドパネル）。
- **Diagnose「なぜ 4xx/5xx か？」** — 即時のルールベース + AI、根拠 + 修正適用付き。
- **Generate Test Cases**（valid/invalid/boundary/sqli/xss/unicode…）。
- AI に送るすべてのペイロードの前に **必須のシークレットスクラバー**（ユニットテスト付き）。

### テスト
- **宣言的アサーションランナー**（status/jsonpath/header/response-time/body）が Send 後に自動実行。
- **コレクション/フォルダの実行** → pass/fail レポート。

### Ops ワークスペース（SSH + データベース）
- SSH/DB のコネクションマネージャー（TOML + キーチェーンのシークレット）+ 接続テスト。
- **DB クエリランナー** は読み取り専用を強制（Postgres/MySQL/SQLite） — パース層で DML/DDL をブロック。
- **SSH コマンドランナー**（tail/grep ログ）。

### アップデート & 配布
- **🚀 自動アップデート**: 新しいバージョンがあるとアプリが通知 → ワンクリックで「アップデートして再起動」（成果物は minisign で署名しアプリ内で検証。初回インストール後は SmartScreen の警告は出ません）。v0.4.2 から利用可能。

### マルチリージョンのワークスペース + ユーティリティ
- **マルチワークスペースレジストリ**: 第一級のワークスペースを複数（personal/shared/team、カラーラベル）、スイッチャー + マネージャー + コマンドパレット。
- **🗄 チームワークスペース（MySQL）**: チームが自前の MySQL サーバーを立て → 各メンバーは接続情報を入力するだけで 1 つのワークスペースを共有。ファイル単位の 3-way 同期（自動 + Sync ボタン）、コンフリクトは両方のコピーを保持、パスワードは OS キーチェーンに。セットアップでは **新規の専用データベースのみ** を作成 — サーバー上の他のデータベースには一切触れません。古い MySQL（MyISAM）でも動作します。
- **ワークスペースごとのシークレット名前空間**（環境名が衝突しても競合しない。古いシークレットを失わない安全なマイグレーション）。
- **タブの保存 & 復元** をワークスペースごとに。
- **コード生成** を多言語で: cURL、raw HTTP、JS fetch/axios、node-fetch、Python requests/httpx、Go、PHP、Rust reqwest。
- **チーム共有**: MySQL 経由のチームワークスペース（上記）、または TOML フォルダを OneDrive/Google Drive/Dropbox/ネットワークドライブに置いてチーム全員で開く。ネイティブバンドル（`.apic.json`）/ Postman v2.1 のエクスポート。

---

## インストール

### ビルド済みバイナリのダウンロード（推奨）
1. 最新の Windows インストーラー（`.msi` または NSIS `.exe`）を **[GitHub Releases](https://github.com/xShiroeNguyenx/api-companion/releases)** から入手。
2. インストーラーを実行。SmartScreen が警告した場合（未署名のため）: *詳細情報 → 実行*。
3. **WebView2 Runtime** が必要（Windows 11 には同梱。Windows 10 ではインストーラーが案内します）。
4. v0.4.2 から: 以降のバージョンは **アプリ内でアップデート** — インストーラーの再ダウンロードは不要です。

### ソースからビルド
ビルド & パッケージングの完全な手順は [docs/RELEASE.md](./docs/RELEASE.md) を参照。概要:

```bash
# 必要環境: Rust ≥ 1.80 + msvc ターゲット、MSVC Build Tools、Node ≥ 18、pnpm ≥ 9
git clone <repo> && cd API-companion
pnpm install
pnpm --filter api-companion-desktop tauri build   # インストーラーは target/release/bundle/ に出力
```

---

## クイックスタート（開発）

```bash
# Rust コア全体のビルド & テスト
cargo build && cargo test

# デスクトップアプリを開発モードで起動（ホットリロード）
pnpm dev            # = pnpm --filter api-companion-desktop tauri dev

# 実 URL に対して HTTP エンジンをスモークテスト
cargo run -p apitest -- https://example.com
```

便利なショートカット: `Ctrl+K` コマンドパレット · `Ctrl+T` 新規タブ · `Ctrl+S` リクエスト保存 · `Ctrl+Enter` 送信。

---

## 環境要件（Windows）

- Rust ≥ 1.80（1.95 で確認済み）+ `x86_64-pc-windows-msvc` ターゲット
- MSVC Build Tools（VC.Tools.x86.x64） — リンクに必要
- Node ≥ 18 + pnpm ≥ 9
- WebView2 Runtime（Windows 11 に同梱）
- Tauri CLI: `cargo install tauri-cli --version "^2"`（または `pnpm tauri` を使用）

macOS/Linux: CI マトリクスからの best-effort ビルド。アルファではまだ十分にテストしていません。

---

## アーキテクチャ & クレートの状態

Rust コアは徹底的にモジュール化されています（各クレートは trait ファーストの契約で、独立してテスト）。**テスト 95 件パス。**

| クレート / コンポーネント | 状態 |
|---|---|
| `crates/ipc-types` — 中心となるデータ契約 | ✅ 3 件 |
| `crates/http-engine` — hyper エンジン（timing/TLS/redirect/decompress/cancel） | ✅ 5 件 + 実エンドポイント検証 |
| `crates/storage` — SQLite の履歴 + 設定 + ワークスペースレジストリ（v5 remote） | ✅ 6 件 |
| `crates/curl-tools` — cURL のインポート/エクスポート | ✅ 7 件 |
| `crates/workspace` — TOML の collections/env + リゾルバ + 継承 + normalize_root | ✅ 14 件 |
| `crates/workspace-sync` — チームワークスペース MySQL（ミラー + 3-way 同期） | ✅ 13 件 |
| `crates/postman-import` — Postman v2.1 の collection + environment | ✅ 5 件 |
| `crates/secrets` — OS キーチェーン（keyring）+ ワークスペース単位スコープ | ✅ 1 件 |
| `crates/ai-core` — BYOK プロバイダ + スクラバー + プロンプト | ✅ 7 件 |
| `crates/smart-vars` — {{uuid.v7}}/{{today+7}}/{{faker.*}}/{{jwt}}/{{otp}} | ✅ 9 件 |
| `crates/assertions` — 宣言的ランナー（status/jsonpath/header/time/body） | ✅ 7 件 |
| `crates/diagnose` — ルールベースのエラー診断 | ✅ 4 件 |
| `crates/ops-db` — 読み取り専用 DB クエリ（sqlparser ガード + sqlx） | ✅ 4 件 |
| `crates/ops-ssh` — システムの ssh バイナリでコマンド実行 | ✅ 1 件 |
| `crates/bundle` — ネイティブ共有フォーマット（エクスポート/インポート） | ✅ 2 件 |
| `crates/codegen` — 多言語のリクエストコード生成（fetch/python/go/php/rust…） | ✅ 6 件 |
| `apps/desktop/src-tauri` — Tauri シェル + 57 コマンド | ✅ 3 件 |
| `apps/desktop` — React フロントエンド | ✅ 型チェック + クリーンなバンドル |

**黄金律:** WebView は自分でネットワークを呼びません — すべてのリクエストは Rust コアを経由します（CORS 回避、シークレット漏洩なし、完全なメタデータ）。

```
crates/     Rust コア — 各クレートは独立モジュール、trait ファーストの契約
apps/       デスクトップアプリ（Tauri + React）
docs/adr/   アーキテクチャ決定記録（ADR）
docs/RELEASE.md  ビルド & リリースパッケージングのガイド
```

---

## ロードマップ

現在のアルファは **Phase 1 M0–M3**（HTTP コア → daily driver → AI アイデンティティ → smart & self-testing）+ **Phase 2 P2-M1**（Ops SSH/DB）+ マルチリージョンのワークスペースシステムを完了。加えてボーナスとしてチームワークスペース MySQL と自動アップデート。

**次:** M4 GraphQL + WebSocket → M5 Diff Engine → M6 gRPC → M7 Plugin SDK（→ 1.0）。3 フェーズの完全な表: **[ROADMAP.md](./ROADMAP.md)**。

ビジョンとアーキテクチャの詳細: **[PLAN.md](./PLAN.md)** · 保留/完了したアイデア: **[ICEBOX.md](./ICEBOX.md)** · 変更履歴: **[CHANGELOG.md](./CHANGELOG.md)**。

---

## コントリビューション & 開発規律

- 現在のマイルストーンの Definition of Done がグリーンになるまで、新しいマイルストーンを開かない（[PLAN.md §5](./PLAN.md) 参照）。
- 新しいアイデアは [ICEBOX.md](./ICEBOX.md) に書き留め、すぐには作らない。
- 各クレートは明確な契約を持ち、AI コーディングエージェントが独立して担当可能。クレートごとに独立してテスト。
- 各リリースの前: `cargo test` + `pnpm --filter api-companion-desktop build` がグリーンであること。CHANGELOG を更新。

## ライセンス

[MIT](./LICENSE) © API Companion.

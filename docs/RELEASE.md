# Release & Packaging Guide

Hướng dẫn build bản phát hành, đóng gói installer và tạo GitHub Release cho API Companion.
Xem [../CHANGELOG.md](../CHANGELOG.md) cho lịch sử phiên bản.

> 🤖 **Đã có CI/CD tự động** ([.github/workflows](../.github/workflows/)):
> - **`ci.yml`** — mỗi push/PR lên `main`: build frontend + `cargo test` toàn workspace trên **Windows/Linux/macOS**.
> - **`release.yml`** — đẩy tag `v*` → build installer 3 OS bằng `tauri-action` + tạo **GitHub Release (draft, pre-release)** kèm artifact.
>
> Phần build thủ công bên dưới chỉ cần khi muốn test đóng gói **cục bộ**. Luồng phát hành khuyến nghị: xem [§5](#5-phát-hành-qua-cicd-khuyến-nghị).

---

## 1. Yêu cầu build (Windows)

| Công cụ | Phiên bản | Ghi chú |
|---|---|---|
| Rust | ≥ 1.80 | + target `x86_64-pc-windows-msvc` |
| MSVC Build Tools | latest | `VC.Tools.x86.x64` — cần cho link |
| Node | ≥ 18 | |
| pnpm | ≥ 9 | `corepack enable` hoặc `npm i -g pnpm` |
| WebView2 Runtime | Evergreen | Windows 11 có sẵn |
| Tauri CLI | ^2 | `cargo install tauri-cli --version "^2"` (hoặc dùng `pnpm tauri`) |

```powershell
pnpm install     # cài dependency frontend + tauri cli (devDependency)
```

---

## 2. Checklist trước khi release

1. **Test xanh:**
   ```bash
   cargo test                                        # 95 test Rust
   pnpm --filter api-companion-desktop build         # tsc --noEmit + vite build
   ```
2. **Smoke test GUI:** `pnpm dev` → thử gửi request thật, đổi environment, đổi workspace (tab khôi phục), AI generate (nếu có key), Ops query.
3. **Bump version** ở **4 file** (giữ đồng bộ):
   - `Cargo.toml` → `[workspace.package] version`
   - `package.json` → `version`
   - `apps/desktop/package.json` → `version`
   - `apps/desktop/src-tauri/tauri.conf.json` → `version` ← **đây là version hiện trong app/installer**
4. **Cập nhật [CHANGELOG.md](../CHANGELOG.md):** chuyển `[Unreleased]` thành mục version mới + ngày; cập nhật link so sánh cuối file.
5. Cập nhật trạng thái [../README.md](../README.md) và [../PLAN.md](../PLAN.md) nếu milestone đổi.

> Quy ước version: `0.<milestone>.<patch>` trong Phase 1; `1.0.0` = hết Phase 1 (M7). Xem PLAN.md §9.

---

## 3. Build installer

```bash
# Từ thư mục gốc repo
pnpm --filter api-companion-desktop tauri build
# hoặc: pnpm build:app
```

Lệnh này chạy `beforeBuildCommand` (`pnpm build` = tsc + vite) rồi build Rust release + đóng gói.

**Artifact đầu ra** (cargo workspace → target ở gốc repo):

```
target/release/api-companion.exe                          # binary thô
target/release/bundle/nsis/API Companion_0.4.0_x64-setup.exe   # installer NSIS
target/release/bundle/msi/API Companion_0.4.0_x64_en-US.msi    # installer MSI
```

`bundle.targets = "all"` trong `tauri.conf.json` → sinh cả NSIS (.exe) và MSI. Muốn chỉ một loại: đổi thành `"nsis"` hoặc `"msi"`, hoặc chạy `tauri build --bundles nsis`.

---

## 4. Auto-update & ký artifact (có từ v0.4.2)

App dùng `tauri-plugin-updater`: check `latest.json` trên GitHub Releases (endpoint + public key minisign nằm trong `tauri.conf.json > plugins > updater`), tải bản mới, verify chữ ký rồi cài passive + relaunch. UI: banner tự hiện khi có bản mới + lệnh *Check for Updates…* trong palette.

**Chữ ký minisign (KHÔNG phải cert Windows):**

- Keypair tạo bằng `pnpm tauri signer generate`. Public key → `tauri.conf.json`; **private key KHÔNG commit** — hiện giữ tại `C:\Users\DELL\.tauri\api-companion-updater.key` (máy dev chính). **Backup file này cẩn thận: mất key = không phát hành update được cho user cũ.**
- CI cần 2 secret (Settings → Secrets and variables → Actions):
  - `TAURI_SIGNING_PRIVATE_KEY` = **nội dung** file private key
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = password của key (key hiện tại không đặt password → tạo secret giá trị rỗng hoặc bỏ qua)
- Updater artifacts (`.sig`, `latest.json`) chỉ bật trong CI qua `--config src-tauri/tauri.release.conf.json` → **build local không cần key**. Muốn test updater cục bộ: set env `TAURI_SIGNING_PRIVATE_KEY_PATH` rồi build với config đó.
- Lưu ý: chỉ bản cài **từ v0.4.2 trở đi** có updater; user bản cũ hơn vẫn tải installer thủ công một lần.

> ⚠️ **Gotcha khi test updater**: endpoint dùng `releases/latest/download/latest.json`. GitHub coi `/latest/` là release **không phải draft và không phải pre-release**. Workflow tạo draft + prerelease, nên khi publish để updater thấy được, phải **bỏ tick "Set as a pre-release"** và **tick "Set as the latest release"**. Nếu để nguyên pre-release, app sẽ báo "đã mới nhất" dù đã có bản cao hơn. (Muốn giữ nhãn pre-release mà vẫn update được thì đổi endpoint sang URL tag cụ thể — nhưng phải sửa mỗi lần release, không nên.)

## 4b. Code signing Windows (hoãn tới beta)

Bản alpha **chưa ký số** → Windows SmartScreen sẽ cảnh báo "Windows protected your PC" khi cài **lần đầu** (update qua updater không gặp lại cảnh báo). Hướng dẫn người dùng: **More info → Run anyway**.

Trước beta (M5): mua **code signing certificate** (hoặc dùng Azure Trusted Signing), cấu hình trong `tauri.conf.json > bundle > windows > certificateThumbprint` / signing qua CI secret. Ký giảm/loại cảnh báo SmartScreen.

---

## 5. Phát hành qua CI/CD (khuyến nghị)

Workflow `release.yml` tự động build 3 OS + tạo release khi có tag `v*`:

1. Hoàn tất [checklist §2](#2-checklist-trước-khi-release) + commit toàn bộ (version bump + CHANGELOG).
2. **Tag & push** (chạy thủ công — theo quy tắc repo, không tự commit hộ):
   ```bash
   git tag -a v0.4.0 -m "API Companion v0.4.0 — Public Alpha"
   git push origin v0.4.0
   ```
3. GitHub Actions chạy `release.yml`: build Windows (`.exe`/`.msi`), macOS (`.dmg` universal), Linux (`.deb`/`.AppImage`) → tạo **draft pre-release** kèm artifact.
4. Vào **Releases** trên GitHub: kiểm tra draft, dán/tinh chỉnh release notes từ CHANGELOG, bấm **Publish**.

**Yêu cầu:** repo bật GitHub Actions; `GITHUB_TOKEN` mặc định có sẵn (workflow đã khai `permissions: contents: write`). Không cần secret thêm cho bản alpha (chưa ký số).

### Phát hành thủ công (fallback, không dùng CI)
Nếu không dùng CI: build cục bộ theo [§3](#3-build-installer) rồi **Releases → Draft a new release** → chọn tag → upload `*-setup.exe` / `*.msi` từ `target/release/bundle/` → tick *pre-release* → Publish.

> Auto-update đã bật từ v0.4.2 (xem [§4](#4-auto-update--ký-artifact-có-từ-v042)) — release qua CI sẽ tự sinh `.sig` + `latest.json`; **nhớ thêm secret `TAURI_SIGNING_PRIVATE_KEY` trước khi push tag**, thiếu secret build release sẽ fail có chủ đích.

---

## 6. Cross-platform (best-effort)

Windows là primary. macOS/Linux build được từ CI matrix nhưng chưa test kỹ ở alpha:

- **macOS:** `tauri build` → `.dmg` / `.app` (cần macOS runner; app chưa notarize).
- **Linux:** `.deb` / `.AppImage` (cần `libwebkit2gtk`, `libssl`, `librsvg` trên máy build).

Hỗ trợ chính thức macOS/Linux dời tới khi có user thật yêu cầu (PLAN.md §12).

---

## 7. Sau release

- Tạo nhánh/patch `0.4.x` cho bug daily-driver phát hiện sau release.
- Ghi ý tưởng mới vào [../ICEBOX.md](../ICEBOX.md), không mở milestone mới khi DoD hiện tại chưa xanh.
- Mở lại `[Unreleased]` trong CHANGELOG cho vòng phát triển kế tiếp.

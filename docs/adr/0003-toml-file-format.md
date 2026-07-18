# ADR 0003 — Collections lưu dạng TOML, một request một file

**Trạng thái:** Accepted · **Ngày:** 2026-07-13

## Quyết định
Collections/environments lưu thành file **TOML** (qua `toml_edit`), **một request = một file**; body lớn tách sidecar `.body.json`. Có `schemaVersion` + migration từ M1.

## Lý do
- Git diff sạch (giữ comment/formatting); mỗi request một file → merge conflict tối thiểu.
- Không custom DSL (chi phí tooling), không YAML (serde_yaml unmaintained, nhiều bẫy), không JSON gộp (diff noisy).

## Hệ quả
- SQLite chỉ giữ runtime data (history, cache) — files là source of truth cho những gì cần git.
- Secret KHÔNG vào file — chỉ khai báo tên, giá trị ở OS keychain (ADR sẽ bổ sung).

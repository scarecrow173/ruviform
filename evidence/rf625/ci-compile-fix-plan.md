# 内部 Plan — Issue #625 main tip deterministic compile error 2 件の修正

- 文書種別: 内部 plan（`ci-compile-fix-design.md` の実施計画相当）
- 日付: 2026-11-16
- 対象 Revision: `cbcc83437616fdd7418da37183c6be66b6cfe225`

## 実施手順

1. **CI 一次証拠の収集（実測）**
   - `gh run view 43783872777 --json jobs` で失敗 job を列挙
     （`verify (ubuntu-latest)`・`l1-real-verify
     (ubuntu-latest)`・`verify-macos`・`rt-verify`・
     `ci-summary`）。
   - `gh run view --job <id> --log` で E0433 ×9
     （output_ports test）と `unnecessary_cast` ×5
     （events.rs）の一次ログを確認。
2. **原因特定**: `output_ports.rs` の cfg-gated import と
   非 gated test の不整合、`events.rs` の
   `StatesAndFlags`（`DefaultEnumType` = `c_int`/`c_uint`
   platform 揺れ）への `as u32` no-op cast を code path で
   確認（vst3-0.3.0 `support.rs`/`bindings.rs`、
   `ruviform-midi-device` の `system.rs`/`Cargo.toml`、
   既存 `sdk_enum_i32`/`enum_to_*` idiom）。
3. **修正実装**: design の採用案どおり 2 file を編集。
4. **ローカル検証**: `mise run fmt-check`、
   `mise run check|lint|test ruviform-midi-device`、
   `mise run check|lint|test ruviform-plugin-test-vst3`。
5. **補助検証（target 指定）**: `x86_64-unknown-linux-gnu`・
   `aarch64-apple-darwin` に対する cargo check/clippy で
   失敗 cfg 環境を再現検証。
6. **証拠記録**: `evidence/rf625/` に本 doc chain と
   `ci-compile-fix-verification.md` を作成。
7. **レビュー**: review-protocol で blocker を解消。
8. **統合**: `git fetch origin main && git merge origin/main`
   → commit（Conventional Commits）→ `fix/625-ci-lanes` へ
   push → `gh pr create`（`Refs #625`）→ worker_done。

## ファイルマップ

| ファイル | 操作 |
| --- | --- |
| `crates/ruviform-midi-device/tests/output_ports.rs` | `#[cfg(windows)]` 除去（1 行） |
| `crates/ruviform-plugin-test-vst3/src/events.rs` | `flag_to_u32` helper 追加 + 5 箇所置換 |
| `evidence/rf625/ci-compile-fix-{spec,design,plan,task}.md` | 新規（内部 doc chain） |
| `evidence/rf625/ci-compile-fix-verification.md` | 新規（検証台帳） |

## 依存・順序

- T1（証拠収集）→ T2（原因特定）→ T3（実装）→ T4/T5（検証）→
  T6（証拠）→ T7（レビュー）→ T8（統合）の直列。
- 並列化する独立作業なし。merge 前に `git fetch origin
  main && git merge origin/main` で最新 main を取り込む
  （共通契約）。

## リスクと対応

- **off-Windows での再発**: target 指定 check/clippy で
  compile/lint 面は実測済み。test 実行・RT lane は merge 後
  CI が正本（NotRun と正直に記録）。
- **他 file の同型 lint**: `component.rs` は file-level
  `#![allow]` で既に抑止済み（CI 一次ログでも events.rs のみ
  5 件と確認）。`ruviform-plugin-vst3` 側は本 slice 範囲外。
- **main 進行との競合**: `ruviform-midi-device`・
  `ruviform-plugin-test-vst3` は #629/#630 の継続 slice が
  触り得る soft 関係 — merge 直前に `origin/main` を取り込み、
  競合時は生成物でなくコード意味を理解して解消する。

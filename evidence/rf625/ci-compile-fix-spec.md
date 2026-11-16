# 内部 Spec — Issue #625 main tip deterministic compile error 2 件の修正

- 文書種別: 内部 spec（doc-driven-dev-graph フローの spec 相当。
  対象リポジトリ内の内部文書として `evidence/rf625/` に配置。
  canonical docs chain は編集範囲外のため本ファイルが内部正本）
- 日付: 2026-11-16
- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/625
  （CI 失敗 tracker — 本 slice では Close しない）
- 対象 Revision: `cbcc83437616fdd7418da37183c6be66b6cfe225`
  （main tip。Full CI run `43783872777` の headSha と一致）
- 失敗 run: https://github.com/scarecrow173/ruviform/actions/runs/43783872777
- 作業 branch: `fix/625-ci-lanes`（worktree ローカル branch は
  `scarecrow173/rf625-ci-fix`、push 時に `HEAD:fix/625-ci-lanes`）

## What Should Be Built

main tip `cbcc8343` の Full CI（run `43783872777`）が 2 件の
deterministic compile error で失敗中。非 Windows でのみ発火する
2 件を、test fixture / テストコードの compile 修正として解消する。

1. **`crates/ruviform-midi-device/tests/output_ports.rs` の
   E0433 解消** — `#[cfg(windows)]`-gated の
   `ruviform_media_core` import（10-11 行目）に対し、非 gated の
   `real_device_send_smoke` が `MidiGroup`/`MidiChannel`/`MidiKind`/
   `MidiValue`/`NoteKey` を使用していて非 Windows で解決不能
   （CI では E0433 ×9、job: `rt-verify`・`verify-macos`）。
   cfg gate の整合を取る最小修正で解消する。
2. **`crates/ruviform-plugin-test-vst3/src/events.rs` の
   `clippy::unnecessary_cast` ×5 解消** — `StatesAndFlags` 定数への
   `as u32` cast（293, 294, 303, 316, 351 行目）が off-Windows
   （`StatesAndFlags` = `c_uint` = u32）で no-op cast と判定され
   Linux lint lane（`-D warnings`）を落とす（CI では
   `verify (ubuntu-latest)`・`l1-real-verify (ubuntu-latest)`）。
   platform の signedness 差を吸収する型変換に直す。

## Why Now

- `ci.yml` は workflow_dispatch-only のため PR lane ではこれら
  lane が走らず、main tip で失敗が継続再現中（Issue #625 は
  tracker auto-close = 失敗 run を cover する新しい green `ci`
  run が必要）。
- 混入 commit は `7e7270f0`（output slice — output_ports.rs 追加）
  と `de3ed4e8`（test_instrument slice — events.rs 追加）。
  両方とも前回 green run 以降に main へ merge 済み。
- blocking Issue 依存なし。

## Scope

### In Scope

- `crates/ruviform-midi-device/tests/output_ports.rs` の cfg gate
  整合修正（test コードのみ。製品 src・公開 API は不変）。
- `crates/ruviform-plugin-test-vst3/src/events.rs` の
  `StatesAndFlags` → `u32` 変換の型安全化（test fixture crate
  内コード。公開 API 契約は不変 — 本 crate は ABI のみを公開し
  Rust 公開 API を持たない）。
- `evidence/rf625/` 配下の内部文書チェーン（本 spec/design/
  plan/task）と検証記録。

### Out of Scope

- 製品 crate の src / 公開 API 契約の変更
  （`ruviform-midi-device` の src、`ruviform-plugin-test-vst3` の
  他 file、`ruviform_audio_core`/`ruviform_media_core`/`vst3` 側）。
- CI lane 新設・workflow/mise 変更（`.github/**`・`mise.toml`）。
- `docs/` 配下の変更・docs index の再生成（Coordinator が統合時に
  処理）。canonical docs chain への文書追加。
- `Cargo.lock`・ルート `Cargo.toml` の members 構成変更、新規
  dependency・feature flag の追加。
- `*-msrv` 系・`*-full` 系・3OS 横断検証（daily/手動 Full CI
  委譲・`*-msrv` は実行禁止）。
- Issue の Close・auto-close 表現（PR は `Refs #625` のみ。
  merge・Close・tracker auto-close 監視・merge 後の `ci`
  workflow_dispatch 再実行は Coordinator が行う）。
- 既存の他作業（未統合 branch・Draft PR・未追跡ファイル等）への
  干渉、無関係な stage/commit。

## 受入条件（本 slice・Coordinator 確定）

1. `output_ports.rs` の E0433 解消 — cfg gate の整合を取る最小
   修正。採用方式と理由を内部文書に記す。
2. `events.rs` の `clippy::unnecessary_cast` ×5 解消 —
   platform signedness 差を吸収する型変換。`#[allow]` で lint を
   隠さない。`#[expect]` は Windows で unfulfilled になるため
   使わない。Windows と off-Windows の両方で compile・lint が
   通る構造。
3. ローカル lane Pass — `mise run fmt-check`・
   `mise run check|lint|test ruviform-midi-device`・
   `mise run check|lint|test ruviform-plugin-test-vst3`。
4. 正直な検証記録 — off-Windows の compile/lint は本来 PR CI
   管轄だが、`x86_64-unknown-linux-gnu` /
   `aarch64-apple-darwin` target が利用可能なため cargo
   check/clippy の target 指定で補助検証を実施し、結果を
   実測どおり記録する。実行していない検証を Pass と書かない。
5. 証拠記録 — `evidence/rf625/` に検証記録を残す。
6. `review-protocol` レビュー — blocker が残っていない状態を
   承認として扱う。
7. PR は `Refs #625`（Close しない）。

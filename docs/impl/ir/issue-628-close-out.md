---
id: "IMPL-034Y3isAfsgJBhG7QwK2Jg"
type: "impl"
status: "in-progress"
title: "Issue #628 close-out slice"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "devin-worker"
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-628-close-out.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-628-pipeline-wiring.md"
    - "docs/impl/ir/issue-628-scripted-backend.md"
    - "docs/impl/ir/issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-628-close-out.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-628-close-out.jsonl"
    rejected: []
---

# Issue #628 close-out slice

## Summary

Issue #628 close-out slice: `RunMetadata` JSON serialization
（serde derive + `to_json`/`from_json`/`save_json`/`load_json`、
Case Schema v1 kebab-case wire 語彙）、`StreamFault::Cancellation`
fault class（audio stream のみ — production 契約を変えず既存
observable で写像）、docs 陳腐化修正（前 3 slice task status・
crate README 群）、Issue 全 AC の棚卸し表（`evidence/rf628/`）。

## Context

`docs/tasks/issue-628-close-out.md`（Issue `scarecrow173/ruviform#628`
close-out slice）。spec/design/plan/test-spec は同 slug の
`docs/{specs,designs,plans,test-specs}/issue-628-close-out.md`。
前提: 1st slice deterministic core（PR `scarecrow173/ruviform#638`）、
2nd slice scripted backend（PR `scarecrow173/ruviform#659`、merge
`98fd4306`）、3rd slice pipeline wiring（PR `scarecrow173/ruviform#664`、
merge `3566519`）。consumer oracle（AudioLens/Synvelis 独立 oracle）は
両 repo merge 済み。

## Implementation

### Changes

- `crates/ruviform-audio-test/Cargo.toml` — `serde`（derive）/
  `serde_json` 追加（理由コメント付き・workspace 既使用）、
  description を現状へ更新。
- `src/clock.rs` — `JitterSpec`/`VirtualClockConfig` に serde derive
  のみ追加（挙動・RT 契約は不変）。
- `src/metadata.rs` — 4 enum（`CaseRequirement`/`CaseResult`/
  `VerificationLevel`/`EnvironmentKind`）と `RunParameters`/
  `RunMetadata` に serde derive。enum は
  `#[serde(rename_all = "kebab-case")]` で Case Schema v1 語彙
  （`EnvironmentKind::as_str` と同一綴り）。`RunMetadata` に
  `to_json`/`from_json`/`save_json`/`load_json` を追加 — 非 RT
  test-driver API（alloc/IO あり・RT 経路へ割込まない）。stale な
  "JSON deferred" doc を実状へ更新。
- `src/backend.rs` — `StreamFault::Cancellation { at_callback }`
  variant 追加（`#[non_exhaustive]` 維持）+ module doc の fault
  写像一覧と `PumpReport::invalidated` doc を更新。
- `src/backend/stream.rs` — `collect_faults` を `FaultEffects`
  struct 返却へ整理し `cancel` flag を追加。`pump_output`/
  `pump_input` で cancel 時は data callback 不発・frames=0・
  `Backend("cancelled")` error 配送・stream 終端
  （`invalidated`+`dead`）・`callback_index` のみ進行。
  `deliver_cancel_errors`/`cancelled_pump_{output,input}` helper。
- `src/lib.rs` — module doc を現状（JSON metadata・Cancellation・
  pipeline wiring）へ更新。
- `tests/metadata.rs` — JSON round-trip（string/file）・wire 語彙
  pin・`EnvironmentKind` serde == `as_str`・reject 系テスト追加。
- `tests/backend_faults.rs` — cancellation 正負テスト追加:
  output/input scripted・mid-stream inject・`Backend("cancelled")`
  error 単独は終端しない負例・同 index Invalidate 併用時の error
  順序 pin。
- `docs/tasks/issue-628-{deterministic-test-runtime,scripted-backend,
  pipeline-wiring}.md` — status `in-progress`→`done`、checkbox を
  実績へ同期（3 slice は merge 済み・IR completed）。
- `crates/README.md`・`crates/ruviform-audio-test/README.md` —
  stale な "first slice"/"JSON serialization は後続" 記述を現状へ
  更新。CI lane 新設不要（既存 `test`/`test-rt` が自動収集）の
  判断を明記。
- `evidence/rf628/` — Issue #628 全 AC × verdict 棚卸し表を追加。
- `Cargo.lock` — serde/serde_json 追加に伴う再生成。

### Decisions During Implementation

- **serde wire format**: fieldless enum は
  `#[serde(rename_all = "kebab-case")]` で Case Schema v1 語彙に
  一致させた（`deterministic-backend`/`os-l1`/`not-run` 等）。
  `EnvironmentKind::as_str()` と serde 出力の一致を test で pin。
  struct field は snake_case のまま（既存 JSON 慣行と衝突しない）。
- **cancellation 写像**: production `DeviceError`/stream 契約に
  cancel 概念がないため、新 variant・新 API を発明せず、既存
  observable（data callback 不発 + `Backend("cancelled")` error +
  stream 終端 = `Invalidate` と同 lifecycle）で表現。
  `callback_index` は進める（発火自体は消費）、`position`/`frames`
  は進めない（0 frame 配送）。同 index の `Error`/`Invalidate`
  fault も適用し、error 順序は Error→cancelled→StreamInvalidated。
  `Error { Backend("cancelled") }` 単独では終端しないことを負例
  test で固定。
- **JSON API 配置**: `RunMetadata` の method として
  `to_json`/`from_json`/`save_json`/`load_json`。alloc/IO を伴う
  ため非 RT test-driver API として rustdoc に明記。RT contract
  経路・`rt_` テストには関与しない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-628-close-out.jsonl` — serde wire format と
  cancellation 写像の観察・採用判断。

### Rejected

- なし（不採用案は design doc に記録）。

## Validation

canonical 検証は `evidence/rf628/close-out-verification.md` に記録
（Windows 11 x64 / rustc 1.99.1 / 2026-11-15 実施）:

- `mise run fmt-check` — pass
- `mise run check ruviform-audio-test` — pass
- `mise run lint ruviform-audio-test` — pass（初回
  `clippy::unnecessary_wraps` 2 件を `cancelled_pump_*` 戻り値
  `PumpReport` 直返しへ修正して解消）
- `mise run test ruviform-audio-test` — pass（117 tests + 3
  doctests、0 failed）
- `mise run test-rt` — pass（workspace 全 `rt_` filter。
  audio-test `rt_contract.rs` 9 件を個別確認・非回帰）
- `mise exec -- prek run --all-files` — pass
- docs index — `doc_maintenance.js index apply` で 7 dir 再生成、
  再 plan で全 `in-sync`。`doc_status lint` の新規 doc 由来
  warning は先行 slice と同型の reciprocal-relation のみ（非
  blocking・既存 convention）
- 製品 crate src 差分なし（`git diff origin/main --name-only`
  で確認）

## Risks

- `deliver_cancel_errors` は `impl FnMut(DeviceError)` ジェネリック
  で `OutputErrorCallback`/`InputErrorCallback`（同一 underlying
  type）の双方を受ける — 片方の callback 型が将来分岐した場合は
  要見直し。
- cancellation の error callback 順序（Error→cancelled→
  StreamInvalidated）は本 slice で固定した実装順であり、
  production に対応契約はない — test-spec に pin 済み。
- `Cargo.lock` は serde/serde_json の依存追加のみ（既存 lock
  entry 再利用・version 変動なし）。

## Follow-ups

- MIDI 側 fault への cancellation 拡張・MIDI→Host adapter `attach()`
  配線（`scarecrow173/ruviform#630` 領域）・実 OS/EXT-L3 証拠
  （`scarecrow173/ruviform#629` 領域）・3OS/consumer 別集計は
  後続 slice・CI 証拠へ Deferred（`evidence/rf628/ac-inventory.md`
  参照）。

---
id: "PLAN-yYLB0HDhiCqmnfV65T7mNu"
type: "plan"
status: "approved"
title: "Issue #251 Stage W closeout 実施 plan"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-251-stage-w-real-device-closeout-design.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-251-stage-w-real-device-closeout.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-251-stage-w-real-device-closeout.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #251 Stage W closeout implementation plan

## File map

| file | 変更 |
| --- | --- |
| `crates/ruviform-audio-device/tests/real_input_e2e.rs` | latency 観測追加（capture test）・format-mismatch test 新設・device-removal 対話 test 新設・duplex 非同一 probe 追加 |
| `docs/specs/issue-251-stage-w-real-device-closeout.md` | **新規** — spec |
| `docs/designs/issue-251-stage-w-real-device-closeout-design.md` | **新規** — design |
| `docs/plans/issue-251-stage-w-real-device-closeout-plan.md` | **新規** — 本 plan |
| `docs/tasks/issue-251-task1-stage-w-real-device-closeout.md` | **新規** — task |
| `docs/test-specs/issue-251-stage-w-real-device-closeout.md` | **新規** — test spec |
| `docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md` | **新規** — impl record（Stage W matrix） |
| `docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md` | NotRun matrix の実測反映のみ |
| `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md` | 管理 index 再生成（`doc-maintenance apply`） |

`src/*.rs`・`Cargo.toml`・`Cargo.lock`・`mise.toml` は原則無変更（実機検証で欠落が判明した場合のみ `src/wasapi_input.rs`/`stream.rs`/`device.rs`/`error.rs` の小規模補修を許容し、理由を PR に記録）。

## Task 分解（単一 task doc に集約 — `docs/tasks/issue-251-task1-...`）

1. **T1-a lane 拡張**: `real_input_e2e.rs` に実効 latency 集計 atomics + summary 行、
   `input_latency()` の WASAPI `Ok(Some)` assert、非 F32/範囲外 config rejection test、
   duplex 非同一 domain probe、env var ゲートの対話式 device-removal test
   （`CONOUT$` 経由の指示表示 helper 含む）を追加する。
2. **T1-b headless 検証**: `fmt-check`/`check`/`lint`/`test`/`test-rt`/`build`/
   `lint-audio-real-input` を走らせ、mock/headless 回帰なしを確認する。
3. **T1-c 実機検証**: `mise run test-audio-real-input` を実機（Windows 11 +
   Audient iD4 WASAPI）で実行し lane 出力を証拠化。operator 同席時は
   `RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` を付けて抜去検証も実施。
4. **T1-d 証拠記録**: 新規 IR + headless IR NotRun matrix 更新 + 管理 index 再生成。
   PR 本文に「Stage W Pass / Stage M,L NotRun — Cross OS 完了ではない」を明記。

## 検証マトリクス

| 検証 | lane |
| --- | --- |
| fmt / check / lint / test / test-rt / build | `mise run fmt-check` `check` `lint` `test` `test-rt` `build` |
| lane ターゲット lint（real-device feature） | `mise run lint-audio-real-input` |
| 実機 capture e2e + duplex + latency + format-mismatch + xrun 観測 | `mise run test-audio-real-input`（実機 + capture 権限がある場合。なければ NotRun） |
| 実機 device 抜去 → typed error | `RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS=<n>` 付き対話 run（operator 同席時のみ。未実施なら手順記録つき NotRun） |
| Stage M/L | 本 slice 対象外 — `NotRun` のまま matrix に残す |

## リスクと rollback

- 実機未接続（`Present:False`）→ lane が loud `SKIP` し証拠行が `NotRun` に留まる。
  接続を依頼してから再実行。全検証行が NotRun のままなら closeout として未完成を
  正直に報告する。
- 対話 test の指示が `cargo test` の capture に隠れる → `CONOUT$` 直接書き込みで
  実時間表示（失敗時は `eprintln!` fallback）。
- 抜去検証未実施 → 失敗ではなく `NOT-EXERCISED` 記録。matrix/PR に残存項目として明記。
- lane 拡張で既存テストが壊れた → `real_input_e2e.rs` の差分 revert で即復帰
  （`#![cfg]` + `#[ignore]` ゲートのため既定 lane に影響しない）。

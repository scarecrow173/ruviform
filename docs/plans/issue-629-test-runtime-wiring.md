---
id: "PLAN-g2t6LzqvnsCpXGOdR0kf1G"
type: "plan"
status: "approved"
title: "Issue #629 AC3 Test Runtime wiring plan"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-629-test-runtime-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-test-runtime-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
    - "docs/tasks/issue-629-test-runtime-wiring.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-test-runtime-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 AC3 Test Runtime wiring plan

## スコープ分割

単一 slice。依存方向裁定 → 実装 → 検証 → 証拠・文書同期の順。

## ファイルマップ

### 変更

- `crates/ruviform-audio-test/Cargo.toml` — `[dev-dependencies]`
  に `ruviform-audio-device`・`ruviform-midi-device`（path）を
  追加。依存追加の記録コメントを付す。
- `Cargo.lock` — path 依存 edge 追加に伴う再生成。
- `crates/ruviform-audio-test/src/metadata.rs` —
  `RunMetadata::for_environment` additive 追加 + ヘルパ。
- `crates/ruviform-audio-device/tests/virtual_endpoint.rs` —
  module doc の stale 記述のみ同期（挙動不変）。
- `crates/ruviform-midi-device/tests/loopback.rs` — 同上。
- `docs/designs/issue-629-wasapi-endpoint.md`・
  `docs/designs/issue-629-virtual-endpoint-lane.md`・
  `docs/designs/issue-629-provisioning-runbook.md` — AC3 関連
  Deferred Design Concerns の同期。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` —
  Evidence 節へ append-only 追記。
- `evidence/rf629/` — self-run 記録の新規ファイル。

### 新規

- `crates/ruviform-audio-test/tests/os_endpoint.rs` — wiring
  test（deterministic leg: non-ignored / OS endpoint leg:
  `#[ignore]`）。
- `docs/specs|designs|plans|tasks|test-specs|impl/ir|impl/exp/
  issue-629-test-runtime-wiring.md`（+ exp.jsonl）— 本 chain。

## タスク分解

- T1: 依存方向裁定を design に記録（完了 — 本 design §1）。
- T2: `Cargo.toml` dev-dep 追加 + `Cargo.lock` 再生成。
- T3: `metadata.rs` に `for_environment` 追加。
- T4: `tests/os_endpoint.rs` 実装（deterministic leg +
  audio/midi OS endpoint leg + RunMetadata emit）。
- T5: device test module doc 同期（2 ファイル）。
- T6: 検証 matrix 実行・失敗時は修正して再実行。
- T7: self-run evidence（`test-audio-virtual-endpoint`・
  `test-midi-virtual-endpoint`・`test-midi-real-device`・
  wiring test 直接実行）を `evidence/rf629/` に記録。
- T8: stale design 文書 3 件 + survey Evidence 追記。
- T9: impl/ir・impl/exp(+jsonl) 作成、review-protocol 実施。
- T10: commit・push・PR 作成・worker_done。

## 依存関係

T2→T3→T4→T6→T7 の直列。T5/T8 は T4 と並行可能。T9 は T6/T7
完了後。T10 は全完了後。

## リスクと rollback

- compile 失敗（device crate の API 想定違い）→ test target を
  修正。device `src/` 変更が必要になった場合は `ask`。
- `#[ignore]` wiring test の compile が全 OS で壊れる →
  `cfg` ゲートではなく fail-closed `NotRun` 設計を維持。
- rollback: dev-dep 追加と新規 test ファイルを revert すれば
  既存 lane・契約に影響しない（additive のみ）。

## 検証 matrix

`mise run fmt-check`、`check`/`lint`/`test` ×
`ruviform-audio-device`・`ruviform-midi-device`・
`ruviform-audio-test`、`lint-audio-real-input`、
`test-audio-virtual-endpoint`、`test-midi-virtual-endpoint`、
`test-midi-real-device`、
wiring test 直接実行（補助証拠）、`prek run --all-files`。
`*-msrv`/`*-full` は daily CI 管轄で NotRun。

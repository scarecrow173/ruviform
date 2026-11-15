---
id: "PLAN-034XVm4OZxJqPqKqTqVqYq"
type: "plan"
status: "approved"
title: "Issue #629 MIDI loopback round-trip verifier plan"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-loopback.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/tasks/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/test-specs/issue-629-midi-loopback.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-midi-loopback.md"
  implemented-by:
    - "docs/tasks/issue-629-midi-loopback.md"
    - "docs/test-specs/issue-629-midi-loopback.md"
  depends-on:
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-loopback.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-midi-loopback.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-loopback.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-midi-loopback.md"
  verified-by:
    - "docs/test-specs/issue-629-midi-loopback.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI loopback round-trip verifier Implementation Plan

## Goal

`midi_endpoint_probe` を #658 merge 後の実装に合わせて refresh し
（production output view・verdict 修正・WMS availability 検出）、
provisioned MIDI loopback pair 前提の round-trip verifier を
`#[ignore]`-gated real-device test として実装する。stale doc
（audio-test `midi.rs`・survey docs）を修正し、docs chain と
evidence を揃える。

## Scope

単一 implementation stream。Issue #629 の 3rd slice のみ。
実 provisioning・`virtual-os-endpoint` evidence・reconnect/xrun・
WASAPI・Test Runtime 結線・CI lane・consumer 照合は後続 slice
（spec Out of Scope 参照）。`mise.toml`・workflow・`Cargo.toml`・
`Cargo.lock`・`POLICIES.md`・`AGENTS.md` は変更しない（依存追加・
lane 登録ともに不要を design で確認済み — `test-midi-real-device`
lane は `--ignored` を自動収集）。

## Upstream Documents

- `docs/specs/issue-629-midi-loopback.md`（approved）
- `docs/designs/issue-629-midi-loopback.md`（approved）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（gap #1・E2 crash）
- `docs/specs/issue-629-midi-output.md` + design/plan/task/test-spec
  （2nd slice chain — output port 契約の正本）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3・仮想優先・証拠要件）

## Design Inputs

- `docs/designs/issue-629-midi-loopback.md`（approved）

## File Map

- `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs` —
  output port view・verdict refresh・WMS availability・module doc。
- `crates/ruviform-midi-device/tests/loopback.rs`（新設）—
  `loopback_round_trip` + `real_device_silent_output_smoke`
  （ともに `#[ignore]`）。
- `crates/ruviform-audio-test/src/midi.rs` — 冒頭 doc comment の
  stale 修正のみ。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` — Known Code
  Gap #1 resolved 更新 + Evidence 追記。
- `docs/designs/issue-629-virtual-endpoint-survey.md` —
  `print_port_view` の output view 追記。
- `docs/test-specs/issue-629-virtual-endpoint-survey.md` — probe
  出力契約（output view）追記。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-629-midi-loopback.md`
  — docs chain。
- `docs/impl/{ir,exp}/issue-629-midi-loopback.md` + `*-exp.jsonl` —
  impl 記録。
- `evidence/rf629/` — 検証記録（`midi_endpoint_probe-windows-local.txt`
  更新 + slice3 verification 記録）。
- `docs/*/README.md` — doc-maintenance / sync_impl_index 再生成。

## Implementation Sequence

- [x] T1: spec → design → plan → test-spec → task docs chain 作成
- [ ] T2: probe refresh（output view・verdict・WMS detection・doc）
- [ ] T3: `tests/loopback.rs` verifier + silent-send smoke
- [ ] T4: stale doc 修正（audio-test `midi.rs`・survey 3 docs）
- [ ] T5: 検証 lane（fmt-check/check/lint/test/prek）+ Windows probe
  再実行 + `test-midi-real-device` 実機実行・evidence 記録
- [ ] T6: impl-doc（ir/exp）+ docs index 再生成 + review-protocol
- [ ] T7: commit/push/PR/worker_done

## Dependencies and Risks

- port trait は read-only 参照 — `midi_input`/`midi_output` の公開
  契約を変更しない。verifier は `Box<dyn …>` 経由のみ。
- pair 照合は `info().name` 一致（id 体系が方向で異なるため）。
  name match は candidate であり virtual の証明ではない — verdict
  文言と test-spec で固定。
- `midiOutMessage`+`DRV_QUERYDEVICEINTERFACE` は呼ばない（crash
  既知）— verifier は production API のみ、probe も既存方針を維持。
- `docs/*/README.md` index は生成物 — doc-maintenance /
  sync_impl_index で再生成。`Cargo.lock` 変更なしの見込み。

## Task Breakdown

- `docs/tasks/issue-629-midi-loopback.md`（T1〜T7 を内包）。

## Verification

- [ ] `mise run fmt-check` — Pass
- [ ] `mise run check ruviform-midi-device` — Pass
- [ ] `mise run lint ruviform-midi-device` — Pass
- [ ] `mise run test ruviform-midi-device` — Pass
- [ ] `mise run check|lint|test ruviform-audio-test` — Pass
  （doc comment のみの変更 — 直接影響先として検証）
- [ ] `mise exec -- prek run --all-files` — Pass
- [ ] Windows 実機: `midi_endpoint_probe` 再実行 exit 0 +
  evidence 更新
- [ ] `mise run test-midi-real-device` — 実機実行、pair 未
  provision では loopback verifier が NotRun を記録（Fail に
  しない）、silent-send smoke の結果を記録
- [ ] MSRV lane — NotRun（daily CI 管轄、指定どおり実行しない）

## Open Questions / Gaps

- pair が複数ある環境での選択は「先頭 + 全 candidate 列挙」—
  provisioning 実測後に必要なら引数化を後続で検討。
- WMS 検出は `Get-Command`/`Get-Service` の best-effort —
  PowerShell module の有無と service 状態を分けて記録する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch）
- Independent work streams: none
- User approval required before dispatch: review-protocol で代替
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- name match による pair 特定の正しさ（id 体系差・同名 port の
  取り違え）。
- verifier の fail-closed 境界（NotRun を Pass にしない経路が
  存在しないか）。
- probe verdict 文言が #658 後の実装と一致しているか。

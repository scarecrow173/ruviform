---
id: "PLAN-034XCkcPRJrWqtaLLfUlAS"
type: "plan"
status: "draft"
title: "Issue #629 virtual endpoint probe plan"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-629-virtual-endpoint-survey.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  implemented-by:
    - "docs/tasks/issue-629-virtual-endpoint-survey.md"
  depends-on:
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  derived-by:
    - "docs/tasks/issue-629-virtual-endpoint-survey.md"
  verifies:
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
  verified-by: []
  references:
    - "evidence/rf629/endpoint_probe-windows-local.txt"
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/629"
  deferred-by: []
---

# Issue #629 virtual endpoint probe plan

## Goal

Issue #629 first slice（investigation + capability-probe）を完了させる: 仮想 endpoint 候補の調査記録、capability probe PoC 2 本、fail-closed な証拠、後続 slice への引き継ぎ文書。

## Scope

本 plan は spec/design の In Scope のみを対象とする。runtime controller・Backend 配線・CI lane・driver 実装は対象外。

## Upstream Documents

- `docs/specs/issue-629-virtual-endpoint-survey.md`
- `docs/designs/issue-629-virtual-endpoint-survey.md`
- `docs/discovery/issue-629-virtual-endpoint-survey.md`

## Design Inputs

- probe は手動 example（CI 非実行）・control-path のみ・`#[cfg(windows)]` 隔離・fail-closed verdict。
- OS API 面は example 内 `win` module に閉じ、enabled feature のみ使用。
- `IsFormatSupported` は `ppclosestmatch` out-param 必須（実測）。`midiOutMessage` interface-id は呼ばない（実測 crash）。

## File Map

| File | 変更 |
| --- | --- |
| `docs/discovery/issue-629-virtual-endpoint-survey.md` | 新規 — 候補比較・capability matrix・ギャップ・実測 |
| `docs/specs/issue-629-virtual-endpoint-survey.md` | 新規 — slice 要件・AC |
| `docs/designs/issue-629-virtual-endpoint-survey.md` | 新規 — probe 設計 |
| `docs/test-specs/issue-629-virtual-endpoint-survey.md` | 新規 — probe 契約規則 |
| `docs/plans/issue-629-virtual-endpoint-survey.md` | 本書 |
| `docs/tasks/issue-629-virtual-endpoint-survey.md` | 新規 — T1 |
| `docs/impl/ir/issue-629-virtual-endpoint-survey.md` | 新規 — 実装記録 |
| `docs/impl/exp/issue-629-virtual-endpoint-survey.md` | 新規 — 試行記録 |
| `crates/ruviform-audio-device/examples/endpoint_probe.rs` | 新規 — WASAPI capability probe |
| `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs` | 新規 — WinMM capability probe |
| `evidence/rf629/endpoint_probe-windows-local.txt` | 新規 — audio probe 実測出力 |
| `evidence/rf629/midi_endpoint_probe-windows-local.txt` | 新規 — MIDI probe 実測出力 |
| `docs/*/README.md`（管理 index） | 再生成のみ |

## Implementation Sequence

1. 候補調査（web 一次情報）→ discovery doc。
2. probe example 2 本を実装 → `cargo check` → Windows 実機実行 → crash/quirk を修正 → `evidence/rf629/` 保存。
3. spec/design/test-spec/plan/task/IR/exp の doc chain を作成。
4. index 再生成 → canonical 検証（fmt/check/lint/test/prek）→ review-protocol → commit/push/PR。

## Dependencies and Risks

- 依存: 既存 `windows = "0.62"`（audio）・`windows-sys = "0.61"`（midi）の enabled feature のみ。新規依存なし。
- リスク: `midiOutMessage` crash（回避済み）・`IsFormatSupported` quirk（out-param で回避済み）・name heuristic の精度（unconfirmed 表記で限定）。

## Task Breakdown

- T1（`docs/tasks/issue-629-virtual-endpoint-survey.md`）: 調査 + probe PoC + doc chain + 検証の全工程（本 slice は単一 task）。

## Verification

- `mise run fmt-check`
- `mise run check|lint|test ruviform-audio-device` / `mise run check|lint|test ruviform-midi-device`
- 補助（非 canonical）: `mise exec -- cargo run -p <crate> --example <probe>` — Windows 実機での手動実行、出力を `evidence/rf629/` へ。
- `mise exec -- prek run --all-files`
- doc-status/index 整合確認。

## Open Questions / Gaps

- 採用 endpoint の確定は provisioning 実測後（後続 slice/ADR）。
- `midiOut*` 送信経路・`MidiPortOrigin` 識別は後続 slice。
- 非 Windows lane での `NotRun` 出力は 3OS CI で確認（ローカルは Windows のみ）。

## Self-Review

- scope は spec/design と一致。file map は編集許可範囲内のみ。生成 index は再生成のみ。

## Implementation Handoff

- task T1 へ。probe の OS API 知見（`IsFormatSupported` 規約・`midiOutMessage` 制限・not-present activate 拒否）は discovery/design/IR に記録済み。

## Review Handoff

- review-protocol で docs+code をレビュー。blocker 解消後に status 昇格。

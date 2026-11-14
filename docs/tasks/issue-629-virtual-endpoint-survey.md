---
id: "TASK-034XCkcQ3OJUyxLkwM19xs"
type: "task"
status: "done"
title: "Issue #629 virtual endpoint probe task"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-629-virtual-endpoint-survey.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-virtual-endpoint-survey.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
    - "docs/impl/ir/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
  derived-by:
    - "docs/impl/ir/issue-629-virtual-endpoint-survey.md"
  verifies:
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
  verified-by: []
  references:
    - "evidence/rf629/endpoint_probe-windows-local.txt"
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint probe task

## Purpose

Plan `PLAN-034XCkcPRJrWqtaLLfUlAS` の実装 task。Issue #629 first slice（investigation + capability-probe）を 1 task で完結させる。

## Context

- probe は手動 example・control-path・`#[cfg(windows)]` 隔離・fail-closed verdict（test-spec R-PROBE-01〜08）。
- 編集許可範囲は spec 指定の doc chain・2 example・`evidence/rf629/`・管理 index のみ。`src/`・`Cargo.toml`・`mise.toml`・workflow は触らない。
- driver install 禁止（本機に driver を導入しない）。

## Work Items

- [x] W1: `docs/discovery/issue-629-virtual-endpoint-survey.md` — 候補比較・capability matrix・コードギャップ・provisioning 方針・実測記録
- [x] W2: `crates/ruviform-audio-device/examples/endpoint_probe.rs` — WASAPI/MMDevice 列挙 + format query + provisioning verdict
- [x] W3: `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs` — WinMM caps 列挙 + loopback 検出 + provisioning verdict
- [x] W4: 両 probe を Windows 実機実行し `evidence/rf629/` に出力保存（MIDI `midiOutMessage` crash を修正して完走）
- [x] W5: doc chain（spec/design/plan/task/test-spec/IR/exp）作成
- [x] W6: 管理 index 再生成 + canonical 検証（fmt-check / check / lint / test 両 crate / prek 全 Pass）
- [x] W7: review-protocol 自己レビュー + blocker 解消（clippy/doc_markdown 修正、front-matter 相互参照修正）
- [x] W8: commit → push → PR → worker_done

## Acceptance

- spec の In Scope 全項目が実装・記録されている。
- `mise run fmt-check` / `check|lint|test ruviform-{audio,midi}-device` / `prek run --all-files` が Pass。
- probe 出力が `evidence/rf629/` にあり、未 provisioning → `NotRun`/`NOT_PROVISIONED` が確認できる。
- `virtual-os-endpoint` evidence を主張する記述がない。

## Notes

- probe の実測 quirk（`IsFormatSupported` out-param 必須・`midiOutMessage` crash・not-present activate `E_NOTFOUND`）は discovery/design/IR に記録済み。
- 後続 slice（runtime controller・Backend 配線・CI）への hand-off は plan Open Questions に記録。

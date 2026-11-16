---
id: "PLAN-034Yo3vhS2za5tvUPvykUV"
type: "plan"
status: "approved"
title: "Issue #253 virtual-first OS-endpoint MIDI e2e verifier plan"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-253-virtual-midi-e2e.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-virtual-midi-e2e.md"
    - "docs/test-specs/issue-253-virtual-midi-e2e.md"
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
    - "docs/designs/issue-253-virtual-midi-e2e.md"
  derived-by:
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #253 virtual-first OS-endpoint MIDI e2e verifier Implementation Plan

## Goal

実 WinMM backend → provisioned loopback-style port pair →
`InputTx::pump`/`InputStage` → `attach` → `PluginNode` → typed
`GraphRuntime` → out-of-process VST3 `test_instrument` → L1 PCM
oracle の合成 e2e を、`#[ignore]`-gated・fail-closed の手動
verifier として `tests/os_endpoint_midi_e2e.rs` に実装する。

## Scope

単一 implementation stream。production コード変更なし — test
file 1 本 + 承認済み dev-dep 1 件 + docs/evidence のみ。

## Upstream Documents

- `docs/specs/issue-253-virtual-midi-e2e.md`

## Design Inputs

- `docs/designs/issue-253-virtual-midi-e2e.md`

## File Map

- `crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
  - 新設。fixture plumbing・attach rig・pair 選択・stamp 契約・
    anchor 較正・PCM/counter/close oracle・`#[ignore]` entry。
- `crates/ruviform-plugin-test-vst3/Cargo.toml`
  - `[dev-dependencies]` に `ruviform-midi-device` path dep を
    追加（Coordinator 承認済み・理由コメント付き）。
- `Cargo.lock`
  - dev-dep 追加に伴う再生成差分のみ。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/
  issue-253-virtual-midi-e2e.md`（exp は `-exp.jsonl` companion
  付き） — doc chain。
- `evidence/rf253/` — 検証記録（Case schema 語彙・正直な
  NotRun 記録）。

## Implementation Sequence

- [x] 先例調査: `loopback.rs`（pair 選択・close semantics）・
  `l1_real_test_plugin_e2e.rs`（fixture/attach/graph/PCM oracle）・
  `input.rs`（`InputTx::pump`/`PumpStats`）・`attach.rs`
  （`AttachConfig::input`/`Attached`）・`PluginNode::process`
  （control command と input 収集の順序）・media-core 型。
- [x] `Cargo.toml` に `ruviform-midi-device` dev-dep を追加し
  `Cargo.lock` を再生成。
- [x] `tests/os_endpoint_midi_e2e.rs` を実装（初版は API 推測を
  含んでいたため実 API に合わせて全面改訂 — `attach` 戻り値・
  `InputStageCounters` field 名・`set_clock_anchor` 等を実コードで
  確認済み）。
- [x] lint 修正: `doc_markdown` の backtick・`too_many_lines` の
  helper 分解・cast lint の checked 変換/`#[expect]`（repo 既存
  pattern）。
- [x] compile/lint/fmt の補助検証（cargo 直接実行 — canonical は
  mise lane）。
- [x] docs chain（spec→design→plan→test-spec→task→ir→exp）と
  evidence 記録。
- [x] review-protocol レビュー・指摘対応（blocker 無し —
  ACCEPT with non-blocking notes）。
- [x] canonical mise lane（`check-l1-real-plugin`/
  `lint-l1-real-plugin`/`test-l1-real-plugin`/`fmt-check`/
  `check|lint|test ruviform-plugin-test-vst3`/`prek`）— 全て
  exit 0、`evidence/rf253/` に記録。
- [x] 補助実走（`--ignored` 個別指定）— `verdict: NotRun —
  NOT_PROVISIONED`（0 input / 1 output ports）を evidence に記録。
- [ ] commit → push → PR（`Refs #253`）。

## Dependencies and Risks

- `attach`/`InputTx::pump`/`InputStage`/typed graph/L1 fixture は
  全て merge 済み — 新規 production コード不要。
- risk: name match が virtual endpoint を証明しない — verdict/
  docs/evidence で `virtual-os-endpoint` 非主張を固定。
- risk: `mise.toml` lane 未登録のため canonical lane は compile/
  lint のみ — 実走は補助証拠として明記し、lane 登録は PR 本文
  「未対応の作業」へ。
- risk: provisioned pair が無い環境では Pass 経路を一度も実行
  できない — pair 存在時の Fail/Pas 判定は code review と
  scripted 先例の oracle 形状で担保し、実走 Pass は後続 slice。

## Task Breakdown

- 単一 task（`docs/tasks/issue-253-virtual-midi-e2e.md`）— 分割
  不要。

## Verification

- [x] `cargo check -p ruviform-plugin-test-vst3 --features
  l1-runner --tests`（補助）— Pass。
- [x] `cargo clippy -p ruviform-plugin-test-vst3 --features
  l1-runner --tests -- -D warnings`（補助）— Pass。
- [x] `cargo check -p ruviform-plugin-test-vst3`（feature なし、
  補助）— Pass（`#![cfg]` gate）。
- [x] `cargo fmt -p ruviform-plugin-test-vst3 -- --check`（補助）
  — Pass。
- [x] `mise run fmt-check` / `mise run check
  ruviform-plugin-test-vst3` / `mise run lint
  ruviform-plugin-test-vst3` / `mise run test
  ruviform-plugin-test-vst3` / `mise run check-l1-real-plugin` /
  `mise run lint-l1-real-plugin` / `mise run test-l1-real-plugin`
  / `mise exec -- prek run --all-files`（canonical）— 全て
  exit 0。
- [x] `mise exec -- cargo test -p ruviform-plugin-test-vst3
  --features l1-runner --test os_endpoint_midi_e2e -- --ignored`
  （補助証拠）— exit 0、`verdict: NotRun — NOT_PROVISIONED`。

## Open Questions / Gaps

- provisioned 仮想 MIDI pair の有無は環境依存 — 無ければ本
  verifier の Pass 経路は未実走（正直な `NotRun`）。
- `mapped == delivered` counter 仮定は「全 event が処理済み
  block 範囲に写像される」前提に依存 — stamped span 超過は
  `MAX_BLOCKS` で事前 Fail。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 slice は単一 worker の
  doc-driven slice）。
- Independent work streams: none。
- User approval required before dispatch: yes（Coordinator が
  review-protocol review の blocker 無しを承認として扱う）。

## Review Handoff

- reviewer focus: fail-closed 境界（NotRun/Fail/Pass の分類が
  spec/test-spec と一致するか）・oracle の独立性（期待値を被試験
  経路が生成しないか）・`virtual-os-endpoint` 非主張の徹底・
  dev-dep が承認範囲（workspace path dep のみ）に収まるか。

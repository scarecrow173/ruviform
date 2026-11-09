---
id: "IMPL-034Unx21W1IKfK78iReH5b"
type: "impl"
status: "completed"
title: "Issue #410 view command ledger bounded retention and correlation hardening"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-410-view-command-ledger-bounded-retention-and-correlation-hardening.md"
      - type: "file"
        path: "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/plans/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/tasks/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
      - type: "file"
        path: "docs/test-specs/issue-410-view-command-ledger-retention-correlation-guarantees.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #410 view command ledger bounded retention and correlation hardening

## Summary

`ViewHost::command_index`・`CommandBinding::outbox`・`push_event` の
イベント backlog・`validate_spec` のコマンド owner 重複・単調
カウンタの wrap を、view/behavior 層の retention + 相関 semantics
として hardening した。長寿命 view がコマンド/イベント状態を
無制限に蓄積しないこと、receipt 相関が lifecycle 境界を越えて
正しいこと、曖昧な `control_id` 共有を黙って誤配しないことを
保証する。projection envelope への epoch segment 追加は wire
protocol 判断として本 slice の対象外（別スライス）。

## Context

PR #406 / RF387 の独立レビューで指摘された retention・相関の
ギャップを是正する Issue #410 の、`view.rs`/`behavior.rs` 内の
retention + dedup slice。文書チェーンは spec → design → plan →
task → test-spec を `docs/` 配下に作成済み。

## Implementation

### Changes

- `ViewHost::command_index` に `MAX_COMMAND_INDEX = 512` を導入。
  `CommandBinding::record_ledger_state`（`pub(crate)`）が返す
  liveness class（Dead → Settled → LiveNonExpect → LiveExpect、
  同 class は command id 辞書順）で決定的に eviction し、
  全件 LiveExpect の場合は `MAX_PENDING_RECORDS` と同じく
  yield して in-flight を stranded にしない。
- `CommandBinding::outbox` を `MAX_OUTBOX_DISPATCHES = 64` に
  bound。未 drain の dispatch は最古から捨てる。outbox は有界な
  dispatch 履歴であり、権威ある配送経路は host の
  `ViewEvent::Intent` stream。
- `push_event` の 512-cap を正直な契約に修正。coalescible 優先の
  うえで droppable が無ければ最古を drop する（non-droppable も
  失われ得ることをコメントで明示）。drop された `Intent` は
  `retire_undelivered_intent` が receipt routing を除去し、
  owner binding の未解決 receipt-expecting record を
  `expire_command` で delivery-lost（`Timeout`）にする。
  settled record は触らず timeout を捏造しない。
- `validate_spec` で canonical mount（flat key == spec key）の
  command owner id 重複を reject。owner は control の実効
  `control_id`（明示または key fallback）または action-bearing
  component（Button・選択可能 Table/Tabs）の key。
  tab alias 経由の同一 node は重複とみなさない。
- `key_for_control` は同一 `control_id` を持つ mounted entry が
  複数ある場合に canonical mount（key == spec key）を優先し、
  alias は辞書順最小を fallback として detach/unmount 帰属を
  決定的にする。
- `invoke`/`emit_selection` の binding key を `owner_key_for`
  経由で canonical owner id に正規化。tab alias 経由で mount
  された action component が alias キーで command を mint して
  dedup 空間・`release` の対象外になる穴を塞ぐ
  （review で検出・修正）。
- `InteractionControl::sequence`/`atomic_counter`、
  `InteractionDispatcher::synthetic_seq`、
  `CommandBinding::counter`/`observed_seq`/`generation`、
  `ViewHost::generation`/`mount_epoch`/`action_sequence` の
  `+= 1` を `AttachEpochAllocator::mint` の checked 先例に合わせ
  `checked_add(1).expect(...)` 化。直接 panic を持つ public
  関数に `# Panics` 節を追加。

### Decisions During Implementation

- eviction の同 class tie-break は command id の辞書順。
  FIFO ではないが決定的であり、ledger の retention 保証に
  順序は要求されない。
- `expire_command` は `status.is_none() && expect_receipt` の
  record のみ `expired` を立てる。`on_tick` の timeout 条件と
  同じで、非 receipt command に timeout 概念は無い。
- index entry が既に eviction 済みの場合、command id は
  binding namespace で一意なため `bindings` を走査して owner を
  特定する fallback を `retire_undelivered_intent` に置いた。
- u64 カウンタ枯渇は到達不能だが、wrap は順序を静かに壊すため
  `MAX_PENDING_RECORDS`/`AttachEpochAllocator` と同じく
  正直な panic とした。

## Related Experiments

### Adopted

<!-- 実験ログは未作成。アプローチは既知の先例（MAX_PENDING_RECORDS、
AttachEpochAllocator::mint）の適用であり、探索的実装ではなかった。 -->

### Rejected

<!-- 同上。 -->

## Validation

`mise` タスクで実施（Windows、rust toolchain は repo pin）:

- `mise run fmt-check` — exit 0
- `mise run check` — exit 0
- `mise run lint` — exit 0
- `mise run test` — exit 0（ruviform-ui unit 94 tests を含む
  workspace 全件 pass）
- `mise run test-ui-component-view` — exit 0（17 tests pass）

補助証拠（canonical = mise）: `cargo test -p ruviform-ui --lib`
で新規回帰テストを個別確認。

## Risks

- 全件 LiveExpect のとき `command_index` は bound を超えて
  yield する。実効上の上限は binding 数 × `MAX_PENDING_RECORDS`
  で有界だが、極端な spec 規模では index が 512 を超え得る。
  `MAX_PENDING_RECORDS` と同じ honesty であり設計どおり。
- event backlog drop で配送を失った非 receipt command は
  timeout を持たず未解決のまま残る。`on_tick` の既存契約と
  同じ範囲であり本 slice の対象外。

## Follow-ups

- projection envelope への epoch segment 追加（wire-protocol
  判断・別スライス/契約 Issue）。
- dead-generation `PendingRecord` zombie cleanup、GPUI
  reentrant-borrow nit は対象外として別 dispatch/Issue。
- Issue #464（GUI scenario/receipt verification）は別タスク。

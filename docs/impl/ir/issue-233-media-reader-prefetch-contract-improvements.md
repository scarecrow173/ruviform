---
id: "IMPL-034QoBjMxcoddobxwYw9Tj"
type: "impl"
status: "completed"
title: "Issue 233 media-reader prefetch contract improvements"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/233"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md"
      - type: "file"
        path: "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
      - type: "file"
        path: "docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md"
      - type: "file"
        path: "docs/designs/issue-233-media-reader-prefetch-improvements.md"
      - type: "file"
        path: "docs/plans/issue-233-media-reader-prefetch-improvements.md"
      - type: "file"
        path: "docs/test-specs/issue-233-media-reader-prefetch-improvements.md"
      - type: "file"
        path: "docs/tasks/task-233-t1-contract-docs.md"
      - type: "file"
        path: "docs/tasks/task-233-t2-discard-pending.md"
      - type: "file"
        path: "docs/tasks/task-233-t3-reopen-at.md"
      - type: "file"
        path: "docs/tasks/task-233-t4-verify-ir.md"
      - type: "file"
        path: "docs/impl/ir/issue-233-media-reader-prefetch-contract-improvements.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/rt_fill.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/task-233-t4-verify-ir.md"
    - "docs/tasks/task-233-t1-contract-docs.md"
    - "docs/tasks/task-233-t2-discard-pending.md"
    - "docs/tasks/task-233-t3-reopen-at.md"
    - "docs/plans/issue-233-media-reader-prefetch-improvements.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md"
    - "docs/designs/issue-233-media-reader-prefetch-improvements.md"
    - "docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-233-media-reader-prefetch-improvements.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 233 media-reader prefetch contract improvements

## Summary

Issue #233 の 4 candidate を全て additive に landing した。#197 contract の互換性を維持したまま、doc 明確化 2 件（candidate 1・3）と feature 追加 2 件（candidate 2・4）を実装した。

Per-candidate outcome:

| Candidate | 要求 | 結果 |
| --- | --- | --- |
| 1 | `fill` `expected_first` semantics の明確化 | **implemented doc-only** — stream-position assertion である旨を module contract・`fill` rustdoc・`FillReport.mismatch`・README に明記 |
| 2 | `seek` stale pending play-through 対策 | **implemented feature** — `PrefetchHandle::seek_discard_pending`（producer pending purge、opt-in flag）+ `PrefetchConsumer::discard_pending`（RT-safe 有界 drain、`DiscardReport` 返却） |
| 3 | `pump()`/`PumpOutcome` contract の明確化 | **implemented doc-only** — ≤1 sequenced item per call・command-first order・`Backpressured` は retryable・marker が producer `pending` に残留し得る旨を module contract・enum/method rustdoc・README に明記 |
| 4 | `reopen` direct reopen-to-seek-target | **implemented feature** — `PrefetchHandle::reopen_at`（reopen→seek fused、boundary は実 landed 位置を報告） |

Declined/deferred なし。ADR で却下した代替案（shared requested-generation cell・`PrefetchItem::Discard` variant・pending-purge-only）は設計判断として下記に記録。

## Context

- 実施タスク: `docs/tasks/task-233-t1-contract-docs.md`〜`task-233-t4-verify-ir.md`
- 上流: `docs/plans/issue-233-media-reader-prefetch-improvements.md`、`docs/designs/issue-233-media-reader-prefetch-improvements.md`、`docs/specs/issue-233-media-reader-prefetch-contract-improvements.md`、`docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md`
- 動機: Synvelis #24 E2E-L1（Local WAV Import → Verified Source → Track/Clip → Generic Graph Playback）で #197 を実使用した際の 4 改善候補。Synvelis 側は quiesce-drain→seek workaround 済みで、永続的な app 側重複は入れていない。契約は generic に保ち Synvelis/AudioLens 固有 semantics は導入しない。

## Implementation

### Changes

- `crates/ruviform-media-reader/src/prefetch.rs`
  - Module `## Contract` に `expected_first` の stream-position assertion 説明と `**Drive.**` bullet（pump ≤1 item・command-first・pending marker 残留・manual driver 反復契約）を追加。
  - `PrefetchConsumer::fill` rustdoc・`FillReport` struct/field doc を stream-position assertion 表現に修正。
  - `PumpOutcome`・`PrefetchProducer::pump` rustdoc を ≤1 item/retry 契約で拡張。
  - `PrefetchCmd::Seek` に `discard_pending: bool` を追加（private enum、wire 非公開）。`seek` は `false`（既存動作）、新規 `seek_discard_pending` は `true` を送る。`exec_cmd` で `pending.clear()` を boundary push 前に適用。
  - `PrefetchCmd::Reopen` に `first: Option<u64>` を追加。`reopen` は `None`（frame-0 anchor 維持）、新規 `reopen_at` は `Some(first)` を送る。成功時は post-reopen `seek_source_frame` → landed 位置の `Boundary`、seek 失敗時は `Fault{Unseekable}` + reopened 位置の `Boundary`、reopen 失敗時は既存の fault + `stream_ended` 経路を維持。
  - `PrefetchConsumer::discard_pending(&RtContext) -> DiscardReport` を追加。in-flight `current` を free ring へ返却し、ring item を `step()` と同一の sequencing 規則で処理（gen-mismatch は stale-drop、matching `Fault`/`End` は state 遷移、最初の `Boundary` で generation/first_frame を adopt・fault/end state クリア・停止）。data-ring capacity + in-flight 1 block で有界、alloc/lock/blocking なし。
  - `#[non_exhaustive]` `DiscardReport { items, blocks, boundary: Option<u64> }` を追加。
- `crates/ruviform-media-reader/src/lib.rs` — `DiscardReport` を re-export。
- `crates/ruviform-media-reader/README.md` — contract 明確化・新 API・RT-suitable 一覧に `discard_pending` を追記（日本語、既存スタイル）。
- `crates/ruviform-media-reader/tests/prefetch.rs` — 新規 7 テスト（seek default play-through・drain-to-boundary・boundary None・pending purge・reopen_at landed・reopen_at seek failure・reopen frame-0 regression）。
- `crates/ruviform-media-reader/tests/rt_fill.rs` — `rt_consumer_discard_pending_no_alloc`（`assert_no_alloc` 配下、zero-alloc/bounded 検証）。

### Decisions During Implementation

- **`seek`/`reopen` の既存動作は不変**: `discard_pending`/`first` は private `PrefetchCmd` の opt-in field であり、既存メソッドは `false`/`None` を送る薄い wrapper に集約した（`seek_cmd`/`reopen_cmd` private helper）。public surface の breaking change なし。
- **pending purge は producer 側 queue のみ**: ring backlog は consumer `discard_pending` が担当する責務分離（ADR で pending-purge-only を却下した根拠どおり、ring 内 item は consumer ownership）。
- **drain は最初の `Boundary` で停止**: その後の new-generation content は queued のまま次の `fill` へ渡る。generation adoption は boundary-driven の既存 model を変更しない（shared requested-generation cell / `PrefetchItem::Discard` variant は ADR で却下 — public enum 互換性リスクと generation model 変更に見合わない）。
- **reopen_at の landed 報告**: `seek_source_frame` は requested≠landed があり得るため、boundary `first_frame` は `reader.position()`（実位置）を採用。`SeekReceipt` の requested-versus-landed semantics と整合。
- **Residual limitation（documented）**: drain 完了後・boundary 到着前に届いた item は discard できない（boundary-driven model の当然の帰結）。strict な再 anchor は `seek_discard_pending` pumped 後に `discard_pending` を呼ぶ使い方を rustdoc に明記。
- **テスト失敗の修正**: `rd02_reopen_at_seek_failure_faults_then_reanchors` の初回実測で、pre-boundary の新 generation `Fault` は consumer 未 adopt のため `fault_events` ではなく `stale_drops` に計上されることが判明 — `step()` semantics と整合する正しい挙動であり、assertion を `stale_drops >= 1` に修正（実装は不変）。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64、Git Bash、mise 管理 rust 1.99.0。

| 検証 | 結果 |
| --- | --- |
| `mise run fmt-check` | pass |
| `mise run check` | pass |
| `mise run lint` | pass（`-D warnings` 下で `unused_must_use` 1 件を `let _ =` で解消後 clean） |
| `mise run test` | pass（workspace 全量 green。media-reader: prefetch.rs 20 tests・rt_fill.rs 3 tests・既存テスト unmodified で全 pass） |
| `mise run test-rt` | pass（`rt_consumer_discard_pending_no_alloc` を含む rt_ lane） |
| review-protocol | 2-reviewer 独立レビュー実施 → verdict **ACCEPT WITH NON-BLOCKING NOTES**（詳細は Follow-ups 節） |

## Risks

- `discard_pending` の residual window（drain 完了〜boundary 到着の間に届いた item）は契約上の documented limit であり、boundary-driven generation model を変えない限り除去できない。Synvelis 側の quiesce-drain→seek より厳密だが、boundary 未着の時点では同じ制約がある。
- `reopen_at` は replacement source の seekable を pre-check しない（新 source の capability は reopen 後でないと不明）— seek 失敗は `Fault{Unseekable}` + boundary で報告する経路を採用。
- `pending.clear()` は command batch 内で FIFO 順に処理されるため、同 batch 内の後続 command marker は消えない（後続 command は clear 後に実行される）— ordering 上安全だが、同一 batch で seek_discard_pending の**後**に seek を送ると前者の boundary が残り得る（caller の責務として contract 記述どおり）。

## Follow-ups

Review-protocol verdict: **ACCEPT WITH NON-BLOCKING NOTES**。2 reviewer（cooperative/adversarial）計 13 candidate、P0/P1 なし。新規 doc 文言の不正確箇所・tautological assertion・`eof_seen` telemetry parity・lib.rs RT 一覧 drift・README `close` 誤分類は修正済み。残る CONFIRMED P2/P3（いずれも pre-existing contract semantics で本 change の regression ではないため記録・非修正）:

- **新 generation `Fault` が boundary 手前で stale-drop される**（P2、両 reviewer 指摘）: `seek`/`reopen_at` の失敗経路が `Fault{gen=N}` → `Boundary{gen=N}` の順で emit するため、consumer（gen=N-1 adopt 中）は fault を `fault_events`/`FaultCounts` ではなく `stale_drops` へ計上する。#197 の `seek` 失敗 arm 由来の既存 semantics であり `#233` は同順序を維持。観測性を上げるには Boundary 先行化が必要で、これは #197 公開挙動の変更（非 additive）となるため本 issue の scope 外 — 別 issue 候補として記録。
- **reopen 失敗経路は `Boundary` を emit しない**（P2、pre-existing）: `open`/`reader.reopen` 失敗時は `Fault` + `End` のみで generation 未 adopt のまま starve する。#197 既存挙動。`reopen_at` rustdoc に failure-path 記述を追加済み。
- **pending 経路の `End` requeue が `end_emitted` を立てず duplicate `End` を emit し得る**（P3、pre-existing wart、consumer 側は idempotent で benign）。`seek_discard_pending` の `pending.clear()` はこの queue を purge する点で wart を打ち消す方向に作用。
- **`discard_pending` の fault arm 系・End-then-Boundary 系の test coverage**（P3）: in-flight `current` drain は `rd02_discard_pending_returns_inflight_block` で補強済み。fault arm は seeded fault fixture が必要で cost 見合い — 既存 fault 経路テスト（rd04 系）と `step()` semantics 一致で cover。
- Synvelis/AudioLens 側での新 API adopt（`seek_discard_pending` + `discard_pending` ペア、`reopen_at`）は consumer repo 側の follow-up — Ruviform 側 contract は本 IR で完了。

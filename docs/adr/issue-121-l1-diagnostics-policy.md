---
id: "ADR-034O9Rrn3iBoq4HxT07eVQ"
type: "adr"
status: "accepted"
title: "issue-121-l1-diagnostics-policy"
created: "2026-10-29"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/discovery/issue-121-diagnostics.md"
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/impl/ir/rt-observability-poc.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/discovery/issue-121-diagnostics.md"
  derived-by:
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/test-specs/issue-121-l1-audio-counters-tests.md"
    - "docs/test-specs/issue-121-l1-stderr-drain-tests.md"
    - "docs/test-specs/issue-121-l1-diagnostics-retention-tests.md"
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
    - "docs/test-specs/issue-121-l1-parity-nonregression-tests.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# issue-121-l1-diagnostics-policy

## Context and Problem Statement

L1 の audio underrun は計数されず、子 stderr は親へ継承され、transport/protocol 等の失敗原因は `HostError::Ipc` に畳み込まれる。#117 の Bundled では複数インスタンスが子を共有する。RT 処理、公開通知、子の生成失敗、破棄時の既存契約を保ちながら診断できる境界が必要である。

本 ADR は accepted。詳細設計（[design](../designs/issue-121-l1-diagnostics.md)）は承認済みであり、実装は設計・本書を根拠とする。#101 はクレートを構成候補とし、#104 の PoC は本番 API への昇格を意味しない。

## Decision Drivers

* RT 経路で確保・ロック・I/O・無界な待機を追加しない。
* 観測対象の所有粒度（インスタンスの計数と子の stderr）を偽らない。
* 未生成 handle を捏造せず、既存の `Failed` と exit status の意味を保持する。
* wire と `HostError` を変更しない。変更する公開 API のソース互換性を明示する。

## Considered Options

* **A**: `ruviform-observability` の共有 counter、非 RT snapshot、子ごとの bounded stderr drain、host 側原因通知。
* **B**: host ローカル counter のみとし、stderr は inherit、エラーは `Ipc` のままにする。
* **C**: 毎ブロックの診断を通知または `rt_bridge` レコードで流す。
* **D**: stderr を opt-in にする。

## Decision Outcome

Chosen option: **A**。ユーザーは observability クレートの新設と常時 piped・per-child drain を選択した。counter snapshot は高頻度の数値集計に適し、イベントごとの通信を要しない。`rt_bridge` の実装は本件の要件ではない。詳細設計で確定した主要点:

- **counter**: `dropped_submits`/`passthrough_submits`（warmup 込み）/`results_failed_observed`/`results_dropped` の 4 本 — `InstShared` 内 `Arc<AtomicU64>` Relaxed、leg swap/checkout で維持、re-mint で新 counter、destroy で not-found。
- **stderr**: `Stdio::piped` + per-child drain（読み取り義務と保持を分離 — 満杯でも読み続ける）。64 KiB tail 優先 ring + 完了失敗 retention 8 件（512 KiB は retained stderr payload のみの上限）。`STDERR_DRAIN_GRACE` 500 ms・子終了観測起算・延長なし、終了時 read fd close（heir 後続 write は error — 明示受容）。終了保証は OS 別限定。
- **pre-mint 失敗**: handle 無し notification variant は追加しない — error result（`HostError::SpawnFailed { cause, diagnostics_id }`）+ retention 登録時連続採番の diagnostics ID で query 可能な bounded 診断レコード（`phase`/`cause` 区別・found/evicted/not-found 識別）。rollback は既存 kill+reap 契約、bounded ≠ 期限内終了完了を意味しない。
- **cause 語彙**: `FailureCause`（`Copy`・`#[non_exhaustive]`）7 variant、first-observed を同一 lock 内で固定。`InstanceFailed` に `cause` field + variant レベル `#[non_exhaustive]` — 完全 pattern の source break を workspace 内限定で受容。wire・`HostError` wire tag は不変（`HostError` は codec 対象外）。

### Consequences

* Good, because endpoint checkout 中も共有 counter を読め、子単位の stderr と exit fact を区別できる。
* Bad, because 子ごとの drain スレッドと bounded storage が必要であり、#120 の再評価条件に関連する。
* Bad, because 既存 struct variant の field 追加は enum の `#[non_exhaustive]` だけではソース互換にならない。互換性影響を開示する。

### Confirmation

各 AC と #101 の依存方向を設計レビュー、RT 無割当テスト、audio/video・Bundled E2E、stderr の飽和・heir fixture で確認する。

## Implementation Plan

* Affected paths: `Cargo.toml`、新規 `crates/ruviform-observability/`、`crates/ruviform-plugin-host/src/{composite,endpoint,supervisor}.rs`、`src/l1/{mod,data,video}.rs` と関連テスト。
* Patterns to follow: #104 の counter pair、#117 の子／instance 失敗境界、既存 endpoint の所有移動。
* Patterns to avoid: RT 側のロックや I/O、stderr を instance に帰属させること、`ChildExit` に原因を混ぜること。
* Dependencies or configuration: plugin-host から observability への下向き依存を評価する。`ruviform-rt` からの新規依存、wire 変更、外部 backend は本 ADR の対象外。

## Verification

* [ ] `mise run fmt-check`、`check`、`lint`、`test`、`test-rt` と文書監査。
* [ ] 失敗原因と実 exit status を分離し、生成前失敗を偽 handle なしに観測できる。
* [ ] stderr 高負荷、consumer 不在、heir 継承下の終了を検証する。

## Pros and Cons of the Options

### A — counter と child drain

* Good: 既存の RT 境界と子単位の事実に合う。
* Bad: drain と失敗情報の所有・終了設計が必要。

### B — 現状維持

* Good: 追加のスレッドや公開型が不要。
* Bad: Issue の診断要件を満たさず、クレート新設のユーザー判断にも反する。

### C — 毎ブロックのイベント

* Good: 個別イベントを表現できる。
* Bad: 集計のみの要求には転送・容量管理が過剰である。

### D — opt-in stderr

* Good: 無効時の資源コストを避けられる。
* Bad: 予期しない初回失敗の診断を逃し、常時配管のユーザー判断に反する。

## More Information

詳細と未解決事項は [design](../designs/issue-121-l1-diagnostics.md) と [discovery](../discovery/issue-121-diagnostics.md) を参照する。

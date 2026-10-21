---
id: "IMPL-034JyAqvg5hMrhqv41ZXfk"
type: "impl"
status: "completed"
title: "リタイアチャネルの実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/retire.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "docs/tasks/implement-rt-retire-channel.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-rt-retire-channel.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-rt-spsc-channel.md"
  blocks:
    - "docs/tasks/implement-rt-snapshot-exchange.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-rt-foundation.md"
    - "docs/designs/rt-foundation-api-design.md"
    - "docs/adr/rt-foundation-primitives-implementation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-channel-and-snapshot-contracts.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# リタイアチャネルの実装

## Summary

`ruviform-rt` に遅延ドロップ retire channel（`retire_channel` /
`Retirer<T>` / `DropCollector<T>`）を実装した。RT 側で退役した重い
オブジェクトの `Drop`（dealloc・ロック・非有界ユーザコード）を非 RT
側の `drain` へ送る SPSC ベースの basedrop 型機構で、満杯時は
`core::mem::forget` の leak-on-full に倒れる（drop-on-full は RT 側
dealloc のため ADR で不採用）。公開 API は `retire_channel`（non-RT
only）、`Retirer::retire`（RT-suitable）、`DropCollector::{drain,
is_empty}`（non-RT only）の 4 項目（→ レビュー対応で
`overflows()` が両端点に追加され計 6 項目）。`drain` はチャネル受信分
（capacity 上限の bounded pop）とローカル pending `Vec` の双方を呼び出し
スレッド上で drop する。

## Context

task `TASK-034JxO9ex0aVfyvjYkcaWT`（`implement-rt-retire-channel.md`）。
plan `PLAN-034JxNegJktSAJZ2LuO0N8` Phase B、design
`DESIGN-034JgsK9khgDsPqG04Sp6i` の retire.rs API sketch が正本。T2 の
SPSC（`channel`/`Producer`/`Consumer`/`TryPushError`）をそのまま再利用し
`rtrb` の上に構成 — basedrop 依存は採否 ADR で不採用済み。

## Implementation

### Changes

- `crates/ruviform-rt/src/retire.rs`（新規・約 370 行）:
  `retire_channel<T: Send + 'static>(capacity)` は内部 `channel` を構築して
  `(Retirer, DropCollector)` を返す。`Retirer` は `Producer<T>` を wrap し、
  `retire` は `try_push` → `TryPushError::Full(value)` 時に
  `core::mem::forget(value)`。`DropCollector` は `Consumer<T>` + ローカル
  pending `Vec<T>` を保持し、`drain()` は `capacity` 回を上限に `try_pop`
  ループで受信分を drop した後 pending を `clear`（戻り値は合計 drop 件数）。
  `pub(crate) push_local` はタスク 4 の snapshot が publish で追い出した
  `Arc` を同一 drain 経路に乗せる seam。モジュール doc に teardown 注記
  （最後の端点 drop 時に残留値は drop 側スレッドで破棄される wrapped-ring
  意味論 — `Retirer` より `DropCollector` を長生きさせること）を記載。
- `crates/ruviform-rt/src/lib.rs`: `mod retire;` +
  `pub use retire::{DropCollector, Retirer, retire_channel};`。
- テスト（retire.rs `#[cfg(test)]`）: `rt_retire_then_drain_drops`・
  `rt_retire_full_forgets_not_drops`・`rt_retire_no_alloc`（T2 の
  `crate::alloc_counter` を再利用）・`rt_endpoint_send_bounds`・
  非 RT の `local_pending_drops_on_drain`（`push_local` seam の検証）・
  `local_pending_drops_on_drain` 等。dealloc 観測は `Drop` で
  `Arc<AtomicBool>` を立てる `DropFlag` プローブ型。

### Decisions During Implementation

- **PR レビュー対応（P1）— leak-on-full の保証範囲を明確化＋飽和の
  可観測性**: レビュー指摘どおり `mem::forget` ベースの
  leak-on-full はリーク総量の上限を保証しない（spec/ADR の設計意図
  どおりだが保証範囲の明文化が不足）。採用した境界修正:
  `Retirer`/`DropCollector` 間で共有する `Arc<AtomicUsize>`
  overflow カウンタを追加し、両端点に `overflows() -> usize`
  （Relaxed 読み・RT-suitable/non-RT only を各々ラベル）を公開。
  `retire` の forget パスで increment — 飽和は silent ではなく
  観測可能という設計（reviewer 提示の選択肢のうち「失敗の可観測性」
  に相当）。
- **PR 再レビュー対応（P1）— 保証範囲のさらなる正確化＋assert! 昇格**:
  「drain 間隔あたり bounded」という表現も不正確だった（キュー滞留
  ≤ capacity は同区間の `mem::forget` 回数を拘束しない — `retire`
  呼び出し回数・レートに契約上の上限がないため、drain 間隔内でも
  任意回数リークし得る）。正確な保証は「キュー容量・`retire` ごとの
  処理量が有界・飽和件数が観測可能」までであり、リーク総量は
  drain 間隔・生涯のいずれでも非有界と明記。呼称も誤読を招く
  `bounded-leak` から機構記述の `leak-on-full` へ全コード・正典
  ドキュメント（spec/ADR/design/test-spec/plan/IR）で改名し、design
  の退役経路契約・spec 受入基準・test-spec 不変条件に「メモリ使用量
  の厳密な上限は保証しない」を明記。容量 0 前提は `debug_assert` では
  release で無効化されるため、構築は非 RT 経路であることから
  `assert!` に昇格（全ビルドで拒否。should_panic テストは cfg ゲート
  解除で全ビルド対象とし、`cargo test --release` で release 相当挙動も
  実測検証）。teardown 順序（collector が retirer を outlive）は
  `rt_retire_teardown_collector_outlives` でテスト確定。

- `push_local` に `#[allow(dead_code)]` を付与（`#[expect]` ではない）:
  タスク 4 が `snapshot.rs` を追加するまで本番呼び出し元が存在せず
  `-D warnings` ゲートで `dead_code` になるため。`expect` は利用開始後に
  unfulfilled となり再び lint エラーになるため `allow` を選択。コメントで
  snapshot 到着までの暫定措置である旨を明記。
- `drain` のチャネル側ループを `capacity` 回に上限化: `pop_latest` と同じ
  bounded-loop 規則に揃え、drain 中の並行 retire で非有界ループにならない
  ことを保証。
- `is_empty` はチャネル近似値 + pending 実値の合成として「観測時点近似」
  を doc 明記（spsc の capacity/is_* 診断規約に整合）。

## Related Experiments

### Adopted

なし — 方式は design/ADR で確定済みの既知解。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p ruviform-rt`: 19 unit + 32 doctest 合格
  （→ レビュー対応後は 24 unit + 36 doctest 相当に増加: overflow /
  teardown / zero-capacity テスト追加。最終数はワークスペース
  `mise run test` の出力を参照）
- `mise exec -- cargo test -p ruviform-rt rt_`: `rt_` 命名 14 件のみ選択
  （retire 追加分 4 件を含む）・全合格、`rt_` 部分文字列の混入なし
- `mise exec -- cargo clippy -p ruviform-rt --all-targets -- -D warnings`:
  0 warnings（`push_local` の dead_code は上記 `#[allow]` で解消）
- `mise run fmt-check`: 差分なし
- `node .agents/skills/doc-status/scripts/audit_docs.js --type task --json` /
  `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json`:
  0 findings

## Risks

- `push_local` の `#[allow(dead_code)]` はタスク 4 で snapshot が呼び出し元に
  なった時点で除去可能（残置しても害はないが、除去を T4 の完了条件に含める
  ことを推奨）。
- 満杯時 `forget` は意図された leak-on-full であり（リーク総量は非有界 — 保証は容量・呼出処理量・可観測性まで）、容量設計を誤るとリークが
  増える — `capacity` の caller contract は API doc に記載済み。

## Follow-ups

- タスク 4（`implement-rt-snapshot-exchange.md`）: `push_local` を
  `SnapshotWriter::publish` の退役 Arc 経路から呼び、`#[allow(dead_code)]`
  を除去する。no-alloc テストは `crate::alloc_counter` を再利用すること
  （`#[global_allocator]` は lib テストバイナリに一つしか置けない）。

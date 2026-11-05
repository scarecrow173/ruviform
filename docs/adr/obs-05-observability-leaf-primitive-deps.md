---
id: "ADR-BbCUPCZvdC16eKp8RlfHvT"
type: "adr"
status: "accepted"
title: "OBS-05: ruviform-observability の ruviform-rt / ruviform-media-core への一方向依存"
created: "2026-11-05"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/280"
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
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../designs/shared-foundations-and-modularization-policy.md"
    - "../designs/issue-277-observability-core-source-contract.md"
    - "../specs/issue-277-observability-core-source-contract.md"
    - "../specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "../designs/issue-280-rt-record-bridge-clock-correlation.md"
  refines:
    - "../specs/issue-277-observability-core-source-contract.md"
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../designs/shared-foundations-and-modularization-policy.md"
  derived-by:
    - "../designs/issue-280-rt-record-bridge-clock-correlation.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/101"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# OBS-05: ruviform-observability の ruviform-rt / ruviform-media-core への一方向依存

## Context and Problem Statement

Issue #280（OBS-05）は `ruviform-observability` に RT → 非 RT の固定長
Record Bridge（`rt_bridge`）と Clock/Correlation 語彙（`record`）を追加する。
輸送には有界 SPSC リングが、clock context には domain/epoch/timebase の
語彙が必要であり、それぞれ `ruviform-rt::channel` と
`ruviform-media-core::{ClockDomainId, ClockEpochId, TimeBase, InputStamp,
TimestampProvenance}` に**既に実在する単一の所有元**がある。

一方で OBS-02（Issue #277）spec の AC-6 は「`ruviform-observability` は他
ruviform クレートへ依存しない」を no-cycle 証明として採用しており、
`crates/README.md` の Primitives 表も同クレートを依存なしとして記述する。
ここで決めるのは「OBS-05 が必要とする再利用を、どの依存方向で実現するか」
という crate DAG の一点である。

## Decision Drivers

- 第 2 のリング実装・第 2 の clock 語彙を作らない（Issue 本文の指示・
  AGENTS の DRY/単一 authority 原則）。
- `ruviform-rt → ruviform-observability` のような逆方向依存を作らない
  （Issue 本文の明示禁止）。
- `ruviform-rt` の既存公開 API・RT 契約・「最下位・相互依存なし」の位置を
  変更しない（#101 政策: `channel` の Foundation 移動は既定路線ではない）。
- `ruviform-media-core` の clock 語彙は device stamp・`EngineFrame`・
  `ClockAnchor`/`map_stamp_to_frame` と同一 identity 空間であり、
  observability が独自語彙を持つと同一概念が重複定義される。
- OBS-02 AC-6 の「依存なし」は core/source 契約の no-cycle 証明であり、
  全期間の依存禁止規則としては意図されていない。

## Considered Options

* **Option 1 — `ruviform-observability` から `ruviform-rt`・`ruviform-media-core` への一方向 path 依存を追加する（採用）**: SPSC と clock 語彙を既存の単一所有元からそのまま再利用する。依存は葉 primitive への下向き一方向で、no-cycle を維持する。OBS-02 AC-6 は「core/source 契約は domain 依存を要しない」という範囲の記述として再解釈し、本 ADR で「葉 primitive への一方向依存は許可」に改訂する
* **Option 2 — `ruviform-observability` 内に独自 SPSC ring と独自 clock 語彙を実装する**: 依存ゼロを維持できるが、`ruviform-rt::channel`（RT 契約試験済み）と media-core の clock 語彙を重複実装することになり、Issue 本文の「既存 SPSC 基盤を再利用する・第 2 のリングを作らない」指示と矛盾する
* **Option 3 — bridge を `ruviform-rt` 側に置く**: `ruviform-rt` が record/OBS 語彙を持つか `ruviform-observability` への依存を持つ必要があり、#101 政策（`rt_bridge` は observability 所有・`ruviform-rt → foundation` 系の新規依存は既定路線ではない）と Issue 本文の禁止事項に反する
* **Option 4 — bridge を新規独立クレートに置く**: Issue 本文の「`ruviform-observability` 再利用・独立 tracing クレートを作らない」に反し、crate 境界・公開面・CI lane を不必要に増やす

## Decision Outcome

**Option 1 を採用する。** `ruviform-observability` の `[dependencies]` に
`ruviform-rt` と `ruviform-media-core` の `path` 依存を追加する。

具体的な規則:

- 依存は**一方向・葉 primitive のみ**:
  `ruviform-observability → ruviform-rt`、`ruviform-observability →
  ruviform-media-core`。逆方向（`rt`/`media-core` → `observability`）や
  domain foundations / engines / adapters への依存は作らない。
- `ruviform-rt` 側の公開 API・実装・RT 契約・「最下位・相互依存なし」の
  位置は**変更しない**。`channel`/`retire_channel`/`slot` は `rt` に残る
  （#101 政策どおり移動しない）。
- `ruviform-media-core` からは `ClockDomainId`・`ClockEpochId`・`TimeBase`・
  `InputStamp`・`TimestampProvenance` の語彙のみを再利用する（値型・
  `Copy`・`const`・RT 適合）。時刻変換 authority（`ClockAnchor`/
  `map_stamp_to_frame`）は media-core 側に残り、observability は変換 API を
  持たない。
- 外部 crates.io 依存・新規 feature は追加しない。`assert_no_alloc` は
  `dev-dependencies` のまま維持する。
- OBS-02 spec AC-6 の「observability は他 ruviform クレートへ依存しない」は、
  本 ADR をもって「**葉 primitive への一方向依存のみ許可**」へ改訂したもの
  として扱う。`crates/README.md` の同クレート行と crate-level doc の依存
  方針を本決定に合わせて更新する。

### Consequences

- 良い影響: SPSC と clock 語彙が単一所有元に統一され、RT 契約試験・
  clock 変換 authority の重複が避けられる。`bridge` が rtrb 系の外部型を
  露出せず、`ruviform-rt` の公開 API だけを経由する。
- 悪い影響/コスト: `ruviform-observability` が完全な依存ゼロではなくなり、
  その利用者は `ruviform-rt`・`ruviform-media-core` も推移的に取り込む。
  どちらも葉 primitive で依存木の増分は小さい。
- フォローアップ: `crates/README.md` Primitives 表・`ruviform-observability`
  README/lib.rs の依存方針記述を更新。将来の別 Issue で OBS-07/03 が
  collector/export 依存を追加する場合はこの ADR を再参照する。

### Confirmation

- `cargo metadata`/`mise run check` で依存 DAG が一方向・無 cycle であること。
- `crates/README.md`・`ruviform-observability` README の依存記述が本 ADR と
  一致すること。
- Issue #280 の Acceptance に「逆方向依存・外部依存追加がない」ことが
  試験・レビューで確認されること。

## Implementation Plan

- `docs/designs/issue-280-rt-record-bridge-clock-correlation.md` が本 ADR に
  `derives-from` で接続し、依存方向の判断を引き継ぐ。
- T-1/T-2 タスクで `Cargo.toml` に `path` 依存を追加し、`lib.rs` の依存
  方針節と README 群を更新する。

## Verification

- `mise run check` / `mise run lint` で依存解決と lint ゲートを通す。
- `cargo tree -p ruviform-observability` で `ruviform-rt`・`ruviform-media-core`
  のみが ruviform 依存として現れ、逆方向 edge がないことを目視確認。

## Pros and Cons of the Options

### Option 1 — 葉 primitive への一方向依存（採用）

- 長所: 単一所有元・RT 契約試験の再利用・Issue 本文と #101 政策に整合。
- 短所: `ruviform-observability` が依存ゼロではなくなる（葉のみに限定）。

### Option 2 — 独自 ring・独自 clock 語彙

- 長所: 依存ゼロ維持。
- 短所: 試験済み基盤の重複実装・語彙分裂・Issue 本文の指示と矛盾。

### Option 3 — bridge を `ruviform-rt` に置く

- 長所: ring との物理的距離が近い。
- 短所: 逆方向依存か語彙混入が必須。Issue 禁止事項・#101 政策に反する。

### Option 4 — 新規独立クレート

- 長所: 依存方向の自由度が最大。
- 短所: 「`ruviform-observability` 再利用」の指示と矛盾。境界・CI コスト増。

## More Information

- Issue #280（OBS-05）・#277（OBS-02 AC-6 の原文）・#101（共有基盤政策）。
- `docs/designs/shared-foundations-and-modularization-policy.md` の
  「RT → 非 RT の有界観測レコード転送機構は `ruviform-observability` の
  `rt_bridge` が所有する」という指名と、`ruviform-rt → foundation` 系の
  新規依存を既定路線としない記述。

---
id: "SPEC-034UwbURsJwbo65nV81E1T"
type: "spec"
status: "approved"
title: "Issue #410 P2 — view command ledger retention 検証と projection correlation floor"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
    - "https://github.com/scarecrow173/ruviform/pull/502"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/impl/ir/issue-410-p2-view-ledger-correlation-hardening.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/387"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/issue-414-projection-controlled-display.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  derived-by:
    - "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 P2 — view command ledger retention 検証と projection correlation floor

## What Should Be Built

Issue #410（view command ledger の bounded retention & correlation
semantics hardening）の第 2 スライス。slice 1（PR #502）で実装済みの
`command_index` bound・`outbox` bound・`push_event` drop 契約・
command owner id dedup・checked counter を、Issue 受入条件の全 5 行に
対して検証・固定し、slice 1 が wire-protocol 判断として deferred
した **projection correlation 契約** を本 slice の範囲で定義・実装する。

具体的には以下を `crates/ruviform-ui` 内で実現する。

- `ViewHost` が `control_id` ごとの受理済み projection revision の
  high-water mark（revision floor）を attach epoch の存続期間中
  保持し、`unmount` → `mount` を跨いでも floor を維持する。floor を
  advance しない projection は stale として drop し、remount 後の
  fresh binding に古い revision が一度だけ誤適用される経路を閉じる。
- `detach` → `attach`（binding 存続）と `unmount` → `mount`
  （binding 再生成）の両 epoch 境界で、順序逆転した古い projection
  が適用されないことを unit test で固定する。
- slice 1 の実装（`MAX_COMMAND_INDEX` の liveness-class eviction、
  `MAX_OUTBOX_DISPATCHES`、512-cap の `Intent` 退役、owner id dedup、
  checked counter）が Issue 受入条件 5 行を満たすことを対応 test と
  共に確認し、不足があれば補う。

## Why Now

slice 1（PR #502）は retention bound・outbox bound・delivery-loss
退役・owner dedup・checked 化を実装したが、Issue 受入条件のうち
「mount/attach epoch を跨ぐ古い projection が新 binding に誤適用
されない correlation 契約」は未着手のまま残った。`apply_projection`
は `control_id` のみで相関し、`unmount` で `bindings` が clear
されると fresh binding の `last_observed` が `None` に戻るため、
remount 前に発行された古い revision の projection が一度だけ受理
され得る（次の新しい projection で自己修復するが、その一度の誤適用
は controlled display に stale 値を一瞬書き込み得る）。

envelope への epoch segment 追加は wire-protocol 変更を伴うため
slice 1 でも deferred とされたが、host 側で可能な correlation 強化
（受理済み revision の高水位マークを epoch 境界で維持する floor）は
wire 変更なしに実装でき、誤適用経路の大部分を閉じられる。

## Users and Value

- `ruviform-ui` の consumer（Synvelis、AudioLens、`ruviform-ui-test`/
  `ruviform-ui-gpui` の検証基盤）に対し、view の remount・detach
  を跨いだ projection の順序保証を提供する。長時間稼働 view で
  ledger が bounded に保たれ、receipt/projection の帰属が epoch
  境界で破綻しないことを保証する。
- driver/renderer 実装者に対し、projection correlation の責務境界
  （host が検証できる範囲と delivery path の義務）を明示する。

## Scope

### In Scope

- `crates/ruviform-ui/src/view.rs` と
  `crates/ruviform-ui/src/behavior.rs` 内の projection correlation
  floor 実装と、slice 1 受入条件の検証・補完。
- 上記に対応する回帰 unit test。
- 本 slice の spec/design/plan/task/test-spec/impl 文書チェーン。

### Out of Scope

- projection envelope（`Event::Notify` data / `StateProjection`）
  への epoch segment 追加 — wire-protocol 変更。別スライス/契約
  Issue（slice 1 spec の Deferred Design Concerns を継承）。
- `ruviform-ui-test` / `ruviform-ui-gpui` / `ruviform-ui-driver` /
  `ruviform-poc-native-dock` の変更 — 既存契約の消費のみ。
- `PendingRecord` dead-generation zombie の整理（別 Issue 候補）。
- `GpuiSurfaceAdapter` reentrant-borrow nit（rf388-s3 領域）。
  契約は本 slice のレビュー記録に残す。
- consumer（synvelis/audio-lens）側の変更。

## Requirements

- `ViewHost::command_index` の `Applied`/`Accepted`/`Queued` を含む
  保持レコードは、settle/timeout/epoch 終了に整合した上限・解放
  条件（`MAX_COMMAND_INDEX` bound + liveness-class eviction +
  detach/unmount clear）で bounded であり、長時間・大量 command の
  回帰 test がある（slice 1 実装の検証）。
- `CommandBinding::outbox` は `MAX_OUTBOX_DISPATCHES` で上限管理
  され、未 drain でも無制限に増加しないことが検証されている
  （slice 1 実装の検証）。
- `MAX_VIEW_EVENTS` = 512 超過時に配送されない `Intent` と
  `PendingRecord` の整合（delivery-lost → `Timeout`、receipt
  routing 除去、settled record 非影響、late receipt unrouted）が
  test で固定されている（slice 1 実装の検証）。
- `ViewHost` は `control_id` ごとの受理済み projection revision
  floor を attach epoch の存続期間中保持し、floor を advance
  しない projection を stale として drop する。`unmount` →
  `mount`・`detach` → `attach` の両境界で順序逆転・remount の
  誤適用が起きないことを test で固定する。
- 同一 spec 内の実効 command owner id 重複は `InvalidSpec` で
  reject され、mount/unmount の登録・detach attribution が一意に
  解決される（slice 1 実装の検証）。wraparound（checked+panic）
  と reentrant borrow の契約はレビュー記録に残す。

## Acceptance Criteria

- [ ] `command_index` の保持レコードに settle/timeout/epoch 終了に
  整合した上限・解放条件があり、長時間・大量 command で bounded
  である回帰 test がある。
- [ ] `CommandBinding::outbox` が消費・drain または上限管理され、
  未 drain でも無制限でないことが検証されている。
- [ ] 512 件容量超過時に配送されない `Intent` と `PendingRecord`
  の整合・欠落と timeout が test で明示されている。
- [ ] mount/attach epoch を跨ぐ古い projection が新 binding に
  誤適用されない correlation 契約が定義され、順序逆転・remount
  が検証されている。
- [ ] 同一 `control_id` の複数 component の禁止/共有意味論が定義
  され、mount/unmount 登録・detach attribution が検証され、
  wraparound/reentrant borrow 契約がレビュー記録に残っている。
- [ ] 本 slice の文書チェーン（spec/design/plan/task/test-spec/
  impl record）が揃っている。

## Deferred Design Concerns

- projection envelope への attach/mount epoch segment 追加:
  wire-protocol 変更を伴うため本 slice では扱わない。consumer
  契約の合意が前提。再着手条件: envelope 契約 Issue の起票・
  合意。無視した場合のリスク: host 間誤配送・revision 単調性違反
  など floor が検証できない経路で古い projection が誤適用され得る
  （delivery path の attach_epoch pin 留保）。
- `PendingRecord` の dead-generation zombie 整理: slice 1 からの
  持ち越し。per-binding bound の内側に限定される。再着手条件:
  実害の観測または ledger 見直し Issue。
- `GpuiSurfaceAdapter` reentrant-borrow nit: 別領域（rf388-s3）。
  本 slice では契約のレビュー記録のみ行う。

## Sources and Relations

- Issue #410 本文（PR #406 独立レビュー RF387 の non-blocking
  hardening cluster 指摘を集約）。本 slice はその受入条件の残行を
  対象とする。
- PR #502 / slice 1 文書チェーン
  （`docs/specs|designs|plans|tasks|test-specs|impl/ir/
  issue-410-view-command-ledger-*.md`）— 実装済み契約の正本。
- `docs/specs/issue-387-component-view-specs.md`（Component/View
  contract の親仕様）。
- `docs/specs/issue-414-projection-controlled-display.md`
  （projection → controlled display の先行 slice）。

---
id: "IMPL-034Uwia1yemB1uLcIVLdNU"
type: "impl"
status: "completed"
title: "Issue #410 P2 view ledger correlation hardening — projection revision floor"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md"
      - type: "file"
        path: "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
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
    - "docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
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

# Issue #410 P2 view ledger correlation hardening — projection revision floor

## Summary

Issue #410 の第 2 スライス。slice 1（PR #502）で実装済みの
bounded retention・outbox 上限・delivery-loss 退役・owner dedup・
checked counter が Issue 受入条件 5 行を満たすことを対応 test で
検証・固定し、未対応だった projection correlation に
**revision floor** を実装した。`ViewHost` が `control_id` ごとの
受理済み projection revision の高水位を attach epoch の存続期間中
保持し、`unmount` → `mount` を跨いでも維持するため、remount 後の
fresh binding に古い revision の projection が誤適用される経路が
閉じた。

## Context

`apply_projection` は `control_id` のみで binding に相関し、
revision 順序は `CommandBinding::on_projection` の
`last_observed` 比較だけが guard していた。`unmount` は
`bindings.clear()` で binding を破棄するため、remount 後の fresh
binding は任意の最初の revision を受理し得た（次の新しい
projection で自己修復するが、一度の誤適用は controlled display
に stale 値を書き込み得る）。envelope への epoch segment 追加は
wire-protocol 判断として deferred のまま、host 側で可能な
correlation 強化として floor を選択した。文書チェーンは
spec → design → plan → task → test-spec を `docs/` 配下に作成済み。

## Implementation

### Changes

- `crate::behavior::revision_advances`（`pub(crate)`）を新設し、
  `Revision` の advance 判定（`Num` は厳密増加・`Str` は相違・
  型跨ぎは常に advance）を一箇所に集約。`CommandBinding::
  on_projection` の in-binding guard をこの helper に置き換え、
  host-level floor と同一規則を共有する（DRY）。
- `ViewHost::projection_floors: HashMap<ControlId, Revision>` を
  新設（private）。`apply_projection` で floor を advance しない
  projection を `false` で drop し、受理時のみ floor を更新する
  （棄却で巻き戻らない）。`unmount`/`detach`/`update` では clear
  しない — floor の終端は `ViewHost` の `Drop`（attach epoch の
  終了）と一致する。
- `apply_projection` の doc comment に correlation 契約を記述:
  revision は `control_id` ごとに attach epoch 存続期間で単調、
  `apply_projection`/`deliver_event` が canonical entry point、
  host 間の順序・単調性自体は delivery path の義務（envelope
  epoch segment は deferred）。
- `unmount` のコメントを更新し、floor が意図的に存続することを
  明示。
- 回帰 unit test を `view.rs` に追加:
  `projection_floor_rejects_stale_revision_across_remount`、
  `projection_floor_survives_control_absent_remount`、
  `projection_orders_across_detach_attach`、
  `projection_floors_are_independent_per_control`、
  `deliver_event_drops_stale_projection_after_remount`。

### Decisions During Implementation

- floor は binding ではなく host レベルに置いた。binding は
  `unmount` で破棄されるため、epoch を跨ぐ状態の所有者は host
  が正しい境界。`binding_mut()` 経由の直接 `on_projection` は
  floor を迂回するが、receipt 側の `command_index` 迂回と同型の
  上級 API 境界として契約化した。
- floor は受理時のみ更新する。detached 中・binding 不在・
  stale 棄却の projection は floor を動かさない（高水位を
  巻き戻さない・先取りしない）。
- `StateProjection`/`Revision` の型は不変。`ruviform-ui-test` が
  struct literal で構築するため field 追加は本 slice の編集範囲
  （`crates/ruviform-ui` のみ）では不可能であり、調査のうえ
  host-side floor を選択した。
- revision advance 規則は binding guard と共有 helper 化し、
  floor が独自の順序規則を持たないようにした。`Str` revision の
  unordered 性（相違=advance）は既存近似を継承し、契約として
  文書化した。

## Related Experiments

### Adopted

<!-- 実験ログは未作成。アプローチは既存 guard（`last_observed`
revision 比較）の host-level への拡張であり、探索的実装では
なかった。 -->

### Rejected

<!-- 同上。envelope epoch segment は wire-protocol 判断として
spec の Deferred Design Concerns に記録。 -->

## Validation

`mise` タスクで実施（Windows、rust toolchain は repo pin）—
全て exit 0:

- `mise run fmt-check`
- `mise run check`
- `mise run lint`
- `mise run test`（`cargo test --workspace`。ruviform-ui unit
  99 tests を含む全 crate pass）
- `mise run test-rt`
- `mise run test-ui-component-view`（17 tests）
- `mise run test-ui-driver-contract`（7 tests）
- `mise run test-ui-parity`（12 tests）
- `mise run test-ui-native-slot`（13 tests）
- `mise run test-ui-golden`（8 tests）
- `mise run test-ui-scenario-contract`（fake 16 + gpui 7 tests）
- `mise run test-ui-gpui`（gpui-test feature。unit 69 +
  consumer_observation 20 + lab_story 4 + scenario 7 等）
- `mise run test-ui-consumer-observation`（20 tests）
- `mise run test-ui-test-msrv`（rust@1.89.0 で ruviform-ui-test）
- `mise run test-ui-webview`（10 tests）

NotRun: `test-ui-binding` / `test-ui-components` /
`test-ui-headless` / `test-ui-showcase` / `test-ui-viz` /
`test-ui-guitest` / `test-ui-browser` — いずれも `ui/*` の
TypeScript package 向け lane であり、本 slice が変更した
`crates/ruviform-ui`（Rust crate）への依存を持たない。
Unsupported: 実 GPUI/OS window の手動検証 lane
（`example-ui-gpui-lifecycle` 等）— headless/fake で検証する
slice 契約どおり。

補助証拠（canonical = mise）: `cargo test -p ruviform-ui --lib`
で新規回帰テストを個別確認。

## Review Record

review-protocol（subagent 無し・分離 pass で実施）:

- Reviewer A（仕様整合）: spec/design/plan/task/test-spec の
  関係・用語・受入条件対応を照合 — blocker 無し。
- Reviewer B（敵対的）: floor の更新タイミング（受理時のみ）、
  `unregister_control`/`release` が `bindings` を残すこと、
  `deliver_event` → `apply_projection` の canonical 経路、
  `attach_epoch` 境界の扱いをコードで追跡。
- Issue Candidate → 判定: test-spec Rule「floor は control_id
  ごとに独立」に対応 test が無い → CONFIRMED P1 →
  `projection_floors_are_independent_per_control` を追加して
  解消。`update` 経路は binding が存続するため floor の
  clear/非 clear は観測不能（P3・記録のみ）。
- 結論: CONFIRMED P0/P1 の残存なし → ACCEPT。

## Risks

- consumer が revision domain をリセットすると floor が全
  projection を reject し display が凍結する。binding 内 guard
  と同じ失敗型であり新規リスクではないが、契約として
  「revision は attach epoch 内で単調」を spec/design/test-spec
  に明記した。revision リセットが必要な consumer は attach
  epoch 境界（新しい `ViewHost`）で行う。
- floor map は `control_id` 数に比例するが、spec 規模と同程度
  で有界。`unmount` 後に再登場しない control の floor も host
  生存中は残る（小さい `Revision` 値のみ）。
- envelope epoch segment（wire-protocol）は deferred のまま。
  floor は防御的多層の一段として残せる。

## Follow-ups

- projection envelope への attach/mount epoch segment 追加
  （wire-protocol 判断・別スライス/契約 Issue）。
- dead-generation `PendingRecord` zombie cleanup、GPUI
  reentrant-borrow nit は対象外として別 dispatch/Issue。
- Issue #464（GUI scenario/receipt verification）は別タスク。

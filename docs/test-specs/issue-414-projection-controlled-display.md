---
id: "TSPEC-bN4yZaDfS6xOvJpM8qTu5H"
type: "test-spec"
status: "approved"
title: "Issue #414 — projection→controlled display の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/414"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-414-projection-controlled-display.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-387-component-renderer.md"
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-414-projection-controlled-display.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-414-projection-controlled-display.md"
    - "docs/designs/issue-414-projection-controlled-display.md"
  verified-by: []
  references:
    - "docs/test-specs/issue-387-component-renderer.md"
  defers: []
  deferred-by: []
---

# Issue #414 — projection→controlled display の検証契約

## Feature: authoritative projection → controlled display

`ViewHost::apply_projection` が受理した `StateProjection` は、
binding の `observed` 記録に加えて `InteractionControl` の
controlled display（`snapshot.display` = renderer が読む値）を
更新しなければならない。

### Rule 1: idle 中の非ゼロ外部更新は display を更新する

- **Given** idle な control（gesture なし）
- **When** revision が進む projection が `apply_projection` に届く
- **Then** `observe().display` / `committed` が payload の値に
  更新され、`observed` も記録され、phase は `Observed`。
  event/intent は一切 mint されない（silent controlled write）。

検証：`view::tests::projection_updates_controlled_display_when_idle`、
`ui_dvr_09_projection_updates_controlled_display`、
`rvf_view_inbox_projection_updates_controlled_display`（GPUI）。

### Rule 2: gesture preview は authoritative write に優先する

- **Given** active gesture（Begin 済み、Preview で局所値を表示中）
- **When** projection が届く
- **Then** `observed` は記録されるが `display` は preview のまま。
  gesture 終了時（commit / cancel / detach / focus-loss）に staged
  truth が `committed_value` へ flush される。

検証：`view::tests::projection_defers_to_gesture_preview` /
`projection_staged_during_cancelled_gesture_lands`、
`behavior::tests::external_write_stages_during_gesture` /
`external_write_lands_after_cancel`、
`ui_dvr_09_projection_respects_gesture_preview`、
`rvf_view_inbox_projection_defers_to_active_gesture`（GPUI）。

### Rule 3: stale/revision/reattach の既存契約を壊さない

- 進まない revision の projection は `apply_projection` が false を
  返し display に触れない。
- detach 中の projection は吸収（false）、display 不変。
  reattach 後の advancing projection は通常通り display を更新。
- binding 生存の re-register（descriptor 変更 / remove+re-add）では
  `last_observed` が新 mapper 経由で display に復元される。

検証：`view::tests::stale_projection_leaves_display_untouched` /
`detached_binding_absorbs_projection_until_reattach` /
`reregister_restores_last_observed_display`、
`ui_dvr_09_projection_detach_reattach`。

### Rule 4: payload は無条件に f64 解釈しない

- decode は `projection_value_input` の明示 shape のみ：
  有限 number→Plain、`{"plain":n}`→Plain、`{"normalized":n}`→
  Normalized。それ以外は `None`。
- `None` の場合も binding は `observed` を記録するが display は
  不変。
- `Steps`/`Extreme` 等の相対・jump 入力は絶対 truth として拒否。
- range policy（clamp/reject）は user input と同一 mapper 経路。

検証：`behavior::tests::projection_value_input_shapes` /
`external_write_commits_when_idle` / `external_write_honors_range_policy`、
`view::tests::undecodable_projection_records_observed_only`、
`ui_dvr_09_projection_updates_controlled_display`（非 control 値
payload の項）。

### Rule 5: renderer が参照する値まで検証する

- `observed.is_some()` だけを証拠にしない。GPUI 側は
  `control_element` が読む `snapshot.display.normalized` そのものを
  inbox → `apply_projection` の実経路で assert する。
- 実 OS・実 window の検証は本 Issue の範囲外 —
  `real-os` layer は `not_run`（owner: Issue #313）として記録し、
  Pass とは記載しない。

検証：`rvf_view_inbox_projection_updates_controlled_display` /
`rvf_view_inbox_projection_defers_to_active_gesture`、
`driver-lab/cases.json` UI-DVR-09。

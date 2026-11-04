---
id: "IMPL-RrIh5XTCXgmjRS7m6giD6V"
type: "impl"
status: "completed"
title: "Issue #307: attach 時 frame_base 検証 context（accessors + typed mismatch 拒否）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/307"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/frame_base.rs"
      - type: "file"
        path: "docs/specs/issue-307-attach-frame-base-verified-context.md"
      - type: "file"
        path: "docs/designs/issue-307-attach-frame-base-verified-context.md"
      - type: "file"
        path: "docs/plans/issue-307-attach-frame-base-verified-context.md"
      - type: "file"
        path: "docs/test-specs/issue-307-attach-frame-base-verified-context.md"
      - type: "file"
        path: "docs/tasks/issue-307-t1-attach-frame-base-verified-context.md"
      - type: "file"
        path: "docs/impl/ir/issue-307-attach-frame-base-verified-context.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/attach.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/control.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/tasks/issue-307-t1-attach-frame-base-verified-context.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-307-attach-frame-base-verified-context.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-307-attach-frame-base-verified-context.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-307-attach-frame-base-verified-context.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #307: attach 時 frame_base 検証 context

## Summary

`AttachConfig.frame_base` は `PluginNode` の `TransportScheduler` にのみ届き、非 RT 側 `PluginNodeControl` から参照・検証する術がなかったため、Synvelis `ParamDispatch::new(control,node,frame_base)` のように外部供給 base と「AttachConfig と等値必須」を前提にするしかなかった。本実装は `attach()` が node/control 双方に同一の `cfg.frame_base` を供給する単一起源とし、control に `frame_base()` / `check_frame_base()` / `plan_builder()` を追加、`install_plan` の戻り値を `InstallPlanError`（`#[non_exhaustive]`・`QueueFull`/`FrameBaseMismatch`）に拡張して異値 plan を queue 介入前に型付き拒否する。比較は gcd 正規化 `TimeBase` の `PartialEq`（`2/96000 == 1/48000`）で、rate conversion・時刻 scale 変更は行わない。clock epoch・plan generation・transport position は独立 identity のまま維持され、RT callback への Mutex/alloc/blocking の追加はない。

## Context

- 実施タスク: `docs/tasks/issue-307-t1-attach-frame-base-verified-context.md`
- 上流: `docs/specs/issue-307-attach-frame-base-verified-context.md`（AC-1..6）、`docs/designs/issue-307-attach-frame-base-verified-context.md`（API 形態確定: `Attached` 形状を変えない `PluginNodeControl` アクセサ型を採用）、`docs/plans/issue-307-attach-frame-base-verified-context.md`、`docs/test-specs/issue-307-attach-frame-base-verified-context.md`
- 原因: frame base の所有が node（scheduler）側に限定され、control は `PluginNodeControl::new(ctrl_tx, report_rx, status)` で base 非保持。`install_plan` も `plan.frame_base` を scheduler base と照合せず stage していた（`TransportScheduler::install_plan` 側も未照合で、builder 経由の正しい構築に暗黙依存）。

## Implementation

### Changes

- `crates/ruviform-audio-plugin-bridge/src/control.rs`
  - `PluginNodeControl` に `frame_base: TimeBase` フィールドを追加し、crate-private ctor を `new(tx, rx, status, frame_base)` に拡張。
  - `frame_base() -> TimeBase`（`const fn`・`#[must_use]`）— attach 時点で固定・不変の verified context。rustdoc に epoch/position/generation との非混同を明記。
  - `check_frame_base(base) -> Result<(), FrameBaseMismatch>` — 実質同値（正規化 `PartialEq`）で照合し、`expected`/`actual` payload を持つ型付き拒否を返す。consumer ctor が `?` で propagate する用途を想定。
  - `plan_builder(generation) -> TransportPlanBuilder<EventKind>` — verified base に束縛した builder を mint する正規構築経路。
  - `install_plan` 戻り値を `Result<(), QueueFull>` → `Result<(), InstallPlanError>` に変更。`check_frame_base` を `try_push` 前に実行し、異値 plan は queue 介入・`control_overflows` 増分・plan/node 状態変更なしに拒否。
  - 新型 `FrameBaseMismatch`（struct・`expected`/`actual` pub・`Display`/`Error`）と `InstallPlanError`（`#[non_exhaustive]` enum・`QueueFull(QueueFull)`/`FrameBaseMismatch(FrameBaseMismatch)`）。
- `crates/ruviform-audio-plugin-bridge/src/attach.rs` — `PluginNodeControl::new` に `cfg.frame_base` を供給し、node/control 単一起源 invariant を comment で明示。
- `crates/ruviform-audio-plugin-bridge/src/node.rs` — `PluginNode::frame_base() -> TimeBase`（`const fn`）を `self.scheduler.frame_base()` 委譲で追加。RT-suitable（read-only field access）。
- `crates/ruviform-audio-plugin-bridge/src/lib.rs` — `FrameBaseMismatch`・`InstallPlanError` を re-export。
- `crates/ruviform-audio-plugin-bridge/tests/frame_base.rs` — 新規 6 テスト（test-spec oracle 実装）: accessor 往復一致・正規化同値受理・check ok/err payload・異値 install 拒否の非破壊性（`control_overflows` 不変・report なし・正 plan はその後正常 install/配送）・`plan_builder` 束縛 plan の絶対 frame 配送（min/max_frames=64/128 rig で call 分割も検証）・`device_restart` が epoch を rebase しても `frame_base()` 不変・queue-full が `InstallPlanError::QueueFull` で型維持。

### Contract amendment — observable behavior changes (migration note)

| Path | Before | After |
| --- | --- | --- |
| `PluginNodeControl::install_plan` 戻り値 | `Result<(), QueueFull>` | `Result<(), InstallPlanError>`（`#[non_exhaustive]`・`QueueFull` variant で従来意味を保持） |
| foreign-base plan の install | 照合なしで stage（tick 解釈ずれのまま配送され得た） | queue 介入前に `FrameBaseMismatch{expected,actual}` で拒否・非破壊 |
| `PluginNodeControl`/`PluginNode` | frame base 取得手段なし | `frame_base()` アクセサ（additive）・`check_frame_base`・`plan_builder` |

signature 変更は `install_plan` のみで、既存呼出は全て `.unwrap()` 経由のため source-compatible（crate 内 callers: `tests/e2e.rs`・`tests/midi_wiring.rs`・`tests/param_delivery.rs` — 全 pass）。error 型を名指しする downstream は `QueueFull` → `InstallPlanError::QueueFull` への机械移行が必要（本 repo に該当なし）。

### Decisions During Implementation

- **`PluginNodeControl::frame_base()` アクセサ型を採用**（棄却: `Attached` へ `AttachedClockContext` 追加）— `Attached` の形状・field を変えず後方互換を最大化しつつ、control が verified context の唯一の非 RT 出口になる。design 記録どおり。
- **拒否は queue 介入前**: `try_push` 前に base 照合することで、拒否が overflow counter・report・installed plan に副作用を持たないことを保証（非破壊性 AC を構造で満たす）。
- **`plan_builder` を提供**: 「拒否」だけでなく「正しく構築する経路」を control 自身が mint し、consumer が外部 base を手作りする動機を除去。
- **`TransportScheduler::install_plan`（media-core/plugin-core 側）は変更しない**: 下位層の generic scheduler は base を contract ではなく builder 入力として受ける設計であり、検証責務は bridge の所有境界（control）に置く。scheduler 側に二重検証を入れると同一規則の重複実装になる。

## Verification

- OS: Windows 11 x64（Git Bash） / toolchain: `cargo 1.99.0`（mise pin・MSRV 1.89 準拠コード）
- `mise run fmt-check` — exit 0
- `mise run check` — exit 0（workspace・all-targets）
- `mise run lint` — exit 0（clippy `-D warnings`・全 workspace）
- `mise run test` — exit 0（workspace 全量・116 test groups・0 failed。`tests/frame_base.rs` 6/6 pass）
- `mise run test-rt` — exit 0（`rt_` フィルタ lane・0 failed。bridge 既存 RT no-alloc テスト含む）
- doc lint（`doc_status.js lint`・全 docs 走査）— issue-307 関連 finding 0 件（index membership/navigation/relation 全て解消）

NotRun（未実測・明記）: 実 plugin/実機 OS 上での精度・性能計測、consumer 実 repo でのビルド（#219/#294 の cross-consumer lane は coordinator/PR 側で実施判断）。本変更は非 RT control 側の accessor + 事前検証のみで RT callback 経路に新規 alloc/Mutex/blocking を導入しないため、RT no-alloc 非回帰は `test-rt` lane 通過で担保。

## Consumer Impact Materials

- **synvelis**: **impacted（positive・対応は別 Issue #101）** — `ParamDispatch::new(control,node,frame_base)` が受け取る外部 `frame_base` に対し、`control.check_frame_base(frame_base)?` で bridge 起源と型付き照合する道ができた。`control.plan_builder(gen)` / `control.frame_base()` を使えば外部 base 供給自体も除去可能。API 追加のみで破壊的変更は `install_plan` 戻り値型のみ（synvelis 側が `QueueFull` を名指ししている場合は variant 経由へ機械移行）。
- **audio-lens**: **no impact** — 変更は `ruviform-audio-plugin-bridge` に限定。issue-219 IR 記録の consumer inventory では audio-lens は `ruviform-ui`/`ruviform-ui-webview`/`ruviform-viz` 系の利用で plugin-bridge に依存しない。#219/#294 lane での実ビルド確認は未実施（NotRun）。

## Related Experiments

### Adopted

None.

### Rejected

None.

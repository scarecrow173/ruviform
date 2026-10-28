---
id: "PLAN-034NXq4i4gZXaqNEEwBZpL"
type: "plan"
status: "approved"
title: "Issue #118 実施計画 — video/NONRT プラグインの L1 経路（wire v3・vshm・video runner・video() façade）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-118-wire-v3-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-118-video-failed-parity-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-118-video-e2e-tests.md"
      - type: "file"
        path: "docs/tasks/l1-wire-v3-video-codec.md"
      - type: "file"
        path: "docs/tasks/l1-video-runner-and-erased-surface.md"
      - type: "file"
        path: "docs/tasks/l1-video-host-surface.md"
      - type: "file"
        path: "docs/tasks/l1-video-fixture-and-e2e.md"
      - type: "file"
        path: "docs/tasks/l1-video-docs-sweep.md"
      - type: "file"
        path: "docs/tasks/l1-video-l1-final-verification.md"
      - type: "file"
        path: "docs/impl/ir/issue-118-video-l1-implementation.md"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/video_plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/vshm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-video-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_video_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/vshm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plain_param.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-export/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
      - type: "file"
        path: "docs/specs/issue-118-video-l1.md"
      - type: "file"
        path: "docs/designs/issue-118-video-l1.md"
      - type: "file"
        path: "docs/designs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/designs/engine-plugin-host-integration.md"
      - type: "file"
        path: "docs/specs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/engine-plugin-host-integration.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-wire-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-proxy-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-runner-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/real-adapter-l1-e2e-tests.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-118-video-l1.md"
  implemented-by:
    - "docs/tasks/l1-wire-v3-video-codec.md"
    - "docs/tasks/l1-video-runner-and-erased-surface.md"
    - "docs/tasks/l1-video-host-surface.md"
    - "docs/tasks/l1-video-fixture-and-e2e.md"
    - "docs/tasks/l1-video-docs-sweep.md"
    - "docs/tasks/l1-video-l1-final-verification.md"
  depends-on:
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
    - "docs/adr/ofx-video-plugin-scope.md"
    - "AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
    - "docs/discovery/issue-118-video-l1.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  derived-by:
    - "docs/tasks/l1-wire-v3-video-codec.md"
    - "docs/tasks/l1-video-runner-and-erased-surface.md"
    - "docs/tasks/l1-video-host-surface.md"
    - "docs/tasks/l1-video-fixture-and-e2e.md"
    - "docs/tasks/l1-video-docs-sweep.md"
    - "docs/tasks/l1-video-l1-final-verification.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-118-wire-v3-codec-tests.md"
    - "docs/test-specs/issue-118-video-failed-parity-tests.md"
    - "docs/test-specs/issue-118-video-e2e-tests.md"
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #118 実施計画 — video/NONRT プラグインの L1 経路（wire v3・vshm・video runner・video() façade） Implementation Plan

> status: approved — Refs #118。approved design（`docs/designs/issue-118-video-l1.md`）+ 2 本の accepted ADR（`issue-118-video-l1-host-surface`・`issue-118-video-l1-create-path-segment`）を正本として、`VideoPluginInstance` を L1 子プロセスで spawn・lifecycle・render できるようにする 6-task DAG。GitHub merge は lead の authoritative `gh` のみ — 本計画は PR merged / issue closed の主張を含まない。

## Goal

`WIRE_VERSION` 2→3 bump の内側に video/NONRT データ面を確立する — wire v3 codec（動詞 tag 12-22・Reply tag 4-7・`VideoProcessConfig`/`VideoFrameDesc`/`ColorInfo`/`PlainParamValue`/`PlainParamInfoOwned` codec・`WIRE_MAX_VIDEO_*`）・per-instance `vshm` segment（header 28 B + in/out 各 1 領域・canonical packing）・`erase_video`/`run_video` の runner 側 video 表面・`PluginHost::video() -> VideoPluginHost<'_>` 借用 façade（`Route::OutOfProcessVideo`・Dedicated 固定・`render_timeout` 60 s）を実装し、#117 確定の `Failed` 粒度・通知・inert 規則を video 経路に verbatim 写像する。

## Non-Goals

- GPU/texture interop・OFX GPU suites・真の zero-copy shm→frame backing（`Arc<[u8]>` 制約 — v1 は copy-in/copy-out）
- RT 契約全般（video は全操作 NONRT — `rt_` テスト・`assert_no_alloc`・`prepare_endpoint`/RT leg・pass-through 語彙は video に適用しない）
- OFX 実バイナリの L1 E2E・video real-adapter opt-in lane — **video の real-plugin lane は v1 に存在しない**（`mise run test-l1-real-plugin`/`lint-l1-real-plugin` は CLAP/VST3 audio のみを対象とし維持。OFX L1 runner 組込み・規格 fixture E2E は後続 Issue — spec Out of Scope と同じ disposition）
- async render・pool depth>1・slot ring・format 再交渉・Bundled video（構造的に表現不能 — `VideoOutOfProcessSpec` に `isolation` 字段を持たない pin）
- video supervisor/`RestartRecipe` 変種（#116 util 層は audio 形 — spec/design deferred 表どおり）
- L1 GUI・engine 側 video port 接続・複数 wire version 共存・audio `process` 経路/`SLOT_BYTES`/doorbell 意味論の変更
- 歴史 record（#76 spec/design・旧 plan/task/test-spec・task-4 IR の当時記述）の書き換え — #117 と同じ grep 例外ポリシー（historical 文書は現行契約の正本ではない）

## Scope and Split Boundary

単一 plan・6 task。分割軸は「crate 境界 × 検証可能性」で、#117 と同じく wire → 子側 → host 側 → E2E → docs → gate の流れ:

- **wire v3 codec は基盤（T1）** — 新動詞・Reply・video 型 codec・`InvalidValue`・`WIRE_MAX_VIDEO_*`・`wire::vshm` 定盤は全下流の前提。ただし #117 T1 と違い **additive bump**（既存 tag・envelope・audio shm は無変更）なので atomic cutover ではない — plugin-core 単独で完結し、既存 suite は `WIRE_VERSION` 定数の両側使用で green に留まる（`Hello.version`/shm header の version 比較は定数参照のため bump 自体は非破壊）
- **runner 側（T2）と host 側（T3）は並行可能** — plugin-export（`run_video`/`VideoEntry`/attach 検証）と plugin-host（`video()` façade/`Slot`/`vshm::Segment`/`Creating` ゲート）は別 crate・別ファイル集合。共有するのは T1 の wire 契約のみ。T2 が plugin-core に触れるのは新規 `video_plugin.rs` のみで T3 のファイルと非交差
- **fixture + 公開面 E2E（T4）は T2+T3 の join** — `l1-test-video-worker`（hand-rolled・audio fixture 無変更）+ `tests/l1_video_e2e.rs`（`PluginHost::video()` 公開 API のみ — #116 `l1_supervisor_e2e` 先例どおり `#[path]` harness なしの別 target）
- **docs sweep（T5）は T4 と並行可**（doc file のみ・code file と非交差）、**final gate + closeout（T6）** が収束点

## Files / Ownership

| task | owner crate | primary files | scope |
| --- | --- | --- | --- |
| T1 `l1-wire-v3-video-codec` | plugin-core | `wire.rs`, `boundary.rs`, `plain_param.rs`, `lib.rs` | `WIRE_VERSION`=3・動詞 tag 12-22・Reply tag 4-7・video 型 codec・`WIRE_MAX_VIDEO_*`・`wire::vshm` 定盤・`HostError::InvalidValue` tag 10・`PlainParamInfoOwned` |
| T2 `l1-video-runner-and-erased-surface` | plugin-export + plugin-core(1 file) | `video_plugin.rs`(新), `export/src/lib.rs`, `export/src/vshm.rs`(新), `export/src/bin/ruviform-plugin-l1-video-runner.rs`(新), `export/tests/l1_video_runner_e2e.rs`(新), `export/Cargo.toml` | `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video` + encode obligations (b)(c)(d)・`run_video` + `VideoRunner`/`VideoEntry` + attach 2 層検証・stash-replay initialize・param id 先行検査・`BlockIn` 静かに drop・reference bin + MiniHost E2E |
| T3 `l1-video-host-surface` | plugin-host | `src/video.rs`(新), `composite.rs`, `lib.rs`, `l1/mod.rs`, `l1/vshm.rs`(新), `l1/video.rs`(新), `Cargo.toml` | `VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost<'a>`・`Route::OutOfProcessVideo`/`VideoSpawnSpec`・`Slot` 12 match-site arm・`descriptors` kind-tag・`InstKind` 分離・`create_video_instance`・**`Creating` ゲート拡張**・`request_timed`・`vshm::Segment`・`VideoL1Instance` |
| T4 `l1-video-fixture-and-e2e` | plugin-host(bin/tests) | `src/bin/l1-test-video-worker.rs`(新), `tests/l1_video_e2e.rs`(新), `Cargo.toml` | hand-rolled video worker + fault flags・公開 API のみの video E2E（worker + real video runner 両 lane）・`Failed` parity E2E |
| T5 `l1-video-docs-sweep` | docs/metadata | crate README×3・親文書 addenda×4・test-spec reconcile×4・bin doc | README `run_video`/`video()` 追記・deferred 行 re-engagement 注記・v2 記述の v3 reconcile |
| T6 `l1-video-l1-final-verification` | 全列 | workspace 全体・`docs/impl/ir/` | Full Gate・AC1-AC9/R-V1-V6 証拠・IR・status 同期 |

cross-crate 依存の形: T1 は plugin-core 単独。T2/T3 は T1 のみに依存し互いに非交差（T2 は plugin-core の新規 1 file + plugin-export、T3 は plugin-host のみ — 共有 file ゼロ）。T4 は T2（real video runner bin）と T3（façade + worker 用 wire）の join。T5 は T2-T4 の事実確定後なら T4 と並行可能。T6 が全収束点。

## Ordering Rationale

1. **wire v3（plugin-core）が基盤** — 動詞・Reply・video 型・bound・`vshm` layout 定盤は T2（runner が decode・attach）も T3（host が encode・segment create）も参照する唯一の正本。`InvalidValue` は T2（runner `id ∈ parameters()` 拒否）と T3（façade pre-validate）の双方が使う
2. **T2 ∥ T3** — crate と file が非交差。`run_video` の意味論は wire 契約と `erase_video` に閉じ、`video()` façade の意味論は wire 契約と `L1Child`/`Inner` 共有機構に閉じる。統合面でのみ出会う
3. **T4 は join 点** — E2E は façade→wire→worker/runner の全系統を通すため両側の完成が前提。worker bin は hand-rolled で T2 の `run_video` を使わないが、E2E の real-runner lane（`ruviform-plugin-l1-video-runner` 経由の `PluginHost::video()` 駆動 — `l1_proxy_e2e` が worker と real runner の両 lane を持つ先例どおり）は T2 の bin が要る
4. **T5 ∥ T4 → T6** — docs は実装着地後、gate は最後

crate 別並行性: T2 と T3 は完全並行可能（merge 前 branch 上でも commit が独立して buildable — plugin-core への additive 追加のみが T1 で先行確定済みのため）。T4 内の worker 実装と E2E 記述は同 file 集合なので直列。

## Task DAG

```
T1 l1-wire-v3-video-codec                    (plugin-core — AC1/R-V1/R-V6 codec 層)
  ├─ T2 l1-video-runner-and-erased-surface   (plugin-export — AC6/AC7/R-B7/R-V2 子側/R-V6 attach 層)
  └─ T3 l1-video-host-surface                (plugin-host — AC2/AC5/R-F5 host 面/encode obligation (a)/Creating ゲート)
       └─ T4 l1-video-fixture-and-e2e        (join — AC2-AC6/R-V2/R-V3/R-V5 E2E)
            └─ T5 l1-video-docs-sweep        (docs/metadata — T2-T4 以後・T4 と並行可)
                 └─ T6 l1-video-l1-final-verification  (AC8/AC9 + 全 AC 収束)
```

| task | one-line scope |
| --- | --- |
| T1 `l1-wire-v3-video-codec` | `WIRE_VERSION`→3・`Command` tag 12-22（`CreateVideoInstance`/`VideoInitialize`/`VideoActivate`/`VideoStartProcessing`/`VideoRender`/`VideoStopProcessing`/`VideoDeactivate`/`VideoGetParams`/`VideoGetParamInfo`/`VideoGetParamValue`/`VideoSetParamValue`）・`Reply` tag 4-7（`Rendered`/`Params`/`ParamInfo`/`ParamValue`）・`VideoProcessConfig`/`PixelFormat`/`VideoFrameDesc`/`ColorInfo`/`PlainParamValue`/`PlainParamInfoOwned` codec・`WIRE_VIDEO_MAX_{DIM,PLANES,STRIDE,FRAME_BYTES,SEGMENT_BYTES}`・`wire::vshm` header 定盤・`HostError::InvalidValue`（tag 10）を additive に実装する |
| T2 `l1-video-runner-and-erased-surface` | `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video`（plugin-core `video_plugin.rs` 新設・encode obligations b/c/d の normalize 境界）・`run_video(factories)`/`VideoRunner`/`VideoEntry`・`vshm` attach 2 層検証・stash config → `initialize(&config)` 再生・param `id ∈ parameters()` 先行検査・`BlockIn` 静かに drop・`ruviform-plugin-l1-video-runner` reference bin + MiniHost runner E2E を実装する |
| T3 `l1-video-host-surface` | `video.rs` 新設（`VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost<'a>`/`descriptor` lookup）・`Route::OutOfProcessVideo`+`VideoSpawnSpec`・`Slot::OutOfProcessVideo` 12 match-site arm・`descriptors` kind-tag + Scan filter・`InstKind` 分離・`create_video_instance`・**`Creating` ゲート拡張（L1914-1915）**・`request_timed`/`render_timeout`・`l1/vshm.rs`・`l1/video.rs` `VideoL1Instance`（render path + façade pre-validate）を実装する |
| T4 `l1-video-fixture-and-e2e` | `l1-test-video-worker`（hand-rolled video worker + fault flags）と `tests/l1_video_e2e.rs`（`PluginHost::video()` 公開 API のみ・worker lane + real `ruviform-plugin-l1-video-runner` lane）で scan→create→lifecycle→render→param→destroy・frame 内容・`Failed` parity・cross-kind を E2E 検証する |
| T5 `l1-video-docs-sweep` | crate README×3・親文書 4 本の deferred 行 re-engagement 注記・test-spec×4 の v3 reconcile・残存 v2 literal sweep を更新する |
| T6 `l1-video-l1-final-verification` | Full Gate（fmt-check/check/lint/test/test-rt/bench-smoke/cargo doc/doc-audit + audio real-plugin lane 非退行）・AC1-9/R-V1-6 証拠収集・IR・status 同期で closeout する |

並行 group: `{T1}` → `{T2 ∥ T3}` → `{T4 ∥ T5}` → `{T6}`。T4 と T5 の並行は doc file と code file の非交差に依存 — T5 は T2/T3 の実装事実が確定した時点で開始可能（T4 の E2E 結果を待たず README/親文書を書ける。test-spec reconcile は T4 green を待たない）。

## Test Oracles

| test-spec | role |
| --- | --- |
| `issue-118-wire-v3-codec-tests.md`（新規） | v3 増分 oracle: 動詞 tag 12-22・Reply 4-7・video 型 codec round-trip/拒否・`WIRE_MAX_VIDEO_*`・presence strictness・`InvalidValue` tag 10・`Hello.capabilities` reserved-zero 維持・`vshm` header/byte math |
| `issue-118-video-failed-parity-tests.md`（新規） | `Failed` 粒度 oracle: 子スコープ terminal・帰属可能 per-instance（`Rendered.output` semantic 違反）・inert（post-removal/`Destroying`/`Failed` 宛・runner 宛 inbound `BlockIn`）・wedge timeout・mid-verb 死亡の全分岐（#117 確定契約の video 写像） |
| `issue-118-video-e2e-tests.md`（新規） | 公開面 E2E oracle: `video()` façade の scan→create→lifecycle→render→param→destroy・frame 内容/pass-through・cross-kind 拒否・Dedicated 構造 pin・encode obligation (a)・`Creating` ゲートの E2E 観測 |
| `plugin-l1-wire-codec-tests.md`（v3 更新対象） | 既存 codec baseline oracle の v3 版（`WIRE_VERSION`=3・動詞表・Hello wording）— T5 で実装着地と verify/reconcile |
| `plugin-l1-proxy-contract-tests.md`（v3 更新対象） | handshake/spawn 契約の v3 追従（audio 専用の記述は無変更維持 — kind 記述を wire に足さない決定の裏付け確認）— T5 で verify/reconcile |
| `plugin-l1-runner-e2e-tests.md`（v3 更新対象） | audio runner 契約の v3 追従・video runner との非交差注記 — T5 で verify/reconcile |
| `real-adapter-l1-e2e-tests.md`（v3 更新対象） | audio real lane の v3 追従確認 — T5 で verify/reconcile |

## Verification Matrix（spec AC1–AC9 + R-B/R-V → task/test-spec）

| 契約 | task | 検証 |
| --- | --- | --- |
| AC1 video wire codec | T1 | wire unit test + `issue-118-wire-v3-codec-tests` |
| AC2 video lifecycle parity | T2/T3/T4 | runner dispatch 単体（T2）+ façade mirror unit（T3）+ `issue-118-video-e2e-tests` |
| AC3 frame 輸送 | T1/T2/T3/T4 | desc codec/`vshm` byte math（T1）+ attach 2 層検証（T2）+ segment/repack（T3）+ E2E frame 内容一致（T4） |
| AC4 `Failed` parity | T2/T3/T4 | `issue-118-video-failed-parity-tests`（全分岐 — runner normalize・demux・fan-out・inert・timeout・mid-verb） |
| AC5 isolation（Dedicated 構造 pin） | T3/T4 | `VideoOutOfProcessSpec` に `isolation` 字段がない型レベル pin（compile-time）+ `issue-118-video-e2e-tests` の route 分離観測 |
| AC6 param チャネル | T1/T2/T3/T4 | codec（T1）+ runner `id ∈ parameters()` 先行検査・encode obligations b/c（T2）+ façade pre-validate obligation (a)（T3）+ E2E param get/set（T4） |
| AC7 runner registry | T2 | `l1_video_runner_e2e`（MiniHost — `CreateVideoInstance` 検証 chain・`InvalidHandle`・dup・`InstanceLimit`・panic 正規化・cross-kind create `Err(InvalidHandle)`） |
| AC8 audio 無退行 | T1-T6 全 gate | 各 task の既存 suite green + T6 Full Gate |
| AC9 CI/監査 | T6 | `mise run` 系列 + `doc_status.js audit` |
| R-B1 lifecycle 動詞表現 | T1/T3 | codec 動詞表 + façade lifecycle gate（`Err(Lifecycle)`/`Err(Failed)`） |
| R-B2 descriptor+shm 分離 | T1/T2/T3 | `VideoFrameDesc` codec + `vshm` in/out 領域 + copy-in/copy-out |
| R-B3 同期 render・commit-on-success | T3/T4 | `render_timeout` request + `Err` 時 out 領域不読 + E2E `fail-render` |
| R-B4 bounded decode/attach | T1/T2/T3 | codec bounds + attach 2 層 + host early-check（R-V6 と同証拠） |
| R-B5 `Failed` 粒度適用 | T2/T3/T4 | AC4 と同証拠 |
| R-B6 audio 意味論無変更 | T1-T6 | audio codec/slot/doorbell/runner/facade の diff ゼロ + suite green（AC8 と同証拠） |
| R-B7 registry 同居・panic 正規化 | T2 | `erase_video` adapter + `VideoRunner` dispatch + panic fixture E2E |
| R-B8 plain param 動詞 | T1/T2/T4 | codec + runner dispatch + E2E |
| R-V1 codec round-trip/拒否 | T1 | `issue-118-wire-v3-codec-tests` |
| R-V2 video E2E | T4 (+T2 runner E2E) | `issue-118-video-e2e-tests` + `l1_video_runner_e2e` |
| R-V3 `Failed` parity | T2/T3/T4 | `issue-118-video-failed-parity-tests` |
| R-V4 audio 無退行 | T1-T6 | AC8 と同証拠 |
| R-V5 isolation 挙動 | T3/T4 | AC5 と同証拠（「非対応の形で pin」側で満たす — `isolation` 字段不存在） |
| R-V6 bounds hostile decode/attach | T1/T2/T3 | codec 拒否行列（T1）+ attach 拒否分岐（T2）+ `Instantiate` early 拒否（T3） |

## Risks / Readiness

| risk | 対処 |
| --- | --- |
| `Creating` ゲート拡漏れ（`l1/mod.rs` L1914-1915 が `CreateVideoInstance` を受理せず create が恒に `Err(Ipc)`） | T3 の必須 touch-point として task doc に明記 + T4 E2E で scan→create 成功を観測（design の Verification Notes 行どおり） |
| `try_recv_notification` の video arm 漏れ（`let Some(Slot::OutOfProcess(_))` が video slot を静かに skip し `InstanceFailed` が永遠に失われる） | 12-site 表を T3 task doc に複写・各 site の確定値を design 表と 1:1 対応付け + T4 E2E で video `InstanceFailed` fan-out を観測 |
| `Rendered.output` semantic 違反の帰属誤り（codec 層に region-fit を置くと帰属不能→子スコープ殺しに化ける / host 側検証漏れで違反 desc を受理） | codec=bounds 層・post-decode semantic=受け手分岐の責務分離を T1/T2/T3 task doc に明記。非 canonical desc fixture flag（`--noncanonical-output`）で帰属可能 per-instance `Failed` を E2E pin |
| encode obligations 未実装で「契約適合だが wire 非表現」な値が decode 拒否→子死（非有限 `Double`・over-limit `ids`/`choices`・非有限 `display_*`） | obligation (a) は T3 façade pre-validate、(b)(c)(d) は T2 `erase_video` adapter/attach 境界に集約 — task doc に (a)-(d) の配置表を明記 + worker/runner fixture で各 obligation の defined `Err` を pin |
| `descriptors` 無タグのまま video 登録が audio `Scan` に漏れる | T3 で entry kind-tag + Scan filter を実装し、audio/video 各 `Scan` の列挙分離を unit + E2E pin |
| `InstShared` の audio data-plane 字段を video entry が誤用（`staging`/`result_tx`/`shm` が `InstKind` 未分離だと None チェックの散在 or 誤 arm） | `InstKind{Audio{shm,staging,result_tx}, Video{config}}` の分離を T3 で構造義務化 — video entry は audio data-plane 字段を持たないことを型で pin |
| `render_timeout` と `command_timeout` の取り違え（全動詞が `command_timeout` のまま or render も固定 timeout） | `request_timed` 一般化 + `VideoRender` のみ `render_timeout` を T3 に明記、`--hang-after-msgs` worker flag で timeout 分岐を T4 E2E pin |
| `l1-test-video-worker` の flag/dispatch mirror ずれ（audio fixture との二重保守で wire 意味論が drift） | worker は `wire` crate の codec をそのまま使い registry/dispatch のみ hand-roll — audio fixture 無変更が R-B6 の構造的保証。flag 語彙は audio と同族（crash/hang/stop-reading/leave-heir/hello-version）+ video 固有（fail-render/noncanonical-output/fail-initialize/fail-start-processing/fail-stop-processing/fail-deactivate）のみ |
| 512 MiB 級 segment の確保失敗・CI 環境の shm 上限 | host early-check（`frame_bytes`/`segment_len` cap → `Prepare(UnsupportedConfig)`）で上限を pin。E2E は小 frame（1080p 以下）を使い大 segment は byte math unit test で担保 |
| 既存 flake（`l1_proxy_e2e` 負荷依存）が video 列で再発 | 既知 flake は孤立 rerun で切り分け。video E2E は deterministic な fixture flag + notify polling（`l1_supervisor_e2e` の `NOTIFY_TIMEOUT` パターン）を踏襲 |

## Rollback Strategy

- **commit 単位**: T1 は additive codec commit（wire enum variant/定数の追加のみ — 既存 tag・envelope・audio shm に diff なし）。T2/T3 は crate 別に独立 revert 可能（plugin-export 群・plugin-host 群は file 非交差）。各 task の最後は `mise run check`/`lint`/`test`/`test-rt` green を必須とし、red なら同 task 内で完結する revert を選択
- **`WIRE_VERSION` 3 への rollback**: T1 単体 revert で `WIRE_VERSION` が 2 に戻れば全系統が v2 整合に復帰（video 動詞は未使用で死語彙 — host/runner はまだ発行しない）。T2+ 以後に codec 欠陥が見つかった場合の封じ込めは「video route 登録を公開しない」ではなく v3 codec 自体の修正（単一最新版方針 — version 選択経路は存在しない）
- **façade 無効化**: `PluginHost::video()` の借用 façade は additive — `register_out_of_process` 不可の場合でも audio 面は無影響。video route への登録自体を止める 1 箇所 revert（`video()` accessor）で公開面を閉じられる（構造的 pin — audio surface は video route を見ないため）
- **`vshm` layout 変更時**: segment は create/attach 双方が同じ `wire::vshm` 定盤を読むため、layout 修正は T1 内で完結（header field の解釈を変える場合は `wire_version` 一致検証が不一致 attach 拒否として fail-closed に効く）

## Implementation Sequence

- [ ] T1 `l1-wire-v3-video-codec` — plugin-core wire v3 additive codec + `vshm` 定盤 + `InvalidValue` + `PlainParamInfoOwned`
- [ ] T2 `l1-video-runner-and-erased-surface` — `erase_video` + `run_video` + video registry + reference bin + runner E2E（T3 と並行可）
- [ ] T3 `l1-video-host-surface` — `video()` façade + `Slot`/`Route` video arm + `Creating` ゲート + `vshm::Segment` + `VideoL1Instance`（T2 と並行可）
- [ ] T4 `l1-video-fixture-and-e2e` — `l1-test-video-worker` + `tests/l1_video_e2e.rs`（T5 と並行可）
- [ ] T5 `l1-video-docs-sweep` — README・親文書 addenda・test-spec reconcile
- [ ] T6 `l1-video-l1-final-verification` — Full Gate + IR + closeout

## Dependencies and Risks

- 前提: `docs/designs/issue-118-video-l1.md`（approved）・`docs/adr/issue-118-video-l1-host-surface.md`・`docs/adr/issue-118-video-l1-create-path-segment.md`（両 accepted）・`docs/specs/issue-119-l1-wire-version-policy.md`（wholesale bump・単一最新版）・`docs/adr/issue-117-bundle-child-failed-granularity.md`（`Failed` 契約の正本）
- crate dep edge（新規 — design 検証済み）: `ruviform-plugin-host → ruviform-video-core`（新規 `[dependencies]`）・`ruviform-plugin-host → ruviform-media-core`（dev-dep → `[dependencies]` 昇格 — `Cargo.toml` L36 確認済み・`ruviform-audio-core` の #113 昇格と同経路）・`ruviform-plugin-export → ruviform-video-core`（新規）・`ruviform-plugin-export → ruviform-media-core`（dev-dep → 昇格）。全て domain-foundations 層への下向き edge — transitive 増加なし・DAG 向き不変
- `unsafe` 新規導入は `l1/vshm.rs` と `export/vshm.rs` の `bytes`/`bytes_mut` accessor 2 箇所ずつ（audio `shm.rs` と同じ `#[expect(unsafe_code)]` + SAFETY コメントパターン）。`#[non_exhaustive]` は `VideoHostCommand`/`VideoHostReply`/`HostError::InvalidValue`（既存 enum への variant 追加 — enum 自体は既に `#[non_exhaustive]`）に必須
- 検証列は全 task 共通: `mise run fmt-check` → `check` → `lint` → `test` → `test-rt`。T6 で `bench-smoke` + `cargo doc -p {core,host,export} --no-deps` + doc audit + `lint-l1-real-plugin`/`test-l1-real-plugin`（audio lane 非退行のみ — video real lane は存在しない）を追加

## Task Breakdown

- `docs/tasks/l1-wire-v3-video-codec.md` — T1
- `docs/tasks/l1-video-runner-and-erased-surface.md` — T2
- `docs/tasks/l1-video-host-surface.md` — T3
- `docs/tasks/l1-video-fixture-and-e2e.md` — T4
- `docs/tasks/l1-video-docs-sweep.md` — T5
- `docs/tasks/l1-video-l1-final-verification.md` — T6

## Verification

- [ ] `mise run fmt-check` が差分なしで終了する
- [ ] `mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全て成功する（video 経路は `rt_` 対象外 — `test-rt` は audio 非退行の証拠としてのみ通る）
- [ ] `mise run bench-smoke`・`mise exec -- cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` が warning 0 で完了する
- [ ] `mise run lint-l1-real-plugin`・`mise run test-l1-real-plugin` が pass（audio real lane 非退行 — video real-adapter lane は v1 に存在しないことを IR に記録）
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す
- [ ] spec AC1–AC9・R-V1–R-V6 全行が検証 matrix の task/test-spec に対応付けられている
- [ ] `grep -rn "WIRE_VERSION" crates/` で `= 2` 残存が historical 注記以外に存在しない・`VideoOutOfProcessSpec` の定義に `isolation`/`lookahead_blocks` 字段がない（compile-time pin — ソースレビュー）
- [ ] `l1/mod.rs` の `InstPhase::Creating` ゲートが `CreateInstance` と `CreateVideoInstance` の両方を受理する（`matches!(cmd, wire::Command::CreateInstance { .. } | wire::Command::CreateVideoInstance { .. })` 相当）

## Open Questions / Gaps

- **`l1-test-video-worker` の descriptor/param fixture 形** — audio worker は単一 descriptor + normalized param を報告する。video worker の mock plugin（deterministic pixel transform・`dev.ruviform.test` 系 id・plain param 集合の形 — choices 持ち 1 件 + `Double` 1 件程度）は T4 で具体化する。E2E が必要とするのは (a) render 出力の pixel 一致 oracle、(b) pts/duration/color pass-through 観測、(c) `ParamInfo(None)`/`Err(InvalidValue)` 透過のための既知 id 集合 — これを満たす最小形
- **`PluginHost::descriptor(handle)` 新規 surface** — design は「video façade が提供するなら新規 surface」と条件付き。T3 で提供するかは implementation 判断（`descriptors` 登録表からの薄い lookup — 提供しない場合は `VideoPluginHost` の doc に「handle→descriptor lookup は v1 未提供」と明記）
- **`#[doc(hidden)] kill_child` の video handle への適用** — audio `kill_child` は `#[doc(hidden)]` verification knob。design の slot 表は「動く」を pin しているため video facade 経由でも同じ経路を使う（新規 knob は作らない）
- **test-spec の `verifies` 対象** — 本 plan では spec+design（+Failed parity spec は #117 ADR）に限定。ADR の `Confirmation`/`Verification` 行の一部は test-spec 経由でなく task `## Verification` の runnable command で担保する（compile-time pin 等、test-spec が intent を持たない機械確認）

## Self-Review

- [x] Every spec requirement maps to a task（AC1-9・R-B1-B8・R-V1-V6 → Verification Matrix 全行）
- [x] No placeholders remain（Open Questions は判断点を列挙 — 未知を隠さない）
- [x] File paths, relations, status values, and terminology are consistent（design の file map・動詞/tag・bound 名と一致）

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation — T2/T3・T4/T5 が独立 stream
- Independent work streams: T2 ∥ T3（crate/file 非交差）・T4 ∥ T5（code file と doc file 非交差）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 確定済み前提（再決定しない — design confirmed 行）: 輸送モデル（per-instance segment + descriptor-over-Command/Reply）・Dedicated 固定・別立て video 表面・copy-in/copy-out・`WIRE_VERSION` 2→3・`Failed` parity verbatim
- 判断点として残したもの: Open Questions の 4 行（worker fixture 形・`descriptor` surface・`kill_child` 適用確認・verifies 対象の切り分け）
- reviewer 注目点: T2 ∥ T3 の共有 file ゼロ確認（`video_plugin.rs` は T2 側 file）・`Creating` ゲートと `try_recv_notification` arm が T3 の必須 touch-point として task doc に転記されていること・encode obligations (a)-(d) の T2/T3 配置分割が design の責務分離どおりであること

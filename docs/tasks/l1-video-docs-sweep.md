---
id: "TASK-034NXuA0ENhlZxgP8zpnqd"
type: "task"
status: "todo"
title: "L1 video docs sweep（crate README・親文書 re-engagement 注記・test-spec v3 reconcile・literal sweep）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-export/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
      - type: "file"
        path: "docs/designs/ruviform-plugin-system-design-baseline.md"
      - type: "file"
        path: "docs/designs/ruviform-video-pipeline-design-baseline.md"
      - type: "file"
        path: "docs/designs/ruviform-plugin-l1-audio.md"
      - type: "file"
        path: "docs/adr/audio-plugin-host-facade-layered-routing.md"
      - type: "file"
        path: "docs/adr/l1-parent-child-ipc-and-shm.md"
      - type: "file"
        path: "docs/specs/issue-117-l1-failed-parity.md"
      - type: "file"
        path: "docs/specs/issue-119-l1-wire-version-policy.md"
      - type: "file"
        path: "docs/test-specs/issue-114-l1-runner-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-116-l1-proxy-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-l1-failed-parity-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-119-wire-version-policy-tests.md"
      - type: "file"
        path: "docs/impl/ir/ir-001-audio-plugin-l1-implementation.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-118-video-l1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/tasks/l1-video-runner-and-erased-surface.md"
    - "docs/tasks/l1-video-host-surface.md"
  blocks:
    - "docs/tasks/l1-video-l1-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-video-fixture-and-e2e.md"
    - "docs/tasks/l1-wire-v2-docs-sweep.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  defers: []
  deferred-by: []
---

# L1 video docs sweep（crate README・親文書 re-engagement 注記・test-spec v3 reconcile・literal sweep）

## Work

video L1 が着地した後のドキュメント整合 sweep — `WIRE_VERSION` の「audio wire v2 / 現在値」という古い wording・`Run`/`run` が「the L1 entry」のまま残る記述・`Slot`/`Route`/`descriptors` の stale literal・crate README の責務表・親 baseline 文書への #118 re-engagement 注記・**test-spec v3 reconcile**（#114/#116/#117/#119 test-spec が v2 前提で書かれている場合の「#118 で wire v3・`HostError` tag 10 追加」への加筆 — #117 IR の親文書 re-engagement パターンと同じ）。コード変更はないが、stale wording が残ると後続 task が v2 を真実として読み間違えるため、T6 final verification の前に必ず通す。

1. `crates/ruviform-plugin-core/README.md` — `wire` module の責務表に video codec（tag 12-22/4-7・`WIRE_MAX_VIDEO_*`・`wire::vshm` 定盤）を加筆・`PlainParamInfoOwned`/`VideoFrameDesc`/`HostError::InvalidValue` の re-export 記述・`video.rs`/`video_plugin.rs` の trait 対応表（`VideoPluginInstance` ↔ `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video`）・「wire version は 2」の現行値記述を v3 へ更新
2. `crates/ruviform-plugin-export/README.md` — `run` が「audio の L1 runner entry」であることと `run_video`（`Vec<Box<dyn VideoPluginFactory>>`・video registry・`vshm` attach・stash config replay・inbound `BlockIn` inert/direction-illegal 規則）が parallel な別 entry であることを明記・`vshm` module と `ruviform-plugin-l1-video-runner` bin の役割・`l1_video_runner_e2e` suite・`ruviform-video-core`/`ruviform-media-core` dep edge の追加根拠（`ErasedVideoPlugin`/`VideoFrame`/`MediaTime` を名指す — `ruviform-audio-core` の #113 昇格と同経路）
3. `crates/ruviform-plugin-host/README.md` — `video.rs`/`l1/video.rs`/`l1/vshm.rs` の責務表行・`PluginHost::video()` 借用 façade（`VideoPluginHost<'a>`）・`VideoOutOfProcessSpec`（Dedicated 固定 — `isolation`/`lookahead_blocks` 字段なしの構造 pin）・`Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo`・`descriptors` kind-tag・`render_timeout`/`command_timeout` 分離・`Instantiate` bound 拒否・`Render` の `Err(InvalidValue)` input 照合・`[[bin]] l1-test-video-worker`・`tests/l1_video_e2e.rs`・`ruviform-video-core`/`ruviform-media-core` dep edge の追加根拠
4. `docs/designs/ruviform-plugin-system-design-baseline.md` — 「audio `HostCommand`/`HostReply`」→「audio `HostCommand`/`HostReply`（video は `VideoHostCommand`/`VideoHostReply` — #118 で追加）」へ re-engagement 注記・`OutOfProcessSpec`/`OutOfProcessIsolation` が audio 限定であること・`Route`/`Slot`/`descriptors` に video arm が生えたこと・`plugin_id` 名空間は単一（双方向 `DuplicatePluginId`）・`prepare_endpoint`/`process`/`latency_frames`/`plugin_mut` が video に `Err(Unsupported)`/`None`/`Err(EndpointUnavailable)` で答えることを明記（audio 正本の記述を書き換えるのでなく、「#118 で video surface が追加され以下は audio 正本のまま」の注記を足す — #117 IR のパターン）
5. `docs/designs/ruviform-video-pipeline-design-baseline.md` — `VideoPluginInstance` trait の契約（`Error=PluginFailed`・`initialize(&VideoProcessConfig)`・`activate`/`start_processing`/`render(&VideoFrame,&mut VideoFrame,MediaTime)`/`stop_processing`/`deactivate`/`destroy`・`parameters()`/`parameter()`/`parameter_value()`/`set_parameter()`）が L1 child 表面（`ErasedVideoPlugin`/`run_video`/`CreateVideoInstance` stash replay）でそのまま動くこと・`VideoFrame`/`PixelFormat`/`ColorInfo`/`MediaTime` が wire codec 対象になったことを注記（video pipeline 正本は audio 寄りの記述がないため薄い注記で足りる）
6. `docs/designs/ruviform-plugin-l1-audio.md` — audio L1 正本の wire version 記述を v3 へ（`Hello{version}`/shm header `wire_version`/`WIRE_VERSION` 定数 — audio codec 自体は tag 0-11/0-3 で不変だが `WIRE_VERSION` は 3）・`Slot`/`Route`/`descriptors` に video arm が生えたこと・`InstShared` の `InstKind` 分離・`request` の `Creating` ゲートが `CreateVideoInstance` も受理すること（audio `L1Child` の `Creating` 検査を狭く保つと video create が `_ => Err(Ipc)` に落ちるため拡張 — #117 IR の「必須 touch-point」パターン）・`request_timed` 一般化（audio は `command_timeout` 既定のまま）・`run`/`serve`/`Runner`/`InstanceEntry` が audio 限定であることを明記（`run_video`/`VideoRunner`/`VideoEntry` は別 module — `export/lib.rs` の parallel entry）
7. `docs/adr/audio-plugin-host-facade-layered-routing.md` — `PluginHost::video()` 借用 façade（`VideoPluginHost<'a>` — `&mut PluginHost` 借用で audio facade 層と同じ所有構造・単一 plugin_id 名空間・`execute` が slot 分岐で cross-kind `Err(Unsupported)`）を re-engagement 注記で追加 — audio facade 正本の階層ルーティングは video にそのまま適用されないが借用構造と名空間は共有する
8. `docs/adr/l1-parent-child-ipc-and-shm.md` — `vshm` segment（`MAGIC b"RVLV"`・`HEADER_LEN=28`・in/out 2 region・`segment_len=28+2×frame_bytes`）が audio `shm` とは別定盤であること・`frame_bytes` は `PixelFormat`/`VideoProcessConfig` 由来であることを注記（audio `shm` の `in+out` レイアウトは channel×frames で、video は plane 連結 canonical — 別 segment 型を持つことを明記）
9. `docs/specs/issue-117-l1-failed-parity.md`・`docs/specs/issue-119-l1-wire-version-policy.md` — `#118 で wire v3 が着地し `Failed` 規則・`InstanceFailed` exactly-once・`Err` reply mirror 不変・`Hello` strict version・`Capabilities` reserved 規則が video にも適用された` という re-engagement 注記（#117 IR の「spec/ADR に「#N で再エンゲージ」を追加」パターン — audio spec を書き換えるのでなく「#118 で video にも適用」の注記を足す）
10. `docs/test-specs/issue-114-l1-runner-tests.md`・`docs/test-specs/issue-116-l1-proxy-tests.md`・`docs/test-specs/issue-117-l1-failed-parity-tests.md`・`docs/test-specs/issue-119-wire-version-policy-tests.md` — **v3 reconcile**: 各 test-spec の Examples/Rules が `WIRE_VERSION = 2` または tag 0-11/0-3 前提の wording であれば「#118 で `WIRE_VERSION` は 3・`Command` tag 12-22・`Reply` tag 4-7・`HostError` tag 10 が additive 追加 — audio tag/envelope/scope 規則は不変」という note を追記（test-spec の本体を書き換えるのでなく現行値との整合 note を足す — #119 spec L38 の additive 規則と整合）。`issue-118-*` 3 test-spec は新規なのでこの reconcile は不要
11. `docs/impl/ir/ir-001-audio-plugin-l1-implementation.md` — IR の「既知差分」節に `#118 で video L1 経路を追加（`VideoHostCommand`/`VideoPluginHost`/`Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo`/`run_video`/`vshm`/`WIRE_VERSION`=3・audio 経路は tag 0-11/0-3 で不変）` の re-engagement 注記を追加（audio IR を書き換えるのでなく、「#118 で video 経路が追加された」差分注記 — #117 IR の L1/audio 差分パターン）
12. stale literal sweep — `grep -rn "WIRE_VERSION" docs/ crates/*/README.md`・`"audio wire v2"`・`"the L1"`・`"run is the"`・`"Slot::"`・`"descriptors"`・`"HostCommand"`・`"Route::"` の残存を走査し、video L1 で真実が変わった記述（`WIRE_VERSION` の現行値・`run` が唯一の entry・`Slot` が audio-only・`descriptors` が untagged・`Route` が 3 variant・`HostCommand` が唯一の command 面）を #118 の re-engagement 注記で修正

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T5 — T2/T3/T4 着地後の整合 sweep）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md)（全節 — host 面・runner 面・wire 契約の記述正本）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md) R-B8（test-spec v3 reconcile の根拠 — additive 規則）
- [issue-117-bundle-child-implementation.md](../impl/ir/issue-117-bundle-child-implementation.md)（re-engagement 注記の pattern source — 「#N で再エンゲージ」の文体・差分注記の粒度）

## Dependencies

- 先行タスク: `l1-video-runner-and-erased-surface` + `l1-video-host-surface` + `l1-video-fixture-and-e2e`（code surface 確定後の doc sweep — T4 の fixture/bin/E2E 着地を含む）
- `docs/test-specs/issue-118-*.md` 3 件は新規（v3 前提で記述済み — reconcile 対象外）

## Affected Files

- `crates/ruviform-plugin-core/README.md` — wire/video trait/codec/re-export 表・`WIRE_VERSION`=3
- `crates/ruviform-plugin-export/README.md` — `run_video`/`vshm`/bin/suite/dep edge 表
- `crates/ruviform-plugin-host/README.md` — `video()`/`VideoOutOfProcessSpec`/`l1-test-video-worker`/`l1_video_e2e`/dep edge 表
- `docs/designs/ruviform-plugin-system-design-baseline.md` — video surface re-engagement 注記
- `docs/designs/ruviform-video-pipeline-design-baseline.md` — `VideoPluginInstance` L1 写像注記
- `docs/designs/ruviform-plugin-l1-audio.md` — wire v3・`InstKind`/`Creating`/`request_timed`/`run_video` 注記
- `docs/adr/audio-plugin-host-facade-layered-routing.md` — `video()` 借用 façade 注記
- `docs/adr/l1-parent-child-ipc-and-shm.md` — `vshm` 別定盤注記
- `docs/specs/issue-117-l1-failed-parity.md`・`docs/specs/issue-119-l1-wire-version-policy.md` — #118 re-engagement 注記
- `docs/test-specs/issue-114-l1-runner-tests.md`・`issue-116-l1-proxy-tests.md`・`issue-117-l1-failed-parity-tests.md`・`issue-119-wire-version-policy-tests.md` — v3 reconcile note
- `docs/impl/ir/ir-001-audio-plugin-l1-implementation.md` — #118 差分注記

## Done When

- [ ] `grep -rn "WIRE_VERSION" docs/ crates/*/README.md` の残存記述が v3 現行値を反映し、`"audio wire v2"`・`"run is the"`・`"the L1"` の stale wording がない
- [ ] `grep -rn "Slot::\|Route::\|descriptors\|HostCommand" docs/ crates/*/README.md` で video arm 追加後の真実（`OutOfProcessVideo`/kind-tag/`VideoHostCommand`）と矛盾する記述がない
- [ ] 3 crate README に video module/bin/suite/dep edge の責務表行があり、`ruviform-video-core`/`ruviform-media-core` 追加根拠が記述されている
- [ ] 親 baseline（plugin-system・video-pipeline・plugin-l1-audio）と ADR 2 件に #118 re-engagement 注記がある
- [ ] #117/#119 spec と #114/#116/#117/#119 test-spec に v3 reconcile 注記があり、`issue-118-*` test-spec は新規 v3 前提で reconcile 不要と明記されている
- [ ] `ir-001-audio-plugin-l1-implementation.md` に #118 差分注記がある

## Verification

- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — `blocking: 0`・`findings: []`（本 task 自体の front-matter 整合を含む）
- [ ] `mise run fmt-check` / `check` / `lint` — doc-only 変更でも workspace 全体が clean
- [ ] `prek run --all-files`（または `mise exec -- prek run --all-files`）が pass — README/docs の markdownlint・link check を含む
- [ ] stale literal sweep（上記 grep コマンド）の残存が注記で整合している

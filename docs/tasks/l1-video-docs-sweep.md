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
        path: "docs/designs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/designs/engine-plugin-host-integration.md"
      - type: "file"
        path: "docs/specs/engine-plugin-host-integration.md"
      - type: "file"
        path: "docs/specs/issue-117-bundle-child-process.md"
      - type: "file"
        path: "docs/specs/issue-119-l1-wire-version-policy.md"
      - type: "file"
        path: "docs/adr/issue-117-bundle-child-failed-granularity.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-wire-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-proxy-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-runner-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/real-adapter-l1-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-wire-v2-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
      - type: "file"
        path: "docs/impl/ir/issue-117-bundle-child-implementation.md"
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
    - "docs/tasks/l1-video-fixture-and-e2e.md"
  blocks:
    - "docs/tasks/l1-video-l1-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
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

video L1 が着地した後のドキュメント整合 sweep — `WIRE_VERSION` の「audio wire v2 / 現在値」という古い wording・`Run`/`run` が「the L1 entry」のまま残る記述・`Slot`/`Route`/`descriptors` の stale literal・crate README の責務表・親 baseline 文書への #118 re-engagement 注記・**test-spec v3 reconcile**（`plugin-l1-*` 3 本・`real-adapter-l1-e2e`・`issue-117-*` 3 本が v2 前提で書かれている箇所への「#118 で wire v3・`HostError` tag 10 追加」加筆 — #117 IR の親文書 re-engagement パターンと同じ）。コード変更はないが、stale wording が残ると後続 task が v2 を真実として読み間違えるため、T6 final verification の前に必ず通す。

1. `crates/ruviform-plugin-core/README.md` — `wire` module の責務表に video codec（tag 12-22/4-7・`WIRE_MAX_VIDEO_*`・`wire::vshm` 定盤）を加筆・`PlainParamInfoOwned`/`VideoFrameDesc`/`HostError::InvalidValue` の re-export 記述・`video.rs`/`video_plugin.rs` の trait 対応表（`VideoPluginInstance` ↔ `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video`）・「wire version は 2」の現行値記述を v3 へ更新
2. `crates/ruviform-plugin-export/README.md` — `run` が「audio の L1 runner entry」であることと `run_video`（`Vec<Box<dyn VideoPluginFactory>>`・video registry・`vshm` attach・stash config replay・inbound `BlockIn` inert/direction-illegal 規則）が parallel な別 entry であることを明記・`vshm` module と `ruviform-plugin-l1-video-runner` bin の役割・`l1_video_runner_e2e` suite・`ruviform-video-core`/`ruviform-media-core` dep edge の追加根拠（`ErasedVideoPlugin`/`VideoFrame`/`MediaTime` を名指す — `ruviform-audio-core` の #113 昇格と同経路）
3. `crates/ruviform-plugin-host/README.md` — `video.rs`/`l1/video.rs`/`l1/vshm.rs` の責務表行・`PluginHost::video()` 借用 façade（`VideoPluginHost<'a>`）・`VideoOutOfProcessSpec`（Dedicated 固定 — `isolation`/`lookahead_blocks` 字段なしの構造 pin）・`Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo`・`descriptors` kind-tag・`render_timeout`/`command_timeout` 分離・`Instantiate` bound 拒否・`Render` の `Err(InvalidValue)` input 照合・`[[bin]] l1-test-video-worker`・`tests/l1_video_e2e.rs`・`ruviform-video-core`/`ruviform-media-core` dep edge の追加根拠
4. `docs/designs/plugin-l1-out-of-process.md` — audio L1 正本 design への v3 reconcile + re-engagement 注記: `WIRE_VERSION = 2` の現行値記述（L153・L201-203 あたり — `Hello{version}`/shm header `wire_version`/`WIRE_VERSION` 定数）を v3 へ・audio codec 自体は tag 0-11/0-3 で不変だが `WIRE_VERSION` は 3・`Slot`/`Route`/`descriptors` に video arm（`OutOfProcessVideo`/kind-tag）が生えたこと・`InstShared` の `InstKind` 分離・`request` の `Creating` ゲートが `CreateVideoInstance` も受理すること・`request_timed` 一般化（audio は `command_timeout` 既定のまま）・`run`/`serve`/`Runner`/`InstanceEntry` が audio 限定であること（`run_video`/`VideoRunner`/`VideoEntry` は別 module）・`vshm` segment が audio `shm` とは別定盤であること・Deferred 行「video/NONRT プラグインの L1 経路 … video plugin は L1 不可」（L313 あたり）の trigger が #118 で消費された旨の re-engagement 注記（audio 正本の記述を書き換えるのでなく注記を足す — #117 IR のパターン）
5. `docs/specs/plugin-l1-out-of-process.md` — 同 design の spec 側: audio-only 前提の requirement 記述に「#118 で video surface（`VideoHostCommand`/`VideoPluginHost`/`vshm`）が追加 — 本 spec の要件は audio 正本のまま」の re-engagement 注記（spec status は `proposed` のまま — status flip は #118 外）
6. `docs/designs/engine-plugin-host-integration.md` — 決定 k「video NONRT 境界（L285-289 あたり — `L1 video wire は確立しない`）」の trigger が #118 で消費された旨の re-engagement 注記（wire プロトコル・shm スロット・runner は audio `process` 専用のまま、という記述に「#118 で video 専用 wire（tag 12-22/4-7）・`vshm`・`run_video` runner が別経路として確立」の注記）
7. `docs/specs/engine-plugin-host-integration.md` — spec 側の同 deferred 記述（「video NONRT 境界の明文化 + 延期 — L1 video wire は確立しない」）に #118 で video L1 経路が確立した旨の注記
8. `docs/specs/issue-117-bundle-child-process.md`・`docs/specs/issue-119-l1-wire-version-policy.md` — 「#118 で wire v3 が着地し `Failed` 規則・`InstanceFailed` exactly-once・`Err` reply mirror 不変・`Hello` strict version・`Capabilities` reserved 規則が video にも適用された」という re-engagement 注記（#117 IR の「spec/ADR に『#N で再エンゲージ』を追加」パターン — audio spec を書き換えるのでなく注記を足す）
9. `docs/adr/issue-117-bundle-child-failed-granularity.md` — Failed 粒度正本 ADR に「#118 で同粒度規則が video にも verbatim 適用（wire v3 — child-scope 基礎・帰属可能 `Rendered.output` semantic violation のみ per-instance）」の re-engagement 注記
10. test-spec v3 reconcile（`docs/test-specs/` 既存 7 本）: `plugin-l1-wire-codec-tests.md`・`plugin-l1-proxy-contract-tests.md`・`plugin-l1-runner-e2e-tests.md`・`real-adapter-l1-e2e-tests.md`・`issue-117-wire-v2-codec-tests.md`・`issue-117-bundle-e2e-tests.md`・`issue-117-bundle-failed-granularity-tests.md` — Examples/Rules が `WIRE_VERSION = 2` または tag 0-11/0-3 前提の wording であれば「#118 で `WIRE_VERSION` は 3・`Command` tag 12-22・`Reply` tag 4-7・`HostError` tag 10 が additive 追加 — audio tag/envelope/scope 規則は不変」という note を追記（本体を書き換えるのでなく現行値との整合 note を足す — #119 spec の additive 規則と整合）。`issue-118-*` 3 test-spec は新規（v3 前提で記述済み）なので reconcile 対象外
11. `docs/impl/ir/issue-117-bundle-child-implementation.md` — IR の Follow-ups/既知差分相当の節に「#118 で video L1 経路を追加（`VideoHostCommand`/`VideoPluginHost`/`Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo`/`run_video`/`vshm`/`WIRE_VERSION`=3・audio 経路は tag 0-11/0-3 で不変）」の re-engagement 注記を追加（#117 IR を書き換えるのでなく差分注記を足す）
12. stale literal sweep — `grep -rn "WIRE_VERSION" docs/ crates/*/README.md`・`"audio wire v2"`・`"the L1"`・`"run is the"`・`"Slot::"`・`"descriptors"`・`"HostCommand"`・`"Route::"` の残存を走査し、video L1 で真実が変わった記述（`WIRE_VERSION` の現行値・`run` が唯一の entry・`Slot` が audio-only・`descriptors` が untagged・`Route` が 3 variant・`HostCommand` が唯一の command 面）を #118 の re-engagement 注記で修正。**Rust 側 stale comment の disposition（doc-only sweep — 記録のみ・本 task では Rust を触らない）**: `crates/ruviform-plugin-export/src/lib.rs` L137 冒頭の `// Wire v2:` コメントは v3 着地で stale 化するが同 file を触るのは T2 — T2 の実装 note に「`run_video` 追加時に comment-only で v3 wording へ更新」を割り当て済み。`crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` の `wire-v2` 記述コメントは **accepted-stale** — spec R-B6 が audio `l1-test-worker` 差分 0 を pin しており本 Issue では触らない（sibling `l1-test-video-worker` は初版から v3 wording で書く）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T5 — T2/T3/T4 着地後の整合 sweep）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md)（全節 — host 面・runner 面・wire 契約の記述正本）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md) R-B8（test-spec v3 reconcile の根拠 — additive 規則）
- [issue-117-bundle-child-implementation.md](../impl/ir/issue-117-bundle-child-implementation.md)（re-engagement 注記の pattern source — 「#N で再エンゲージ」の文体・差分注記の粒度）

## Dependencies

- 先行タスク: `l1-video-runner-and-erased-surface` + `l1-video-host-surface` + `l1-video-fixture-and-e2e`（code surface 確定後の doc sweep — crate README が `[[bin]] l1-test-video-worker`・`tests/l1_video_e2e.rs` を記述する以上 T4 の fixture/bin/E2E 着地が前提。front-matter `depends-on` も 3 件全てを含む）
- `docs/test-specs/issue-118-*.md` 3 件は新規（v3 前提で記述済み — reconcile 対象外）

## Affected Files

- `crates/ruviform-plugin-core/README.md` — wire/video trait/codec/re-export 表・`WIRE_VERSION`=3
- `crates/ruviform-plugin-export/README.md` — `run_video`/`vshm`/bin/suite/dep edge 表
- `crates/ruviform-plugin-host/README.md` — `video()`/`VideoOutOfProcessSpec`/`l1-test-video-worker`/`l1_video_e2e`/dep edge 表
- `docs/designs/plugin-l1-out-of-process.md` — wire v3・`Slot`/`Route`/`InstKind`/`Creating`/`request_timed`/`run_video`/`vshm` 別定盤・deferred 行 re-engagement 注記
- `docs/specs/plugin-l1-out-of-process.md` — audio 正本のまま維持の旨の #118 注記
- `docs/designs/engine-plugin-host-integration.md` — 決定 k trigger 消費の re-engagement 注記
- `docs/specs/engine-plugin-host-integration.md` — video L1 確立の注記
- `docs/specs/issue-117-bundle-child-process.md`・`docs/specs/issue-119-l1-wire-version-policy.md` — #118 re-engagement 注記
- `docs/adr/issue-117-bundle-child-failed-granularity.md` — Failed 粒度の video 適用注記
- `docs/test-specs/plugin-l1-wire-codec-tests.md`・`plugin-l1-proxy-contract-tests.md`・`plugin-l1-runner-e2e-tests.md`・`real-adapter-l1-e2e-tests.md`・`issue-117-wire-v2-codec-tests.md`・`issue-117-bundle-e2e-tests.md`・`issue-117-bundle-failed-granularity-tests.md` — v3 reconcile note（7 本）
- `docs/impl/ir/issue-117-bundle-child-implementation.md` — #118 差分注記

## Done When

- [ ] `grep -rn "WIRE_VERSION" docs/ crates/*/README.md` の残存記述が v3 現行値を反映し、`"audio wire v2"`・`"run is the"`・`"the L1"` の stale wording がない
- [ ] `grep -rn "Slot::\|Route::\|descriptors\|HostCommand" docs/ crates/*/README.md` で video arm 追加後の真実（`OutOfProcessVideo`/kind-tag/`VideoHostCommand`）と矛盾する記述がない
- [ ] 3 crate README に video module/bin/suite/dep edge の責務表行があり、`ruviform-video-core`/`ruviform-media-core` 追加根拠が記述されている
- [ ] 親文書（`plugin-l1-out-of-process` design+spec・`engine-plugin-host-integration` design+spec）と `issue-117-bundle-child-process`/`issue-119` spec・`issue-117-bundle-child-failed-granularity` ADR に #118 re-engagement 注記がある
- [ ] 既存 test-spec 7 本（`plugin-l1-*`×3・`real-adapter-l1-e2e`・`issue-117-*`×3）に v3 reconcile 注記があり、`issue-118-*` test-spec は新規 v3 前提で reconcile 不要と明記されている
- [ ] `issue-117-bundle-child-implementation.md` に #118 差分注記がある
- [ ] Rust 側 stale comment disposition が記録されている（`export/src/lib.rs` → T2 担当・`l1-test-worker.rs` → R-B6 差分 0 pin で accepted-stale）

## Verification

- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — `blocking: 0`・`findings: []`（本 task 自体の front-matter 整合を含む）
- [ ] `mise run fmt-check` / `check` / `lint` — doc-only 変更でも workspace 全体が clean
- [ ] `prek run --all-files`（または `mise exec -- prek run --all-files`）が pass — README/docs の markdownlint・link check を含む
- [ ] stale literal sweep（上記 grep コマンド）の残存が注記で整合している

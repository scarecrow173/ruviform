---
id: "IMPL-034O4mVSR9nSSp2IH0H53q"
type: "impl"
status: "completed"
title: "Issue #118 video/NONRT プラグインの L1 経路（wire v3・vshm・video runner・video() façade）— implementation"
created: "2026-10-29"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-118-video-l1-implementation.md"
    modified:
      - type: "file"
        path: "docs/specs/issue-118-video-l1.md"
      - type: "file"
        path: "docs/plans/implement-issue-118-video-l1.md"
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
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-fmt-check-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-check-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-lint-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-test-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-test-first-flake-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-test-rt-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-bench-smoke-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-lint-l1-real-plugin-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-test-l1-real-plugin-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-cargo-doc-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-prek-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-l1-video-e2e-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-l1-video-runner-e2e-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-l1-proxy-e2e-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-l1-supervisor-e2e-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-l1-proxy-flake-rerun-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-doc-audit-initial-20261029.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-doc-audit-20261029.log"
  implements:
    - "docs/tasks/l1-video-l1-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/specs/issue-118-video-l1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
    - "docs/discovery/issue-118-video-l1.md"
    - "docs/impl/ir/issue-117-bundle-child-implementation.md"
    - "docs/impl/ir/issue-116-supervisor-implementation.md"
    - "docs/impl/ir/issue-119-l1-wire-version-policy-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/specs/issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-118-wire-v3-codec-tests.md"
    - "docs/test-specs/issue-118-video-failed-parity-tests.md"
    - "docs/test-specs/issue-118-video-e2e-tests.md"
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/123"
    - "https://github.com/scarecrow173/ruviform/issues/131"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #118 video/NONRT プラグインの L1 経路（wire v3・vshm・video runner・video() façade）— implementation

## Summary

Issue #118 T6（`l1-video-l1-final-verification`）完了 — T1–T5 で着地した video/NONRT L1 経路（`WIRE_VERSION` 2→3 additive bump・video `Command` tag 12-22・`Reply` tag 4-7・`HostError::InvalidValue` tag 10・`VideoProcessConfig`/`VideoFrameDesc`/`ColorInfo`/`PlainParamValue`/`PlainParamInfoOwned` codec・`WIRE_VIDEO_MAX_*`・`wire::vshm` header 定盤・per-instance `vshm` segment・`erase_video`/`ErasedVideoPlugin`/`VideoPluginFactory`・`run_video`/`VideoRunner`/`VideoEntry`・`ruviform-plugin-l1-video-runner` reference bin・`PluginHost::video() -> VideoPluginHost<'_>` 借用 façade・`VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`（Dedicated 構造 pin・`render_timeout` 60 s）・`Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo`/`InstKind` 分離・`descriptors` kind-tag・`Creating` ゲート拡張・`request_timed`・`l1-test-video-worker` hand-rolled fixture・`l1_video_e2e` 33 tests + `l1_video_runner_e2e` 20 tests）に対し、plan actions の最後段として **Full Gate を実施**し、AC1–AC9 + R-B1–B8 + R-F1–F6 + R-V1–V6 の証拠写像・canonical closeout（IR・status 同期・索引整合）を行った。

Full Gate **12 コマンド実実行 + focused suite 4 本 + flake 孤立 rerun**（全 log を `docs/impl/exp/data/issue-118-final-gate-*-20261029.log` に保存）。`mise run test` は初回・2 回目・3 回目が既知負荷依存 flake 2 件（`hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded` — `l1_proxy_e2e.rs:314`/`:453` の `L1Instance::spawn` unwrap が `command_timeout=500ms` の handshake 待ち超過で `Err(Ipc)`）で **exit 101**、4 回目で **exit 0** — **41 test binaries + merged doctests・1436 passed・0 failed・0 ignored**。flake は #117 IR・plan risk 表どおりの spawn-budget 競合であり、同 suite standalone（120/120 × 2 回）と孤立 rerun（2/2 pass — `issue-118-final-gate-l1-proxy-flake-rerun-20261029.log`）で切り分け済み。`l1-test-worker` は #118 diff 範囲で **zero-diff** のため #118 由来の退行ではない。その他全コマンドは初回から exit 0: `test-rt` は `rt_` name filter で **実一致 139・0 failed**（#117 計測 132 から +7 — 差分は本 Issue 区間外の #116/#126 系 rt_ 追加。video は NONRT のため `rt_` 非対象・audio RT suite の無退行証拠として通過）、opt-in lane `mise run test-l1-real-plugin` は 3 段全 green（fixture cdylib + runner bin 生成 → `--lib --bins` CLAP lib 63+bin 2・VST3 lib 47+bin 2 → `--ignored` CLAP 3+VST3 3 — v3 handshake 経由。video real-adapter lane は v1 に存在しない — plan Non-Goals 明記どおり audio lane のみの非退行）。

literal sweep 最終確認: `WIRE_VERSION` は `3` 単一（`crates/ruviform-plugin-core/src/wire.rs` — `= 2` 残存は doc 内の歴史注記のみ）・`VideoOutOfProcessSpec` に `isolation`/`lookahead_blocks` 字段なし（compile-time pin — source grep）・`InstPhase::Creating` ゲートは `CreateInstance | CreateVideoInstance` の両方を受理（`l1/mod.rs` `matches!` arm — plan Verification 行どおり）。

closeout として **AC1–AC9・R-B1–B8・R-F1–F6・R-V1–V6 ↔ task/test-spec ↔ 実 test/証跡の写像表**を本 IR に記録し、spec `approved → implemented`・plan `approved → completed`・6 task `todo → done`（T1–T5 は per-task closeout flip 未実施だった residual を T6 で修正 — 後述 incident）・索引 4 件（tasks・plans・specs・impl/ir README）を同期。doc audit は最終編集後に `blocking: 0`・`findings: []` を確認。**GitHub merge・issue close は lead の authoritative `gh` で別途実施 — 本 IR・本 commit は PR merged / issue closed の主張を含まない。**

## Context

approved plan `docs/plans/implement-issue-118-video-l1.md`（PLAN-034NXq4i4gZXaqNEEwBZpL）の T6（`docs/tasks/l1-video-l1-final-verification.md` — Full Gate・AC/R-V 証拠収集・IR・status 同期）。upstream: spec `docs/specs/issue-118-video-l1.md`（approved → implemented・AC1–AC9 + R-B/R-F/R-V）・approved design `docs/designs/issue-118-video-l1.md`・accepted ADR `docs/adr/issue-118-video-l1-host-surface.md`（別立て video 表面・Dedicated 固定）・`docs/adr/issue-118-video-l1-create-path-segment.md`（`CreateVideoInstance` での attach 解決）・7 test-spec（3 新規 `issue-118-*` + 4 baseline reconcile）。T1 `l1-wire-v3-video-codec`・T2 `l1-video-runner-and-erased-surface`・T3 `l1-video-host-surface`・T4 `l1-video-fixture-and-e2e`・T5 `l1-video-docs-sweep` は全て実装着地済み（review は T1/T2/T3 APPROVE-WITH-FIXES — fixes は `9f42a51`/`d793a18`/`7360fb9` に着地、T4 APPROVE）であり、本 task は全 task 完了後の **Full Gate** + canonical closeout を担う。先例: Issue #117 `l1-bundle-child-final-verification`（T6）・Issue #116 `l1-supervisor-final-verification`（T4）・Issue #114 `real-adapter-final-verification`（Task 5）— Full Gate 実績・AC-to-test 写像・status 同期・「実測しない lane の偽装排除」の同型 closeout。

## Implementation

### Changes

動作を変えるコード変更はゼロ — 検証記録 + 帳簿 + 正本同期のみの task（Rust ソース diff なし・`git diff --check` clean）。

- **Full Gate 実施（12 コマンド + focused 4 本・全 log 保存）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・opt-in lane `lint-l1-real-plugin`/`test-l1-real-plugin`・`RUSTDOCFLAGS="-D warnings" mise exec -- cargo doc -p ruviform-plugin-host -p ruviform-plugin-core -p ruviform-plugin-export --no-deps`・`mise exec -- prek run --all-files`・doc audit（`doc_status.js audit --json` — 初回 + 最終編集後の 2 回実施）・focused `cargo test -p ruviform-plugin-host --test l1_video_e2e`/`--test l1_proxy_e2e`/`--test l1_supervisor_e2e`・`-p ruviform-plugin-export --test l1_video_runner_e2e`・flake 孤立 rerun。結果は Validation 表。
- **AC/R-B/R-F/R-V 証拠 matrix** — 後述 coverage 表（plan Verification Matrix を正本として使用・design `## Requirements` 表と 1:1 照合）。
- **residual sweep（R-B6）** — `git diff fc02103...HEAD` は #116 commits を含むため #118 範囲は `3a1d7d0`（plan-review fix・T1 parent）..HEAD で検査:
  - `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — **zero-diff**（audio fixture 無変更の構造的保証）。
  - `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — 7 site の `spawn_scanned(&spawn_spec)` → `spawn_scanned(&spawn_spec.view())` 機械的適合のみ（`SpawnSpecView` 署名変更の呼び出し側適合 — review 済み touch-point。audio 意味論差分なし）。
  - `wire.rs` — audio verb tag 0-11・Reply tag 0-3・`HostError` tag 0-9・instance envelope・scope 判定表は無変更（差分は import 行再編・`WIRE_VERSION` 2→3・doc wording・`encode_command`/`encode_reply` の `Result` 返却化 — video codec の fallible 化に伴う署名変更で audio arm は byte-for-byte 同一出力）・`shm` mod 無タッチ。
  - `l1/mod.rs` — 除去行は全て enumerated touch-point: `InstShared` の `staging`/`shm`/`result_tx` を `InstKind::Audio{..}` 内へ移動（`push_result`/`armed_staging`/`audio_shm` accessor が同値を返す — `InstKind::Video` は `None`/no-op で in-protocol 到達不能と doc 明記）・`scan_match`/`spawn_scanned` が `&SpawnSpecView<'_>` を受ける（`SpawnSpec::view()` が同字段を貸す）・`Creating` ゲートに `CreateVideoInstance` arm 追加・`request` は `request_timed(instance, cmd, self.command_timeout)` へ委譲（timeout 値不変）。audio 経路の制御フロー・判定表・timeout 値は不変。
  - `composite.rs` — `Slot::OutOfProcess` variant 自体は doc 更新のみ・`Slot::OutOfProcessVideo`/`Route::OutOfProcessVideo`/`DescriptorEntry{kind}` は additive・audio `Scan` は `DescriptorKind::Audio` filter で video 登録を返さない（分離 pin）・`try_recv_notification`/`destroy`/`kill_child`/`lifecycle_state` は両 kind arm で video は kind 非依存経路を共有。
- **status 同期**: spec `approved → implemented`（実装完了・全 AC 証跡あり — AC checkbox は #113/#114/#117 先例どおり未チェックのまま証跡を本 IR coverage 表へ集約）・plan `approved → completed`（body `> status:` 行を完了形へ更新・`changes` を着地値で確定）・T6 task `done`（frontmatter・`implemented-by` に本 IR を登録・Done When/Verification checkbox を実績 annotate で `[x]` 化）。T1–T5 task doc は status flip 未実施だったため `todo → done` + checkbox `[x]` 化の residual 修正を実施（後述 incident）。
- **plan `changes` 着地値確定**: 予定リストに対する実績差分は modified +4 file — `crates/README.md`（T5 sweep・plan の crate README 列挙が crate-level README×3 のみで workspace 索引を漏らしていた）・`crates/ruviform-plugin-core/src/param.rs`（T1 — `ParamFlags::from_bits` 追加・wire codec の unknown-bit strict 拒否用）・`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（T3 — `spec.view()` 適合 7 site）・`docs/test-specs/README.md`（plan-package commit `37a0354` の test-spec 3 行索引 — `docs/tasks/README.md` と同じ plan-package 帳簿）。declared `added` 19 file は全て実在・declared `modified` 33 file は全て実 diff あり。
- **索引同期**: `docs/tasks/README.md`（6 行 `todo → done` — T1–T5 の index 行も未同期だった residual を修正）・`docs/plans/README.md`（`approved → completed`）・`docs/specs/README.md`（`approved → implemented`）・`docs/impl/ir/README.md`（本 IR 行追加 — 末尾追記の時系列順 convention に従う）。

### Whole-issue scope（6 task 集約）

| Task | Task 文書 | scope | Commit |
| --- | --- | --- | --- |
| 前提 | —（discovery・spec・design・2 ADR・plan・test-spec×3・task×6 文書パッケージ） | — | `c402808`・`67784a4`・`17c5a35`・`0f2c084`・`37a0354`・`3a1d7d0` |
| T1 | `l1-wire-v3-video-codec.md` | `WIRE_VERSION`→3・`Command` tag 12-22・`Reply` tag 4-7・video 型 codec・`WIRE_VIDEO_MAX_*`・`wire::vshm` 定盤・`HostError::InvalidValue` tag 10・`PlainParamInfoOwned`・`ParamFlags::from_bits` | `7ccad3b` + `7360fb9` |
| T2 | `l1-video-runner-and-erased-surface.md` | `video_plugin.rs`（`ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video`・encode obligations b/c/d）・`run_video`/`VideoRunner`/`VideoEntry`・`export/vshm.rs` attach 2 層検証・stash replay・reference bin・`l1_video_runner_e2e` 20 tests | `353db29` |
| T3 | `l1-video-host-surface.md` | `video.rs`（`VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost<'a>`）・`Route`/`Slot` video arm・`descriptors` kind-tag・`InstKind` 分離・`create_video_instance`・`Creating` ゲート拡張・`request_timed`・`l1/vshm.rs`・`l1/video.rs` `VideoL1Instance` | `d4c6cfd` + `9f42a51` |
| T4 | `l1-video-fixture-and-e2e.md` | `l1-test-video-worker`（hand-rolled・fault flags）・`tests/l1_video_e2e.rs` 33 tests（worker lane + real runner lane・`Failed` parity・frame 内容・cross-kind） | `37417a3` + `d793a18` |
| T5 | `l1-video-docs-sweep.md` | crate README×3 + crates index・親文書 addenda×4・spec×2 + ADR×1 + #117 IR の #118 注記・test-spec×7 の v3 reconcile・literal sweep | `cfbbb33` |
| T6 | `l1-video-l1-final-verification.md` | Full Gate・AC/R-V 証拠・IR・status 同期 | 本 docs commit |

### Acceptance criteria coverage（spec AC1–AC9 + R-B1–B8 + R-F1–F6 + R-V1–V6 ↔ task/test-spec ↔ 実 test/証跡）

| 契約 | spec 行 | task / test-spec | 実 test・証跡 |
| --- | --- | --- | --- |
| AC1 | video wire codec — v3 で video 動詞・desc・param・video 型 round-trip・defined 拒否・Hello 不一致失敗・audio codec 無変更 | T1 / `issue-118-wire-v3-codec-tests` + `plugin-l1-wire-codec-tests` | `video_verbs_roundtrip`・`video_replies_roundtrip`・`video_reply_tags_and_presence_are_strict`・`video_verbs_are_instance_scoped`（`NONE`/`≥64` → `Malformed`）・`video_process_config_codec_roundtrip_and_bounds`・`video_frame_desc_codec_roundtrip_and_bounds`・`color_info_codec_roundtrip_and_strict_presence`・`plain_param_value_codec_roundtrip_and_rejections`・`plain_param_info_owned_codec_roundtrip_and_contract`・`invalid_value_roundtrips_as_wire_tag_10`・`video_decode_is_bounded_against_adversarial_payloads`（非 panic・非無制限 alloc pin）・`hello_roundtrip_and_version_is_carried_verbatim`・`hello_v3_keeps_version_verbatim_and_capabilities_reserved`（全 `crates/ruviform-plugin-core/src/wire.rs` test mod）・`vshm_header_layout_is_self_consistent`・`vshm_frame_bytes_and_segment_len_math`・`vshm_header_write_and_validate`・`WIRE_VERSION: u16 = 3`・audio codec 既存 test（`command_roundtrip_covers_every_verb`・`reply_roundtrip_covers_every_payload`・`reply_roundtrip_carries_every_host_error` — tag 10 含む全 tag で v3 下 green）・Hello 不一致 handshake 失敗 `hello_version_mismatch_fails_instantiate`（`tests/l1_video_e2e.rs:1692` — `--hello-version=99`） |
| AC2 | lifecycle parity — scan→create→`initialize(VideoProcessConfig)`→activate→start→render→stop→deactivate→destroy・違法遷移 `Err(Lifecycle)`・`Failed` 宛 `Destroy` 以外 `Err(Failed)`・render 失敗時出力不 commit | T2/T3/T4 / `issue-118-video-e2e-tests` | `happy_path_lifecycle_and_scan`（`l1_video_e2e.rs:454` — 全遷徙 + `Destroy` `Ok` + slot 解放）・`render_gates_and_config_match`（575 — `Processing` 以外の render は `Err(Lifecycle)`・input config 不一致 `Err(InvalidValue)`）・`fail_initialize_keeps_created`（1391）・`fail_start_processing_stays_activated`（1413）・`fail_stop_processing_commits_activated`（1433 — commit-then-advisory）・`fail_deactivate_commits_initialized`（1453 — 同）・`fail_render_mirror_invariant`（1474 — `Err(PluginFailed)` で mirror 不変・out 領域不 commit・子生存）・`lifecycle_transitions_and_stash_replay`（`l1_video_runner_e2e.rs:812` — stash config → `initialize` 再生・違法遷移 `Err(Lifecycle)` 両向き）・`deactivate_error_is_commit_then_advisory`（901）・`render_failures_leave_output_uncommitted_and_the_child_alive`（1081 — commit-on-success）・unit `render_rejects_non_processing_mirror`・`execute_err_reply_leaves_mirror`・`execute_err_reply_commits_advisory_verbs`・`execute_lifecycle_err_is_precommit`（`src/l1/video.rs` test mod）・`Failed` 宛 verb `Err(Failed)` は `kill_child_converges_failed_and_notifies_once`（1169）の post-kill verb 列で pin |
| AC3 | frame 輸送 — input が segment 経由で届き output が `Reply` 到達時に有効・`Rgba8` 以外 wire 表現可能・FIFO・`(seq, instance)` 相関 | T1/T2/T3/T4 / `issue-118-video-e2e-tests` | `render_content_and_metadata_passthrough`（513 — deterministic invert の byte 一致 oracle + pts/duration/`ColorInfo` pass-through）・`runner_lane_invert_happy_path`（1745 — real `ruviform-plugin-l1-video-runner` lane の同一 oracle）・`render_round_trips_valid_output`（`src/l1/video.rs:896` — repack→`Rendered`→out 領域復元）・desc codec `video_frame_desc_codec_roundtrip_and_bounds`・canonical math `vshm_frame_bytes_and_segment_len_math`・host segment `create_writes_the_canonical_header`・`regions_slice_to_frame_bytes`・`create_named_refuses_an_unbounded_config`（`src/l1/vshm.rs` test mod）・child attach `attach_accepts_a_matching_segment`・`attach_rejects_every_structural_mismatch`・`attach_rechecks_the_frame_size_bound`（`export/src/vshm.rs` test mod）・`render_descriptor_semantics_are_checked`（`l1_video_runner_e2e.rs:1013` — copy-in/out・region 検証）・`output_desc_reports_canonical_placement_and_frame_metadata`（`export/src/lib.rs` test mod — tightly-packed canonical packing）・`PixelFormat` 全 variant codec round-trip（`video_frame_desc_codec_roundtrip_and_bounds` — `Rgba8` 以外 wire 表現） |
| AC4 | `Failed` parity — (a) 子死亡/帰属不能→全 live `Failed`+全 waiter `Err(Ipc)`+`InstanceFailed` fan-out (b) 帰属可能→instance のみ (c) inert (d) `Shutdown` 後 non-event (e) mid-verb 死亡分岐 | T2/T3/T4 / `issue-118-video-failed-parity-tests` | (a) `kill_child_converges_failed_and_notifies_once`（1169 — `kill_child` → mirror `Failed` + `InstanceFailed{handle, exit}` exactly-once + 全後続 verb `Err(Failed)`/`Err(Ipc)`）・`mid_verb_child_death_notifies`（1269 — `--crash-after-msgs=3` で lifecycle verb 中死亡 → waiter `Err(Ipc)` + 通知） (b) `noncanonical_output_converges_that_instance_only`（1336 — `--noncanonical-output` の well-formed-but-lies `Rendered` → 当該 instance `Failed` 収束・caller `Err(Ipc)` — Dedicated 1 子=1 instance で観測）・unit `render_malformed_output_converges_instance`（`src/l1/video.rs:864` — demux 合成 frame 経由の `converge_instance`） (c) `inbound_block_in_is_dropped_silently`（`l1_video_runner_e2e.rs:1278` — runner 宛 inbound `BlockIn` を drop・no-reply・子生存） (d) `shutdown_and_eof_exit_cleanly`（1329 — `Shutdown` → clean exit・EOF non-event）・`kill_child_converges_failed_and_notifies_once`（exit 後に追加通知なし） (e) `mid_render_child_death`（1237 — `--crash-after-msgs=6` render 中死亡 → waiter `Err(Ipc)` + `InstanceFailed`）・`mid_create_child_death_no_handle_no_notification`（1309 — `--crash-after-msgs=2` create 中死亡 → `Err(Ipc)`・handle 未発行・通知なし）・`mid_destroy_child_death_frees_slot`（1293 — destroy 中死亡 → `Failed`+通知+`Destroy` `Ok`+slot 解放）・`direction_illegal_inbound_messages_kill_the_child`（1296 — 帰属不能の direction 違反 → 子殺し = 子スコープ terminal） |
| AC5 | isolation — R-F4 確定値どおり video route isolation が観測可能 | T3/T4 / `issue-118-video-e2e-tests` | **Dedicated 構造 pin**: `VideoOutOfProcessSpec` に `isolation` 字段が存在しない（`src/video.rs` — `grep` 証跡・`isolation`/`lookahead_blocks` 不在の compile-time pin — 「非対応の形で pin」側で満たす）・`video()` 登録は常に `Route::OutOfProcessVideo`（pool 字段なし — `video_register_out_of_process` `src/composite.rs:540`）・`kill_child` が video handle で動く（Dedicated で per-instance=per-child — `kill_child_converges_failed_and_notifies_once`）・cross-kind 分離 `cross_kind_scan_and_instantiate`（887 — audio `Scan` が video descriptor を返さず・video `Instantiate` が audio `plugin_id` を `Err(UnknownPluginId)`）・`duplicate_plugin_id_both_directions`（1084 — audio→video・video→audio 登録衝突 `Err(DuplicatePluginId)`） |
| AC6 | param チャネル — `parameters`/`parameter`/`parameter_value`/`set_parameter` wire 越し・`PlainParamInfo`（String choices）bounded 往復・未知 `ParamId` 定義答え | T1/T2/T3/T4 / `issue-118-video-e2e-tests` | `params_ids_and_metadata`（646 — `parameters` 一覧・`PlainParamInfo`（choices 持ち 1 件 + `Double` 1 件）往復）・`param_values_round_trip_and_unknown_ids`（708 — `parameter(id)` → `ParamInfo(None)`・`parameter_value`/`set_parameter` → `Err(InvalidValue)` の defined 拒否・known id の get/set round-trip）・`set_param_value_non_finite_rejected_without_wire`（774 — 非有限 `Double` → `Err(InvalidValue)`・wire 非送信 — obligation (a) façade pre-validate）・`fail_param_value_known_id_only`（1523 — known id の `Err(PluginFailed)` 透過 — obligation (b)）・`overlimit_params_plugin_failed_child_alive`（1578 — over-limit `ids`/`choices` report → `Err(PluginFailed)`・子生存 — obligation (c)）・runner lane `runner_lane_overlimit_params_plugin_failed_child_alive`（1920）・runner E2E `parameter_dispatch_prechecks_membership`（940 — `id ∈ parameters()` 先行検査）・`over_limit_and_non_finite_reports_are_normalized`（1223 — encode obligations b/c/d の normalize 境界）・codec `plain_param_value_codec_roundtrip_and_rejections`・`plain_param_info_owned_codec_roundtrip_and_contract`・unit `video_set_param_rejects_non_finite_values`（`src/composite.rs:3040`） |
| AC7 | runner registry — video registry で instance dispatch・panic→`Err(PluginFailed)`・`CreateVideoInstance` mint・上限/重複/未登録 defined 拒否で runner 不死 | T2 / `issue-118-video-e2e-tests` + `l1_video_runner_e2e` | `happy_path_scan_create_render_destroy`（`l1_video_runner_e2e.rs:411` — MiniHost raw-wire 駆動 scan→create→render→destroy）・`unregistered_handles_are_invalid_handle`（683 — 未登録 `i` 宛 video 動詞 → flagged `Reply{i,seq,Err(InvalidHandle)}`・runner 継続）・`duplicate_handle_is_ipc`（657）・`instance_limit_is_a_defined_rejection`（776）・`cross_kind_create_is_a_defined_rejection`（706 — audio `CreateInstance` 宛 video registry 拒否）・`unknown_plugin_id_is_a_defined_rejection`（500）・`missing_segment_is_ipc_and_leaves_no_entry`（518）・`attach_rejections_are_ipc_and_keep_the_child_alive`（538 — attach 2 層検証全分岐）・`argv_is_a_usage_rejection`（490）・panic 正規化 `panic_initialize_is_plugin_failed_and_child_survives`（1160）・`factory_panics_are_plugin_failed`（1189）・`runner_lane_panic_initialize_plugin_failed`（`l1_video_e2e.rs:1947` — 公開 lane での panic→`Err(PluginFailed)`・instance のみ失敗）・`shutdown_and_eof_exit_cleanly`（1329 — `Shutdown` normalize で runner exit 0） |
| AC8 | audio 無退行 — 既存 L1 audio E2E・codec・`rt_` テスト全て無変更でグリーン | T1-T6 全 gate | `mise run test` exit 0（41 binaries・1436 passed — `l1_proxy_e2e` 120・`l1_runner_e2e` 21・`l1_supervisor_e2e` 22・`l1_real_adapter_e2e` compile-only 0・`plugin_graph_e2e` 13 等 audio suite 全 green）・`mise run test-rt` exit 0（`rt_` 実一致 139・0 failed — `rt_l1_*` audio RT suite 無退行）・`mise run test-l1-real-plugin` exit 0（CLAP/VST3 3 段全 green）・diff 証跡（`l1-test-worker.rs` zero-diff・`l1_proxy_e2e.rs` は `spec.view()` 適合 7 site のみ・audio touch-point 列挙どおり — Changes 節） |
| AC9 | CI/監査 — `mise run` 系列全 pass・doc audit blocking 0 | T6 | 本 Full Gate — Validation 表（12 コマンド + focused 4 本・`test` のみ flake 切り分け後 exit 0）・doc audit 最終 `blocking: 0`・`findings: []` |
| R-B1 | lifecycle 動詞表現 — `VideoInitialize` config 持ち・`render` が `{seq 相関, at}` を運び `Reply` が成功/失敗・違法遷移 `Err(Lifecycle)`・`Failed` 宛 `Err(Failed)` | T1/T3 / `issue-118-wire-v3-codec-tests` | `video_verbs_roundtrip`・`video_verbs_are_instance_scoped`（codec 動詞表 pin）・`VideoInitialize{config}`/`VideoRender{at, input}` encode/decode（`wire.rs` codec）・E2E `render_gates_and_config_match`・`fail_*` series（違法遷移 `Err(Lifecycle)`）・`kill_child_converges_failed_and_notifies_once`（`Failed` 宛 `Err(Failed)`） |
| R-B2 | descriptor + shm payload 分離 — wire が `{format,w,h,pts,duration,color,plane 記述, seq 相関}`・plane bytes は per-instance segment・copy-in/copy-out | T1/T2/T3 / `issue-118-wire-v3-codec-tests` | `VideoFrameDesc` codec（`video_frame_desc_codec_roundtrip_and_bounds` — 全字段 round-trip）・`wire::vshm` in/out 領域 byte math（`vshm_frame_bytes_and_segment_len_math`）・`l1/vshm.rs`/`export/vshm.rs` segment create/attach・`render_descriptor_semantics_are_checked`・`render_content_and_metadata_passthrough`（copy-in/copy-out の内容一致 oracle） |
| R-B3 | 同期 render・commit-on-success — `Ok` reply 到達時のみ out 有効・`Err`/timeout/死亡時は出力不読 | T3/T4 / `issue-118-video-e2e-tests` + `issue-118-video-failed-parity-tests` | `render_wedge_uses_render_timeout`（1624 — `--hang-after-msgs=5` で render のみ wedge → `render_timeout` で `Err(Ipc)`+`Failed` 収束）・`fail_render_mirror_invariant`（`Err` reply で out 領域不読）・`render_failures_leave_output_uncommitted_and_the_child_alive`・`mid_render_child_death`（死亡時出力不読）・`request_timed` 分離（`command_wedge_uses_command_timeout` 1664 — 他動詞は `command_timeout`） |
| R-B4 | bounded decode/attach — `WIRE_MAX_VIDEO_*` で hostile dims・巨大要求・stride/len 不整合を defined 拒否 | T1/T2/T3 / `issue-118-wire-v3-codec-tests` | `video_decode_is_bounded_against_adversarial_payloads`・`video_process_config_codec_roundtrip_and_bounds`・`video_frame_desc_codec_roundtrip_and_bounds`・`instantiate_config_bound_rejections`（`l1_video_e2e.rs:819` — dims/format/frame_bytes 超過が `Instantiate` で `Err(Prepare(UnsupportedConfig))` — spawn 前 early-check）・`create_named_refuses_an_unbounded_config`・`attach_rejects_every_structural_mismatch`・`attach_rechecks_the_frame_size_bound`・`check_input_desc_rejects_region_overflow_as_ipc`・`check_input_desc_rejects_config_mismatch_as_unsupported_config`（`export/src/lib.rs` test mod — attach/desc 両層） |
| R-B5 | `Failed` 粒度 video 適用 — #117 確定形 verbatim | T2/T3/T4 / `issue-118-video-failed-parity-tests` | AC4 行と同証拠（子スコープ terminal・帰属可能 per-instance・inert・通知 fan-out・mid-verb 全分岐 — 同じ `L1Child`/`Inner`/demux/converge 機構を共有） |
| R-B6 | audio 意味論無変更 — 既存 codec・slot・doorbell・`Failed` 分岐・Bundled pool 無退行 | T1-T6 | diff 証跡（Changes 節 — `l1-test-worker.rs` zero-diff・`l1_proxy_e2e.rs` は `spec.view()` 7 site・touch-point 列挙どおり: `Creating` ゲート・slot arm・`SpawnSpecView`・`InstKind` 移動は accessor 同値）+ `mise run test`/`test-rt`/`test-l1-real-plugin` 全 green・`l1_proxy_e2e` 120 tests（bundled/dedicated/`Failed`/doorbell 全行不変） |
| R-B7 | registry 同居 — video registry が `ErasedPlugin` audio registry と同居・panic `Err(PluginFailed)` 正規化・未登録宛 flagged `Err(InvalidHandle)` | T2 / `issue-118-video-e2e-tests` + `l1_video_runner_e2e` | `run_video`/`VideoRunner` が audio `run` と別 registry で同居（`export/src/lib.rs` — `VideoEntry` 型分離）・`unregistered_handles_are_invalid_handle`・`cross_kind_create_is_a_defined_rejection`・panic `panic_initialize_is_plugin_failed_and_child_survives`・`factory_panics_are_plugin_failed`・`runner_lane_panic_initialize_plugin_failed`・`direction_illegal_inbound_messages_kill_the_child`（audio 動詞宛 dispatch は direction 違反で defined） |
| R-B8 | plain param 動詞 — `parameters`/`parameter`/`parameter_value`/`set_parameter` wire 越し | T1/T2/T4 / `issue-118-video-e2e-tests` | AC6 行と同証拠（codec + runner `id ∈ parameters()` 先行検査 + façade pass-through + E2E） |
| R-F1 | 輸送確定形 — per-instance 専用 video segment + descriptor-over-Command/Reply（`VideoRender{at,input}` → `Rendered{output}`・新 doorbell kind なし・create 時 attach 解決） | T1/T2/T3 / `issue-118-wire-v3-codec-tests` + `issue-118-video-e2e-tests` | `VideoRender`/`Rendered` codec（`video_verbs_roundtrip`/`video_replies_roundtrip`）・`vshm::Segment::create` が `create_video_instance` 内で mint（`src/l1/mod.rs` — `CreateVideoInstance{plugin_id, shm_key, config}` が `os_id` を運ぶ）・`attach_accepts_a_matching_segment`/attach 拒否列・doorbell kind 無追加（`wire.rs` doorbell codec 無 diff — `doorbells_roundtrip` 不変）・E2E render happy path |
| R-F2 | render timeout — video 専用 `render_timeout`（既定 60 s）を `VideoRender` reply wait に・`command_timeout`（10 s）は他動詞 | T3/T4 / `issue-118-video-e2e-tests` | `render_wedge_uses_render_timeout`（`--hang-after-msgs=5`）・`command_wedge_uses_command_timeout`（`--hang-after-msgs=4` — 分離 pin）・`request_timed`（`src/l1/mod.rs:2157` — `request` は `command_timeout` 委譲）・`VideoOutOfProcessSpec.render_timeout` 字段（`src/video.rs`） |
| R-F3 | pool depth — depth=1 固定・canonical packing（`stride=row_bytes` tightly packed）・slot ring 字段なし | T1/T2/T3 / `issue-118-wire-v3-codec-tests` | `vshm_frame_bytes_and_segment_len_math`・`canonical_desc`/`output_desc_reports_canonical_placement_and_frame_metadata`・`regions_slice_to_frame_bytes`・`check_input_desc_rejects_non_canonical_placement_as_ipc`（非 canonical 拒否 — depth=1 packing の pin） |
| R-F4 | isolation 既定 — video は Dedicated のみ・`isolation` 字段非存在で `Bundled`+video を構造的に表現不能 | T3/T4 / `issue-118-video-e2e-tests` | AC5 行と同証拠（`isolation`/`lookahead_blocks` 字段不在の compile-time pin・`Route::OutOfProcessVideo` は pool 字段を持たない） |
| R-F5 | host 統合面 — `PluginHost::video(&mut self) -> VideoPluginHost<'_>` 借用 façade・`VideoHostCommand`/`VideoHostReply`（`#[non_exhaustive]`）・単一 `plugin_id` 名前空間・cross-kind 定義答え | T3/T4 / `issue-118-video-e2e-tests` | `PluginHost::video()`（`src/composite.rs:522`）・`VideoPluginHost`（`src/video.rs` — `register_out_of_process`/`execute`/`lifecycle_state`/`kill_child`/`try_recv_notification`）・`cross_kind_scan_and_instantiate`・`cross_kind_verbs_unsupported_and_shared_destroy`（933 — video handle 宛 audio verb `Err(Unsupported)`・audio handle 宛 video verb 同・`Destroy`/`lifecycle_state` は unified slot で共有）・`direct_plugin_host_methods_on_video_handle`（1121 — video handle で audio `HostCommand` 直行も `Err(Unsupported)`/`EndpointUnavailable`）・`duplicate_plugin_id_both_directions` |
| R-F6 | `Scan`/capability 広告 — wire に kind 記述を足さない（`Hello.capabilities` reserved-zero 維持）・kind 混同は host 側 field-match で検出 | T1/T3 / `issue-118-wire-v3-codec-tests` | `hello_v3_keeps_version_verbatim_and_capabilities_reserved`（`wire.rs:4958` — v3 で capabilities 非零拒否維持）・`descriptors` kind-tag の host-only 分離（`cross_kind_scan_and_instantiate` — audio/video `Scan` が互いの登録を返さない・wire `Scanned` payload に kind 字段なし） |
| R-V1 | codec round-trip + 拒否テスト | T1 / `issue-118-wire-v3-codec-tests` | AC1 行と同証拠（round-trip・tag 厳格性・instance scope・bounds・presence strictness・`InvalidValue` tag 10・`Hello` v3 pin） |
| R-V2 | video E2E — mock runner で scan→create→lifecycle→render→param→destroy・frame 内容正当性 | T4 (+T2) / `issue-118-video-e2e-tests` | `l1_video_e2e` 33/33（worker lane + real runner lane — `happy_path_*`・`render_content_and_metadata_passthrough`・`runner_lane_invert_happy_path`・`params_*`・`param_values_*`・`fail_*` series）・`l1_video_runner_e2e` 20/20（MiniHost raw-wire lane） |
| R-V3 | `Failed` parity — 子死亡/pipe 破壊/mid-render/帰属可能違反/inert/`Shutdown` 後 frame 全分岐 | T2/T3/T4 / `issue-118-video-failed-parity-tests` | AC4 行と同証拠 |
| R-V4 | audio 無退行 | T1-T6 / 全 suite | AC8 行と同証拠 |
| R-V5 | isolation 挙動 — R-F4 確定値どおり機能 | T3/T4 / `issue-118-video-e2e-tests` | AC5 行と同証拠（「非対応の形で pin」側 — `isolation` 字段不存在） |
| R-V6 | bounds — hostile config/dims/plane desc が decode/attach 両層で defined 拒否・無制限 alloc なし | T1/T2/T3 / `issue-118-wire-v3-codec-tests` | R-B4 行と同証拠 + `attach_rejections_are_ipc_and_keep_the_child_alive`（bad magic/version/mismatch/oversize の全分岐で `Err(Ipc)` 且つ子生存 — runner が死なない）・`decode_rejects_impossible_counts_without_allocating` 系の codec 層 pin（audio 踏襲 — `video_decode_is_bounded_against_adversarial_payloads` が video 側の同型 pin） |

### Per-task gate evidence（各 commit・本 gate 実測）

- T1（`7ccad3b`+`7360fb9`）: wire v3 additive codec + `vshm` 定盤 + `InvalidValue` + `PlainParamInfoOwned` + `ParamFlags::from_bits` — `wire.rs` test mod 13 新規 + presence strictness pin（`7360fb9`）が全 pass（plugin-core lib 99 tests green）。
- T2（`353db29`）: `erase_video` + `run_video` + video registry + reference bin — `l1_video_runner_e2e` 20 tests・export lib test（`check_input_desc_*`/`output_desc_*`）・`export/vshm.rs` attach test が全 pass。
- T3（`d4c6cfd`+`9f42a51`）: `video()` façade + `Slot`/`Route`/`InstKind`/`Creating`/`request_timed`/`vshm::Segment`/`VideoL1Instance` — `composite.rs` video unit test 4・`l1/mod.rs` `creating_gate_accepts_create_video_instance`/`create_video_instance_*`・`l1/video.rs` mirror/render unit test 9・`l1/vshm.rs` segment test 4 が全 pass（plugin-host lib 172 tests green）。
- T4（`37417a3`+`d793a18`）: `l1-test-video-worker` + `l1_video_e2e` 33 tests（worker lane + real runner lane）— focused run で 33/33 pass。
- T5（`cfbbb33`）: docs/metadata sweep — README・親文書 addenda・test-spec reconcile が doc audit clean（`blocking: 0`・`findings: []`）。
- T6（本 commit）: Full Gate 12 コマンド + focused 4 本 — Validation 表。

### Spec/design/test-spec relation state

- `docs/specs/issue-118-video-l1.md` — `status: "approved" → "implemented"`。AC checkbox は #113/#114/#117 先例どおり未チェックのまま（証跡は本 IR coverage 表）。`implemented-by` は `[design, plan]` のまま・`verified-by` は test-spec 3 件のまま変更なし（IR/task への upstream `implemented-by` edge は `inconsistent-reciprocal-relation` と衝突するため張らない — #117 convention）。
- `docs/designs/issue-118-video-l1.md`・`docs/adr/issue-118-video-l1-host-surface.md`・`docs/adr/issue-118-video-l1-create-path-segment.md` — `approved`/`accepted` 維持・`implemented-by: []` 維持（#117 closeout convention — upstream traceability は既存 edge で完備）。
- `docs/plans/implement-issue-118-video-l1.md` — `approved → completed`・body `> status:` 行を全 task 完了記録へ更新・`changes` を着地値へ確定（modified +4 file — 前述）・`implemented-by` は 6 task のまま。
- `docs/test-specs/issue-118-*.md` 3 件 — `approved` のまま無変更（test-spec は oracle であり実施状態を持たない — #118 3 件は plan 作成時点で approved 済み。`verified-by` に IR は追加しない — IR `verifies` 空欄 convention と衝突）。baseline `plugin-l1-wire-codec-tests`・`plugin-l1-proxy-contract-tests`・`plugin-l1-runner-e2e-tests`・`real-adapter-l1-e2e-tests` は T5 reconcile 済み `approved` 維持 — IR `verified-by` へ列挙（一方向 edge・#117 precedent）。
- T1–T5 task doc — `todo → done`（**incident**: per-task closeout での status flip が未実施だった — #117 では各 task closeout で同期済み・#118 T6 task doc も「同期済みのはず」と想定していたが実際は 6/6 task が `todo` 残存。task doc + index 行を本 commit で修正し本 IR に記録。`implemented-by: []` 維持 — IR への edge は T6 task のみ）・Done When/Verification checkbox を実績 annotate で `[x]` 化（#117 done-task convention — 全 checkbox `[x]`）。
- plan `implemented-by` → 6 task・task `implements` → plan・T6 task `implemented-by` → 本 IR — 相互 link 完備。

### Decisions During Implementation

- **`mise run test` の flake 切り分けと 4 回目 pass** — 初回・2 回目・3 回目が同一 2 件（`hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded`）で exit 101。両者とも `fast_timeout_spec`（`command_timeout=500ms` — `l1_proxy_e2e.rs:70-74`）下の `L1Instance::spawn` unwrap が `Err(Ipc)` — spawn+handshake が 500ms budget を超える負荷依存 flake（plan risk 表「500ms spawn budget 競合」・#117 IR「未発生」と同じ既知 flake）。切り分け: (1) suite standalone `l1_proxy_e2e` が 120/120 pass × 2 回、(2) 2 件の孤立 rerun が 2/2 pass（`--test-threads 2` — log 保存）、(3) `l1-test-worker` は #118 範囲 zero-diff で spawn 経路は `spawn_scanned(&spec.view())` の borrow のみ — #118 由来の退行機序が存在しない、(4) `cargo test` が failure 時に後続 binary を abort するため未実行だった末尾 crate（plugin-ofx/plugin-vst3/rt/video/video-core/video-proc + plugin-host doctest）は個別 `cargo test` で全 green を確認、(5) 4 回目の full run が exit 0（1436 passed・当該 2 件も pass）。失敗 log は `test-first-flake` 名で保存し証跡化。
- **Full Gate コマンド集合は task doc 列挙どおり** — `fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`lint-l1-real-plugin`/`test-l1-real-plugin`/`cargo doc`（3 crate・`RUSTDOCFLAGS="-D warnings"`）/`prek`/doc audit + focused suite 4 本（task prompt の enumerated 集合 — `l1_video_e2e`・`l1_video_runner_e2e`・`l1_proxy_e2e` regression・`l1_supervisor_e2e` regression）。video real-adapter lane は **v1 に存在しない**（plan Non-Goals・spec Out of Scope どおり — audio CLAP/VST3 lane のみ実行・「対象外」として記録、未実行を実行と偽装しない）。
- **T1–T5 task status residual** — per-task flip convention（#117 先例）では各 task closeout で `done` 同期される想定だったが、T1–T5 全 doc が `status: "todo"`・index 行も `todo` 残存だった。T6 task doc の「未同期が残っていた場合は index 行・task doc を修正して IR に incident として記録」条項に従い、6 task doc の `status` + checkbox + index 6 行を本 commit で修正（`updated` は既に着地日の `2026-10-29` — 変更なし）。Done When/Verification の `[x]` 化は #117 done-task convention に従った実績 annotate（本 gate の実測 + 各 task commit message の検証報告に基づく）。
- **plan `changes` 着地値** — declared `added` 19 file は全て実在、declared `modified` 33 file は全て実 diff あり。漏れ 4 file（`crates/README.md`・`param.rs`・`l1_proxy_e2e.rs`・`docs/test-specs/README.md`）を `modified` へ追記。pre-plan commit（discovery/spec/design/ADR）由来の doc 差分は #117 convention どおり `changes` 対象外（plan input）。
- **IR `verifies` は空・`verified-by` は test-spec 群** — #117/#119 IR convention（impl の `verifies` は空欄が型 convention・verify 主体は test-spec。spec `verified-by` に IR は追加しない — `inconsistent-reciprocal-relation` が IR `verifies` 空欄と衝突するため）。
- **`cargo doc` は `RUSTDOCFLAGS="-D warnings"` 下で実施** — 「warning 0」を exit 0 で機械証明する #113/#114/#117 と同形（初回から warning 0・#117 の `wire.rs` intra-doc link 欠陥と違い補正不要だった — 同 file は #117 T6 で補正済み）。
- **doc audit は編集前 + 最終編集後の 2 回実施** — 初回（gate 時点）で既に `blocking: 0`・`findings: []`（T5 の reconcile が clean）。最終編集後に再実施して `[]` を維持することを確認（Validation 表）。

### 既知差分（design/spec との intentional deviation — L0-faithful / pinned）

- **video `Err` reply で mirror 不変（audio `mark_failed` との意図的 divergence）** — audio では flagged `Err` reply が in-flight doorbell を `mark_failed` 合成で解く（result leg への失敗伝搬）。video には doorbell/result leg が存在しないため、`Err` reply は `err_committed_state` の commit-then-advisory verbs（`VideoStopProcessing`→`Activated`・`VideoDeactivate`→`Initialized`）以外で mirror を一切動かさない — `Err(PluginFailed)`/`Err(Lifecycle)`/defined 拒否で子は生存し `Failed` 化しない（L0 `VideoPluginInstance` 契約の写像 — `fail_render_mirror_invariant`・`execute_err_reply_leaves_mirror` で pin）。
- **`--leave-heir-after-msgs`/`--stop-reading-after-msgs` は fixture 語彙として存在するが公開 video API 経路では untested** — pinned vocabulary parity（audio worker との同族 flag 集合）として worker に実装済みだが、これらが exercise するのは kind 非依存の child-transport internals（wedged pipe write・heir-held stdout の EOF 区別）で audio lane（`l1_proxy_e2e`/`l1_runner_e2e`）が既に pin。video には大型 payload verb（`RestoreState` 相当）がなく、kind 非依存 internals の重複検証は #117 lane に譲る（plan/test-spec の意図的 scope）。
- **`--fail-activate` は語彙に存在しない** — `ActivateFailed` は両 lane で untested（audio fixture にも同 flag なし — spec レベルの scope 外）。
- **`descriptor(handle)` は v1 未提供** — handle→descriptor lookup API は公開面に生えない（`src/video.rs` doc 明記 — audio host にも per-handle descriptor accessor はなく、`Scan` が descriptor source・pinned）。
- **video real-adapter lane は v1 に存在しない** — `mise run test-l1-real-plugin`/`lint-l1-real-plugin` は CLAP/VST3 audio のみ（OFX L1 runner 組込み・規格 fixture E2E は後続 Issue — spec Out of Scope・plan Non-Goals 明記）。
- **`VideoOutOfProcessSpec` の `isolation`/`lookahead_blocks` 非存在** — compile-time pin（Bundled+video 構造的表現不能・`lookahead_blocks` は audio-only — `SpawnSpecView` が `lookahead_blocks` を含まないことで handshake 経路に出ないことを型で pin）。
- **`BlockIn` は video で inert** — runner 宛 inbound `BlockIn` は静かに drop（`inbound_block_in_is_dropped_silently` — video は doorbell kind を持たないため audio data-plane frame は未定義入力として drop）。

## Related Experiments

### Adopted

なし — 検証記録 + 帳簿のみの task（実験ログなし）。gate log 18 本は `docs/impl/exp/data/issue-118-final-gate-*-20261029.log` に保存。

### Rejected

- **flake 2 件を「環境制約」として未解消のまま記録する案** — 孤立 rerun・standalone suite・末尾 crate 個別実行で全緑を確認した上で 4 回目の full run が exit 0。負荷依存 flake の切り分けは plan risk 表の既定手順どおりで、test 変更や budget 緩和は行わない（pre-existing flake — 恒久対策が必要なら別 Issue の論点）。
- **T1–T5 status residual を task 差し戻しで処理する案** — 実装・検証自体は各 task で完結済み（commit message の gate 報告 + 本 gate で再実測 green）であり、未同期は帳簿記入漏れのみ。T6 task doc の「index 行・task doc を修正して IR に incident として記録」条項どおり T6 内で修正。

## Validation

| gate | コマンド | 結果 |
| --- | --- | --- |
| fmt-check | `mise run fmt-check` | exit 0 |
| check | `mise run check` | exit 0（`cargo check --workspace --all-targets` dev・default 構成） |
| lint | `mise run lint` | exit 0（`cargo clippy --workspace --all-targets -- -D warnings` clean） |
| test | `mise run test` | 初回・2 回目・3 回目 **exit 101**（`l1_proxy_e2e` の既知負荷依存 flake 2 件のみ — `hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded`）→ 4 回目 **exit 0**（**41 test binaries + merged doctests・1436 passed・0 failed・0 ignored** — `l1_proxy_e2e` 120・`l1_video_e2e` 33・`l1_runner_e2e` 21・`l1_video_runner_e2e` 20・`l1_supervisor_e2e` 22 含む。`#[ignore]` E2E は `host` feature 未 compile で default 経路に出ない）。切り分け log: `test-first-flake`（初回失敗）・`l1-proxy-flake-rerun`（孤立 rerun 2/2 pass）・`l1-proxy-e2e`（standalone 120/120） |
| test-rt | `mise run test-rt` | exit 0（`cargo test --workspace rt_` — **実一致 139・0 failed**。video は NONRT のため `rt_` 非対象 — audio RT suite 無退行の証拠として通過） |
| bench-smoke | `mise run bench-smoke` | exit 0（`cargo bench -p ruviform-math -p ruviform-rt -- --test` 計測なし 1 回） |
| lint-l1-real-plugin | `mise run lint-l1-real-plugin` | exit 0（clippy `--all-targets --features host -D warnings` — bridge・runner bin・E2E target 0 warning。audio lane のみ — video real lane は v1 対象外） |
| test-l1-real-plugin | `mise run test-l1-real-plugin` | exit 0 — **3 段全 green**: `cargo build --features host`（fixture cdylib + runner bin 生成）→ `--lib --bins`（CLAP lib 63+bin 2・VST3 lib 47+bin 2）→ `--ignored`（CLAP 3 + VST3 3 — `real_adapter_l1_full_series`/`real_adapter_l1_kill_reports_failed_once`/`real_adapter_l1_missing_runner_is_ipc_rejection` 各 ×2・実子プロセス・v3 handshake（spawn→Hello→Scan→CreateInstance）経由） |
| cargo-doc | `env RUSTDOCFLAGS="-D warnings" mise exec -- cargo doc -p ruviform-plugin-host -p ruviform-plugin-core -p ruviform-plugin-export --no-deps` | exit 0・warning 0（初回から pass） |
| prek | `mise exec -- prek run --all-files` | exit 0（全 hook pass — `check illegal windows names` は no files で Skipped） |
| l1-video-e2e | `mise exec -- cargo test -p ruviform-plugin-host --test l1_video_e2e` | exit 0（**33/33** — worker lane + real runner lane） |
| l1-video-runner-e2e | `mise exec -- cargo test -p ruviform-plugin-export --test l1_video_runner_e2e` | exit 0（**20/20** — MiniHost raw-wire lane） |
| l1-proxy-e2e | `mise exec -- cargo test -p ruviform-plugin-host --test l1_proxy_e2e` | exit 0（**120/120** — audio suite regression・standalone） |
| l1-supervisor-e2e | `mise exec -- cargo test -p ruviform-plugin-host --test l1_supervisor_e2e` | exit 0（**22/22** — #116 supervisor regression） |
| doc-audit | `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` | 初回（編集前）exit 0・`blocking: 0`・`findings: []` → 最終編集後再実施で同値維持 |
| literal-sweep | `grep -rn "WIRE_VERSION" crates/`・`VideoOutOfProcessSpec` 字段・`l1/mod.rs` `Creating` ゲート | `WIRE_VERSION = 3` 単一（`= 2` は doc 内歴史注記のみ）・`isolation`/`lookahead_blocks` 字段不在（compile-time pin）・`matches!(cmd, CreateInstance \| CreateVideoInstance)` arm 存在 |

対象 test-spec: `docs/test-specs/issue-118-wire-v3-codec-tests.md`・`issue-118-video-failed-parity-tests.md`・`issue-118-video-e2e-tests.md`（新規 oracle — 全行が上記実 test/証跡へ写像済み）・`plugin-l1-wire-codec-tests.md`・`plugin-l1-proxy-contract-tests.md`・`plugin-l1-runner-e2e-tests.md`・`real-adapter-l1-e2e-tests.md`（T5 reconcile 済み `approved` — `verified-by` edge）。

## Risks

- **video real-adapter lane の欠如** — v1 は mock video fixture（`l1-test-video-worker` hand-rolled + `ruviform-plugin-l1-video-runner` reference bin）で wire/意味論を確定したが、OFX 実バイナリの L1 E2E は未検証（spec Out of Scope の確定 deferred — #114 と同じ disposition 判断）。`erase_video`/`run_video` の encode obligation は trait 契約上の pin であり、実 adapter が接続された時点で初めて実負荷で検証される。
- **既知負荷依存 flake の継続観測** — `l1_proxy_e2e` の `hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded`（`command_timeout=500ms` の spawn+handshake budget 競合・pre-existing）は本 gate で 3 回発火し 4 回目で通過。再発時は孤立 rerun で切り分け（plan risk 表の既定手順 — #117 IR と同記述）。
- **`--leave-heir`/`--stop-reading`/`ActivateFailed` は untested 経路** — video lane で公開 API から到達不能（kind 非依存 internals は audio lane で pin 済み・Activate 失敗は両 lane で語彙外）。video 固有の経路でこれらが必要になった時点の follow-up。
- **depth=1 同期 render の throughput 上限** — R-F3 確定どおり in/out 各 1 領域の同期 request/response（async render・pool depth>1・pipeline 化は design deferred — render latency が制御面を占有する。連続フレーム処理の需要が出た時点の follow-up）。
- **`rt_` name filter は substring match** — `sho**rt_**input` 系の偶然一致を含む件数である点は #126/#130 Risks と同一の既知 caveat（無退行判定は before/after 同一条件の件数比較で成立 — 今回 139 件）。
- **真の zero-copy は別拡張** — v1 は copy-in/copy-out（`Arc<[u8]>`/`SharedPlane` 制約 — 4K RgbaF32 ≈126 MiB 級でも copy コストを払う確定契約。shm 領域の直接 `VideoFrame` view/backing 抽象拡張は別 Issue）。

## Follow-ups

- Issue #119 — `WIRE_VERSION` 単一最新版方針の正本（v3 bump は本 Issue で実施済み — 方針自体の保守追跡）。
- Issue #120 — 共有リーパー/io 多重化（video 子も含む子単位 I/O スレッド集約先 — #117 からの継続論点）。
- Issue #123 — `runner_program()` 解決・frame-I/O dedup 残件。
- Issue #131 — OFX UAF residuals（video L1 経路が OFX adapter の L1 利用前提となった場合の関連）。
- design/spec deferred: GPU/texture interop・OFX GPU suites・真の zero-copy shm→frame backing・async render / pool depth>1・Bundled video の HoL 局所化・engine 側 video port 接続・video GUI/`OpenGui` 系・video supervisor/`RestartRecipe` 変種・`descriptor(handle)` lookup 公開面・instance-scoped fault 注入語彙の video 版。
- **GitHub merge・issue close は lead の authoritative `gh` のみ** — 本 task では merge を実施しない（証跡収集と doc 同期に留まる）。

## Sources and Relations

- Issue [#118](https://github.com/scarecrow173/ruviform/issues/118): video/NONRT プラグインの L1 経路 — wire v3 codec・per-instance vshm・video runner・`video()` façade の全実装と検証。
- Spec [issue-118-video-l1.md](../specs/issue-118-video-l1.md)（`implemented` — AC1–AC9 + R-B/R-F/R-V）・Design [issue-118-video-l1.md](../designs/issue-118-video-l1.md)（approved）・ADR [issue-118-video-l1-host-surface.md](../adr/issue-118-video-l1-host-surface.md)（accepted — 別立て video 表面・Dedicated 固定）・[issue-118-video-l1-create-path-segment.md](../adr/issue-118-video-l1-create-path-segment.md)（accepted — create 時 attach 解決）・Plan [implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（`completed`）・Task [l1-video-l1-final-verification.md](../tasks/l1-video-l1-final-verification.md)（T6・`done`）。
- 検証 oracle: [issue-118-wire-v3-codec-tests](../test-specs/issue-118-wire-v3-codec-tests.md)・[issue-118-video-failed-parity-tests](../test-specs/issue-118-video-failed-parity-tests.md)・[issue-118-video-e2e-tests](../test-specs/issue-118-video-e2e-tests.md)・[plugin-l1-wire-codec-tests](../test-specs/plugin-l1-wire-codec-tests.md)・[plugin-l1-proxy-contract-tests](../test-specs/plugin-l1-proxy-contract-tests.md)・[plugin-l1-runner-e2e-tests](../test-specs/plugin-l1-runner-e2e-tests.md)・[real-adapter-l1-e2e-tests](../test-specs/real-adapter-l1-e2e-tests.md)。
- 先行記録: [issue-117-bundle-child-implementation](issue-117-bundle-child-implementation.md)（v2 bundle 機構 — video の `Failed` 粒度・inert・通知契約の写像元・closeout exemplar）・[issue-116-supervisor-implementation](issue-116-supervisor-implementation.md)・[issue-119-l1-wire-version-policy-implementation](issue-119-l1-wire-version-policy-implementation.md)（wholesale bump 方針の実装記録）・[issue-76-l1-shm-data-plane-task-4-implementation](issue-76-l1-shm-data-plane-task-4-implementation.md)（video 経路の明示 deferred 起点）。
- Refs: Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)・[#113](https://github.com/scarecrow173/ruviform/issues/113)・[#114](https://github.com/scarecrow173/ruviform/issues/114)・[#117](https://github.com/scarecrow173/ruviform/issues/117)・[#119](https://github.com/scarecrow173/ruviform/issues/119)・[#120](https://github.com/scarecrow173/ruviform/issues/120)・[#123](https://github.com/scarecrow173/ruviform/issues/123)・[#131](https://github.com/scarecrow173/ruviform/issues/131)。

## Review Handoff

- AC/R-B/R-F/R-V 写像表の test 名・file:line が HEAD と一致するか・`mise run test` の flake 記述精度（3 回発火 → 孤立切り分け → 4 回目 pass）・T1–T5 status residual の incident 記録・plan `changes` 着地値（modified +4 file）・`video()` façade doc の `descriptor(handle)` 未提供明記・doc audit の最終 `findings: []` が編集後再実施であるかを確認されたい。**GitHub issue close・merge 判定は本 IR の scope 外 — push/PR は行っていない。**

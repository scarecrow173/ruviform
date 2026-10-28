---
id: "IMPL-034Nm7kRxQvTyHwPzSdGfJb"
type: "impl"
status: "completed"
title: "Issue #117 L1 bundle 子プロセス（1 子 N インスタンス）— implementation"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-117-bundle-child-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "docs/specs/issue-117-bundle-child-process.md"
      - type: "file"
        path: "docs/plans/implement-issue-117-bundle-child-process.md"
      - type: "file"
        path: "docs/test-specs/issue-117-wire-v2-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-e2e-tests.md"
      - type: "file"
        path: "docs/tasks/l1-bundle-child-final-verification.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-fmt-check-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-check-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-lint-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-test-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-test-rt-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-bench-smoke-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-lint-l1-real-plugin-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-test-l1-real-plugin-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-cargo-doc-before-fix-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-cargo-doc-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-prek-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-doc-audit-20261028.log"
  implements:
    - "docs/tasks/l1-bundle-child-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/specs/issue-117-bundle-child-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/discovery/issue-117-bundle-child-process-discovery.md"
    - "docs/impl/ir/issue-119-l1-wire-version-policy-implementation.md"
    - "docs/impl/ir/issue-114-real-adapter-final-verification-closeout-task-5-implementation.md"
    - "docs/impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-wire-v2-codec-tests.md"
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #117 L1 bundle 子プロセス（1 子 N インスタンス）— implementation

## Summary

Issue #117 T6（`l1-bundle-child-final-verification`）完了 — T1–T5 で着地した bundle 実装（wire v2 instance envelope + `CreateInstance`/`DestroyInstance` 動詞 + `InstanceLimit` tag 9・runner/worker registry・`L1Child`+`InstShared` split・demux 判定表・`OutOfProcessIsolation{Bundled(default),Dedicated}` + route pool）に対し、plan actions の最後段として **Full Gate を 1 回実施**し、AC1–AC12 の証拠写像・canonical closeout（IR・status 同期・索引整合）を行った。

Full Gate **12 コマンド実実行**（全 log を `docs/impl/exp/data/issue-117-final-gate-*-20261028.log` に保存）。初回 pass で唯一 `cargo doc` が失敗 — `pub mod shm` の doc comment 内 8 intra-doc link（`[`HEADER_LEN`]` 等）が `shm` 子モジュールの非公開パスへ裸名で張られており rustdoc が解決不能（**merge-base `d71bdd0` 時点から存在する pre-existing 欠陥** — 本 gate が `ruviform-plugin-core` を対象とする初回実施で初めて検出）。#117 のコード欠陥ではなく doc comment のみの修正のため、task doc の「同 task で完結する fix」条項に従い `[`shm::…`]` 修飾へ補正し再実行で warning 0・exit 0 を確認（before/after 両 log 保存）。**その他 11 コマンドは初回から全 exit 0**: `test` は **59 test binaries・1239 passed・0 failed・0 ignored**（`#[ignore]` E2E は `host` feature 未 compile で default 経路に出ない）、`test-rt` は `rt_` name filter で **実一致 132・0 failed**（#114 計測 129 から +3 — T4 追加の bundle RT 3 件）、opt-in lane `mise run test-l1-real-plugin` は 3 段全 green（fixture cdylib + runner bin 生成 → `--lib --bins` CLAP lib 63+bin 2・VST3 lib 47+bin 2 → `--ignored` CLAP 3+VST3 3 — 実 fixture cdylib を実子プロセス経路で exercise・v2 handshake（spawn→Hello→Scan→CreateInstance）経由）。既知負荷依存 flake（`wedged_child_write_converges_and_teardown_is_bounded`・`hang_converges_on_command_timeout` 系）は **未発生 — 1 発目から全 pass**。

literal sweep 最終確認: `WIRE_VERSION` は `2` 単一（wire.rs:103・`= 1` 残存なし）・`--ruviform-` 言及は全て「退役フラグの defined usage 拒否」という v2 契約自身の記述/検証コード（`export/lib.rs` argv 拒否・`l1-test-worker` `--ruviform-` 検出 exit 2・`e2e_argv_rejections`）か歴史記述 — 生きた v1 argv-parse/spawn 残存ゼロ。`one child`/`singleton`/`per-instance` 言及は bundle/dedicated の現行契約記述か OFX 無関係分のみ。

closeout として **AC1–AC12 ↔ task/test-spec ↔ 実 test/証跡の写像表**を本 IR に記録し、spec `draft → implemented`・plan `approved → completed`・3 test-spec `proposed → approved`・T6 task `done`・索引 5 件（tasks・plans・specs・test-specs・impl/ir README）を同期。doc audit は最終編集後に `blocking: 0`・`findings: []` を確認。**GitHub merge・issue close は lead の authoritative `gh` で別途実施 — 本 IR・本 commit は PR merged / issue closed の主張を含まない。**

## Context

approved plan `docs/plans/implement-issue-117-bundle-child-process.md`（PLAN-034NlJVJxcfEkAxcclU9Zw）の T6（`docs/tasks/l1-bundle-child-final-verification.md` — Full Gate・AC1–AC12 証拠収集・IR・status 同期）。upstream: spec `docs/specs/issue-117-bundle-child-process.md`（draft・AC1–AC12）・approved design `docs/designs/issue-117-bundle-child-process.md`（軸 1-14 確定・Verification Notes L394-408 が AC 証拠 oracle）・accepted ADR `docs/adr/issue-117-bundle-child-failed-granularity.md`（`Failed` 粒度・通知・隔離選択面の公開契約）・6 test-spec（3 新規 `issue-117-*` + 4 更新済み）。T1 `l1-wire-v2-codec-cutover`・T2 `l1-child-instance-split`・T3 `l1-route-pool-and-isolation-surface`・T4 `l1-bundle-e2e-and-granularity-verification`・T5 `l1-wire-v2-docs-sweep` は全て `done` であり、本 task は全 task 完了後の **Full Gate 1 回のみ** + canonical closeout を担う。Issue #114 `real-adapter-final-verification`（Task 5）・#113 `engine-plugin-final-verification`（Task 7）が先例 — Full Gate 実績・AC-to-test 写像・status 同期・「実測しない lane の偽装排除」の同型 closeout。

## Implementation

### Changes

動作を変えるコード変更はゼロ — 検証記録 + 帳簿 + 正本同期のみの task。唯一のソースファイル差分は `crates/ruviform-plugin-core/src/wire.rs` の doc comment link 補正（pre-existing 欠陥 — 後述）。

- **Full Gate 実施（12 コマンド・全 log 保存）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・opt-in lane `lint-l1-real-plugin`/`test-l1-real-plugin`・`cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps`（`RUSTDOCFLAGS="-D warnings"`）・`mise exec -- prek run --all-files`・doc audit（`doc_status.js audit --json`）。結果は Validation 表。
- **`wire.rs` doc link 補正（pre-existing 欠陥の doc-only fix）**: `pub mod shm` の doc comment（wire.rs:155-168）が `[`HEADER_LEN`]`/`[`SLOT_BYTES`]`/`[`FLAG_TRANSPORT`]`/`[`TRANSPORT_LEN`]`/`[`RESULT_LEN`]`/`[`EVENT_WIRE_SIZE`]`/`[`SAMPLES_LEN`]`/`[`SAMPLE_STRIDE`]` を裸名で link 化していたが、これらは `shm` モジュール内 `pub const` で outer doc は親（`wire`）スコープで解決されるため rustdoc broken-intra-doc-links（`cargo doc` gate 初実施で検出 — merge-base `d71bdd0` と同一テキストの pre-existing）。`[`shm::…`]` 修飾へ 8 箇所補正（`WIRE_MAX_FRAMES`/`CHANNELS`/`EVENTS` は wire 直下のため裸名のまま正しく解決済み・変更なし）。行為変更なし・rustfmt/clippy に影響なし（fmt-check/lint は補正後の状態で再確認 — `cargo doc` 再実行が exit 0・warning 0 を機械証明）。初回失敗 log は `cargo-doc-before-fix` として保持。
- **literal sweep 最終確認（task doc 項目 4）**:
  - `grep -rn "WIRE_VERSION" crates/` — `pub const WIRE_VERSION: u16 = 2`（wire.rs:103）以外の `1` リテラル残存なし。`+1`/`-1` 系は全て「`WIRE_VERSION + 1` で mismatch を作る」拒否テスト側の意図的記述（`e2e_version_mismatch_is_a_parent_side_handshake_failure` 等）。
  - `grep -rn '\-\-ruviform-' crates/` — 全ヒットが v2 契約自身: `export/src/lib.rs:141` の argv 拒否（`run()`-based runner は全 argv を usage reject exit 2）・`l1-test-worker.rs:118` の `--ruviform-` prefix 検出→exit 2・`e2e_argv_rejections` の退役フラグ拒否 pin・各 bin doc/README の退役記述。host spawn 側は `l1/mod.rs:1711` `.args(&spec.args)` のみ — **生きた v1 bootstrap flag の生成・parse 残存ゼロ**。
  - `one child`/`per-instance spawn`/`1 子`/`singleton` — bundle（1 子 N instance）・dedicated（1 instance 1 子）の現行契約記述、及び OFX `host.rs` の無関係 singleton のみ。`l1_proxy_e2e.rs:1401` の "singleton leg" は dedicated 経路を指す記述コメント（historical record 相当）。
- **AC1–AC12 証拠 matrix** — 後述 coverage 表（plan Verification Matrix を正本として使用・design Verification Notes L394-408 を oracle として照合）。
- **status 同期**: spec `draft → implemented`（実装完了・全 AC 証跡あり — task doc の「実装完了時の convention」条項どおり schema 完了状態 `implemented` を選択・AC checkbox は #113/#114 先例どおり未チェックのまま証跡は本 IR coverage 表に集約）・plan `approved → completed`（body `> status:` 行を完了形へ更新）・3 test-spec `proposed → approved`（実測証跡を持つ）・T6 task `done`（frontmatter・`implemented-by` へ本 IR を登録・Done When/Verification checkbox を実績 annotate で `[x]` 化）。T1–T5 task 文書は全て既に `done`（T4 の index 行のみ未同期だった — 後述）。
- **索引同期**: `docs/tasks/README.md`（T4 行 `todo → done` — doc audit が初回に 1 finding として検出した未同期残留を解消 + T6 行 `todo → done`）・`docs/plans/README.md`（`approved → completed`）・`docs/specs/README.md`（`draft → implemented`）・`docs/test-specs/README.md`（3 行 `proposed → approved`）・`docs/impl/ir/README.md`（本 IR 行追加 — 末尾追記の時系列順 convention に従う）。
- **spec `> **DRAFT**` banner** — `implemented` へ同期（design/ADR で全 OPEN 確定済みの旨を banner 自身が記述していたため過去形へ補正・L181 の AC 注記と整合）。

### Whole-issue scope（6 task 集約）

| Task | Task 文書 | scope | Commit |
| --- | --- | --- | --- |
| 前提 | —（discovery/draft spec・design・ADR・plan・test-spec×3・task×6 文書パッケージ） | — | `a64df14`・`57a08e5`・`13ca734`・`5f8adde`・`cd69e48`・`48aadde` |
| T1 | `l1-wire-v2-codec-cutover.md` | `WIRE_VERSION`→2・`[seq][instance]` envelope・`CreateInstance`/`DestroyInstance`・`InstanceLimit` tag 9・`WIRE_MAX_INSTANCES`/`WIRE_INSTANCE_NONE`・runner/worker registry・argv v2・host 単一 instance v2・両 harness | `4b5627c` + `6b3797a` |
| T2 | `l1-child-instance-split.md` | `Shared`/`Inner` → `L1Child`（子共有）+ `InstShared`（per-instance）split・driver multi-drain・reader demux 判定表・converge split・`Destroying` gate・install/uninstall・last-ref teardown | `92f31fa` + `468d294` |
| T3 | `l1-route-pool-and-isolation-surface.md` | `OutOfProcessIsolation{Dedicated,Bundled}`（`#[non_exhaustive]`・default Bundled）・`Route::OutOfProcess` struct variant + pool・instantiate/destroy 写像・pool lifecycle | `13e6cec` + `58bf8e5` |
| T4 | `l1-bundle-e2e-and-granularity-verification.md` | bundle E2E（`l1_proxy_e2e` 99→100 tests）・demux 判定表/inert/doorbell 合成 frame 検証・dedicated 非回帰・rt_ bundle・real runner bundle | `cec29ff` + `4d73ea0` |
| T5 | `l1-wire-v2-docs-sweep.md` | design inventory の docs/metadata 全量（README・bin doc・Cargo.toml comment・shm doc・adapter comment・test-spec reconcile・親文書 addenda・literal sweep） | `10561d3` |
| T6 | `l1-bundle-child-final-verification.md` | Full Gate・AC 証拠・IR・status 同期 | 本 docs commit |

### Acceptance criteria coverage（spec AC1–AC12 ↔ task/test-spec ↔ 実 test/証跡）

| AC | spec 受入基準 | task / test-spec | 実 test・証跡 |
| --- | --- | --- | --- |
| AC1 | wire v2 codec — `WIRE_VERSION = 2`・instance envelope・新動詞 round-trip・v1 Hello/不正 instance defined 拒否・`InstanceLimit` tag 9 両向き pin | T1 / `issue-117-wire-v2-codec-tests` + `plugin-l1-wire-codec-tests` | `command_roundtrip_covers_every_verb`・`reply_roundtrip_covers_every_payload`・`reply_roundtrip_carries_every_host_error`（`InstanceLimit` 含む）・`doorbells_roundtrip`（`crates/ruviform-plugin-core/src/wire.rs:2027,2073,2098,2133`）・scope 判定表 `scan_is_the_only_child_scoped_verb`・`instance_envelope_rejects_out_of_range_values`（`WIRE_MAX_INSTANCES`/`NONE-1` 拒否）・`doorbells_require_a_live_instance`（`NONE` 不整合拒否）（2177,2216,2261）・`create_instance_payload_bounds_are_enforced`（2288）・`decode_rejects_*` 系列（declared lengths/counts/reserved/tag — 2411-2580・非 panic・非無制限 alloc pin `decode_rejects_impossible_counts_without_allocating`:2494）・`host_error_field_roundtrip`（2671 — encode tag 9→`InstanceLimit`・decode→同値の両向き + `encode_rejects_endpoint_unavailable_and_restores_out` wildcard arm pin）・`hello_roundtrip_and_version_is_carried_verbatim`（1989）・`WIRE_VERSION: u16 = 2`（103）・handshake 失敗: host 側 `handshake_version_mismatch`（`tests/l1_proxy_e2e.rs:345`）・runner 側 `e2e_version_mismatch_is_a_parent_side_handshake_failure`（`l1_runner_e2e.rs:826`）・shm header version `e2e_shm_header_wire_version_mismatch_is_ipc_reply`（750） |
| AC2 | bundle 制御面 — 同一 route の 2 回目 `Instantiate` が子追加 spawn せず `CreateInstance` で成立・独立 handle/lifecycle/process 経路・pool 選択述語 | T3/T4 / `issue-117-bundle-e2e-tests` | `bundled_route_shares_one_child_across_instances`（`tests/l1_proxy_e2e.rs:2620` — 公開 `PluginHost` + default `Bundled` route・2 `Instantiate` → 1 子（kill fan-out で運用証明）・独立 handle/lifecycle/state/data plane）・`bundled_child_hosts_independent_instances`（882 — `pub(crate)` 層・lowest-free mint `i=0,1`・`live_count`/`pool_eligible`・interleaved doorbell 無交差）・pool 選択 `bundled_scan_selects_evicts_and_spawns`（`src/composite.rs:2056` — eligible 子 select・terminal evict・full/draining skip・empty spawn・失敗時 poisoned 残留なし）・`isolation_defaults_to_bundled`・`register_out_of_process_builds_the_struct_variant`（2006,2025）・real runner `runner_bin_serves_bundled_gain_instances`（2998 — `ruviform-plugin-l1-runner` 実子へ multi-`CreateInstance`・param 独立） |
| AC3 | `(instance, seq)` 相関・混線なし | T2/T4 / `issue-117-bundle-failed-granularity-tests` + `issue-117-bundle-e2e-tests` | `demux_delivers_waiter_correlated_reply`（`src/l1/mod.rs:3647` — waiter instance 照合）・`demux_reply_instance_mismatch_is_child_scope`（3673 — 他 instance 宛 reply は child-scope 違反）・`demux_waiterless_reply_scope_by_instance_phase`（3702 — instance phase で scope 判定）・`demux_flagged_reply_completes_doorbell_failed`（3748 — `Reply{i,seq,Err}` が `block_in_flight` を解く複合キー）・E2E 無交差 `bundled_child_hosts_independent_instances`（882 — disjoint input range・foreign sample は undefined として検出される oracle）・`bundled_route_shares_one_child_across_instances`（2620）・runner 側 `e2e_bundle_instances_are_independent`（`l1_runner_e2e.rs:1037`） |
| AC4 | `Failed` 粒度全分岐 — 子 scope terminal・attributable instance-scope・inert・post-`Shutdown` non-event・`Destroying` doorbell window | T2/T4 / `issue-117-bundle-failed-granularity-tests` | (a) 子 scope: `converge_child_fans_out_per_minted_instance`（3920）・`bundled_child_death_fans_out_to_siblings`（`composite.rs:2168`）・E2E kill（882,2620）・判定表行 `demux_reply_instance_mismatch_is_child_scope`・`demux_waiterless_reply_none_stamp_is_child_scope`・`demux_blockdone_out_of_range_instance_is_child_scope`・`demux_second_hello_and_parent_kinds_are_child_scope`・`demux_waiterless_reply_out_of_range_is_child_scope`（3673,4071,4095,4125,4215） (b) attributable: `demux_unknown_blockdone_converges_one_instance`・`demux_unknown_flagged_reply_converges_one_instance`（3775,4153 — 対象のみ `Failed`・`Inner.terminal` 非変更）・`converge_instance_resolves_only_that_instance`（4403）・wire fidelity `bundled_failed_instance_destroy_recovers_handle`（1003 — 兄弟生存） (c) inert: `demux_frames_inert_by_phase_and_removal`（4269 — registered-but-not-live/`i ∉ instances` 静黙 drop・違反・通知なし）・`demux_destroying_window_vs_failed_inert`（3824）・`remove_instance_completes_residual_doorbells`（4022） (d) non-event: `demux_post_shutdown_frames_are_nonevents`・`demux_post_terminal_frames_are_nonevents`（3890,4367） (e) doorbell window: `driver_command_destroy_flips_destroying_then_gates`・`drain_submits_rings_doorbells_and_gates_dead`・`complete_doorbell_reattaches_view_on_handle_reuse`（4557,4681,4807 — `Destroying` gate で新規 submit `mark_failed` 合成・in-flight doorbell 通常完了・reply で entry 除去） 定義済み拒否: `demux_delivers_per_command_error_rejection`（4187 — `Err(HostError)` は `Failed` 化しない） |
| AC5 | 通知 — bundle crash で各インスタンスへ `InstanceFailed` 1 つずつ・`exit` 同値・per-instance 収束 `{None,None}`・mid-Create 通知なし・mid-Destroy 通知+`Ok` | T2/T3/T4 / 同上 | fan-out `converge_child_fans_out_per_minted_instance`（3920 — minted instance 毎に 1 通知）・`bundled_child_death_fans_out_to_siblings`（2168 — 2 通知・同一 `exit`）・E2E `bundled_route_shares_one_child_across_instances`・`bundled_child_hosts_independent_instances`（2620,882 — `exit_a == exit_b`・各 1 通知のみ）・per-instance `{None,None}` `bundled_failed_instance_destroy_recovers_handle`（1003）・mid-`CreateInstance` 通知なし `bundled_mid_create_death_notifies_only_minted`・`bundled_fresh_create_death_leaves_pool_unpoisoned`（2797,2850 — 未 mint handle は通知せず `Err(Ipc)`）・mid-`DestroyInstance` 通知+`Ok` `bundled_mid_destroy_death_keeps_notification`・`bundled_mid_destroy_death_ok_and_sibling_failed`（1108,2901 — `Destroy` `Ok`・兄弟 `Failed`+通知） |
| AC6 | runner registry — instance dispatch・panic→`PluginFailed`・`CreateInstance` 上限/duplicate defined `Err`・未登録 key 宛 flagged `Reply{InvalidHandle}`・runner 生存 | T1 / `plugin-l1-runner-e2e-tests` + `issue-117-bundle-e2e-tests` | `e2e_bundle_instances_are_independent`（`l1_runner_e2e.rs:1037`）・`e2e_unregistered_handle_is_invalid_handle`（1122 — 未登録 `i` 宛動詞/doorbell → flagged `Reply{i,seq,Err(InvalidHandle)}`・runner 継続）・`e2e_destroyed_handle_is_invalid_and_remintable`・`e2e_duplicate_instance_is_ipc_rejection`・`e2e_create_instance_unknown_plugin_id`・`e2e_instance_limit`（1166,1202,1228,1257 — capacity > dup の検証順）・panic 正規化 `e2e_plugin_panic_is_plugin_failed_and_child_survives`・`e2e_descriptor_panic_at_create_is_plugin_failed_and_child_survives`・`e2e_create_panic_is_plugin_failed_and_handle_stays_free`・`e2e_descriptor_panic_at_scan_is_plugin_failed`（851,907,935,957）・argv v2 `e2e_argv_rejections`（663 — `--ruviform-*` exit 2）・`e2e_shutdown_normalizes_all_instances`（1298 — N instance mid-lifecycle `Shutdown` → exit 0）・shm attach/header/depth `e2e_shm_*`（705-770）・host 側 `Scan`-verified id の `Err(UnknownPluginId)` → `Err(Ipc)` fold は wire fidelity 層 `bundled_*` で担保 |
| AC7 | dedicated 無退行 — `Dedicated` route で既存契約不変 | T1/T4 / `plugin-l1-proxy-contract-tests` + `issue-117-bundle-e2e-tests` | `dedicated_route_isolates_instances_per_child`（`l1_proxy_e2e.rs:2930` — 2 `Instantiate` → 各々専用子・kill は `a` のみ `Failed`・`b` は `Processing` 継続で block 配信継続）・`dedicated_destroy_is_capacity_1`（`composite.rs:2215`）・既存 `l1_proxy_e2e` suite 全 100 tests green（`command_parity_*`/`crash_transitions_*`/`l1_kill_reports_failed_once`/`process_*` 等の singleton 契約行が v2 handshake 経由で不変 — gate `test` log）・`l1_runner_e2e` 21 tests green・opt-in lane `--ignored` CLAP 3+VST3 3 が v2 handshake（spawn→Hello→Scan→CreateInstance）経由で実 pass |
| AC8 | RT 契約 — bundle 経路で `rt_` グリーン | T4 / `rt-verification-contracts` | `rt_l1_bundled_process_no_alloc`（`l1_proxy_e2e.rs:1406` — bundle 経路の doorbell/staging process leg が `assert_no_alloc` 窓内・`Failed` 化しない）・`rt_l1_process_path_no_alloc`・`rt_l1_result_leg_try_ops`（1178,1297）・`rt_l1_detached_process_no_alloc`（`src/l1/mod.rs:3553` — endpoint detach→process→leg return）・`rt_l1_arm_apply_no_alloc`・`rt_l1_endpoint_*` — `mise run test-rt` 実一致 **132・0 failed**（#114 計測 129 + bundle 3 件・`rt_l1_bundled_process_no_alloc` は log で実 pass 確認） |
| AC9 | 上限/handle reuse/live_count — `InstanceLimit` defined 拒否・lowest-free mint・wire-ID reuse の stale doorbell 遮断・`Failed`+live 子への `DestroyInstance` | T1/T3/T4 / `issue-117-bundle-e2e-tests` | `bundled_create_past_capacity_is_instance_limit`（`l1_proxy_e2e.rs:1139` — 実子で 64 mint 後 `Err(InstanceLimit)`・子健全）・`e2e_instance_limit`（`l1_runner_e2e.rs:1257` — registry full・capacity が dup より先・destroy で capacity 復帰）・unit `create_instance_on_full_child_is_instance_limit`・`mint_instance_lowest_free_and_capacity`・`remove_instance_frees_the_lowest_handle`・`live_count_counts_registered_until_reclaimed`（`src/l1/mod.rs:5103,4049,5088,5071` — registered 全件（`Failed` 未 `Destroy` 含む）を数える）・handle reuse `bundled_failed_instance_destroy_recovers_handle`（1003 — `Failed`+live 子へ `DestroyInstance` 送信→slot 回収→同 `i` remint）・`e2e_destroyed_handle_is_invalid_and_remintable`（1166）・stale doorbell 遮断 `complete_doorbell_reattaches_view_on_handle_reuse`（4807 — 前世代 ring へ届かない）・`demux_destroying_window_vs_failed_inert`（3824） |
| AC10 | CI/監査 — `mise run` 系列全 pass・doc audit findings 0 | T6 | 本 Full Gate — Validation 表（11 gate + cargo-doc 補正後再実施で全 exit 0）・doc audit 最終 `blocking: 0`・`findings: []` |
| AC11 | pool/子 lifecycle — shutdown-on-empty・rollback・eviction・last-ref teardown | T2/T3/T4 / `issue-117-bundle-e2e-tests` | `bundled_last_destroy_shuts_down_child_and_respawns`（`l1_proxy_e2e.rs:2749` — 最後の `Destroy` で空子 evict+teardown・次 `Instantiate` が fresh spawn）・`bundled_destroy_tears_down_only_when_empty`（`composite.rs:2140` — live 兄弟時は evict せず）・eviction `bundled_scan_selects_evicts_and_spawns`（2056 — scan 通過時の terminal evict）・rollback `bundled_fresh_create_death_leaves_pool_unpoisoned`（2850 — fresh-spawn 失敗は子ごと teardown・pool 未 poisoned）・shared-child `CreateInstance` 失敗の instance-entry のみ rollback `create_instance_ok_then_converge_rolls_back`（`src/l1/mod.rs:4962`）・`create_instance_reply_error_surfaces`（4998）・last-ref teardown `bundled_last_facade_teardown_keeps_siblings_alive`（1061 — 兄弟生存・最後の referrer が bounded `Shutdown`+reap）・`dedicated_destroy_is_capacity_1`（2215） |
| AC12 | in-flight lifecycle crash — mid-Create 死亡 `Err(Ipc)`+通知なし+entry 除去・mid-Destroy 死亡 registered `Failed`+通知+`Destroy` `Ok` | T2/T4 / `issue-117-bundle-failed-granularity-tests` + `issue-117-bundle-e2e-tests` | mid-`CreateInstance`: `bundled_mid_create_death_notifies_only_minted`（2797 — reused 子・waiter `Err(Ipc)` wake・未 mint handle は通知なし・minted 兄弟のみ fan-out・route 復帰）・`bundled_fresh_create_death_leaves_pool_unpoisoned`（2850 — fresh 子・同 arm・`live_count` リークなし）・demux `demux_delivers_reply_on_creating_entry`（`src/l1/mod.rs:4239` — `Creating` phase entry 宛 reply 受理）・mid-`DestroyInstance`: `bundled_mid_destroy_death_ok_and_sibling_failed`（2901 — `Destroy` `Ok`・slot 解放・兄弟 `Failed`+通知）・`bundled_mid_destroy_death_keeps_notification`（1108 — minted 通知が entry 除去後も届く） |

### Per-task gate evidence（各 commit・本 gate 実測）

- T1（`4b5627c`+`6b3797a`）: wire v2 codec 全系統原子 cutover — codec unit test・runner/worker registry・両 harness v2・MiniHost bundle runner E2E。本 gate で wire codec tests（`wire.rs` mod tests）・`l1_runner_e2e` 21 tests が全 pass。
- T2（`92f31fa`+`468d294`）: `L1Child`+`InstShared` split・demux 判定表・converge split・`Destroying` gate — `src/l1/mod.rs` unit test（demux/converge/mint/pool-eligible/rollback/doorbell 系列）が全 pass。
- T3（`13e6cec`+`58bf8e5`）: `OutOfProcessIsolation` + pool — `src/composite.rs` unit test（isolation default/route variant/scan/destroy/dedicated）が全 pass。
- T4（`cec29ff`+`4d73ea0`）: bundle E2E + 判定表検証 — `l1_proxy_e2e` 100 tests（bundle 行 8 + rt_ bundle 1 + dedicated 非回帰）・`l1_runner_e2e` 21 tests 全 pass。
- T5（`10561d3`）: docs/metadata sweep — 本 gate の literal sweep で残存ゼロを再確認・test-spec 4 件更新済み。
- T6（本 commit）: Full Gate 12 コマンド — Validation 表。

### Spec/design/test-spec relation state

- `docs/specs/issue-117-bundle-child-process.md` — `status: "draft" → "implemented"`・`> **DRAFT**` banner を implemented 記述へ補正。AC checkbox は #113/#114 先例どおり未チェックのまま（証跡は本 IR coverage 表）。`verified-by` は test-spec 6 件のまま変更なし（IR は `verified-by` edge の対象外 — test-spec が verify を担う convention）。
- `docs/designs/issue-117-bundle-child-process.md` — `status: "approved"` 維持。実装と design 決定（軸 1-14・判定表 L304-320）の差異は T1-T5 の各 commit message・task doc 実績 annotate の範囲で、契約レベルの相違は観測されず正本への追記不要（#114 の「契約レベルの design 差異ではなく追加反映不要」と同基準）。design Verification Notes の checkbox は先例どおり tick しない。
- `docs/plans/implement-issue-117-bundle-child-process.md` — `approved → completed`・body `> status:` 行を全 task 完了記録へ更新。
- 3 test-spec（`issue-117-wire-v2-codec-tests`・`issue-117-bundle-failed-granularity-tests`・`issue-117-bundle-e2e-tests`）— `proposed → approved`（実測証跡: coverage 表の実 test 名・gate log）。`plugin-l1-*` 更新済み 3 spec・`real-adapter-l1-e2e-tests` は T5 で reconcile 済みの `approved` 維持。
- plan `implemented-by` → 6 task・task `implements` → plan・T6 task `implemented-by` → 本 IR — 相互 link 完備。

### Decisions During Implementation

- **`cargo doc` 初回失敗の切り分けと doc-only fix** — `RUSTDOCFLAGS="-D warnings"` 下で `pub mod shm` doc comment の 8 intra-doc link が `error: unresolved link`（exit 101）。失敗は本 branch のコード差分ではなく doc comment 由来（merge-base `d71bdd0` と同一テキスト — `18530b1` 以来の pre-existing で、`cargo doc` を plugin-core 対象に走らせる gate が本 task で初実施のため未検出だった）。task doc が「実装の不整合・欠落があれば同 task で完結する fix を追加（新機能は追加しない）」を許容しており、かつ変更が doc comment のみ（行為不変・rustfmt/clippy 非影響）のため、最小修復 `[`X`]` → `[`shm::X`]`（outer doc は `wire` スコープで解決される）を施して再実施 — 補正後 exit 0・warning 0。失敗 log を `cargo-doc-before-fix` として保存し証跡化した。**コード（行為）への patch は行っていない** — 実装欠陥が見つかった場合は task 差し戻し対象だが、これは T5 sweep 領域の doc 欠落であり T6 の doc/verification 作業域内。
- **Full Gate コマンド集合は task doc 列挙どおり** — `fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`lint-l1-real-plugin`/`test-l1-real-plugin`/`cargo doc`（3 crate）/`prek`/doc audit + prompt の doc audit — `build` は task doc の列挙に含めなかった（`check` で型検査済み・#114 は prompt 補強で追加した先例だが本 task doc の Verification 列挙を正本とした）。opt-in lane は本 box で fixture/adapter bin が build 可能なため **実測で env-limited 記録は不要**（3 段全実行 — `required-features="host"` の runner bin・fixture cdylib を lane 自身が生成）。
- **flake 切り分け**: 既知負荷依存 flake（`wedged_child_write_converges_and_teardown_is_bounded`・`hang_converges_on_command_timeout` — T1/T3/T4 報告の 500ms spawn budget 競合）は **未発生**（1 発目から全 pass・孤立 rerun 不要 — `test` log の `l1_proxy_e2e` 100/100 で確認）。
- **spec status は `implemented`** — #114 と同基準（実装完了・全 AC 証跡あり）。task doc の「`status: draft → implemented`」指定どおり。
- **`cargo doc` は `RUSTDOCFLAGS="-D warnings"` 下で実施** — 「warning 0」を exit 0 で機械証明する #113/#114 と同形。
- **IR `verifies` は空・`verified-by` は test-spec 群** — #119 IR の convention に準拠（impl の `verifies` は空欄が型 convention・verify 主体は test-spec。spec `verified-by` に IR は追加しない — 追加すると doc audit の `inconsistent-reciprocal-relation` が IR `verifies` 空欄と衝突するため）。

## Related Experiments

### Adopted

なし — 検証記録 + 帳簿のみの task（実験ログなし）。gate log 12 本は `docs/impl/exp/data/issue-117-final-gate-*-20261028.log` に保存。

### Rejected

- **`cargo doc` 欠陥を「env-limited 記録」として未修正のまま通す案** — pre-existing とはいえ rustdoc broken link は T5（docs sweep）領域の doc 欠落であり、「同 task で完結する fix」条項の範囲内のため doc-only 補正を選択。行為変更を含む修正や大きな欠落であれば前 task 差し戻しだった。

## Validation

| gate | コマンド | 結果 |
| --- | --- | --- |
| fmt-check | `mise run fmt-check` | exit 0 |
| check | `mise run check` | exit 0（`cargo check --workspace --all-targets` dev・default 構成） |
| lint | `mise run lint` | exit 0（`cargo clippy --workspace --all-targets -- -D warnings` clean） |
| test | `mise run test` | exit 0（**59 test binaries・1239 passed・0 failed・0 ignored** — `l1_proxy_e2e` 100・`l1_runner_e2e` 21 含む。`#[ignore]` E2E は `host` feature 未 compile で default 経路に出ない） |
| test-rt | `mise run test-rt` | exit 0（`cargo test --workspace rt_` — **実一致 132・0 failed**・`rt_l1_bundled_process_no_alloc` 等 bundle RT 3 件追加で #114 計測 129 から +3・無退行） |
| bench-smoke | `mise run bench-smoke` | exit 0（`cargo bench -p ruviform-math -p ruviform-rt -- --test` 計測なし 1 回） |
| lint-l1-real-plugin | `mise run lint-l1-real-plugin` | exit 0（clippy `--all-targets --features host -D warnings` — bridge・runner bin・E2E target 0 warning） |
| test-l1-real-plugin | `mise run test-l1-real-plugin` | exit 0 — **3 段全 green**: `cargo build --features host`（fixture cdylib + runner bin 生成）→ `--lib --bins`（CLAP lib 63+bin 2・VST3 lib 47+bin 2）→ `--ignored`（CLAP 3 + VST3 3 — `real_adapter_l1_full_series`/`real_adapter_l1_kill_reports_failed_once`/`real_adapter_l1_missing_runner_is_ipc_rejection` 各 ×2・実子プロセス・v2 handshake 経由） |
| cargo-doc | `env RUSTDOCFLAGS="-D warnings" mise exec -- cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` | **初回 exit 101**（`wire.rs` `pub mod shm` doc の 8 intra-doc link 未解決 — pre-existing 欠陥）→ doc-only 補正後 **exit 0・warning 0**（before/after 両 log 保存） |
| prek | `mise exec -- prek run --all-files` | exit 0（全 hook pass — `check illegal windows names` は no files で Skipped） |
| doc-audit | `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` | exit 0・`blocking: 0`・`findings: []`（最終編集後に実施。初回は `docs/tasks/README.md` T4 index 行未同期の warning 1 件を検出 → 同期で解消） |
| literal-sweep | `grep -rn "WIRE_VERSION" crates/`・`grep -rni '\-\-ruviform-\|one child\|per-instance spawn\|1 子\|singleton' crates/` | `WIRE_VERSION = 2` 単一・`--ruviform-` は退役拒否契約/歴史記述のみ・bundle/dedicated 現行記述以外の残存なし |

対象 test-spec: `docs/test-specs/issue-117-wire-v2-codec-tests.md`・`issue-117-bundle-failed-granularity-tests.md`・`issue-117-bundle-e2e-tests.md`（`proposed → approved` — 実測証跡あり）・`plugin-l1-wire-codec-tests.md`・`plugin-l1-proxy-contract-tests.md`・`plugin-l1-runner-e2e-tests.md`・`real-adapter-l1-e2e-tests.md`（T5 reconcile 済み `approved` — `verified-by` edge）。

## Risks

- **opt-in lane は実行されなければ腐る** — `#[ignore]` + `required-features="host"` 分離の性質上、`l1-real-verify` の手動 dispatch と `mise run test-l1-real-plugin` 明記で緩和（#114 Risks と同記述）。本 task で実測済みのため現時点では lane は健在 — v2 handshake（`CreateInstance` 含む）を実 runner で exercise 済み。
- **既知負荷依存 flake の継続観測** — `l1_proxy_e2e` の `wedged_child_write_converges_and_teardown_is_bounded`・`hang_converges_on_command_timeout`（500ms spawn budget 競合・pre-existing）は本 gate 未発生だが残存リスク（再発時は孤立 rerun で切り分け — T1/T3/T4 と同記述）。
- **bundle は liveness 隔離を trade する** — R-G3/ADR の確定契約どおり、1 instance の hang は共有 `serve` を wedge させ command timeout 経由で同居全員を `Failed` にし得る（隔離は state/attribution 分離であってアドレス空間隔離ではない — 選択語彙の doc に公開契約として明記済み。利用側は隔離強度が必要な route では `Dedicated` を選ぶ）。
- **instance-scoped fault 注入語彙は deferred** — design L388 の確定 deferred どおり、`converge_instance`（帰属可能違反）の検証は demux 合成 frame unit test + `test_converge_instance` seam + 子 scope kill E2E の代替構成（per-instance dishonest-child fixture 未実装 — bundle 収束経路の実シナリオ検証が必要になった時点の follow-up）。
- **`rt_` name filter は substring match** — `sho**rt_**input` 系の偶然一致を含む件数である点は #126/#130 Risks と同一の既知 caveat（無退行判定は before/after 同一条件の件数比較で成立）。
- **空子の spawn コスト削減効果は route 単位に限定** — 明示 pool キー・route 跨ぎ bundle・keep-alive/warm pool は design deferred（`#[non_exhaustive]` で拡張余地を残す確定契約）。

## Follow-ups

- Issue #118 — video/NONRT データ面・`wire::shm` 第 2 レイアウト（out of scope 確定の追跡先）。
- Issue #120 — 共有リーパー/io 多重化（bundle 後の子単位 I/O スレッド集約先）。
- Issue #122 — platform/process/ipc 共通クレート抽出評価（#117 着地を前提とした延期の再評価）。
- Issue #123 — `runner_program()` 解決・frame-I/O dedup 残件。
- design deferred: 明示 pool キー混在 bundle・pool keep-alive・`BundleFailed` 相関通知 variant・instance-scoped fault 注入語彙・複数 reader/demux スレッド。
- **GitHub merge・issue close は lead の authoritative `gh` のみ** — 本 task では merge を実施しない（証跡収集と doc 同期に留まる）。

## Sources and Relations

- Issue [#117](https://github.com/scarecrow173/ruviform/issues/117): L1 bundle 子プロセス（1 子 N インスタンス）— wire v2 instance envelope・runner registry・child/instance split・pool + 隔離選択面の全実装と検証。
- Spec [issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md)（`implemented` — AC1–AC12）・Design [issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md)（approved — Verification Notes L394-408 が証拠 oracle）・ADR [issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)（accepted — `Failed` 粒度・通知・隔離選択面）・Plan [implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（`completed`）・Task [l1-bundle-child-final-verification.md](../tasks/l1-bundle-child-final-verification.md)（T6・`done`）。
- 検証 oracle: [issue-117-wire-v2-codec-tests](../test-specs/issue-117-wire-v2-codec-tests.md)・[issue-117-bundle-failed-granularity-tests](../test-specs/issue-117-bundle-failed-granularity-tests.md)・[issue-117-bundle-e2e-tests](../test-specs/issue-117-bundle-e2e-tests.md)・[plugin-l1-wire-codec-tests](../test-specs/plugin-l1-wire-codec-tests.md)・[plugin-l1-proxy-contract-tests](../test-specs/plugin-l1-proxy-contract-tests.md)・[plugin-l1-runner-e2e-tests](../test-specs/plugin-l1-runner-e2e-tests.md)・[real-adapter-l1-e2e-tests](../test-specs/real-adapter-l1-e2e-tests.md)。
- 先行記録: [issue-119-l1-wire-version-policy-implementation](issue-119-l1-wire-version-policy-implementation.md)（wholesale bump 方針の実装記録）・[issue-76-l1-core-wire-task-1-implementation](issue-76-l1-core-wire-task-1-implementation.md)（wire 導入・bundle non-goal 起点）・[issue-114-real-adapter-final-verification-closeout-task-5-implementation](issue-114-real-adapter-final-verification-closeout-task-5-implementation.md)（closeout 先例）。
- Refs: Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)・[#114](https://github.com/scarecrow173/ruviform/issues/114)・[#118](https://github.com/scarecrow173/ruviform/issues/118)・[#119](https://github.com/scarecrow173/ruviform/issues/119)・[#120](https://github.com/scarecrow173/ruviform/issues/120)・[#122](https://github.com/scarecrow173/ruviform/issues/122)・[#123](https://github.com/scarecrow173/ruviform/issues/123)。

## Review Handoff

- AC 写像表の test 名・file:line が HEAD と一致するか・`cargo doc` 補正が doc comment のみ（`[`X`]`→`[`shm::X`]` 8 箇所）で行為不変か・before/after log が保存されているか・doc audit の最終 `findings: []` が編集後再実施であるか・既知 flake 未発生の記述精度（1 発目全 pass）を確認されたい。**GitHub issue close・merge 判定は本 IR の scope 外 — push/PR は行っていない。**

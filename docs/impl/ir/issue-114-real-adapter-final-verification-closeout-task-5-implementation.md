---
id: "IMPL-034MTY32J8CnNrZoGVraSf"
type: "impl"
status: "completed"
title: "Issue #114 real adapter final verification + closeout — Task 5 implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-114-real-adapter-final-verification-closeout-task-5-implementation.md"
    modified:
      - type: "file"
        path: "docs/tasks/real-adapter-final-verification.md"
      - type: "file"
        path: "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/specs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/designs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/test-specs/adapter-host-bridge-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/real-adapter-l1-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/l1-real-plugin-lane-tests.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
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
        path: "docs/impl/exp/data/issue-114-final-gate-fmt-check-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-check-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-lint-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-rt-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-bench-smoke-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-build-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-l1-real-plugin-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-lint-l1-real-plugin-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-prek-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-doc-audit-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-adapters-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-adapters-normal-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-adapters-host-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-reverse-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-doc-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-adapters-host-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-rt-host-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-runner-direct-launch-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-SUMMARY-20261027.log"
  implements:
    - "docs/tasks/real-adapter-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/tasks/real-adapter-e2e-and-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-113-engine-plugin-final-verification-task-7-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
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

# Issue #114 real adapter final verification + closeout — Task 5 implementation

## Summary

Issue #114 Task 5 完了 — plan actions 15–16（Full Gate + canonical
closeout）を本計画最後段で **1 回のみ**実施した。Full Gate **19
コマンド全て実実行・全 exit 0**・全 log を
`docs/impl/exp/data/issue-114-final-gate-*-20261027.log`（20 本・
SUMMARY 含む）に保存した。`test` は **58 test binaries・1125
passed・0 failed**、`test-rt` は `rt_` name filter で **実一致
129 tests・0 failed**（#113 計測の 129 と同一 — default lane
無退行。T1 が feature `host` 内に追加した
`rt_process_drives_fixture_block_allocation_free` は feature-gated
のため default lane に出ない — opt-in lane 段 2 と専用
`cargo test --features host rt_` run で実 pass を別途実測）。
opt-in lane `mise run test-l1-real-plugin` は 3 段全 green —
build（fixture cdylib + runner bin 生成）→ `--lib --bins`（CLAP
lib 63 + bin 2・VST3 lib 47 + bin 2）→ `--ignored` E2E（CLAP 3 +
VST3 3 — `real_adapter_l1_full_series`/
`real_adapter_l1_kill_reports_failed_once`/
`real_adapter_l1_missing_runner_is_ipc_rejection` 各 ×2、実 fixture
cdylib を dlopen する実子プロセス）。既知 flake 2 件
（`wedged_child_write_converges_and_teardown_is_bounded`・
`rejected_block_replies_resolve_pass_through`・`l1_proxy_e2e` の
load-sensitive 系）は **1 発目から pass — flake 未発生**。`cargo
doc` は `RUSTDOCFLAGS="-D warnings"` 下で zero warnings、`cargo
tree` の三面監査（default full tree・`--edges normal`・
`--features host` + export/host 逆方向）で AC7 の DAG 契約を再実証
— adapter→export(optional)/host(dev) 方向のみ・逆向き依存ゼロ・
`host` off で `clack-host`/`libloading`/`ruviform-plugin-export` は
non-optional（normal）木に出現しない。runner 直接起動 smoke で
AC1 の exit 2（usage reject）/exit 1（missing fixture defined
rejection）を再実測。closeout として **8 AC ↔ actions ↔ test-spec ↔
実 test/証跡の写像表**を本 IR に記録し、task T1–T5 を `done`・plan
を `completed`・3 test-spec を実結果で `approved`・spec を
`implemented` へ同期、索引 5 件（tasks・test-specs・plans・specs・
impl/ir README）を整合させた。実装中に確定した design 差分
（CLAP output 側 bounce 採用・`descriptors()` の `&[…]` 戻り値・
`Drop` 駆動 teardown・VST3 `uid()` platform-order id hex・param 表
load-hoist・lane の `--lib --bins` 3 段化・bridge rt_ test の
feature-gated 追加）を design 正本へ同期反映した。**GitHub
merge・issue close は lead の authoritative `gh` で別途実施 — 本
IR・本 commit のいずれの文書も PR merged / issue closed の主張を
含まない。**

## Context

approved plan
`docs/plans/implement-issue-114-real-adapter-l1-e2e.md`
（PLAN-034MLRinXTGEtewrMHtDSh）の Task 5
（`docs/tasks/real-adapter-final-verification.md`・actions 15–16）。
upstream: spec
`docs/specs/issue-114-real-adapter-l1-e2e.md`（AC1–AC8）・approved
design `docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ
a–j・「検証戦略」「CI lane」「canonical closeout」）・3 test-spec
（`adapter-host-bridge-contract-tests`・`real-adapter-l1-e2e-tests`・
`l1-real-plugin-lane-tests` — plan/task の `verified-by` 先）。前提と
して全実装 task（T1 `clap-host-bridge`・T2 `vst3-host-bridge`・T3
`real-adapter-runner-bins`・T4 `real-adapter-e2e-and-lane`）が
`done` であり、本 task は全 task 完了後の **Full Gate 1 回のみ** +
canonical closeout（IR・coverage 記述・status 同期・design/spec の
実装相違を正本へ反映）を担う。Issue #113 の
`engine-plugin-final-verification`（task 7）が先例 — Full Gate
実績・AC-to-test 写像・status 同期・「実測しない lane の偽装排除」
の同型 closeout。

## Implementation

### Changes

コード変更ゼロ — 検証記録 + 帳簿 + 正本同期のみの task。

- **Full Gate 実施（19 コマンド・全 exit 0）**: `mise run
  fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`build`・
  opt-in lane `test-l1-real-plugin`/`lint-l1-real-plugin`・`mise exec
  -- prek run --all-files`・doc audit（`doc_status.js audit --json`
  — `blocking: 0`・`findings: []`）・`cargo tree` 三面
  （`-p` 両 adapter default・`--edges normal`・`--features host`・
  export/host 逆方向 `--edges normal`）・`cargo doc -p
  ruviform-plugin-clap -p ruviform-plugin-vst3 --features host
  --no-deps`（`RUSTDOCFLAGS="-D warnings"` で warning 0 の機械証明）・
  `cargo test -p ... --features host`（bridge unit 層・`--ignored`
  なし — `#[ignore]` E2E 3+3 が ignored 扱いで除外されることを確認）・
  `cargo test -p ... --features host rt_`（host-gated rt_ 実測 —
  `rt_process_drives_fixture_block_allocation_free` 実 pass）・runner
  直接起動 smoke（両 runner の usage reject exit 2・
  `RUVIFORM_L1_REAL_PLUGIN_DIR=/nonexistent` で missing-fixture
  exit 1）。全 log は `issue-114-final-gate-*-20261027.log`。
- **task/plan/test-spec/spec status 同期**: T1–T5 task 全て `done`
  （T5 文書の frontmatter・Done When/Verification checkbox を実績
  annotate で `[x]` 化）・plan `approved → completed`（`> status:`
  body 行を完了形に更新）・3 test-spec `proposed → approved`
  （実測証跡を持つ — body `> status:` 行も実結果確認済みへ更新）・
  spec `proposed → implemented`（実装完了・全 AC 証跡あり — task
  doc Affected Files の「approved 相当の更新」条件を満たすため、
  schema 上の完了状態 `implemented` に遷移。spec AC checkbox は
  #113 先例どおり未チェックのまま — 証跡は本 IR の coverage 表）。
- **design 正本への実装相違同期**
  （`docs/designs/issue-114-real-adapter-l1-e2e.md`）:
  - 決定サマリ h 行・「CI lane」節の mise.toml sketch・「検証戦略」
    lane 運用行: lane を 2 段 sketch から実装確定の **3 段**（build
    → `--lib --bins` → `--ignored`）へ同期 — `--lib --bins` は T4
    review で確定した修正（runner bin の `#[cfg(test)]` unit test が
    lane に載るための段）。
  - CLAP bridge 節: `descriptors()` の sketch 戻り値を
    `Vec<PluginDescriptor>` → **`&[PluginDescriptor]`**（load 時
    scan の cached 表への slice — VST3 側も同型）へ修正 + 新規
    bullet で **output 側も bounce 採用**の deviation を記録（task
    doc は `HostOutput.channel_mut` の `&mut [f32]` 直接 alias を
    許容していたが、中立 `process_host` の outputs は `&mut dyn
    BufferViewMut` で 1 live borrow につき 1 `&mut [f32]` のみ —
    multi-channel 直接 alias は raw-pointer 組立＝追加 `unsafe` を
    要し、unsafe 境界を `PluginEntry::load` 1 箇所に保つ方針と衝突
    するため、input と対称の owned-scratch copy-back に確定 —
    unconnected output channel は defined drop）+ **`Drop` 駆動
    teardown** bullet（best-effort `stop_processing`→`deactivate`→
    destroy — 中立面に deactivate/destroy hook がないため `Drop`
    で対称収束）。
  - VST3 bridge 節: `descriptors()` 戻り値を同型に修正・id hex の
    **platform-order** 注意を追記（`vst3::uid()` const fn の byte
    layout は platform 依存 — Windows は先頭 GUID field が反転・
    `TUID` 要素は `c_char`（本 platform では `i8`）のため hex 化は
    `cast_unsigned` 等で符号拡張なし reinterpret が必要 — prod
    `tuid_hex` と E2E 側独立 derive の両方で pin）・param 表列挙の
    **`Vst3Library::load` 時点 hoist** を追記（`factories()` は複数
    回呼出可 — per-call 列挙は throwaway instantiate + re-leak を
    招くため intern-once 不変条件を load 時 1 回で保持）。
  - 「検証戦略」の RT レーン行: 「子内 bridge は `rt_` 対象外」の
    記述を実装確定に同期 — T1 review で `rt_process_drives_fixture_
    block_allocation_free`（`assert_no_alloc` 窓内で `process_host`
    駆動）が feature `host` 内に追加済み。default `test-rt` lane の
    一致件数は不変（129）だが、opt-in lane `--lib` 段では bridge
    rt_ test も実行される旨を記録。
- **索引同期**: `docs/tasks/README.md`（T5 `todo → done`）・
  `docs/test-specs/README.md`（3 行 `proposed → approved`）・
  `docs/plans/README.md`（`approved → completed`）・
  `docs/specs/README.md`（`proposed → implemented`）・
  `docs/impl/ir/README.md`（本 IR 行追加）。
- **`docs/test-specs/adapter-host-bridge-contract-tests.md` の
  実装相違修正**: `> status:` 行内の lane 参照が「明示 `--lib`
  step」のまま残っていた（T4 が `--lib → --lib --bins` に lane を
  改訂した際の未同期残留）— `--lib --bins` に修正。

### Whole-issue scope（5 task 集約）

| Task | Task 文書 | Actions | Commit | IR |
| --- | --- | --- | --- | --- |
| 前提 | —（discovery/spec/design/plan/tasks 文書パッケージ） | — | `d1c2504`・`8e197dc` | — |
| 1 | `clap-host-bridge.md` | A1–A4 | `841ed4e` | `issue-114-clap-host-bridge-task-1-implementation.md` |
| 2 | `vst3-host-bridge.md` | A5–A9 | `2a98fc2` | `issue-114-vst3-host-bridge-task-2-implementation.md` |
| 3 | `real-adapter-runner-bins.md` | A10–A12 | `e1fa4d1` | `issue-114-real-adapter-runner-bins-task-3-implementation.md` |
| 4 | `real-adapter-e2e-and-lane.md` | A12–A14 | `70e02b7` | `issue-114-real-adapter-l1-e2e-opt-in-lane-task-4-implementation.md` |
| 5 | `real-adapter-final-verification.md` | A15–A16 | 本 docs commit | 本 IR |

scope 全体: CLAP bridge（`ClapLibrary`/`ClapPluginFactory`/
`ClapPlugin`/`ClapHostError` — `clack-host` optional・双方向 bounce・
intern-once・`Drop` teardown・bridge rt_ test）→ VST3 bridge
（`Vst3Library`/`Vst3PluginFactory`/`Vst3Plugin`/`Vst3HostError` —
`libloading` optional・最小 COM host objects・component+controller
pair・`IParameterChanges` 自前実装・load-hoisted param table）→
規格別 runner bin（`required-features=["host"]`・sibling fixture
解決・`RUVIFORM_L1_REAL_PLUGIN_DIR` shell-only override・exit
1/2 defined 収束）→ L1 E2E + opt-in lane（両規格
`l1_real_adapter_e2e.rs`・`test-l1-real-plugin`/`lint-l1-real-plugin`
mise task・`l1-real-verify` job 3-OS・OFX 境界明文化）→ 本 Full
Gate + closeout。

### Acceptance criteria coverage（spec AC1–AC8 ↔ actions ↔ test-spec ↔ 実 test/証跡）

| AC | spec 受入基準 | actions / test-spec | 実 test・証跡 |
| --- | --- | --- | --- |
| AC1 | runner オーサリング — compile-time 固定の規格別 runner bin・`run` argv 規約整合・実行時 plugin 指定なし | actions 1・5・10–12 / tspec e2e+lane | `src/bin/ruviform-plugin-{clap,vst3}-l1-runner.rs`（`required-features=["host"]` — default build 非生成）+ lane 段 1 `cargo build --features host` で両 runner bin + fixture cdylib 生成（log 確認）+ **runner 直接起動 smoke**: 引数なし → exit 2（`missing --ruviform-shm-key` usage reject — fixture load 通過の証拠）・`RUVIFORM_L1_REAL_PLUGIN_DIR=/nonexistent` → exit 1（stderr diagnostic・pre-`run` defined rejection）— `runner-direct-launch` log。argv 規約不変（`spec.args` 不使用 — T3/T4 IR 決定記録） |
| AC2 | 実 CLAP L1 E2E — spawn→handshake→lifecycle→shm block→state→destroy・gain 正当性 | actions 1–4・10・12 / tspec bridge+e2e | `real_adapter_l1_full_series`（clap — Scan→Instantiate（descriptor field-equality: 親 const pin vs 子合成）→Initialize→Activate→StartProcessing→32 block process（`ParamValue::new(0.25)` → CLAP plain `0.5`・delivered = gain × lookahead 前 input の membership + `delivered >= 8` floor + 先頭 2 call pre-lookahead silence）→SaveState/RestoreState `Err(Unsupported)`（wire 往復）→OpenGui/CloseGui `Err(Unsupported)`（host-side short-circuit）→Stop→Deactivate→Destroy→exit 0）— lane `--ignored` 段 3 pass・`test-l1-real-plugin` log |
| AC3 | 実 VST3 L1 E2E — 同経路（`GetPluginFactory`→`IPluginFactory`→COM） | actions 5–9・10・12 / tspec bridge+e2e | `real_adapter_l1_full_series`（vst3 — 同型・expected `id` は test 側が `vst3::uid(...)` bytes から独立 hex derive・`0.25 → ×0.25` normalized≡plain）— lane `--ignored` 段 3 pass・`test-l1-real-plugin` log |
| AC4 | 異常系維持 — kill → `Failed`+`InstanceFailed` once+pass-through・再起動なし・binary 不在 → defined rejection | action 12 / tspec e2e | `real_adapter_l1_kill_reports_failed_once` ×2（kill→`Failed` 収束→`InstanceFailed` exactly once・exit status 同梱・`Err(Failed)` 系拒否・`Destroy` で escape）・`real_adapter_l1_missing_runner_is_ipc_rejection` ×2（spawn `Err(Ipc)` pre-Hello・`Failed` 未到達・notification なし・route 登録維持 — `Scan` `Ok`）— lane `--ignored` 段 pass |
| AC5 | OFX 境界明文化 — `VideoPluginInstance` NONRT・L1 面非搭載・#118 追跡・延期 trigger | actions 13・16 | `crates/ruviform-plugin-ofx/README.md`「L1（out-of-process）境界 — Issue #114 AC5」節（T4 commit `70e02b7`）— 全操作 NONRT・video wire 非確立のため構造上 L1 経路に載らない・Issue #118 追跡・trigger = NONRT video port 分離要求 — 本 task で存在再確認 |
| AC6 | CI lane — 専用 mise task・`#[ignore]` ゲート・ci.yml 反映記録・欠落 fail | actions 13–14 / tspec lane | `mise.toml` `test-l1-real-plugin`（3 段）+ `lint-l1-real-plugin` 実在・本 gate で実測 exit 0（偽装排除 — fixture+runner 生成と `--ignored` 全 pass が log に在る）。`ci.yml` `l1-real-verify` job（既存 workflow_dispatch workflow 内 additive・3-OS matrix `fail-fast: false`・`verify`/`icc-verify` 同型 steps・別ファイルなし）— `actionlint` clean・workflow_dispatch-only のため実行不可（macOS leg 初実測は dispatch 時） |
| AC7 | 契約維持 — `run`/`OutOfProcessSpec`/wire/`RtLeg` 無変更・`#[non_exhaustive]`・scoped `unsafe` | actions 1–14 + 14・15 | `cargo tree` 三面監査: adapter `--edges normal` default 木に `clack-host`/`libloading`/`ruviform-plugin-export`/`ruviform-plugin-host` 非出現（dev-dep feature union で full tree には dev 経由で出るが normal 木は不変 — Cargo.toml 記載の意図的 union）・`--features host` で optional dep のみ出現・export/host 逆方向 `--edges normal` に adapter 非出現（逆向き依存ゼロ）。default gate 不変グリーン（`test` 1125・`test-rt` 129 — #113 計測と同数）・`check`/`lint` exit 0。`run`/`OutOfProcessSpec`/wire/RtLeg 変更 commit なし |
| AC8 | quality gate — fmt/check/lint/test/test-rt + opt-in lane + doc audit findings 0 | actions 15–16 | 本 Full Gate **19 コマンド全 exit 0**（下記 Validation 表）+ doc audit `blocking: 0`・`findings: []`（編集後の最終 audit も clean — 再実行で確認） |

### Per-task gate evidence（各 IR 記録・本 gate 実測）

- Task 1（CLAP bridge・`841ed4e`）: `fmt-check`/`check`/`lint`/`test`
  全 exit 0・`cargo test -p ruviform-plugin-clap --features host`
  **63 unit test**（bridge test 17 件含む）・rt_ test
  `rt_process_drives_fixture_block_allocation_free` 追加・敵対的
  review 確定修正 7 件 + output bounce deviation 1 件。
- Task 2（VST3 bridge・`2a98fc2`）: 同 gate 全 exit 0・`cargo test
  -p ruviform-plugin-vst3 --features host` **47 unit test**・COM host
  objects（`IParameterChanges`/`IParamValueQueue`/`IHostApplication`/
  `IComponentHandler`）自前実装・review 確定修正反映。
- Task 3（runner bins・`e1fa4d1`）: 同 gate 全 exit 0 + runner
  unit test 各 2 件 + 直接起動 smoke（exit 2/1 pin）。
- Task 4（E2E + lane・`70e02b7`）: 同 gate 全 exit 0 + lane 実測
  green（`--lib --bins` 63+2/47+2・`--ignored` 3+3）+ review 確定
  修正 6 件（`--lib --bins` 化・delivered floor・macOS note 等）。
- Task 5（本 task）: Full Gate 19 コマンド全 exit 0 — 下記
  Validation 表。

### Spec/design/test-spec relation state

- `docs/specs/issue-114-real-adapter-l1-e2e.md` — `status:
  "implemented"`（`proposed` から遷移 — task doc Affected Files の
  「実装完了・全 AC 証跡ありなら `approved` 相当の更新」条件を
  満たすため、schema の完了状態 `implemented` を選択。実装記録の
  `implemented-by` edge（design+plan）は既存どおり。AC checkbox は
  #113 先例どおり未チェックのまま — 証跡は本 IR coverage 表）。
- `docs/designs/issue-114-real-adapter-l1-e2e.md` — `status:
  "approved"` 維持。**実装確定差分を正本へ同期済み**（Changes 節の
  design 同期リスト: 決定サマリ h・CI lane sketch・検証戦略 lane 行
  の 3 段化・CLAP/VST3 `descriptors()` `&[…]` 戻り値・CLAP output
  bounce・`Drop` teardown・VST3 id hex platform-order・param 表
  load-hoist・bridge rt_ test の feature-gated 追記）。その他の
  実装詳細差異（`MAX_SCAN_*` cap・`min_frames`/`max_frames` 0 拒否・
  `enumerate_params` 空表 fallback・VST3 `block_frames` defensive
  min clamp・init 途中失敗の逆順 terminate・env override の
  `var_os` 解決・`fixture_lib_name` pub・`#[allow(unnecessary_
  cast)]` file-level・`delivered >= 8` membership floor）は各 IR
  の deviation/decision 記録どおり — design の決定文と矛盾しない
  実装粒度の差異のため正本への追記は不要と判断（#113 T7 の
  「契約レベルの design 差異ではなく追加反映不要」と同基準）。
- 3 test-spec — `proposed → approved`（実測証跡を持つ — lane
  実測・gate log・AC 写像）。`adapter-host-bridge-contract-tests`
  の `> status:` 行内に残存していた lane `--lib` 記述を `--lib
  --bins` に修正（T4 lane 改訂の未同期残留）。
- plan `implemented-by`/`derived-by` → 5 task・task 群 `implements`
  → plan・task `implemented-by` → 各 IR — 相互 link 完備（T5 task
  文書の `implemented-by` に本 IR を追記）。

### Decisions During Implementation

- **Full Gate コマンド集合は task doc 列挙 + prompt 補強の和集合**
  — task doc の Verification 13 項に `mise run build`（#113 先例・
  prompt 指定）・`cargo test --features host rt_`（host-gated rt_
  test の実測証明）・`cargo tree --edges normal`/`--features host`/
  逆方向の 3 面監査（DAG 契約を default full tree のみでは証明
  できないため — dev-dep feature union が `clack-host` を dev context
  で表示するのは Cargo.toml 記載の意図的挙動で、non-optional 木
  への非出現は `--edges normal` でのみ判定可能）・runner 直接起動
  smoke（AC1 の exit 2/1 を本 task の証跡として再実測 — T3 記録の
  再確認）を加えた 19 コマンド。
- **`cargo doc` は `RUSTDOCFLAGS="-D warnings"` 下で実施** —
  「warning 0」を exit 0 で機械証明する #113 と同形。
- **spec status は `implemented`** — task doc の「`approved` 相当」
  は条件付き（実装完了・全 AC 証跡あり）で、schema 上
  `implemented` がその完了状態に相当するため選択（#21 spec の
  `approved → implemented` 先例）。#113 は spec を `proposed` に
  留めたが（「spec status 遷移は closeout scope 外・lead 裁定」）、
  本 task doc は spec を Affected Files に明示しており条件を
  満たすため遷移を実施 — lead が `approved`/`proposed` への調整を
  望む場合の差し戻しは 1 行変更で済む旨を報告に明記する。
- **`test-rt` 無退行の証明は件数比較** — `cargo test --workspace
  rt_` 実一致 **129**（#113 final gate 計測の 129 と同一）。
  `rt_` filter は substring match のため `adapt_transport_*` 系も
  カウントに含まれるが、before/after 同一条件での件数比較は
  無退行証明として成立。T1 が feature `host` 内に追加した
  `rt_process_drives_fixture_block_allocation_free` は default lane
  に出ない（feature-gated — task doc の「bridge 追加テストに
  `rt_` 名なし」は default lane の件数不変として確認）・opt-in
  lane 段 2 と `test-rt-host` log で実 pass を別証跡化。
- **dev-dep feature union は監査対象外として明示** — default full
  `cargo tree` で `clack-extensions → clack-host → libloading` が
  表示されるのは dev-dep `clack-extensions { features =
  ["clack-host"] }` の feature union（テスト用 typed-entry 経路の
  意図的構成 — Cargo.toml comment 記載）。AC7 の「non-optional 木」
  判定は `--edges normal` で行い、`clack-host`/`libloading`/
  `ruviform-plugin-export`/`ruviform-plugin-host` が normal 木に
  出ないことを確認した。
- **design Verification Notes の checkbox は未チェックのまま** —
  #113 closeout 先例どおり、design 文書の `- [ ]` 項目は tick
  しない（証跡は IR coverage 表に集約）。
- **既知 flake の扱い** — `l1_proxy_e2e` の load-sensitive 2 test
  （`wedged_child_write_converges_and_teardown_is_bounded`・
  `rejected_block_replies_resolve_pass_through`）は Issue #113 由来
  の既知 flake として観測対象 — 本 gate では **1 発目から全 pass・
  flake 未発生**（`test` log で確認）。再発時は handshake timeout
  / test 直列化で別途 triage（T1/T2 IR の follow-up と同記述）。

## Related Experiments

### Adopted

なし — 検証記録 + 帳簿のみの task（実験ログなし）。gate log 20 本は
`docs/impl/exp/data/issue-114-final-gate-*-20261027.log` に保存。

### Rejected

なし — Full Gate のコマンド集合・log 命名は #113 final-gate 先例を
そのまま踏襲した（代替の検討なし）。

## Validation

| gate | コマンド | 結果 |
| --- | --- | --- |
| fmt-check | `mise run fmt-check` | exit 0 |
| check | `mise run check` | exit 0（`cargo check --workspace --all-targets` dev・default 構成） |
| lint | `mise run lint` | exit 0（`cargo clippy --workspace --all-targets -- -D warnings` clean） |
| test | `mise run test` | exit 0（**58 test binaries・1125 passed・0 failed・0 ignored** — `#[ignore]` E2E は `#![cfg(feature = "host")]` で未 compile・default 経路無差分） |
| test-rt | `mise run test-rt` | exit 0（`cargo test --workspace rt_` — **実一致 129・0 failed**・#113 計測 129 と同一で無退行） |
| bench-smoke | `mise run bench-smoke` | exit 0 |
| build | `mise run build` | exit 0（`cargo build --workspace` dev） |
| test-l1-real-plugin | `mise run test-l1-real-plugin` | exit 0 — **3 段全 green**: `cargo build --features host`（fixture cdylib + runner bin 生成）→ `--lib --bins`（CLAP lib 63 + bin 2・VST3 lib 47 + bin 2）→ `--ignored`（CLAP 3 + VST3 3 — `real_adapter_l1_full_series`/`real_adapter_l1_kill_reports_failed_once`/`real_adapter_l1_missing_runner_is_ipc_rejection` 各 ×2 全 pass・実子プロセス） |
| lint-l1-real-plugin | `mise run lint-l1-real-plugin` | exit 0（clippy `--all-targets --features host -D warnings` — bridge・runner bin・E2E target 0 warning） |
| prek | `mise exec -- prek run --all-files` | exit 0（全 hook pass） |
| doc-audit | `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` | exit 0・`blocking: 0`・`findings: []`（gate 時点 + 編集後最終 audit の両方で clean） |
| cargo-tree-adapters | `cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3` | exit 0 — full tree で dev-dep feature union 経由の `clack-host`/`libloading`/`ruviform-plugin-host` は dev 側のみ |
| cargo-tree-adapters-normal | `cargo tree -p ... --edges normal` | exit 0 — **non-optional 木に `clack-host`/`libloading`/`ruviform-plugin-export`/`ruviform-plugin-host` 非出現**（`host` off で新規 dep が prod 木に出ない） |
| cargo-tree-adapters-host | `cargo tree -p ... --features host` | exit 0 — clap: `clack-host`・`clack-extensions/clack-host`（→`libloading`）・`ruviform-plugin-export` 出現 / vst3: `libloading`・`ruviform-plugin-export` 出現・`ruviform-plugin-host` は両 crate で dev-dep のみ — host 指定時のみ出現を確認 |
| cargo-tree-reverse | `cargo tree -p ruviform-plugin-export -p ruviform-plugin-host --edges normal` | exit 0 — 両 crate の normal 木に adapter（plugin-clap/plugin-vst3/plugin-ofx）非出現・**逆向き依存ゼロ** |
| cargo-doc | `env RUSTDOCFLAGS="-D warnings" cargo doc -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --no-deps` | exit 0 — rustdoc warning 0 の機械証明 |
| test-adapters-host | `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` | exit 0 — CLAP lib 63 + bin 2・VST3 lib 47 + bin 2・`#[ignore]` E2E 各 3 が ignored 扱いで除外（`--ignored` なしで実行されないことを確認） |
| test-rt-host | `cargo test -p ... --features host rt_` | exit 0 — CLAP lib 11 matched（`rt_process_drives_fixture_block_allocation_free` 実 pass 含む）・VST3 lib 8 matched・0 failed |
| runner-direct-launch | `target/debug/ruviform-plugin-{clap,vst3}-l1-runner`（引数なし / `RUVIFORM_L1_REAL_PLUGIN_DIR=/nonexistent`） | exit 2（`missing --ruviform-shm-key` usage reject）/ exit 1（missing fixture stderr diagnostic — pre-`run` defined rejection）× 両 runner |

対象 test-spec: `docs/test-specs/adapter-host-bridge-contract-tests.md`・
`docs/test-specs/real-adapter-l1-e2e-tests.md`・
`docs/test-specs/l1-real-plugin-lane-tests.md`（`verified-by` edge —
全て実結果により `approved` 化済み）。

## Risks

- **opt-in lane は実行されなければ腐る** — `#[ignore]` 分離の性質上、
  `l1-real-verify` の手動 dispatch と runbook/README の `mise run
  test-l1-real-plugin` 明記で緩和（spec AC6・design 決定 h と整合）。
  本 task で実測済みのため現時点では lane は健在。
- **macOS leg は未実測** — `l1-real-verify` は workflow_dispatch 専用で
  本環境から dispatch できないため、raw `.dylib` CLAP 解決と VST3
  `bundleEntry` 経路の macOS 初実測は dispatch 時の platform finding
  として残る（ci.yml comment・test-spec Risk 記載どおり — 赤い場合は
  回帰ではなく platform finding として triage）。
- **`delivered >= 8` は heuristic floor** — 実 spawned adapter の
  pacing が極端に遅い環境では defined 動作でも floor 未達の flaky に
  なり得る（T4 IR Risks と同記述 — membership 検査自体は
  deterministic）。
- **既知 flake（`l1_proxy_e2e` load-sensitive 2 test）の継続観測** —
  本 gate では未発生だが load-sensitive 競合の既知リスクとして残る
  （本 task 非起因・再発時は handshake timeout/直列化で別途 triage）。

## Follow-ups

- **`l1-real-verify` の初回手動 dispatch** — 3-OS matrix で macOS leg
  の raw `.dylib`/`bundleEntry` 経路を初実測（platform finding が
  あれば別 task 化）。
- Issue #118 — OFX/video の L1 経路・video wire 確立（AC5 境界明文化
  の追跡先）。
- Issue #123 — `runner_program`/fixture path の `target/<profile>/`
  解決強化（design Deferred と同項目 — 本 E2E ハーネスも同一規約に
  依存）。
- **GitHub merge・issue close は lead の authoritative `gh` のみ** —
  本 task では merge を実施しない（証跡収集と doc 同期に留まる）。

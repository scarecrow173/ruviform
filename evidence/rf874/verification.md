# ruviform#874 検証記録 — 実 Window RT 非干渉 oracle（real-dock lane catalog case 追加）

実施日: 2026-11-23 / worktree: `rf874-rt-oracle` /
branch: `scarecrow173/rf874-rt-oracle`。OS: Windows /
rustc 1.99.1 (58cf8f9c3 2026-10-13)。base SHA `370fe0c6`
（作業時点の HEAD — 変更は uncommitted で実走、run row の
`repo.sha` は変更適用前 SHA を指す点に注意: JSONL の run row
が記録するのは working tree の HEAD SHA であり本 slice の差分を
含まない）。

本 slice は `crates/ruviform-plugin-dock-test` 内の real lane
catalog/entries/case body + doc chain + README + evidence のみ。
変更物: `src/real_catalog.rs`（3 行追加 + 記述追従）・
`src/real_dock.rs`（`real_entries` 3 entry + case body +
`child_attempt_outcome` 抽出 + 配列長/doc table 追従）・
`tests/console.rs`（41 行追従）・`tests/plugin_dock.rs`
（blocked-probe 行数 32 追従 — 範囲拡張承認済み）・
`README.md`（case 表 3 行追記）・`docs/{discovery,specs,designs,
test-specs}/issue-874-rt-oracle.md` 新設・本ディレクトリ。
`Cargo.toml`（条件付き lease）は不採用 — 既存 dep で足りた。
`Cargo.lock`/`mise.toml`/`docs/**/README.md`/host/plugin/ui 系に
差分なし。

## AC1 — interactive run verdict（実走実施）

`mise run test-ui-plugin-dock-real` を interactive Windows
desktop session（`akiyoshi console`・SESSIONNAME=Console）で
実走。lane exit code は **1** — 既知の正直 `Fail`
（`real-dcomp-bandescape`/`real-dcomp-childinv` の
`be_apply_layered_exstyle`/`ci_apply_layered_exstyle` refused —
base JSONL と同一 verdict で本 slice 非起因）によるもので、
判定は各 case verdict で行う。

`evidence/rf631/plugin-dock-real-scenario.jsonl`（lane 出力副作用
として再生成 — 既存 29 行の verdict 変化 0 件、新規 3 行追加）
の新規 case verdict:

| case | verdict | 実測 |
| --- | --- | --- |
| `real-session-rt-samethread` | **pass** | `test_editor` が `Processing` を維持したまま open/set_size/close を real lease 経由で収束。interleave した 5 block（`rt_pre_open`/`rt_post_open`/`rt_attached_1`/`rt_attached_2`/`rt_post_close`）全て `ProcessStatus::Continue` + bit-exact pass-through（input 0.5f32）。`rt_processing_state` = `Some(Processing)`。session events `[Attached{Docked}, Closed]`。`observed.rt = {blocks:5, block_frames:8, wall_ns_max:55800, wall_ns_total:108300}`（observed 記録のみ・verdict 非入力） |
| `real-session-rt-fault` | **pass** | `test_fault`（同一 cdylib・`descriptors()` 解決・plugin_id `e4913a7c…6f2a`）の `rt_alloc` を block 内 `ParamValue` event で arm/disarm。`output[0][0]==200.0` marker が `rt_fault_armed`/`rt_fault_still_armed` のみに出現し、baseline/disarmed/post-disarm は pass-through — negative control が既知の process() 内 heap alloc を捕捉。teardown（StopProcessing/Deactivate/Destroy）は defined 収束 |
| `real-session-rt-boundary` | **unsupported** | cross-thread 同一 instance pump が現行 API で不可能（`create_sendable`→None で `!Send`→Plain slot・`attach_endpoint` pub(crate)・`PluginHost.l0` 非公開）の理由を detail に記録・evidence tag `session:rt:cross-thread-boundary`。deferred: `scarecrow173/ruviform#872` |

3 case の case 行抜粋（run row 付き）は
`plugin-dock-real-rt-cases.jsonl` に残す。

## canonical lane（mise タスク — 正規入口）

| lane | 結果 | 備考 |
| --- | --- | --- |
| `mise run fmt-check` | PASS（exit 0） | diff なし |
| `mise run check ruviform-plugin-dock-test` | PASS（exit 0） | `cargo check --locked --all-targets -p` |
| `mise run lint ruviform-plugin-dock-test` | PASS（exit 0） | clippy `-D warnings` |
| `mise run test ruviform-plugin-dock-test` | PASS（exit 0） | default features 全 test pass |
| `mise run check ruviform-plugin-dock-test --features real-dock` | PASS（exit 0） | real-dock feature compile |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | PASS（exit 0） | real-dock feature clippy |
| `mise run test ruviform-plugin-dock-test --features real-dock` | PASS（exit 0） | lib 17 + console 14 + plugin_dock 5 passed — `real_catalog_matches_entries`（32 行 pairwise）・`blocked_reports_fail_closed`（32）・`list_include_real_appends_real_lane_rows`（41 = 9+32）・`blocked_probe_rows_carry_no_session_scoped_fields`（32）全て ok |
| `mise run build ruviform-plugin-test-vst3` | PASS（exit 0） | lane 前提の test plugin cdylib |
| `mise run test-ui-plugin-dock` | PASS（exit 0） | headless lane 非回帰（`evidence/rf631/plugin-dock-scenario.jsonl` 再生成） |
| `mise run test-ui-plugin-dock-console` | PASS（exit 0） | `--list`/`--run` 非回帰（`plugin-dock-console.jsonl` 再生成） |
| `mise run test-ui-plugin-dock-real-headless` | PASS（exit 0） | `--blocked-probe` — 32 行全て `NotRun`+`observed.environment_blocked`・`environment_kind=real-window` 0 件の fail-closed を実証（`plugin-dock-real-headless.jsonl` 33 行: run row + 32 case） |
| `mise run test-ui-scenario-contract` | PASS（exit 0） | fake 17 + gpui 7 passed |
| `mise run check-ui-driver-boundary` | PASS（exit 0） | dep-graph/signature/allowlist 全 pass — driver boundary 規約維持（oracle は観測のみ・GUI driver 本体無変更） |
| `mise exec -- prek run --all-files` | PASS（exit 0） | 全 hook（whitespace/yaml/toml/eol/bom/key/actionlint） |
| `mise run test-ui-plugin-dock-real` | exit 1（想定） | interactive 実走 — 上記 AC1 参照。既知 Fail 2 件は base と同一 verdict で非回帰なし |

## AC 対応

- **AC1**（catalog 追加 + interactive run verdict 記録）:
  達成 — 3 case を `REAL_CATALOG`/`real_entries` に追加し、
  interactive 実走で `pass`/`pass`/`unsupported` を記録。
- **AC2**（観測対象・測定方法・閾値の記録）:
  `docs/specs/issue-874-rt-oracle.md` R4（verdict lattice）/R5
  （観測対象と測定方法）/R6（閾値設計 — verdict 入力は defined
  観測契約のみ・`wall_ns` は observed 記録のみ）に確定記録。
- **AC3**（`mise run test` 全 OS CI pass）: ローカルでは
  `mise run test ruviform-plugin-dock-test`（default +
  `--features real-dock`）で担保 — merge 後 main SHA の CI
  green は Coordinator が確認（本 lane は manual/opt-in で
  CI-required gate ではない）。

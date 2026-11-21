---
id: "DISC-rf388-virtual-first-ac-01"
type: "discovery"
status: "approved"
title: "Issue #388 Virtual-first AC 照合 — canonical real-dock lane 証拠 inventory・case→AC 対応根拠・RT 非干渉 rationale"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-388-virtual-first-ac.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/specs/issue-857-pixel-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-388-virtual-first-ac.md"
    - "docs/test-specs/issue-388-virtual-first-ac.md"
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/specs/issue-857-pixel-occlusion.md"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
    - "evidence/rf857/run-record.md"
    - "evidence/rf388-virtual-first-ac/run-record.md"
    - "evidence/rf388-virtual-first-ac/ac-reconciliation.md"
    - "evidence/rf388-native-surface-slot/report.md"
    - "evidence/rf388-native-dock-windowing/report.md"
    - "evidence/rf388-os-l1-ci/report.md"
    - "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-388-virtual-first-ac.md"
  derived-by: []
  verifies: []
  verified-by:
    - "evidence/rf388-virtual-first-ac/ac-reconciliation.md"
metadata:
  issue: 388
---

# Issue #388 Virtual-first AC 照合 — canonical real-dock lane 証拠 inventory

canonical real-dock lane（`evidence/rf631/plugin-dock-real-scenario.jsonl`）
の verdict 集合を Issue #388 の現行 AC 各行と突合するための証拠
inventory・case→AC 対応根拠・RT 非干渉 rationale の記録。AC 各行の
disposition 決定は `docs/specs/issue-388-virtual-first-ac.md`、AC 行 ×
証拠の突合表は `docs/test-specs/issue-388-virtual-first-ac.md`、本記録の
写し要約は `evidence/rf388-virtual-first-ac/` を参照。本 slice は
**docs/evidence のみ** — lane 再実走・verdict 意味論変更・Issue 本文
編集は行わない。

## 主証拠 run 条件（canonical lane）

| 項目 | 値 |
| --- | --- |
| 証拠 JSONL | `evidence/rf631/plugin-dock-real-scenario.jsonl`（canonical 実出力。本 slice では複製・改変しない） |
| run コマンド | `mise run test-ui-plugin-dock-real`（interactive Windows、CI 非収載 lane — Run URL は存在しない） |
| run_id / scenario | `rf631-plugin-dock-real` / `rf631.plugin-dock-real` |
| ruviform worktree SHA | `b27fb6a1f63970479f5c82872f9ba0391f741ed5`（run row `repo.sha` と各 case row の `environment.repo_sha` で一致。main 歴の ancestor — `b27fb6a1` = PR #862 merge、現 HEAD `68c0c269` の 3 commit 前） |
| 解決された gpui rev | `b3144f286b942fbb5f478da73db13a826cef054a`（tag `v0.2.2-rf2`、Cargo.lock。`noredirectionbitmap: false` で DComp on を実証） |
| OS / arch | windows / x86_64、interactive desktop session（全 case row で `environment_kind = "real-window"`） |
| scale_factor / DComp | 1.0 / `gpui_disable_direct_composition = null`（DComp on） |
| display adapter | NVIDIA GeForce RTX 4070 SUPER（DISPLAY1 primary、multi-monitor） |
| plugin fixture | `test_editor`（`ruviform_plugin_test_vst3.dll`、sha256 `0bb0de2017ce7618fa679952387cba3603efb753e5feb5c292cb9a6e5f4b5fda`）— run row `plugin_path_env = RUVIFORM_PLUGIN_DOCK_TEST_VST3` |
| 判定 oracle | 画素系は `win32-bitblt-screen` の screen-composited `EDITOR_COLOR` green 比率 `green_fraction >= 0.5`。editor 操作系は非画素 oracle（session events・child HWND 契約/状態・OS レベル効果 oracle — `real_catalog.rs` doc 参照） |
| window 配置 | 4 window mount（被測定 w1 + sentinel w2–w4）。w1 root `Zed::Window` → container `RuviformSlotContainer` → child `RuviformTestEditor` の実 HWND 階層 |

JSONL は PR #864（`ace2dd56`、real-editor-popup harness）が最後に
再生成したもので、実走は `rf631-popup-harness` worktree 上の
`b27fb6a1`（popup harness merge 前の base）で行われた。verdict 集合は
`evidence/rf857/run-record.md`（`real-dcomp-pixel` 処遇実走）の結果と
整合する。

## verdict 集合の実測

JSONL の case rows は **29 件**で、内訳は
**24 pass + 2 fail + 3 unsupported**（case row のトップレベル
`verdict` フィールドを集計 — `observed`/`evidence` 内の副次 verdict
（step 結果・`note`）は集計対象外）。

- pass（24）: `real-window-mount`/`real-native-slot`/
  `real-editor-attach`/`real-capture`/`real-dcomp-pixel`/
  `real-dcomp-foreground`/`real-dcomp-present`/`real-dcomp-uncovered`/
  `real-dcomp-rasterfix`/`real-multiwindow-focus`/`real-os-input`/
  `real-command-receipt`/`real-window-dpi`/`real-window-close`/
  `real-editor-resize`/`real-editor-redock`/`real-session-terminate`/
  `real-session-degraded`/`real-session-recovery`/`real-session-timeout`/
  `real-editor-visibility`/`real-editor-input`/`real-editor-focus`/
  `real-editor-popup`
- fail（2）: `real-dcomp-bandescape`/`real-dcomp-childinv`
- unsupported（3）: `real-window-ime`/`real-session-crash`/
  `real-session-hang`

前回の pinned-dep 計測（`evidence/rf532/plugin-dock-real-pinned.jsonl`
`57c4619b`、28 case = 22 pass + 3 unsupported + 3 fail）からの差分:
`real-dcomp-pixel` が fail→pass（class B 処遇 rf#857、#861 merge）、
`real-editor-popup` が新規追加（rf#631 popup harness、#864 merge）。
fail→pass 改善と pass case 追加のみで、pass→fail の回帰は存在しない。

## case→AC 対応根拠

AC1（Virtual-first 移行受入）と AC6（Windows GUI-L2/EXT-L3
Required）は同一の証拠集合を共有する。各検証項目と case の対応:

| AC 検証項目 | 対応 case / 証拠 | verdict |
| --- | --- | --- |
| 実画素 | `real-dcomp-pixel` — screen green `0.9999941167472672`（`win32-bitblt-screen`、1278x798、green 1,019,838/1,019,844 px、capture_sha256 `89401018…`）。`occlusion_exstyle` root `noredirectionbitmap: false` で DComp on 実証。`evidence/rf857/run-record.md`（class B 処遇の確定記録）＋ canonical `b27fb6a1` での再 pass | pass |
| Clip | `real-dcomp-uncovered`/`real-dcomp-present` = pass + `real-dcomp-rasterfix` pass（clip-partition 経路）+ PR #501 の ancestor clip 実績（`evidence/rf388-native-dock-windowing/report.md` — `clip_overhang_region`/`clip_direct_region` region 完全一致・`clip_direct_hit_test`） | pass |
| Focus | `real-editor-focus`/`real-multiwindow-focus` = pass（`GetWindowTextW` event-log title token oracle / multi-window focus） | pass |
| Resize | `real-editor-resize` = pass（child `GetWindowRect` geometry oracle） | pass |
| Dock/ReDock | `real-editor-attach`（`child_enumerated` で実 child HWND 確認・`Attached { mode: Docked }` event）/`real-editor-redock`（attach-generation title advance oracle）= pass | pass |
| 実 Window / 実 child HWND / 実 Plugin ABI | 全 case row `environment_kind = "real-window"` + `test_editor`（`crates/ruviform-plugin-test-vst3` — `createView("editor")` が実 `IPlugView` を返し Windows `attached` で host 提供 parent 下に `WS_CHILD` HWND を生成する正規 ABI Test Plugin）+ `child_enumerated`/`occlusion_hit_test` の `RuviformTestEditor` class HWND 記録 + `plugin-artifact` sha256 | pass（証拠属性） |
| RT 非干渉 | lane に明示 RT case なし — 下記「RT 非干渉 rationale」節 | rationale 記録 |
| DComp 既知 Fail は修正・再実走まで Fail | `real-dcomp-bandescape`/`real-dcomp-childinv` = fail を維持（class C — 下記「残存 fail/unsupported の処遇」節）。`real-dcomp-pixel` は修正・再実走済みで pass | fail 維持（honest 記録） |
| Fake Surface / Floating だけで Pass にしない | driver `real-os`・`environment_kind = "real-window"` のみ。Fake/Offscreen/Floating 由来の verdict は存在しない | 充足（証拠属性） |

### #389 lifecycle 共存行の扱い

`real-session-terminate`/`real-session-degraded`/`real-session-recovery`/
`real-session-timeout` の 4 pass 行は、catalog
（`crates/ruviform-plugin-dock-test/src/real_catalog.rs` doc）が
「Issue #389 lifecycle remainder」と明記する lane 共存行であり、
`real-editor-popup` は「Issue #630 AC4 shared remainder」である。
これらは #388 側では**補助 context** として扱い（host が editor の
terminate/degraded/recovery/timeout に耐える実 Window 観測）、#389
や #630 の AC 充足を代行主張しない。

## 残存 fail/unsupported の処遇根拠

### class C: `real-dcomp-bandescape` / `real-dcomp-childinv` — 既知 Fail 継続

canonical JSONL でも fail を維持している。`real-dcomp-childinv` の
`detail`: "editor lifecycle verified but the child-inventory probe did
not complete: `ci_apply_layered_exstyle` refused (win32 error
unknown)"。step 列に `ci_apply_layered_exstyle:fail`・
`ci_apply_layered_lwa:fail` が記録されている（`real-dcomp-bandescape`
も同型）。

処遇の正本は `docs/specs/issue-532-dcomp-residual.md` の確定済み
決裁: 両 case は「`WS_EX_LAYERED` を foreign window へ適用して
presentation 経路を切り替えられるか」を問う診断 probe で、
`SetWindowLongPtrW` silent no-op・`SetLayeredWindowAttributes`
error 87・`WS_CHILD` 状態では latch せず top-level 化後のみ適用
（`latched_as_child=false`/`latched_as_toplevel=true`）・
`WS_CHILD|WS_EX_LAYERED` 新規作成 `0x80070006` により**適用不能が
確定**（class C）。診断目的は達成済みで「既知 Fail として継続」が
決裁されている。本 slice はその決定を引用するのみで、再切り分け・
verdict 昇格・`Unsupported` re-baseline は行わない。

### unsupported 3 行 — capability 境界の honest 記録

| case | required | `detail`（JSONL 正本） | 処遇根拠 |
| --- | --- | --- | --- |
| `real-window-ime` | `ime` | "unsupported capability: Ime" | catalog doc「`Ime` unclaimed, Issue #389 scope」— IME adapter API が未実装で capability が claim されていない技術的非対応。#389 側の追跡は `docs/specs/issue-389-ime-verbs.md`（approved） |
| `real-session-crash` | `native-plugin-dock` | "the in-process lane cannot survive the fixture's process-killing crash fault (test_fault abort/exit — the plugin cdylib shares the host process) and no out-of-process plugin host exists yet; L1EditorOps refuses every editor op with the defined Unsupported" | catalog doc「Issue #389 lifecycle remainder」— in-process lane では fixture のプロセス殺害 fault を lane が生き残れない構造的限界。out-of-process host は #389 L1 領域 |
| `real-session-hang` | `native-plugin-dock` | "the in-process lane cannot survive the fixture's wedging hang fault (test_fault hang — the plugin call never returns and shares the host process) and no out-of-process plugin host exists yet; L1EditorOps refuses every editor op with the defined Unsupported" | 同上（wedging hang fault。`session:hang:in-process-lane-nonviable` の evidence tag 付き） |

3 行とも POLICIES.md §3 の `Unsupported`（根拠を伴う技術的非対応）に
該当し、Pass/Fail への読み替えは行わない。`observed: null`・
`evidence` には `session:*:in-process-lane-nonviable` tag のみ（IME は
空）で、lane 自体は健全に完走している。

## RT 非干渉 rationale（AC1/AC6 の「RT」項）

lane の case 一覧に明示的な RT case は存在しないため、#532 residual
chain と同型の rationale 記録で対応付ける:

- **作用面は UI thread の GUI 経路のみ。** real-dock lane が検証する
  のは NativeSurfaceSlot の宣言・bind・lease・session.open/close・
  child HWND の生成/geometry/visibility/focus/redock・DComp
  presentation 経路・window イベントであり、すべて scenario caller
  thread / UI thread 上の window・compositor 処理。audio RT path
  （process callback・graph・buffer 交換）と共有する state・lock・
  thread は存在しない。
- **lane は audio engine を駆動しない。** `test_editor` は
  parameterless pass-through の editor fixture であり、lane は
  `createView`/`attached`/`setFrame` 等の GUI verb のみを行使し、
  plugin の `process()`・transport を実行しない。RT 側境界を touch
  する変更を本 slice では扱わない。
- **session 生存系の補助観測。** `real-session-terminate`/
  `degraded`/`recovery`/`timeout`（全 pass）は host が editor
  session の終了・劣化・回復・timeout へ耐えて生き残る実 Window
  観測（`host_survives` step・`Terminated { cause: HostLost }` event）
  であり、GUI 経路の健全性の傍証として記録する（#389 lifecycle
  の正式充足ではない）。
- **既存 RT 証拠との対応付け。** headless RT 契約 lane（`test-rt`、
  strict `rt_` contract 166 test）は `evidence/rf388-os-l1-ci/report.md`
  の対象 SHA `e93c438a`・run 43428172820 で Pass を確認済み。ただし
  同 SHA は現 HEAD ではない（後続 merge あり）ため、close 時点 SHA
  での RT 非回帰の再確認は AC7 の coordinator gate に含める。
  実 Plugin GUI hang 時の RT 影響は #389 共同検証領域であり、
  crash/hang fault の実走は上記のとおり `Unsupported`。
- `thread_affinity` step が「session.open/attached は container HWND
  の pump-owning thread ではなく scenario caller thread で実行」という
  Issue #389 D1 deviation を各 case で `note` 記録している — Pass
  基準ではなく観測としての正直な記録であり、#389 側の判断事項。

## 供給物 Issue の状態（#630/#631）

| Issue | 状態 | #388 が消費する供給物 |
| --- | --- | --- |
| #630 | OPEN（残 AC あり: consumer oracle・外部 gate 等） | `test_editor`（VST3 実 ABI + 実 `WS_CHILD` `IPlugView`、CLAP は `clap.ext.gui`）— main 統合済み |
| #631 | OPEN（残 AC あり） | `rf631.plugin-dock-real` lane・pixel/receipt oracle・JSONL 証拠 schema — main 統合済み |

両 Issue の残 AC は本照合を止めない — #388 が必要とする供給物は
main 上で実在し、canonical JSONL がその実走結果を記録している。

## Limitations

- 本 slice は lane 再実走を行っていない（docs-only disposition —
  canonical JSONL `b27fb6a1` を正本とする）。close 時点 SHA での再実走
  要否は coordinator gate。
- run は CI 非収載の interactive lane であり Run URL は存在しない。
  証拠はリポジトリ内 JSONL と `evidence/rf857/run-record.md`。
- `real-dcomp-pixel` の pass は #857 の de-occlusion arm
  （`deocclusion_apply_raise` → `deocclusion_apply_topmost` →
  `deocclusion_restore`）を伴う採取手順で成立しており、外部
  topmost window 遮蔽が残る環境では fail-closed gate が `not_run`
  に退避する（遮蔽時に外部画素を Pass/Fail へ読み替えない実走
  実証済み — `evidence/rf857/run-record.md`）。
- AC7（macOS/Linux L1）の close 時点対象 SHA における 3OS CI 実走
  証拠は本 slice の範囲外 — 前回照合（SHA `e93c438a`・run
  43428172820）は旧 SHA の実績として記録し、判定は `deferred`。
- hwnd → window_id の一意対応は JSONL に記録されていない（w2–w4
  sentinel の同 geometry は #532 chain で記録済みの既知の制限）。

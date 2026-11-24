# Issue #631 — 受入条件（AC）棚卸し表

- Issue: https://github.com/scarecrow173/ruviform/issues/631
- 作成: umbrella AC disposition census（close-out 判定材料）、2026-11-24
- 判定語彙: POLICIES.md の `Required/Deferred/NotApplicable` ×
  `Pass/Fail/NotRun/Unsupported`。実 OS/実 Window・実 plugin ABI の
  成功は canonical ledger の実測値のみを根拠とし、Fake/Headless
  成功を実 GUI の Pass に読み替えない。
- AC 行番号は Issue 本文「### 受入条件」チェックリストの出現順（7 行）。
- 本表は ledger 突合の生データ表。disposition 決定と残件分類は
  `docs/specs/issue-631-ac-inventory.md`、case→AC 対応根拠は
  `docs/discovery/issue-631-ac-inventory.md`、AC×証拠突合表は
  `docs/test-specs/issue-631-ac-inventory.md` を参照。

## 照合した証拠

| 証拠 | 種別 | repo_sha / 条件 | 内容 |
| --- | --- | --- | --- |
| `evidence/rf631/plugin-dock-real-scenario.jsonl` | canonical real-window ledger（read-only） | `a0e3456405092605519077988e012130cf82d32d` | `lane=test-ui-plugin-dock-real`・`run_id=rf631-plugin-dock-real`・Windows interactive・32 case = **32 pass**（DComp on・gpui `v0.2.2-rf2`） |
| `evidence/rf631/plugin-dock-scenario.jsonl` | headless ledger（本 slice で HEAD 再生成） | `06a5a9e0c0ca244bb2b10569a950ef3293ef584f` | `lane=test-ui-plugin-dock`・`run_id=rf631-plugin-dock`・9 case = 7 pass + `editor-attach` unsupported + `dock-pixels` not_run |
| `evidence/rf631/plugin-dock-console.jsonl` | headless console ledger（本 slice で再生成） | `06a5a9e0c0ca244bb2b10569a950ef3293ef584f` | `lane=plugin-dock-console`・同一 9 case・exit 0（Fail verdict なし） |
| `evidence/rf631/plugin-dock-console-list.jsonl` | catalog 投影（本 slice で再生成・補助証拠） | — | 9 catalog row（`runnable: true` 全件） |
| `evidence/rf631/plugin-dock-console-list-include-real.jsonl` | catalog 投影 + real row（本 slice で再生成・補助証拠） | — | 41 row（headless 9 + real 32。crash/hang/rt-boundary の `required` に `out-of-process-plugin-host` を現行化） |
| `evidence/rf631/plugin-dock-real-headless.jsonl` | fail-closed 実証 ledger（read-only） | `a0e3456405092605519077988e012130cf82d32d` | `--blocked-probe`・32 row 全て `not_run` + `environment_blocked`・`environment_kind=real-window` 主張なし |
| `evidence/rf631/plugin-dock-lab.jsonl` | Test Lab lane 記録（read-only） | `eafe0a08b71bd9a74566ae6678c3e0683fce1421` | `lane=plugin-dock-lab`・headless cell 実行記録 |
| `evidence/rf631/real-window-surface.jsonl` | real-window lifecycle 記録（read-only） | — | `window-close-lifecycle`/`window-dpi-geometry` pass |
| `.github/workflows/ci.yml` | 既存 CI 定義（read-only 参照） | HEAD `06a5a9e0` | `ui-verify`/`ui-verify-macos`/`ui-gpui-verify`/`ui-gpui-verify-macos` に headless GUI test seam が収載、real-window lane は workflow_dispatch 専用の opt-in |

## AC 棚卸し

| # | 受入条件（Issue 転記・要約） | verdict | 理由 | 根拠 |
| --- | --- | --- | --- | --- |
| 1 | Windows実Window起動・入力・Receipt・画素Capture・実アプリState照合、2Window Focus/Close/Undo・DPI/IMEをCaseごとに判定できる | **Pass**（Undo は ruviform catalog 対象外の記録済み判断） | `real-window-mount`（起動）・`real-os-input`（OS 入力注入）・`real-command-receipt`（Command→Receipt）・`real-capture`（画素）・`command-receipt`/`real-command-receipt` の `semantic_tree`+`receipts`（実アプリ state 照合）・`real-multiwindow-focus`（2Window Focus）・`real-window-close`（armed Veto→Approve close）・`real-window-dpi`（live-geometry 判定）・`real-window-ime`（IMM32 composition+commit）が全て pass。Undo は `docs/specs/issue-631-real-row.md` が consumer 意味論として ruviform catalog 非収載を決定（`NotApplicable` 相当の記録済み判断） | `plugin-dock-real-scenario.jsonl` @ `a0e34564`（32 pass）。Undo 判断: `docs/specs/issue-631-real-row.md` Out of Scope 節 |
| 2 | 同 Runner から Ruviform Test Lab GUI・Synvelis 実 GPUI Shell・AudioLens 実 GUI へ Driver-neutral にシナリオ投影、製品固有 Oracle は各 Repo | **部分 Pass**（ruviform 側 Test Lab 実在・consumer 投影は external-owner） | `plugin-dock-lab`（feature `lab-gui`）が HEADLESS_MATRIX + real_catalog 統合 catalog を `ruviform-ui` ViewSpec で実 GPUI window 化して存在（dogfooding で defect 発見・修正済みの記録あり）。Synvelis `synvelis#114`/`#176`・AudioLens `audio-lens#127` への実投影は consumer repo 責務で external-owner | `plugin-dock-lab.jsonl`・`docs/{specs,designs,test-specs,discovery}/issue-631-test-lab.md`。consumer 投影 = 各 consumer Issue（OPEN） |
| 3 | Test Plugin Native Editor の実 Child HWND を NativeSurfaceSlot へ結び、既知 DComp Fail を再現した上で画素・Input・Resize・Focus・ReDock の Pass 判定が可能。未解決問題を Test Runner 整備だけで解消済みにしない | **Pass** | `real-native-slot`（NativeSlot declare→bind→lease_parent→ParentSlotLease）・`real-editor-attach`（`test_editor` 実 `IPlugView` が実 `WS_CHILD` HWND を生成・`child_enumerated` で観測）・`real-dcomp-pixel`（screen-composited 画素で editor green 可視）・`real-dcomp-{present,uncovered,bandescape,childinv,rasterfix,foreground}`（DComp 既知 Fail 再現・二分 probe 群）・`real-editor-{input,focus,resize,redock}` 全て pass。class C（as-child `WS_EX_LAYERED` 拒否）は #917 re-scope により `expected_outcome` note として確定機序を記録 — 未解決を解消済みとは記録していない（拒否の `applied`/`win32_error`/`after` 実測値は維持） | `plugin-dock-real-scenario.jsonl` @ `a0e34564`。`docs/specs/issue-532-dcomp-residual.md`（Issue #917・#532 CLOSED） |
| 4 | Screenshot/Pixel diff/semantic tree/Command+Receipt/OS and GPU/Case Seed/Window & Plugin hash、Repo/SHA・Ruviform resolved SHA を保存し、Pass/Fail/NotRun/Unsupported を異なる軸で記録 | **Pass** | case row が `environment`（os/arch/scale_factor/`gpui_disable_direct_composition`/`display_adapters` GPU census）・`case_seed`・`window_identities`（各 window の id/title/geometry/sha256）・`plugin_artifact`（name/version/vendor/sha256）・`repo.sha` を全行に記録。`command-receipt`/`real-command-receipt` の `observed.semantic_tree`+`receipts`、`real-dcomp-pixel` の capture sha256・`pixel-artifacts` 経路。`verdict`（pass/fail/not_run/unsupported）は `required`/`capabilities` と別軸の top-level field | `plugin-dock-real-scenario.jsonl` @ `a0e34564`（environment provenance 全 32 case に記録）・`plugin-dock-scenario.jsonl` @ `06a5a9e0`。AC4 evidence-fields slices（`docs/specs/issue-631-evidence-fields.md`・`issue-631-gpu-field.md`） |
| 5 | Timeout/OS Session なし/Driver Provision 不可は `NotRun` で fail-closed。非提供 Capability は `NotApplicable`、macOS/Linux GUI は Deferred | **Pass** | `--blocked-probe` ledger が real 32 row・CLAP 14 row とも全件 `not_run` + `observed.environment_blocked`（`environment_kind=real-window` を主張しない）を bin 自己検査（違反時 exit 1）。capability 未実証 host では capability-gated `Unsupported`/`NotRun` を記録（`OutOfProcessPluginHost` session-probe・`Ime` claim 条件）。timeout は `real-session-timeout` が定義済 `Failed{Timeout}` event として pass 判定。macOS/Linux GUI は POLICIES §2 どおり Deferred | `plugin-dock-real-headless.jsonl`・`plugin-dock-real-clap-headless.jsonl`（read-only）。`mise.toml` `test-ui-plugin-dock-real*` task 定義（fail-closed exit 0 設計） |
| 6 | GUI Runner/Test Lab 自身が Ruviform 共通 UI/API で作られ、GPUI 固有型を Consumer へ漏らさない | **Pass** | `check-ui-driver-boundary` が CI `ui-gpui-verify` job に収載され、cargo-metadata 依存グラフ + `syn` 公開 signature 漏出を機械検査（`boundary.json` ruleset）。`plugin-dock-lab` は `ruviform-ui` ViewSpec 語彙で構築（共通 API dogfooding、製品意味論を ruviform へ持ち込まない方針は Issue 詳細どおり） | `.github/workflows/ci.yml` `ui-gpui-verify`（step: Driver boundary guard）・`docs/specs/issue-390-driver-contract-lab.md`・`docs/test-specs/issue-390-driver-boundary-lab.md`・`docs/designs/issue-631-test-lab.md` |
| 7 | 3OS OS-L1 GUI Test seam を既存 CI に統合し、Windows 実 Window opt-in Lane を通常 Daily Full CI の PR 必須条件にしない | **Pass**（CI 構造として充足） | headless GUI test seam（`test-ui-headless`/`test-ui-components`/`test-ui-viz`/`test-ui-binding`/`test-ui-guitest`/`test-ui-showcase`/`test-ui-webview`/`check-ui-driver-boundary`/`test-ui-driver-contract`/`test-ui-parity`/`test-ui-native-slot`/`test-ui-component-view`/`test-ui-golden`/`test-ui-scenario-contract`/`test-ui-gpui`/`test-ui-consumer-observation`）が `ui-verify`（ubuntu+windows matrix）・`ui-verify-macos`・`ui-gpui-verify`・`ui-gpui-verify-macos` で 3OS 実走。`test-ui-plugin-dock*`/`test-ui-real-scenario`/`test-ui-plugin-dock-lab` は manual/opt-in で CI-required gate ではない。ci.yml 自体が `workflow_dispatch` 専用で PR trigger を持たず、real-window lane は PR 必須条件に非該当 | `.github/workflows/ci.yml`（verify lane 群 + `ci-summary` needs 一覧）。`docs/development-environment.md` lane 表 |

## verdict 集計

- AC1: **Pass** / AC2: **部分 Pass**（ruviform 側 done・consumer 投影 external-owner）/
  AC3: **Pass** / AC4: **Pass** / AC5: **Pass** / AC6: **Pass** /
  AC7: **Pass**（CI 構造として充足 — 対象 SHA の CI 実走証拠は merge 後 gate）
- Issue checkbox（7 行）は全て `[ ]` のまま — 最終 `[x]` 適用は Coordinator 判定。
- 残件は spec 側「残件の 3 類分類」を参照（settled-Unsupported / implementable /
  externally-gated）。

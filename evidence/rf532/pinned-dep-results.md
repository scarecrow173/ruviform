# Issue #839 gpui fork pin bump — pinned-dep 再検証記録

fork tag `v0.2.2-rf1` → `v0.2.2-rf2`（`scarecrow173/gpui`
`b3144f286b942fbb5f478da73db13a826cef054a` — `patch: composite
hosted child HWNDs through the DComp flip plane`）への manifest
pin 更新後、`[patch]`/path override を一切使わず依存解決した
状態での real lane 再実測記録。slice1 の patched 実測は
`evidence/rf532/clipfix-results.md` を参照。

## pin 解決の確認

- `crates/ruviform-ui-gpui/Cargo.toml`: `gpui = { git =
  "https://github.com/scarecrow173/gpui", tag = "v0.2.2-rf2",
  version = "=0.2.2", optional = true }`（tag 行のみ変更）。
- `Cargo.lock`（`mise exec -- cargo update -p gpui` で再生成、
  差分は gpui source 行のみ）:
  `source = "git+https://github.com/scarecrow173/gpui?tag=v0.2.2-rf2#b3144f286b942fbb5f478da73db13a826cef054a"`
- 補助証拠: `cargo tree -p ruviform-ui-gpui --features gpui` が
  `gpui v0.2.2 (https://github.com/scarecrow173/gpui?tag=v0.2.2-rf2#b3144f28)`
  を解決。mise タスクの build 出力にも同一 source 行が出る
  （`Compiling gpui v0.2.2 (...?tag=v0.2.2-rf2#b3144f28)`）。
- gpui clone（`G:/repositry/audio-products/gpui`、
  `scarecrow173/gpui` fetch 済み）で `git rev-list -n1
  v0.2.2-rf2` = `b3144f286b942fbb5f478da73db13a826cef054a` を
  確認。root `Cargo.toml` に `[patch]`/path override は存在
  しない。

## Run 条件

| 項目 | 値 |
| --- | --- |
| run コマンド | `mise run test-ui-plugin-dock-real`（canonical） |
| ruviform worktree SHA | `57c4619b355f68f57c2a6aac560bf9b5a6e9cdcd`（pin bump commit、JSONL `environment.repo_sha` と一致） |
| 解決された gpui rev | `b3144f286b942fbb5f478da73db13a826cef054a`（tag `v0.2.2-rf2`） |
| OS / arch | windows / x86_64、interactive desktop session（`environment_kind = "real-window"`） |
| scale_factor / DComp | 1.0 / `gpui_disable_direct_composition = null`（DComp on） |
| display adapter | NVIDIA GeForce RTX 4070 SUPER |
| 出力 | `evidence/rf532/plugin-dock-real-pinned.jsonl`（canonical 実出力 `evidence/rf631/plugin-dock-real-scenario.jsonl` のコピー） |
| 判定 oracle | `issue-532-rasterfix` と同一: `rf_screen`（`win32-bitblt-screen`）screen-composited `EDITOR_COLOR` green 比率、`green_fraction >= 0.5` |

## 実測結果（pinned dep、DComp on）

`real-dcomp-rasterfix` — **Pass**（`fix_arm = "baseline"` —
治療 arm 不使用の docked-in-place 達成）

| arm | screen green |
| --- | --- |
| baseline（docked-in-place、治療 arm 不使用） | **0.9999941167472672** |
| pump / reparent / layered | 未到達（`rf_arms` skipped: "baseline screen green already meets the threshold"） |

slice1 patched 実測の `0.999994` を pinned dep で再現。
`rf_baseline_state` で container/child ともに exstyle `0x0`
（layered/topmost/noredirectionbitmap 全て false）— child の
style/parentage 変更なし。`capture_sha256 =
7cbc4372e4a3dc6e4be14817570b4cfd4ce68328efe7e7181147d333a7f1b0f7`。

## Lane verdict 一覧（pinned-dep run）

| case | verdict | 備考 |
| --- | --- | --- |
| `real-dcomp-rasterfix` | **pass** | 上表 — 本 slice の AC 中核 |
| `real-dcomp-pixel` | fail（既知・別 class） | 別 lane window 遮蔽の診断 case — "known GPUI DComp topmost occlusion"。slice1 と同一 verdict・同一原因 |
| `real-dcomp-bandescape` | fail（既知・別 class） | `be_apply_layered_exstyle` refused — foreign child への `WS_EX_LAYERED` 適用可否の診断 |
| `real-dcomp-childinv` | fail（既知・別 class） | `ci_apply_layered_exstyle` refused — 同上 |
| その他 21 case | pass | `real-window-*` / `real-editor-*` / `real-session-*` / `real-dcomp-foreground`/`present`/`uncovered` / `real-capture` / `real-native-slot` / `real-multiwindow-focus` / `real-os-input` / `real-command-receipt` |
| `real-window-ime` / `real-session-crash` / `real-session-hang` | unsupported | 既知（adapter API なし / 未実装 fixture） |

verdict 集合: **24 pass + 3 unsupported + 3 既知 fail** —
slice1 の patched run と同一集合（AC6 維持）。lane exit 1
は `real-dcomp-pixel`/`bandescape`/`childinv` の honest Fail
による設計どおりの終了。

## 検証 lane 実施記録（pinned dep）

| lane | 実行 | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 実施 | pass（exit 0） |
| `mise run check-ui-gpui` | 実施 | pass — pinned `v0.2.2-rf2`（`#b3144f28`）で gpui feature 経路 compile |
| `mise run test-ui-gpui` | 実施 | pass — 78 lib + 22 consumer_observation + 4 lab_story + 7 scenario_contract + 2 windowed_uitest + 1 golden_capture（real-window 3 件は desktop 要で ignore — 設計どおり） |
| `mise run test-ui-plugin-dock` | 実施 | exit 0 |
| `mise run test-ui-plugin-dock-console` | 実施 | `--list` + `--run` exit 0 |
| `mise run test-ui-plugin-dock-real-headless` | 実施 | 全行 `NotRun` + `environment_blocked` の fail-closed mapping 自己検証、exit 0 |
| `mise run test-ui-plugin-dock-real` | 実施 | 上記 verdict 表 — `real-dcomp-rasterfix` Pass（`fix_arm = "baseline"`、screen `0.9999941167472672`）、3 既知 fail で lane exit 1（設計どおり） |
| `mise run check-ui-driver-boundary` | 実施 | pass（dep-graph / signature / allowlist） |
| `mise run lint-ui-gpui` | 実施 | **fail（main 既存 issue・本 slice 対象外）**: `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs:1644` `clippy::cast_sign_loss`（`len as usize`）— slice1 と同一の既存コード由来、gpui 変更とは無関係。ruviform 側コードは本 slice の編集範囲外のため未修正で記録 |

## 残課題

- `real-dcomp-pixel`/`bandescape`/`childinv` の既知 Fail は
  別 class（別 lane window 遮蔽、`WS_EX_LAYERED` 適用可否の
  診断）として残る — 本 fix の対象外。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1`（HWND flip swapchain）
  経路の child 提示は対象外のまま（slice1 補助 run:
  baseline 0.000 → reparent arm 1.000 で Pass — fix は
  DComp path のみに効く）。
- `lint-ui-gpui` の `cast_sign_loss` 修正は main 既存問題
  として別途対応。

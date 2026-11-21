# Issue #388 Virtual-first AC 照合 — canonical real-dock lane run 記録

Issue #388 の AC 照合 disposition slice で突合に用いた canonical
run の引用記録。本ディレクトリは照合結果の記録であり、既存 JSONL
（正本 `evidence/rf631/plugin-dock-real-scenario.jsonl`）の複製は
置かない。doc chain: `docs/discovery/issue-388-virtual-first-ac.md` /
`docs/specs/issue-388-virtual-first-ac.md` /
`docs/test-specs/issue-388-virtual-first-ac.md`。
**Issue #388 は open のまま**（本 slice は `Refs` のみ。Close 判定・
checkbox 適用は Coordinator）。

- Date: 2026-11-21
- 証拠正本: `evidence/rf631/plugin-dock-real-scenario.jsonl`
  （canonical 実出力。本 slice では複製・改変しない）
- run: `mise run test-ui-plugin-dock-real`（interactive Windows
  x86_64、CI 非収載 lane — Run URL なし）
- run_id / scenario: `rf631-plugin-dock-real` /
  `rf631.plugin-dock-real`
- SHA / gpui: ruviform `b27fb6a1`（run row `repo.sha` と全 case row
  `environment.repo_sha` で一致。main 歴 ancestor — PR #862 merge、
  現 HEAD `68c0c269` の 3 commit 前）+ gpui `v0.2.2-rf2` `b3144f2`
- 環境: `environment_kind = "real-window"`（全 case）、scale_factor
  1.0、`gpui_disable_direct_composition = null`（DComp on —
  `occlusion_exstyle` root `noredirectionbitmap: false` で実証）、
  NVIDIA GeForce RTX 4070 SUPER
- plugin fixture: `test_editor`
  （`ruviform_plugin_test_vst3.dll` sha256
  `0bb0de2017ce7618fa679952387cba3603efb753e5feb5c292cb9a6e5f4b5fda`、
  run row `plugin_path_env = RUVIFORM_PLUGIN_DOCK_TEST_VST3`）
- oracle: `win32-bitblt-screen` screen green `>= 0.5`（画素系）+
  session events・child HWND 契約/状態・OS 効果 oracle（editor 操作系、
  非画素）

## JSONL 確認手順（照合に使った方法）

case row = 先頭の `type:"run"` run row を除く、トップレベルに
`case`/`verdict`/`required`/`observed`/`evidence` を持つ JSONL 行。
verdict 集計は case row のトップレベル `verdict` のみを対象とし、
`observed.steps[].verdict`（step 結果・`note`）や `evidence` 内の
副次値は集計に含めない。集計は JSONL 行を `case`/`verdict` キーで
走査して実施（本 worktree で実測・下記件数を確認）。

## verdict 集合の実測値（JSONL 正本）

case rows **29 件**: **24 pass + 2 fail + 3 unsupported**。

| verdict | 件数 | case |
| --- | --- | --- |
| pass | 24 | `real-window-mount`, `real-native-slot`, `real-editor-attach`, `real-capture`, `real-dcomp-pixel`, `real-dcomp-foreground`, `real-dcomp-present`, `real-dcomp-uncovered`, `real-dcomp-rasterfix`, `real-multiwindow-focus`, `real-os-input`, `real-command-receipt`, `real-window-dpi`, `real-window-close`, `real-editor-resize`, `real-editor-redock`, `real-session-terminate`, `real-session-degraded`, `real-session-recovery`, `real-session-timeout`, `real-editor-visibility`, `real-editor-input`, `real-editor-focus`, `real-editor-popup` |
| fail | 2 | `real-dcomp-bandescape`, `real-dcomp-childinv` |
| unsupported | 3 | `real-window-ime`, `real-session-crash`, `real-session-hang` |

**履歴差分:** pinned-dep run（`evidence/rf532/plugin-dock-real-pinned.jsonl`
`57c4619b`、28 case = 22 pass + 3 unsupported + 3 fail）から、
`real-dcomp-pixel` が fail→pass（rf#857 class B 処遇、#861 merge）、
`real-editor-popup` が新規追加（rf#631 popup harness、#864 merge
— 本 JSONL を最後に再生成した commit `ace2dd56`）。fail→pass 改善と
case 追加のみで pass→fail の回帰なし。

## AC 検証項目の主要実測値

| AC 項目 | case | 実測 |
| --- | --- | --- |
| 実画素 | `real-dcomp-pixel` | pass — screen green `0.9999941167472672`（`win32-bitblt-screen`、1278x798、green 1,019,838/1,019,844 px、threshold 0.5、capture_sha256 `89401018a7…`）。#857 de-occlusion arm（raise→topmost→restore）経由・restore 済み |
| Clip | `real-dcomp-uncovered`/`real-dcomp-present`/`real-dcomp-rasterfix` | pass + PR #501 ancestor clip 実績（`evidence/rf388-native-dock-windowing/report.md`） |
| Focus | `real-editor-focus`/`real-multiwindow-focus` | pass |
| Resize | `real-editor-resize` | pass（child `GetWindowRect` oracle） |
| Dock/ReDock | `real-editor-attach`/`real-editor-redock` | pass（`child_enumerated` 実 child HWND・`Attached { mode: Docked }` event・attach-generation title advance） |
| 実 child HWND / 実 Plugin ABI / 実 Window | 全 dock case | `RuviformTestEditor`（`WS_CHILD`、parent = `RuviformSlotContainer`、root = `Zed::Window`）を `child_enumerated`/`occlusion_hit_test`/`parent_children` で実測。`environment_kind="real-window"` |
| 既知 Fail 維持 | `real-dcomp-bandescape`/`real-dcomp-childinv` | fail — class C 確定診断（`ci_apply_layered_exstyle`/`be_apply_layered_exstyle` refused、`lwa` error 87、detail 記録済み）。`docs/specs/issue-532-dcomp-residual.md`「既知 Fail 継続」決裁を引用 |
| unsupported 処遇 | `real-window-ime`/`real-session-crash`/`real-session-hang` | unsupported — IME = "unsupported capability: Ime"（#389 scope）、crash/hang = `session:*:in-process-lane-nonviable`（#389 lifecycle scope） |

## lane exit 状態

本 lane は class C 既知 fail 2 件を含むため exit 1 が常態であり、
`evidence/rf857/run-record.md` と同じ構造（`real-dcomp-pixel` 等の
判定対象 verdict は pass、exit 1 は class C のみ）である。

## Limitations

- 本 slice は lane 再実走を行っていない（docs-only — canonical
  JSONL `b27fb6a1` を正本とする。close 時点 SHA での再実走要否は
  coordinator gate）。
- run は CI 非収載 interactive lane であり Run URL は存在しない。
- `b27fb6a1` は現 HEAD の 3 commit 前 — 間の merge（#864 popup
  harness、#865、#866）は verdict 意味論に触れない範囲だが、close
  時点 SHA での再照合は coordinator gate。
- hwnd → window_id の一意対応は JSONL に未記録（#532 chain の既知
  の制限どおり）。

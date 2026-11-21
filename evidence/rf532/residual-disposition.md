# Issue #532 DComp residual — verdict 集合の実測値と class 別証拠対応付け

根治 fix（Issue #839 → PR #852/#853/#854、gpui `v0.2.2-rf2`
`b3144f2`）統合後、pinned-dep real lane に残る 3 既知 Fail の
class 切り分けを確定した residual slice の証拠対応付け記録。
doc chain: `docs/discovery/issue-532-dcomp-residual.md` /
`docs/specs/issue-532-dcomp-residual.md` /
`docs/test-specs/issue-532-dcomp-residual.md`。
**Issue #532 は open のまま**（本 doc は `Refs` のみ）。

- Date: 2026-11-21
- 証拠正本: `evidence/rf532/plugin-dock-real-pinned.jsonl`
  （canonical 実出力 `evidence/rf631/plugin-dock-real-scenario.jsonl`
  のコピー）
- run: `mise run test-ui-plugin-dock-real`（interactive Windows
  x86_64、DComp on、scale_factor 1.0、NVIDIA RTX 4070 SUPER）
- SHA / gpui: ruviform `57c4619b` + gpui `v0.2.2-rf2` `b3144f2`

## verdict 集合の実測値（JSONL 正本）

case rows **28 件**: **22 pass + 3 unsupported + 3 fail**。

| verdict | 件数 | case |
| --- | --- | --- |
| pass | 22 | `real-window-mount`, `real-native-slot`, `real-editor-attach`, `real-capture`, `real-dcomp-foreground`, `real-dcomp-present`, `real-dcomp-uncovered`, `real-dcomp-rasterfix`, `real-multiwindow-focus`, `real-os-input`, `real-command-receipt`, `real-window-dpi`, `real-window-close`, `real-editor-resize`, `real-editor-redock`, `real-session-terminate`, `real-session-degraded`, `real-session-recovery`, `real-session-timeout`, `real-editor-visibility`, `real-editor-input`, `real-editor-focus` |
| unsupported | 3 | `real-window-ime`, `real-session-crash`, `real-session-hang` |
| fail | 3 | `real-dcomp-pixel`, `real-dcomp-bandescape`, `real-dcomp-childinv` |

**記録の reconciliation:** `evidence/rf532/pinned-dep-results.md`
の要約行は「24 pass + 3 unsupported + 3 既知 fail」と記すが、
同 doc 自身の verdict 表は `real-dcomp-rasterfix` +「その他 21
case」= 22 pass を列挙しており、JSONL 実測も 22 pass。
catalog（`real_entries` = `[ScenarioEntry; 28]`）は run の 28
case rows と一致する。したがって実測 verdict 集合は
**22 pass + 3 unsupported + 3 fail** が正であり、要約行の「24」は
stale な数値である（AC6 の「既存 pass case を維持」という意図は
充足 — pass だった 21 case 全てが維持され、`real-dcomp-rasterfix`
が fail→pass に改善したのみで、pass→fail の回帰は存在しない）。

## class 別の証拠対応付け

### class A（DComp plane の plugin 画素遮蔽）— 解消済み

| 証拠 | 値 |
| --- | --- |
| `real-dcomp-rasterfix` verdict | `pass`（`fix_arm="baseline"` — docked-in-place、治療 arm 不使用） |
| baseline screen green | `0.9999941167472672`（`win32-bitblt-screen`） |
| `rf_baseline_state` | container/child とも exstyle `0x0`（layered/topmost/noredirectionbitmap 全 false） |
| arms | `rf_arms` skipped — "baseline screen green already meets the threshold" |

### class B（`real-dcomp-pixel` — HWND レベル外部遮蔽）

| step / field | 観測 | 解釈 |
| --- | --- | --- |
| `screen_pixels`（fail） | green `0.0` | screen 合成結果に child 画素が無い |
| `windowdc_pixels`（note） | green `1.0` | child の real backing raster が存在（fix の効果） |
| `printwindow_pixels`（note） | green `1.0` | 同上（合成再描画系） |
| `occlusion_exstyle`（note） | root `0x40100`（`noredirectionbitmap=false`）、child/parent `0x0` | fix による flag 除去を確認 |
| `occlusion_zorder`（note） | w1 内で child `z_index=0`、`above_editor=false` | window 内 sibling 遮蔽なし |
| `occlusion_hit_test`（note） | 全 5 probe point → hwnd `160105518` `Zed::Window` `184,89,1552x903` role `other` | **遮蔽源は window 階層外の別 top-level window**（w2–w4 と同一 geometry） |

→ 失敗は lane 設計上の window 配置（複数 top-level window が
被測定 child rect 上に重なる）に起因する **HWND レベル遮蔽** で、
DComp plane class とは別。処遇: 別 Issue 切り出し（起票案は PR
本文）+ #532 内では class B 既知 Fail として維持。

### class C（`real-dcomp-bandescape` / `real-dcomp-childinv` — `WS_EX_LAYERED` 適用不能の確定診断）

| 証拠 | 観測 |
| --- | --- |
| `be_apply_layered_exstyle` / `ci_apply_layered_exstyle` | `applied: false`、`win32_error: null` — `SetWindowLongPtrW` が last-error 成功でも read-back で `WS_EX_LAYERED` absent（silent no-op） |
| `be_apply_layered_lwa` / `ci_apply_layered_lwa` | error 87（`ERROR_INVALID_PARAMETER`）— 非 layered window への LWA 拒否 |
| childinv retry | `latched_as_child=false` / `latched_as_toplevel=true` — `WS_CHILD` 状態では latch せず、top-level 化後のみ適用（child-window-state 制約） |
| `WS_CHILD\|WS_EX_LAYERED` 新規作成 | `0x80070006` で失敗（issue-839 discovery 記録） |
| `be_restore` / `ci_restore` | be_restore=pass（3 bool true）、ci_restore=`ops_ok=false` — probe は完走・復元記録済み |

→ 2 case の `fail` は verdict 契約（必須 op 拒否 = fail）どおりの
honest 記録。診断目的達成済み。処遇: **既知 Fail として継続**
（class C 確定、別 Issue 不要）。`Unsupported` re-baseline /
re-scope は verdict 意味論変更を要するため本 slice では不採用。

## AC 行との突合（要約）

| AC 行 | 証拠 | 判定 |
| --- | --- | --- |
| Regression 固定 | `real-dcomp-pixel` の継続 Fail + `evidence/rf532/` bisect 系記録 | 充足 |
| 採用方式で画素 Pass | `real-dcomp-rasterfix` baseline `0.999994` | 充足 |
| Focus/Resize/ReDock 実走 | `real-editor-focus`/`real-multiwindow-focus`/`real-editor-resize`/`real-editor-redock` pass | 充足 |
| RT | 明示 case なし → rationale 記録（fix は UI thread presentation layer のみ、audio RT と共有 state/lock なし） | 充足（rationale） |
| 既知 Fail 維持 | 3 fail を class B/C で維持（昇格なし） | 充足 |
| L1 契約/Headless/RT 非回帰 ↔ CI | headless lanes 全 pass（pinned-dep-results lane 表）。merge SHA `5f22bee6` 3OS CI = Coordinator gate（NotRun pending） | 部分充足 |
| L2/L3 再開条件記録 | `docs/specs/issue-532-dcomp-residual.md` | 充足（記録） |
| Windows GUI-L2/EXT-L3 実走 | interactive real lane + #630 test_editor 実 ABI child + #631 oracle で再現→修正実走 | 実質充足 |
| macOS/Linux L1 | 対象 SHA 3OS CI = Coordinator gate | NotRun |
| Scope 明記・新規 Workflow なし | `docs/test-specs/issue-532-dcomp-residual.md` Scope 表 | 充足 |

## Limitations

- 遮蔽 window `160105518` と `window_identities`（w2–w4）の対応は
  geometry 一致による推定。hwnd→window_id の一意対応は JSONL に
  未記録。
- 遮蔽除去配置での `real-dcomp-pixel` 再実測は未実施（follow-up
  Issue の検証対象）。
- 本 slice は real lane 再実走を行っていない（docs-only — pinned
  JSONL を正本とする）。

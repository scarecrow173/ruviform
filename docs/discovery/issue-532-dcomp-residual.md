---
id: "DISC-034ZccDcompResidual01"
type: "discovery"
status: "approved"
title: "Issue #532 DComp residual — 根治 fix 統合後の残存 Fail class 切り分け記録"
created: "2026-11-21"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/839"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-residual.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-839-gpui-dcomp.md"
    - "docs/discovery/issue-532-child-inventory.md"
    - "docs/discovery/issue-532-band-escape.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-532-dcomp-residual.md"
    - "docs/specs/issue-839-gpui-dcomp.md"
    - "docs/discovery/issue-839-gpui-dcomp.md"
    - "evidence/rf532/pinned-dep-results.md"
    - "evidence/rf532/plugin-dock-real-pinned.jsonl"
    - "evidence/rf532/residual-disposition.md"
    - "evidence/rf532/band-escape-results.md"
    - "evidence/rf532/child-inventory-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-residual.md"
  derived-by: []
  verifies: []
  verified-by:
    - "evidence/rf532/residual-disposition.md"
metadata:
  issue: 532
---

# Issue #532 DComp residual — 根治 fix 統合後の残存 Fail class 切り分け記録

Issue #839 の根治 fix（gpui fork `v0.2.2-rf2` `b3144f2` の
clip-partition + `WS_EX_NOREDIRECTIONBITMAP` 除去）が main に
統合された後、pinned-dep real lane で残った 3 つの既知 Fail を、
追加の実機調査なしに確定できる class へ切り分けた記録。処遇決定
は `docs/specs/issue-532-dcomp-residual.md`、AC 行との突合は
`docs/test-specs/issue-532-dcomp-residual.md` を参照。

## Run 条件（切り分けの入力）

| 項目 | 値 |
| --- | --- |
| 証拠 JSONL | `evidence/rf532/plugin-dock-real-pinned.jsonl`（canonical 実出力 `evidence/rf631/plugin-dock-real-scenario.jsonl` のコピー） |
| run コマンド | `mise run test-ui-plugin-dock-real`（canonical、interactive Windows） |
| ruviform worktree SHA | `57c4619b355f68f57c2a6aac560bf9b5a6e9cdcd`（JSONL `environment.repo_sha` と一致） |
| 解決された gpui rev | `b3144f286b942fbb5f478da73db13a826cef054a`（tag `v0.2.2-rf2`） |
| OS / arch | windows / x86_64、interactive desktop session（`environment_kind = "real-window"`） |
| scale_factor / DComp | 1.0 / `gpui_disable_direct_composition = null`（DComp on） |
| display adapter | NVIDIA GeForce RTX 4070 SUPER |
| 判定 oracle | `win32-bitblt-screen` の screen-composited `EDITOR_COLOR` green 比率、`green_fraction >= 0.5` |

## verdict 集合の実測

JSONL の case rows は **28 件**で、内訳は
**22 pass + 3 unsupported + 3 fail**。

- pass（22）: `real-window-mount`/`real-native-slot`/
  `real-editor-attach`/`real-capture`/`real-dcomp-foreground`/
  `real-dcomp-present`/`real-dcomp-uncovered`/
  `real-dcomp-rasterfix`/`real-multiwindow-focus`/`real-os-input`/
  `real-command-receipt`/`real-window-dpi`/`real-window-close`/
  `real-editor-resize`/`real-editor-redock`/
  `real-session-terminate`/`real-session-degraded`/
  `real-session-recovery`/`real-session-timeout`/
  `real-editor-visibility`/`real-editor-input`/`real-editor-focus`
- unsupported（3）: `real-window-ime`/`real-session-crash`/
  `real-session-hang`（既知: adapter API なし / 未実装 fixture）
- fail（3）: `real-dcomp-pixel`/`real-dcomp-bandescape`/
  `real-dcomp-childinv`

記録上の不整合: `evidence/rf532/pinned-dep-results.md` の要約行は
「24 pass + 3 unsupported + 3 既知 fail」と記すが、同 doc 自身の
verdict 表は `real-dcomp-rasterfix` +「その他 21 case」= 22 pass
を列挙しており、JSONL 実測も 22 pass である。catalog
（`real_entries`）は `[ScenarioEntry; 28]` で run の case rows と
一致する。本 residual slice では **JSONL 実測の 22 pass** を正本
として記録し、要約行の「24」は stale な数値として扱う
（`evidence/rf532/residual-disposition.md` に同旨を追記）。

## class A: DComp plane の plugin 画素遮蔽 — 解消済み

Issue #532 の本来の責務である「GPUI 0.2.2 の DComp topmost
presentation plane が child HWND raster を覆う」障害は、根治 fix
で解消済み。pinned dep で `real-dcomp-rasterfix` が
`fix_arm = "baseline"`（docked-in-place・治療 arm 不使用）のまま
screen green `0.9999941167472672` で Pass。`rf_baseline_state` で
container/child とも exstyle `0x0` を確認済みで、child の
style/parentage 変更を伴わない。

根拠は `docs/discovery/issue-839-gpui-dcomp.md` と
`evidence/rf532/clipfix-results.md` /
`evidence/rf532/pinned-dep-results.md` に確定済み。本 slice で
再調査・再検証は行わない。

## class B: `real-dcomp-pixel` — 別 top-level window の HWND レベル遮蔽

pinned JSONL の `real-dcomp-pixel` row が示す失敗の構造:

| step | verdict | 観測 |
| --- | --- | --- |
| `screen_pixels`（`win32-bitblt-screen`） | fail | green_fraction `0.0`（child rect `321,153,1278x798`） |
| `printwindow_pixels` | note | green `1.0` — child の合成再描画 raster は存在 |
| `windowdc_pixels` | note | green `1.0` — child の window-DC backing raster が実在（根治 fix による raster 復元の効果そのもの） |
| `occlusion_exstyle` | note | root exstyle `0x40100`（`noredirectionbitmap: false` — fix で除去済み）、child/parent exstyle `0x0` |
| `occlusion_zorder` | note | w1 階層内では child が topmost（`z_index: 0`、`above_editor: false`）— 同 window 内の sibling 遮蔽は無し |
| `occlusion_hit_test` | note | **全 5 probe point が hwnd `160105518`・class `Zed::Window`・rect `184,89,1552x903`・role `other` を返す** |

遮蔽 window `160105518` は lane が spawn する別の top-level
`Zed::Window` で、`environment.window_identities` の w2–w4
（`ruviform plugin-dock B`/`C`/`fader-dock D`、いずれも origin
`192,120`・`1536x864` client、frame 込み rect `184,89,1552x903`
で幾何一致）と同一 geometry を持つ。この top-level window が
docked child の screen rect 全体（`321,153,1278,798` ⊂
`184,89,1552,903`）を被覆しており、screen `BitBlt` oracle は
covering window 側の画素を採取する。

切り分けの結論:

- child raster は存在し（window-DC `1.0`）、w1 内の z-order も
  正常。DComp 経路の合成（class A）は根治 fix で機能している。
- 失敗の直接因は **lane 設計上の window 配置** — 複数 top-level
  window を同一 lane で扱う scenario が、被測定 child の上に別
  window を重ねたまま screen capture する。これは compositor の
  不具合ではなく HWND レベルの外部遮蔽であり、class A とは
  **別 class**。
- case 自体の `detail` 文字列は依然 "consistent with the known
  GPUI DComp topmost occlusion" と報告するが、これは
  `occlusion_hit_test` instrumentation 導入前の文面で、確定した
  class（外部 window 遮蔽）とは一致しない。文言の更新は
  verdict/semantics を触る後続作業の一部として扱う。

## class C: `real-dcomp-bandescape` / `real-dcomp-childinv` — `WS_EX_LAYERED` 適用不能の確定診断

2 case はいずれも「`WS_EX_LAYERED` を foreign（呼出 thread 非所有
あるいは `WS_CHILD` の）window へ適用して presentation 経路を
切り替えられるか」を問う診断 case で、実走により **適用不能**
が確定している。

確定した機構（証拠）:

- `SetWindowLongPtrW(GWL_EXSTYLE, ... | WS_EX_LAYERED)` —
  last-error 上は成功だが `GetWindowLongPtrW` read-back で bit が
  absent（**silent no-op**）。container-level
  （`be_apply_layered_exstyle`）と child-level
  （`ci_apply_layered_exstyle`）の両方で再現。
  `evidence/rf532/band-escape-results.md`、
  `evidence/rf532/child-inventory-results.md`。
- `SetLayeredWindowAttributes(LWA_ALPHA)` — `WS_EX_LAYERED` 非保持
  window への呼出は error 87（`ERROR_INVALID_PARAMETER`）で拒否。
- `real-dcomp-childinv` の retry 実験 —
  `latched_as_child = false` / `latched_as_toplevel = true`。
  `SetParent(GetDesktopWindow())` で top-level 化した後にのみ
  `WS_EX_LAYERED` が latch する。失敗機構は cross-thread ではなく
  **child-window-state 制約**（`WS_CHILD` のままでは exstyle
  write が latch しない）。
- `WS_CHILD | WS_EX_LAYERED` の新規作成は `0x80070006` で失敗
  （`docs/discovery/issue-839-gpui-dcomp.md` 記録済み）。
  `WS_POPUP` + layered なら描画は可能だが in-place hosting の
  要件を満たさない。

両 case の `fail` は、各 case の verdict 契約
（`docs/test-specs/issue-532-band-escape.md` F4 等の「必須
mutation 拒否 = fail」）に従う honest な記録であって、probe
実装の欠陥や再調査が必要な未確定失敗ではない。診断目的
（layered 経路の可否判定）は達成済みで、判定結果は根治 fix
slice での経路棄却（`docs/discovery/issue-839-gpui-dcomp.md` の
「棄却した経路」）に既に反映されている。

### #917 追補 — verdict 契約の意図的改訂（re-scope）

上記の確定機構は不変。Issue #917（`real-dcomp-*` 行の環境要因
`fail` 調査 — lane exit 1 常態化の解消）で、2 case の verdict
契約「必須 mutation 拒否 = fail」を意図的改訂した:

- as-child `be_apply_layered_{exstyle,lwa}`/
  `ci_apply_layered_{exstyle,lwa}` op は「確定 class-C どおりの
  期待拒否」として `verdict: note` + `expected_outcome` の診断行
  へ転換（`record_expected_refusal_row`）。refusal の
  `applied`/`win32_error`/`after`/`latched` 記録は維持 —
  Fail→Pass の改竄ではなく期待値検証への re-scope。
- required op（repaint/frame/reparent/retry/restore 系）の拒否・
  restore 不完全・lifecycle 破壊は従来どおり `Fail`。reparent 後
  top-level での `ci_apply_retry_*` は strict を維持（top-level
  layered は文書化された supported 経路）。
- as-child layered が latch する観測（OS build 依存で確定機序と
  矛盾し得る新規 data）は `contradicts_confirmed_mechanism`
  flag 付きで記録し verdict を動かさない — lane が計るのは
  probe 契約の完遂であって OS 実装詳細の不変性ではない。
- `be_apply_layered_*` は `WS_CHILD` 状態の container、
  `ci_apply_layered_*` は `WS_CHILD` の editor child に対する
  write であり、いずれも本 class-C 機構の対象となる
  as-child 経路である（container write は cross-thread だが
  child-level same-thread write の同じ silent drop が child-state
  制約を確定した — 上記機構確定のとおり）。
- 収束先の選択肢としての `Unsupported`/entry-gate 化は
  採らなかった — case は unsupported ではなく（probe は設計
  どおり完走する）、gate 化は per-run refusal evidence を消す
  ため（`docs/specs/issue-532-dcomp-residual.md`「決定の補足」
  参照）。
- 収束 verdict（2 case の `pass` + lane exit 0）は
  `evidence/rf631/plugin-dock-real-scenario.jsonl` の canonical
  再実走で確認する。

## AC1 の RT 非干渉（rationale 記録）

pinned lane の case 一覧に明示的な RT case は存在しないため、
「画素・Focus・Resize・ReDock・**RT** を修正後に実 Windows GUI
で Pass」の RT 項は rationale 記録で対応付ける:

- 根治 fix の作用面は presentation layer のみ — root window の
  `WS_EX_NOREDIRECTIONBITMAP` 除去、swapchain visual を child
  rect で partition する clip、変化時の visual 再構築 +
  `Commit` + `DwmFlush`、および `present()` / 10ms `WM_TIMER`
  駆動の `sync_child_windows`。すべて UI thread 上の compositor
  処理で、audio RT path（process callback・graph・buffer 交換）
  と共有する state・lock・thread は存在しない。
- fix は child HWND の style/parentage を変更しない
  （`rf_baseline_state` で exstyle `0x0` を実測確認）ため、
  plugin editor の audio 経路に新たな同期点・blocking API を
  導入しない。
- audio RT 非回帰の実証は、fix が RT 側境界に touch しない
  ことの論証（上記）＋既存 RT 検証レーンの継続 green に委ねる
  形態とし、本 slice で新規の RT 実走は要求しない
  （`docs/development-guidelines.md` の RT 検証レーン方針に従う）。

## 採否 handoff 先の状態（確定した事実）

Issue #532 本文の「採否は#262に渡す」「#262 の採用判定の必須
入力」「#388→#262→#419 へ報告」という handoff 形態に対し、
現時点の状態:

| Issue | 状態 | 役割（Issue 本文の記述） |
| --- | --- | --- |
| ruviform#262 | **CLOSED** | GPUI 採用判定の入力先（`GPUI Native First 採用Gate`） |
| ruviform#419 | **CLOSED** | #262 PoC 判定に基づく後継 ADR の最終採用 |
| ruviform#388 | OPEN | NativeSurfaceSlot 契約・Windows Partial の証拠元 |
| ruviform#389 | OPEN | Plugin Editor Session / L0-L1 統合受入 |
| ruviform#630 / #631 | OPEN | Test Plugin 実 ABI / real-window scenario runner |
| synvelis#176 | （cross-repo） | 製品実 Plugin Native Dock の OS 受入 |

採否判定の公式な引き渡し先（#262/#419）は両方 CLOSED であり、
方式 A（GPUI 最小修正 = fork clip-partition）の採用実績を
「#262 の採用判定の必須入力」として渡す形態は現状成立しない。
処遇の提案は `docs/specs/issue-532-dcomp-residual.md` に記録する。

## Limitations

- 遮蔽 window `160105518` と `window_identities` の対応は
  geometry 一致による推定であり、hwnd → window_id の一意な
  対応付けは JSONL に記録されていない（w2–w4 は同一
  geometry のため、どれが最前面かは区別しない）。
- `real-dcomp-pixel` の失敗再現は当該 run の window 配置依存で
  あり、遮蔽 window を除去した配置での screen green 値は本
  slice では未実測（follow-up Issue 側の検証対象）。
- lane 再実走は行っていない（docs-only disposition。実機再確認
  は本 slice の納品範囲外）。

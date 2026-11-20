---
id: "DES-034ZaaUncoveredBaseline001"
type: "design"
status: "approved"
title: "Issue #532 uncovered-baseline bisect — 設計（settle 方式・新 case・own-process inventory）"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-uncovered-baseline.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-532-uncovered-baseline.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-uncovered-baseline.md"
    - "docs/designs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "evidence/rf532/presentation-probe.md"
    - "evidence/rf532/foreground-probe-bisect.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-uncovered-baseline.md"
  derived-by:
    - "docs/test-specs/issue-532-uncovered-baseline.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-uncovered-baseline.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 uncovered-baseline bisect — 設計

## 決定事項

### D1: settle は新 case で行い、`real-dcomp-foreground` を改変しない

残課題 a は「foreground case の topmost stage に settle を挟む」
か「新 case」の2択だった。**新 case `real-dcomp-uncovered`**
（`real-dcomp-present` 直後）を採用する。理由:

- foreground case の step 列（`fg_observe_baseline` …
  `fg_observe_topmost` → `fg_restore`）は PR `#693` で記録済みの
  bisect 系列であり、途中に settle 段を差し込むと系列の比較
  可能性が失われる。
- 本 slice の実験は env-prep・uncovered 判定・settle 曲線・capture
  variants の 4 段から成り、既存 case の 1 stage より大きい単位。
  独立 case の方が実験の読み取りと verdict の分離が素直になる。

### D2: settle は `std::thread::sleep` — message pump は不採用

case body は `run_scenario` の caller thread で走り、GPUI の
app thread は独自に pump・repaint を継続する（`observe_open` が
`&RealOsGuiTest` で、`settle()` 系の `&mut` API に到達しない
事実からもこの構造が確認できる）。したがって caller thread を
sleep しても子 HWND の repaint・DWM の再合成は止まらない。
`std::thread::sleep` で待機時間を明示記録する設計が最も単純で
十分。`PeekMessage` pump の自作は GPUI app thread と無関係な
自 thread のキューを空回しするだけで、観測対象の presentation
経路に寄与しないため不採用。

各 settle 段の待機は**累積方式**で記録する: `settle_ms` 列挙
`[200, 500, 1200]` を cumulative target とし、各 round で
`target - prev` だけ sleep する。これにより t≈0（topmost 直後、
foreground slice と同条件）→ 200ms（~12 frame）→ 500ms
（~30 frame）→ 1200ms（~72 frame）の 4 点の時間曲線が取れる。
flat dark が pre-paint frame の捕捉なら曲線のどこかで green が
現れるはずであり、全点で 0 なら「提示自体が起きていない」証拠
になる。

### D3: `has_update` を paint 滞留の判別子にする

settle 各段で child の `ospres::attributes(hwnd).update`（内部は
`GetUpdateRect`）を記録する。解釈は:

- `has_update == true` が持続: child に paint 要求が滞留している
  — 未処理の repaint があるのに screen に出ないなら presentation
  層の問題。
- `has_update == false` で screen green=0 が持続: child の paint
  は完了しているのに screen に出ない — より強く presentation/
  compositor 層の遮蔽を示唆する。

### D4: uncovered 判定は role ベース

hit-test の各点の `occlusion_role`（`editor_child`/`container`/
`root`/`other`）を見て、**全点が `other` でない**とき
`uncovered:true`。HWND レベルで外部遮蔽が無いことを確認して
初めて「残る no-green は compositor 内部」と解釈できる。
`uncovered:false` の場合も観測は続行し、結果は covered 系の
比較データとして記録する（断言を避ける）。

### D5: env-prep mutation は foreground probe の再利用

遮蔽なし環境の作成手段は、foreground slice で実証済みの
`osmut` staged mutation（raise → `SetForegroundWindow` →
topmost）をそのまま使う。`run_probe_stages`・
`observe_all_children`・`record_stage_observation`・restore
ブロックに `ProbeLabels { step, artifact }` を注入し、step 名と
PNG artifact 名の prefix を `fg` → `uc` に切り替える。これで
mutation/restore の意味論を二重実装せず、step 名の衝突も
避けられる（`uc_apply_raise`/`uc_observe_settled_500ms` 等）。

既存 fg 側の呼び出しは `ProbeLabels{step:"fg",
artifact:"real-dcomp-foreground"}` を渡すだけで step 名を完全に
維持する — 非回帰のため label を literal ではなく param 化する。

### D6: `ospres::Inventory.own_windows` — cap 非依存の自窓列挙

残課題 c: cap 256 で自 window が列挙外になり得た。
`EnumWindows` の全件から `GetWindowThreadProcessId` で自 PID を
先に絞り、自 window は cap に関係なく全件 `TopLevelWindow`
（真の z_index 付き）で `own_windows` に入れる。汎用 `windows`
field の cap 意味論は変えない。`pres_inventory` row にも
`own_windows` を additive 追加し、presentation case 側でも自窓
z_index が記録されるようにする。

### D7: capture variants は settle 後に 1 回だけ

残課題 d（`PW_RENDERFULLCONTENT` 切り分け）は、uncovered+settled
状態で `PrintWindow` flag 変種（`0`/`PW_CLIENTONLY`/
`PW_RENDERFULLCONTENT`/複合）と `capture_window_dc` を child・
container・root に対して 1 回ずつ取る。`record_present_capture`
に `step` name param を追加して `uc_capture` として記録する。
各 settle 段で全 variant を取ると artifact が増えるだけで判別
情報は増えないため、最終 settle 後の 1 回に限定する。

## probe の流れ（`real-dcomp-uncovered`）

1. `uc_inventory` — `ospres::inventory()`（own_windows 含む）を
   記録（残課題 c）。
2. `uc_baseline_state` — `osmut::observe` の baseline snapshot
   （restore の入力）。
3. `uc_observe_baseline` — covered 状態の hit-test+screen。
4. `uc_apply_*`/`uc_observe_*` — raise → foreground → topmost
   （env-prep、fg と同じ mutation 系列）。
5. `uc_uncovered` — topmost 後の全 hit-test 点の role 集計と
   `uncovered` bool（残課題 b の成立判定）。
6. `uc_observe_settled_{200,500,1200}ms` + `uc_settle` — settle
   曲線。`uc_settle` row に `settle_ms`・`settle_kind`・per-child
   `uncovered`/`update_pending`（残課題 a の判別データ）。
7. `uc_capture` — post-settle の PW flag 変種 + window DC（残課題 d）。
8. `uc_restore` + `uc_observe_restore` — verified restore。

## verdict 契約（不変）

- `evaluated` + `executed` + `restored` + `lifecycle_ok` → `Pass`。
- green/hit-test/uncovered/`has_update` の観測値は verdict 非入力。
- `SetWindowPos` 系の apply FALSE と restore 検証失敗 → `Fail`。
- `SetForegroundWindow` FALSE → note のみ（foreground 非対称規則）。
- 非 Windows → `Unsupported`、環境・前提欠落 → `NotRun`。

## 判断の解釈表

| 観測 | 解釈 |
| --- | --- |
| uncovered + settled で green ≥ 0.5 | 外部遮蔽が全原因。presentation-probe の「説明可能」が確定に近づく |
| uncovered + settled で green < 0.5 かつ `has_update==false` 持続 | compositor-internal/presentation 層の遮蔽を強く示唆 |
| uncovered + settled で green < 0.5 かつ `has_update==true` 持続 | child の repaint 自体が滞留 — paint 供給側の問題を示唆 |
| covered（`uncovered:false`）のまま | 環境制御失敗を実測として記録。compositor 判定は持ち越し |
| pre-settle で 0 → settle 中に green 出現 | flat dark は pre-paint frame の捕捉だった |

# Issue #532 band-escape probe — 判別記録

Issue `scarecrow173/ruviform#532` の band-escape slice
（`real-dcomp-bandescape` case + `osmut` container mutation
family）の実走結果と、2 つの「band 脱出」経路の可否記録。
**Issue #532 は open のまま** — 本書は診断証拠であり、
compositor 修正・実装完了の宣言ではない。

- Date: 2026-11-20
- OS: Windows (x86_64)、interactive desktop（NVIDIA RTX 4070
  SUPER、2 monitor attached）
- Lane JSONL（実装 HEAD `3a678254` + working tree 差分）:
  - `evidence/rf631/plugin-dock-real-scenario.jsonl` —
    canonical `mise run test-ui-plugin-dock-real` 出力（24 cases）
  - `evidence/rf532/band-escape.jsonl` — 同一 lane の slice
    側コピー（`--pixel-artifacts evidence/rf532/band-escape`）
- Pixel artifacts: `evidence/rf532/band-escape/
  real-dcomp-bandescape-{baseline,repaint,layered,reparent,restore}-0.png`

## Probe 概略

editor child attach 済み・lease 生存中に実行する staged
mutation + observation（33 steps）:

1. `be_baseline_state` — `osmut::observe_container`（parent /
   exstyle / **class_style** / screen rect / sibling z-order）+
   root snapshot
2. `be_observe_baseline` — 5 点 hit-test + screen capture
3. `be_apply_repaint` — `RedrawWindow(RDW_INVALIDATE|RDW_ERASE|
   RDW_UPDATENOW)` で update region を同期 drain し
   `be_observe_repaint` で再観測（pre-paint frame の嫌疑を除去）
4. `be_apply_layered_*` — `GWL_EXSTYLE |= WS_EX_LAYERED` →
   `SWP_FRAMECHANGED` → `SetLayeredWindowAttributes(0,255,
   LWA_ALPHA)`（layered 経路の試行）
5. `be_apply_reparent_*` — baseline exstyle 復帰 → frame apply →
   `SetParent(GetDesktopWindow())` → baseline screen rect を
   `HWND_TOP` で再配置（reparent 経路の試行）
6. `be_settle`（500ms `std::thread::sleep`）後に
   `be_observe_layered` / `be_observe_reparent` で hit-test +
   screen green を記録
7. `be_restore` + `be_observe_restore` — baseline parent /
   exstyle / rect への verified restore

verdict は「probe 実行 + restore 検証 + lifecycle 収束」で判定
し、screen green / hit-test role は全て note 級 data。mutation
API の拒否は strict fail（既存 foreground/uncovered case と同じ
意味論）。**`exstyle_write` の `applied` は read-back 検証付き**:
`SetWindowLongPtrW` が last-error 上「成功」を返しても
`GetWindowLongPtrW` で要求値が確認できなければ
`applied: false` と記録する（silent drop を成功と誤記しない）。

## 実走結果

### 各 stage の screen green（同一 lane、両 run で再現）

| stage | hit-test role（5 点） | screen green | capture SHA256（先頭 16） |
| --- | --- | --- | --- |
| baseline | `editor_child` ×5 | 0.000 | `7dd8975d5b0921b1` |
| repaint（RDW forced） | `editor_child` ×5 | 0.000 | `7dd8975d5b0921b1` |
| layered（LWA 拒否後） | `editor_child` ×5 | 0.000 | `7dd8975d5b0921b1` |
| reparent（desktop + HWND_TOP） | `editor_child` ×5 | 0.000 | `7dd8975d5b0921b1` |
| restore 後 | `editor_child` ×5 | 0.000 | `7dd8975d5b0921b1` |

全 stage の screen capture が **byte 同一**（SHA256 が baseline
から restore まで不変）。reparent 後の container は z-order で
`below_hwnd` = GPUI root の直上に位置していたにもかかわらず、
合成結果が 1 bit も変化しなかった。

### Layered 経路 — 評価不能（refused at setup）

| op | applied | win32 error | 備考 |
| --- | --- | --- | --- |
| `SetWindowLongPtrW(GWL_EXSTYLE,\|WS_EX_LAYERED)` | **false** | none | last-error 上は成功扱いだが、read-back 検証で `WS_EX_LAYERED` bit が absent — write が silently drop された |
| `SetWindowPos(SWP_FRAMECHANGED,…)` | true | — | |
| `SetLayeredWindowAttributes(0,255,LWA_ALPHA)` | **false** | **87 (`ERROR_INVALID_PARAMETER`)** | `WS_EX_LAYERED` 非保持の window への LWA 呼出拒否と一致 |

重要な補助観測: baseline の `class_style` = `0x4088`
（`CS_GLOBALCLASS|CS_PARENTDC|CS_DBLCLKS`）。文書化された
`WS_EX_LAYERED` 禁止条件（`CS_OWNDC`/`CS_CLASSDC`）は**該当せず**。
container は caller thread（probe 実行 thread）と異なる thread
（GPUI 側の pump-owning thread）に属する child window であり、
silent drop の最有力候補は cross-thread `GWL_EXSTYLE` write の
無効化である（機構は未確定）。

→ **layered 経路は「失敗」ではなく「評価不能」**: この経路を
評価するには `WS_EX_LAYERED` を window-owning thread 側（GPUI
の window 生成/変更経路、すなわち GPUI fork または dock host の
UI thread code）で適用する必要がある。外部 caller thread からの
事後適用はこの topology では成立しない、というのが今回の
観測結果。

### Reparent 経路 — 実行されたが pixel 欠損は不変

| op | applied | 結果 |
| --- | --- | --- |
| `SetWindowLongPtrW(GWL_EXSTYLE,<baseline>)` | true | baseline=0 へ復帰 |
| `SetWindowPos(SWP_FRAMECHANGED,…)` | true | |
| `SetParent(GetDesktopWindow())` | **true** | `parent: 65548`（desktop）に変化 — reparent 自体は成功 |
| `SetWindowPos(HWND_TOP,<baseline-rect>,SWP_NOACTIVATE)` | true | 同一 screen rect `(321,153,1278,798)` に再配置 |

reparent 後: container は top-level window として GPUI root の
直上（`below_hwnd` = GPUI root）に存在し、editor child は全 5
hit 点で `WindowFromPoint` に勝ち続けた。**それでも screen
green は 0.000、capture hash は baseline と byte 同一。**

### Restore — pass（verified）

`be_restore` = pass: `parent_restored` / `exstyle_restored` /
`rect_restored` 全て true。`be_update_pending` も container /
child 双方で `update_pending: false`（forced repaint による
update region drain を確認 — pre-paint frame の嫌疑を排除）。

### Case verdict

`real-dcomp-bandescape` = **fail**（33 steps 中 2 step refused:
`be_apply_layered_exstyle`、`be_apply_layered_lwa`）。これは
probe 設計上「layered mutation が必須経路」であり OS 層で拒否
されたための honest な `fail` であり、probe 実装の欠陥ではない。
同一 lane の `real-dcomp-pixel` は従来どおり `fail`
（best green_fraction=0.000）— Issue #532 の既知 baseline。

## Fix-decision table

| 脱出経路 | 結果 | 解釈 |
| --- | --- | --- |
| minimal layered-container change | **評価不能（refused）** | caller thread からの `WS_EX_LAYERED` 付与は silently drop された。評価には owning thread 側での適用（GPUI fork 内の生成時 style 等）が必要 — その場合は既に「GPUI への変更」であり minimal ではない |
| reparent / owned-popup redesign（既存 container の desktop reparent） | **失敗（executed, 効果なし）** | top-level 化 + `HWND_TOP` + 同一 rect でも screen は byte 同一。container の presentation 関係を変えても child の pixel は合成に現れない |
| 次の direction | **capture-plane / child-level 診断へ escalate** | 下記「解釈」 |

## 解釈（observation と hypothesis を分離）

Observation（確実な記録）:

- reparent で container は実際に desktop child の top-level に
  なり、GPUI root の直上に配置された。
- その状態で screen capture は baseline と byte 同一 —
  reparent による視覚効果は**一切**なかった。
- editor child は reparent 後も全 hit 点で `WindowFromPoint`
  に勝つ（hit-test path は生きている）。
- `PrintWindow`（既存証拠）では child は green を rasterize
  できる（fraction 1.000）。
- 過去の uncovered slice の `GPUI_DISABLE_DIRECT_COMPOSITION=1`
  run も green 0.0 で byte 同一（ただし当該 run で env var が
  GPUI 0.2.2 fork の DComp path を実際に無効化したかは未検証 —
  下記 limitation）。

Hypothesis（記録のみ、verdict ではない）:

- container の z-order/parentage/presentation-plane いずれを
  変えても child raster が最終合成に現れない以上、阻害は
  container の presentation plane ではなく **child HWND 自身の
  raster が composition tree に到達していない** 層にある可能性が
  高い。次の判別対象として有力:
  - child HWND 自身の `GWL_STYLE`/`GWL_EXSTYLE`（例:
    `WS_EX_NOREDIRECTIONBITMAP` 等の非リダイレクト系 bit を
    child が持たないか — container の exstyle は 0 だったが
    child 側は未観測）;
  - child が `CS_PARENTDC` container の下で通常の
    `WM_PAINT` 経路を持つか、redirect surface が DWM に
    供給されているか;
  - GPUI 0.2.2 の editor-host window class（`0x4088`）が child
    の paint/compose を抑制するか。

Limitations:

- layered 経路は評価不能であり「失敗」と確定したわけではない。
  owning thread 適用（fork 変更）なら latch する可能性は残る。
- `GPUI_DISABLE_DIRECT_COMPOSITION` が fork で実際に DComp を
  off にするかは本 slice では再検証していない。
- reparent 後の container 自身の class（`0x4088`）由来の挙動
  （`CS_PARENTDC` 等）が child paint に与える影響は未分離。
- screen capture は `win32-bitblt-screen` backend — DWM 合成結果
  の外部観測であり、compositor 内部状態の直接観測ではない。

## 関連

- `docs/discovery/issue-532-band-escape.md`
- `docs/designs/issue-532-band-escape.md`
- `docs/specs/issue-532-band-escape.md`
- `docs/test-specs/issue-532-band-escape.md`
- `evidence/rf532/uncovered-baseline-bisect.md`
- `evidence/rf532/presentation-probe.md`
- `evidence/rf532/option-a-results.md`

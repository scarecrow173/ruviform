---
id: "DES-034ZaaBandEscapeProbe001"
type: "design"
status: "approved"
title: "Issue #532 band-escape probe — 設計（container mutation family・verified-write・2 経路の試行順序）"
created: "2026-11-20"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-band-escape.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-532-band-escape.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-532-uncovered-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-band-escape.md"
    - "docs/specs/issue-532-band-escape.md"
    - "docs/test-specs/issue-532-band-escape.md"
    - "evidence/rf532/band-escape-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-band-escape.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 band-escape probe — 設計

## 決定事項

### D1: mutation は `osmut` の container family に集約する

既存の方針（`osoccl`/`ospix`/`ospres` は query-only、変異は
`osmut` のみ）に従い、container 対象の変異も全て `osmut.rs`
内の platform module に置く。`real_dock.rs` の case body は
`osmut::` API を呼ぶだけで、Win32 型・unsafe を持たない。

- `observe_container(container) -> ContainerSnapshot` —
  parent/exstyle/class_style/screen rect/sibling z-order/liveness。
- `repaint_container(container)` —
  `RedrawWindow(RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW)`。
- `apply_layered(container) -> Vec<ContainerMutationOutcome>` —
  exstyle OR → `SWP_FRAMECHANGED` → `LWA_ALPHA(255)` の 3 op。
- `reparent_to_desktop(container, baseline_exstyle, rect)` —
  baseline exstyle 復帰 → frame apply → `SetParent(desktop)` →
  `HWND_TOP` で rect 再配置の 4 op。
- `restore_container(container, baseline)` — baseline parent への
  reparent → baseline exstyle → frame apply → `ScreenToClient` で
  変換した baseline rect の復帰 → best-effort の sibling z-order
  再配置 → parent/exstyle/rect の verified 判定。

非 Windows stub は全 API で `applied: false` / `supported: false`
を返し、成功を捏造しない。

### D2: `exstyle_write` は read-back 検証で `applied` を決める

`SetWindowLongPtrW` の成功判定は last-error 規約（`SetLastError(0)`
→ call → `GetLastError`）。しかし今回の実走で、window が別
thread に属する child への `GWL_EXSTYLE` write は last-error 上
「成功」を返しつつ実際には反映されない（silent drop）ことが
観測された。`applied` は「要求した状態変化が実際に起きた」を
意味すべきなので、call 後の `observe_container` read-back で
`exstyle.value == 要求値` を検証し、一致しなければ
`applied: false`・`win32_error: None`（OS が error を返さない
以上）で記録する。`after` snapshot が実際の状態を残すため、
「API は成功を報告したが bit は立たなかった」が evidence から
読み取れる。

### D3: `ContainerSnapshot` に `class_style` を含める

`WS_EX_LAYERED` が latch しない場合、文書化された理由の 1 つは
window class の `CS_OWNDC`/`CS_CLASSDC` 保有。判別には
`GetClassLongPtrW(GCL_STYLE)` の生値が必要。JSON 投影では
`own_dc`/`class_dc` の 2 bit を decode して記録する。実走値は
`0x4088`（`CS_GLOBALCLASS|CS_PARENTDC|CS_DBLCLKS`）で両 bit
非該当 — これにより「class 由来の禁止」は棄却された。

### D4: probe の順序は layered → reparent の分離

`apply_layered`（GPUI parent 下のまま exstyle 変異）を先に試行し、
`reparent_to_desktop` の冒頭で baseline exstyle に戻してから
`SetParent` する。これで「container の style 変更」と「container
の parentage 変更」の効果を分離して記録できる。両 stage の後に
`repaint_container` + `be_settle`（500ms `std::thread::sleep` —
`uc_settle` と同じ方式）+ `be_observe_*` を置く。

### D5: verdict は required mutation 拒否を strict fail とする（Issue #917 で as-child layered を期待値化）

`RedrawWindow`/`SetWindowPos`/`SetParent` は自 process の live
window への正規呼出であり、legitimate な拒否経路を持たない —
拒否は「probe が設計どおり完走しなかった」事実として `fail`。
pixel/hit-test の観測値は verdict を決めない note 級 data。
これは既存 foreground/uncovered case と同じ意味論である。

当初 D5 は `GWL_EXSTYLE`/`SetLayeredWindowAttributes` の拒否も
一律 `fail` とし、実走では `be_apply_layered_exstyle` の拒否に
より `fail` が記録された。その後の確定診断（class C —
`docs/discovery/issue-532-dcomp-residual.md`）で、as-child
`WS_EX_LAYERED` write の silent drop と続く `LWA_ALPHA` の
`ERROR_INVALID_PARAMETER` 拒否は `WS_CHILD` window に対する
確定制約であることが判明した。Issue #917 で verdict 契約を
意図的改訂: `be_apply_layered_exstyle`/`be_apply_layered_lwa`
は「確定機序どおりの期待拒否」の診断行（`verdict: note` +
`expected_outcome` — `record_expected_refusal_row`）として
verdict 入力から分離した。refusal の `applied`/`win32_error`/
`after` 記録は維持され、Fail→Pass の改竄ではなく期待値検証へ
の re-scope である。as-child latch 成功は確定機序と矛盾する
OS 依存の新規 data として `contradicts_confirmed_mechanism`
flag で記録し Pass を妨げない — lane が計るのは probe 契約の
完遂であり OS 実装詳細の不変性ではない。frame commit・
repaint・reparent・restore の required op は従来どおり
strict。

### D6: restore は parent → exstyle → frame → rect → z-order の順

`restore_container` は `SetParent(baseline_parent)` → baseline
`GWL_EXSTYLE` write → `SWP_FRAMECHANGED` → saved screen rect を
`ScreenToClient(baseline_parent)` で parent-client 座標に変換して
`SetWindowPos` → sibling `GW_HWNDPREV` 先頭への best-effort
insert-after。parent/exstyle/rect は deterministic に検証し、
z-order は記録のみ（topmost 以外 exact を主張しない —
`RootSnapshot` と同じ方針）。

## probe の流れ（`real-dcomp-bandescape`）

`prepare_editor_host` → `attempt_real_attach` → lease 生存中に:

1. `be_baseline_state` — container + root snapshot
2. `be_observe_baseline` — hit-test ×5 + screen capture
3. `be_apply_repaint` → `be_update_pending` → `be_observe_repaint`
4. `be_apply_layered_{exstyle,frame,lwa}` →
   `be_apply_layered_repaint` → `be_settle(layered)` →
   `be_observe_layered`
5. `be_apply_reparent_{exstyle,frame,parent,rect}` →
   `be_settle(reparent)` → `be_observe_reparent`
6. `be_restore`（verified）→ `be_observe_restore`
7. `conclude_editor_session` — lifecycle 収束

## verdict 契約（Issue #917 で意図的改訂 — as-child layered を期待値化）

- `pass`: required mutation（repaint/frame/reparent/restore 系）
  applied + restore verified + session lifecycle 収束。as-child
  `be_apply_layered_exstyle`/`be_apply_layered_lwa` の拒否は
  確定 class-C の期待値であり `verdict: note` の診断行として
  記録される（Issue #917）。
- `fail`: required mutation の拒否/失敗、restore 検証の不一致、
  session 破壊。
- `not_run`/`unsupported`: capability/session 不在（headless や
  real-attach 不可では fail-closed で `not_run`）。
- green/hit-test/class_style/z-order/as-child latch 成否は
  verdict を動かさない note 級 data。**screen green が正でも
  Issue #532 は閉じない** — 本 probe の成功は「経路が画素を
  現すか」の観測であって修正の完了ではない。

## 判断の解釈表（設計時点の予約 → 実走結果で確定）

| layered latch 成否 | reparent 後 green | 導出 direction |
| --- | --- | --- |
| 適用・green >0 | — | minimal layered-container change |
| 拒否/不適用 | green >0 | reparent/owned-popup redesign |
| 拒否/不適用 | green =0 | capture-plane/child-level 診断へ escalate |

実走は「拒否 + green =0」— escalate 側が確定した。

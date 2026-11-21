---
id: "SPEC-rf857PixelOcclusion"
type: "spec"
status: "approved"
title: "Issue #857 pixel-occlusion — real-dcomp-pixel の lane 自身 top-level window 遮蔽（class B）処遇設計"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/857"
    - "crates/ruviform-plugin-dock-test/src/real_dock.rs"
    - "crates/ruviform-plugin-dock-test/src/osmut.rs"
    - "crates/ruviform-plugin-dock-test/src/osoccl.rs"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-857-pixel-occlusion.md"
      - type: "file"
        path: "docs/specs/issue-857-pixel-occlusion.md"
      - type: "file"
        path: "docs/test-specs/issue-857-pixel-occlusion.md"
      - type: "dir"
        path: "evidence/rf857/"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-pixel.md"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-857-pixel-occlusion.md"
    - "docs/test-specs/issue-857-pixel-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/specs/issue-839-gpui-dcomp.md"
    - "evidence/rf532/plugin-dock-real-pinned.jsonl"
    - "evidence/rf532/residual-disposition.md"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
    - "evidence/rf857/"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-residual.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-857-pixel-occlusion.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/839"
  defers: []
  deferred-by: []
metadata:
  issue: 857
---

# Issue #857 pixel-occlusion — `real-dcomp-pixel` class B 処遇設計

`real-dcomp-pixel` が lane 自身の spawn する別 top-level
`Zed::Window`（w2–w4）に被測定 child rect を被覆されて fail する
問題（class B — `docs/discovery/issue-532-dcomp-residual.md` で確定）
への処遇設計。調査根拠は
`docs/discovery/issue-857-pixel-occlusion.md`、case と検証の対応は
`docs/test-specs/issue-857-pixel-occlusion.md` を参照。

## 設計決定（確定）

**採用方式: (a) osmut の既存 restore 付き mutation で被測定 root を
sampling 中だけ最前面化する — arm は `raise` → `set_topmost`
pair（`real-dcomp-foreground` の実績順序）+ `after.topmost` 未確認
時の bounded retry（settle + 1 回）。**

`observe_child_pixels`（screen oracle 採取経路）の開始時に
`deocclusion_promote` が `osmut::observe`（first child の `GA_ROOT`
解決 → `RootSnapshot`）→ `osmut::raise_to_top(root)` →
`osmut::set_topmost(root)` を行い、全 child 採取（screen capture・
`PrintWindow`・window DC・`osoccl::probe`）の完了後に
`deocclusion_restore` が `osmut::restore(baseline)` で baseline
band・z-order・foreground を復元する。`set_topmost` の after-
snapshot が `topmost` を示さない場合は `BE_SETTLE_MS`（500ms）の
`std::thread::sleep`（caller thread 上の bounded settle — GPUI app
thread は並行して pump し続ける既存イディオム）の後に retry を 1
回だけ行い、retry の after-snapshot が受理の有無を正直に記録する。
osmut の bounded + restore 契約（`topmost_restored` が唯一の決定的
検証、z-order / foreground は best-effort 記録）にそのまま乗る。

### arm 確定の経緯（実走による 3 段階の確定）

実走で次の 3 段階を踏んで arm を確定した（詳細は
`evidence/rf857/run-record.md`）:

1. **`raise_to_top` 単独**: `HWND_TOP` = non-topmost band 先頭にしか
   配置できず、topmost band の外部 window（本 host 実測:
   `Chrome_RenderWidgetHostHWND` が全画面被覆）を越えられない —
   hit-test 全点 role `other` 残存 → gate が `NotRun` に退避
   （fail-closed の実証でもある）。
2. **`set_topmost` 単独（cold `HWND_TOPMOST`）**:
   `SetWindowPos(HWND_TOPMOST)` が `applied: true` を返しながら
   after-snapshot 上 band 未変更（`topmost: false`・`above_count`
   36 のまま）— この局面での cold band 変更は OS に受理されない
   実測。hit-test `other` 残存 → `NotRun`。
3. **`raise` → `set_topmost` pair（`real-dcomp-foreground` の実績
   順序）**: pinned 証拠でも fg probe がこの順序で `after.topmost:
   true`・`above_count: 9`・hit `editor_child`・green 0.999994 を
   達成している。pixel 経路に同順序を適用した最終 run も
   `after.topmost: true`・`above_count: 8`・hit `editor_child`・
   green `0.999994`・`pass` を達成。cold `HWND_TOPMOST` 不受理の
   mechanism は未確定のため、`after.topmost` 未確認時の bounded
   retry（settle + 1 回）を残す — 未発火でも記録経路は存在する。

### 棄却した方式

- **(b) covering 側 top-level（w2–w4）を被測定 rect 外へ退避/最小化
  する新 mutation family**: osmut に新 family を要し、3 window 分の
  snapshot・move・restore で変異面が増える。加えて lane 外の外部
  window（本 host 実測の Chrome topmost）には無力で、fail-closed
  gate はどのみち必要。採用する利得がない。
- **(c) pixel case を window 単独 session に分離**: sentinel window A
  の役割（最後の window close で GPUI loop が終了）、`window_titles`
  digest、`RUN_ID` / lane 証拠の整合をすべて再設計する必要があり、
  z-order で解決できる問題に対して blast radius が過大。
- **初期配置オフセット変更のみ**: 単一 display では display 境界
  clamp で w2–w4 が同一 client origin（192,120・1536x864）に着地
  し全面重複するため不成立（`docs/discovery/issue-857-pixel-occlusion.md`
  の `window_identities` 実測参照）。

## fail-closed verdict gate（verdict 意味論）

`occlusion_hit_test`（`WindowFromPoint` 5-point probe）の hit role が
`other`（被測定 hierarchy 外の top-level window）を返す間は、screen
oracle 画素を `Pass`/`Fail` のどちらにも昇格させない。

- `record_occlusion_notes` が `osoccl::probe` の `OcclusionFacts` を
  返し、`observe_one_child_pixels` が **評価済み capture を持つ
  child** の hit role を `occlusion_role` で判別して
  `PixelScan.occluded_points` に集計する。
- `PixelScan.occluded_points > 0` のとき `PixelObservation::Occluded`
  を返し、`editor_pixel_body` の verdict 分岐が `not_run(...)` に
  写像する（`environment_blocked` 相当の honest verdict — lane の
  fail-closed 意味論と整合。case-level の `observed` extras には
  `screen_pixels.occluded { occluded_points, evaluated_children,
  covering }` を記録）。
- `Occluded` は `GreenSeen`・`NoGreen` の **判定前** に評価する —
  外部 window の画素が緑に見えても `Pass` に、見えなくても `Fail`
  に読み替えない。
- 脱遮蔽成功 run（`deocclusion_apply_topmost` applied かつ全 hit が
  `editor_child`/`container`/`root`/`none`）では本経路は不発で、
  verdict は従来どおり green oracle の実測に委ねられる。

## verdict 意味論の確定対応表

| 状況 | `PixelObservation` | case verdict |
| --- | --- | --- |
| lifecycle NG | —（`lifecycle_ok` fail 分岐が先） | `Fail`（lifecycle detail） |
| hit `other` が 1 点以上（評価済み child） | `Occluded` | `NotRun`（fail-closed） |
| 全 hit が hierarchy 内・green >= 0.5 | `GreenSeen` | `Pass` |
| 全 hit が hierarchy 内・green < 0.5 | `NoGreen` | `Fail`（遮蔽起因でない実ミス） |
| capture 不能・非表示・children 無し | `NotEvaluable` | `NotRun` |
| open 拒否 / 非 Windows | — | `Unsupported` |

## `detail` 文字列の追従

旧文面「consistent with the known GPUI DComp topmost occlusion」は
instrumentation 導入前のもので確定 class（class B — lane 自身の
top-level window 遮蔽）と一致しない。`NoGreen` 分岐の detail を「the
hit-test resolved inside the measured hierarchy, so the miss is not
lane-window occlusion」へ更新し、被測定 hierarchy が hit-test 可視
なのに緑が出ない実ミス（compositor/raster 起因）である旨を正直に
記録する。

## 対象外（本 slice で扱わない）

- class C（`real-dcomp-bandescape`/`real-dcomp-childinv`）の verdict・
  re-scope — Issue 対象外。既知 Fail 継続で lane exit code は 1 の
  まま（`bin/plugin-dock-real-scenario.rs` の exit 意味論は不変）。
- compositor 側（gpui fork）の変更 — #839 で確定済み。
- `osoccl::probe` の query-only 契約、`RealOsGuiTest` の scenario
  contract 面、driver boundary — 変更なし。

## Limitations

- `set_topmost` が適用されても、採取の瞬間に他 thread が topmost
  window を新たに最前面化する race は理論上残る — その場合は
  hit-test が `other` を返し gate が `NotRun` に退避する（fail-
  closed が race の安全弁）。
- `WindowFromPoint` は hidden/disabled window を skip するため、hit
  role `none` は「非被覆でも child が skip された」可能性を含む —
  gate は `other` のみに発火し、`none` は `child_window`/
  `parent_window`/`root_window` の visible/disabled 記録で判読する
  （従来の skip-semantics 設計を維持）。
- `deocclusion_restore` の z-order / foreground は best-effort —
  `topmost_restored` のみ決定的検証で、残りは観測値として記録する
  （osmut の既存契約どおり）。

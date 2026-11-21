---
id: "DISC-rf857PixelOcclusion"
type: "discovery"
status: "approved"
title: "Issue #857 pixel-occlusion — real-dcomp-pixel lane 自身 top-level window 遮蔽（class B）調査記録"
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
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/specs/issue-532-dcomp-residual.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-857-pixel-occlusion.md"
    - "docs/test-specs/issue-857-pixel-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "evidence/rf532/plugin-dock-real-pinned.jsonl"
    - "evidence/rf532/residual-disposition.md"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
    - "evidence/rf857/"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-532-dcomp-residual.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-857-pixel-occlusion.md"
    - "evidence/rf857/run-record.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/839"
  defers: []
  deferred-by: []
metadata:
  issue: 857
---

# Issue #857 pixel-occlusion — `real-dcomp-pixel` class B 調査記録

Issue #532 の残存 Fail 集合から切り出された class B —
`real-dcomp-pixel` が lane 自身の spawn する別 top-level window に
被測定 child rect を被覆されて fail する問題の、処遇設計に必要な
調査記録。class 切り分けの正本は
`docs/discovery/issue-532-dcomp-residual.md` /
`evidence/rf532/residual-disposition.md`（読み取り専用）。設計確定は
`docs/specs/issue-857-pixel-occlusion.md`、実走証拠は
`evidence/rf857/run-record.md` を参照。

## 前提として確定済みの事実（#532 residual からの転記）

`evidence/rf532/plugin-dock-real-pinned.jsonl` の
`real-dcomp-pixel` row が示す失敗構造（SHA `57c4619b` 当時 —
`real_dock.rs` は本 slice 着手時点で同 SHA から無変更で、証拠は
現行コードの挙動をそのまま記述する）:

- `screen_pixels`（`win32-bitblt-screen`）green_fraction `0.0` —
  child rect `321,153,1278x798` の screen 画素に緑が無い。
- `windowdc_pixels`・`printwindow_pixels` はともに green `1.0` —
  child HWND raster は実在し、class A の根治 fix（gpui fork
  `v0.2.2-rf2`）が raster 復元まで効いている。
- `occlusion_zorder` は w1 階層内の sibling 遮蔽なし
  （`above_editor: false`）。
- `occlusion_hit_test` の全 5 probe point が hwnd `160105518`
  class `Zed::Window` rect `184,89,1552x903` role `other` を返す —
  lane が spawn する別 top-level window（`window_identities` の
  w2–w4 と同一 geometry）が被測定 rect を全域被覆。

つまり失敗の直接因は compositor（class A・#839 で根治済み）では
なく **HWND レベルの外部 top-level 遮蔽** — lane の window 配置と
verdict 意味論の問題であることが確定済み。

## 本 slice で新たに確定した事実

### 1. 初期配置オフセット変更は不成立（display clamp 実測）

scenario は 4 window を mount する（`real_dock.rs` — w1=
`WindowSpec::default()`・w2–w4 に position 指定）。`spec_options`
の写像は display 境界に clamp するため、単一 display 環境では
w2–w4 の指定 position が効かず全て client origin `192,120`・
`1536x864` に着地し、w1（`320,152`・`1280x800`）と全面重複する。
「offset right of A so the windows never overlap」の前提は単一
display では成立しない — 初期配置をずらすだけでは遮蔽を解消
できない。

### 2. non-topmost band の raise では外部 topmost 遮蔽を解けない（実走）

採用方式 (a) の arm として `raise_to_top`（`HWND_TOP`）を初版に
採用したが、interactive 実走で `deocclusion_apply_raise`
applied=true（`above_count` 36→33 = non-topmost band 先頭に到達）
の後も `occlusion_hit_test` 全 5 点が外部
`Chrome_RenderWidgetHostHWND`（0,0,1920x1032 — 全画面被覆）を
返した。`HWND_TOP` は non-topmost band しか動けず、topmost
band に居る外部 window（本 host 実測で `above_count` 33 の帯域
に Chrome 等が存在）を越えられない。

### 3. cold `HWND_TOPMOST` が受理されない局面がある（実走）

`set_topmost` 単独 arm でも `SetWindowPos(HWND_TOPMOST)` が
`applied: true` を返しながら after-snapshot 上 `topmost: false`・
`above_count` 36 のまま — band 変更が OS に受理されなかった。
同一 run 内・同一 root で `real-dcomp-foreground` が実行する
**`raise` → `topmost` の pair** は `after.topmost: true`・
`above_count: 9` で受理される（pinned 証拠でも同一挙動）。
確定 arm はこの実績順序に合わせた。mechanism は未確定（in-
flight の window 作業との相互作用の可能性）で、受理されない
場合の bounded retry（settle + 1 回）も arm に含める。

### 4. fail-closed gate の実走挙動（verdict 意味論の実証）

遮蔽残存時の 2 回の実走では、`occlusion_hit_test` の role
`other` を検出した `PixelObservation::Occluded` が case verdict
`not_run` に写像され、外部 window の画素（green 0.0）は
Pass/Fail どちらにも読み替えられず `screen_pixels.occluded`
extras として記録された — `environment_blocked` 相当の honest
verdict として機能することを実走で確認した。

### 5. 脱遮蔽後の verdict 実測（最終 run）

`raise` → `set_topmost` pair 適用後（`after.topmost: true`・
`above_count: 8`）、`occlusion_hit_test` 全 5 点が role
`editor_child` を返し、`screen_pixels` green `0.999994`（backend
`win32-bitblt-screen`）で case verdict `pass`。`deocclusion_restore`
は `topmost_restored: true`・z-order baseline 復元・foreground
hand-back を達成。詳細は `evidence/rf857/run-record.md`。

## verdict 意味論の変更点（本 slice）

- `PixelObservation` に `Occluded` variant を追加 —
  `record_occlusion_notes` が返す `OcclusionFacts` と
  `occlusion_role` 判別を `PixelScan` 経由で verdict 経路へ
  伝搬する。
- `editor_pixel_body` の verdict 分岐で `Occluded` を
  `not_run(...)` に写像 — `GreenSeen`/`NoGreen` 判定より前に
  評価し、外部 window 画素を Pass/Fail へ昇格させない。
- `NoGreen` 分岐の `detail` 旧文面（"known GPUI DComp topmost
  occlusion" — instrumentation 導入前の文面）を確定 class に
  合わせて更新 — hit-test が被測定 hierarchy 内を返した上での
  実ミスである旨を記録する文面へ。

## Limitations

- cold `HWND_TOPMOST` が受理されない mechanism（in-flight window
  作業との競合か、session.open 直後の transient 状態か）は未確定
  — 採用 arm は実績順序（`raise` → `topmost`）+ bounded retry で
  対処し、残存失敗は fail-closed gate が `NotRun` として正直に
  記録する設計。
- 遮蔽 window と `window_identities` の対応は geometry 一致による
  推定（#532 residual と同じ制約）。
- 本 host の外部 topmost window（Chrome 等）は環境依存 — 遮蔽が
  残存した環境では gate が `NotRun` に退避する（verdict の偽装
  ではなく環境依存として記録される）。

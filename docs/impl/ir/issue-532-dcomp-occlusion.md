---
id: "IMPL-034YomLkmcYL2Rtts2CVCJ"
type: "impl"
status: "completed"
title: "issue-532-dcomp-occlusion"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/osoccl.rs"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-occlusion.md"
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-occlusion.md"
      - type: "file"
        path: "docs/plans/issue-532-dcomp-occlusion.md"
      - type: "file"
        path: "docs/specs/issue-532-dcomp-occlusion.md"
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-occlusion.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-occlusion.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-532-dcomp-occlusion.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/discovery/issue-532-dcomp-occlusion.md"
    - "docs/plans/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
    - "docs/impl/exp/issue-532-dcomp-occlusion-exp.jsonl"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-532-dcomp-occlusion-exp.jsonl"
    rejected: []
---

# issue-532-dcomp-occlusion

## Summary

`rf631.plugin-dock-real` の `real-dcomp-pixel` case に、query-only の
遮蔽診断を追加した。新規 module `osoccl` が GPUI root HWND の
extended style（`WS_EX_NOREDIRECTIONBITMAP` 等）、root/parent の
direct-child z-order、child 矩形内代表 5 点の `WindowFromPoint`
結果を plain JSON 値として採取し、`real_dock.rs` が
`occlusion_exstyle` / `occlusion_zorder` / `occlusion_hit_test`
の note-level step row として記録する。診断は read/query API のみ
で window 生成・移動・style/z-order/region 変更・message 送信を
行わず、既存の screen/PrintWindow/window-DC oracle・`PixelScan`・
`PixelObservation`・lifecycle 契約・case verdict・exit code を
一切変更しない。

## Context

`scarecrow173/ruviform#532` の診断 slice。`docs/discovery` →
`docs/specs` → `docs/designs` → `docs/plans` → `docs/tasks` →
`docs/test-specs`（いずれも `issue-532-dcomp-occlusion`）の
チェーンに従う。目的は遮蔽が Win32 HWND 層（root/container/
sibling/z-order）にあるか GPUI DirectComposition / flip-model
提示面の内部にあるかの二分で、compositor 修正自体は範囲外。

## Implementation

### Changes

- `src/osoccl.rs`（新規・`real-dock` feature）: `probe(child: u64)
  -> OcclusionFacts` 単一入口。Windows 実装は `GetAncestor
  (GA_ROOT)`・`GetWindowLongPtrW(GWL_EXSTYLE)`・`GetWindow
  (GW_CHILD)`+`GetWindow(GW_HWNDNEXT)`・`GetParent`・
  `GetClassNameW`・`GetWindowRect`・`IsWindow`・
  `IsWindowVisible`・`WindowFromPoint` の query API のみ使用。
  HWND は `u64` に narrow、`ScreenRect` は `ospix` の型を再利用。
  非 Windows は stub で `supported=false` を honest に返す。
  `ExstyleFacts` は生値 + flag accessor（`noredirectionbitmap` 等）
  で `struct_excessive_bools` を回避。z-order 列挙は失敗し得ない
  ため `Option<ZOrder>`。
- `src/lib.rs`: `#[cfg(feature = "real-dock")] mod osoccl;` 追加。
- `src/real_dock.rs`: `observe_one_child_pixels` から child attach
  中に `record_occlusion_notes(hwnd, container, steps)` を呼び、
  3 つの `verdict:"note"` row を追加。role 分類
  （`editor_child`/`container`/`root`/`other`/`none`）と
  `above_editor`/`above_container` を付与。診断失敗は note/error
  または unevaluable に留め Pass/Fail を捏造しない。コメントで
  `WindowFromPoint` が HWND manager z-order のみを見ること
  （DComp visual・flip-model plane は HWND でなく hit-test に
  現れない）を明記。
- review-protocol 反映（`2e856bca`）: `WindowRow.disabled`
  （`GWL_STYLE` の `WS_DISABLED`）追加、`OcclusionFacts` に
  probe 時点の chain 状態（`child`/`parent_window`/`root_window`
  の `WindowRow` + `child_exstyle`/`parent_exstyle`）追加 —
  `WindowFromPoint` の hidden/disabled skip semantics により
  non-child answer が "covered OR skipped" を意味するため、
  cover-vs-skip を判別可能にした。hit probe 座標を矩形内に
  clamp（<4px 矩形の端外れ防止）、hit NULL entry を uniform
  key 化。`oschild` の "direct child" コメントを "descendant"
  に訂正（`EnumChildWindows` は再帰）。README case 表に
  occlusion note を追記。discovery/design doc の事実誤認・
  API drift を訂正。

### Decisions During Implementation

- `osoccl` を `ospix` に統合せず独立 module とした — `ospix` は
  画素 capture、`oschild` は child lifecycle、`osoccl` は
  root style/z-order/hit-test と責務が異なり、境界を保つ方が
  一貫する（design doc の決定どおり）。
- probe 点は child 矩形の中心 + 4 隅内側点（`(w/4,h/4)` 等）
  の 5 点 — 矩形全域を代表しつつ API 呼出を最小化。
- z-order は `GetWindow(GW_CHILD)` 起点の sibling walk のみ —
  top-level 間 z-order は `WindowFromPoint` が担うため child
  walk の範囲で十分（DComp visual は HWND でないため列挙対象外、
  これは仕様上の制限として JSONL note に残る）。
- 実機 run は review 反映 commit（`2e856bca`）後に canonical
  baseline と env-on の双方を再実行し、両 JSONL の
  `environment.repo_sha` を commit SHA に揃えた（初回実装
  commit `0fe7bdec`・未コミット tree `cbcc8343` での run も
  exp log に記録）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-532-dcomp-occlusion-exp.jsonl` — baseline/env-on
  実機観測、exstyle flag の env 依存、z-order/hit-test 結果、
  「外部 Chrome HWND が WM z-order で上」の観測事実と
  compositor 内部仮説の未解決判定を記録。

### Rejected

- なし。

## Validation

実施済み（詳細は `evidence/rf631/verification.md`）:

- `mise run fmt-check` / `cargo fmt --all -- --check` — pass
- `mise run check|lint|test ruviform-plugin-dock-test`（default）
  — pass
- `mise run check|lint|test ruviform-plugin-dock-test --features
  real-dock` — pass
- `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console` /
  `test-ui-scenario-contract` / `check-ui-driver-boundary` — pass
- `mise run test-ui-plugin-dock-real`（baseline・interactive
  session・repo_sha `2e856bca`）— exit 1、`real-dcomp-pixel`
  verdict `fail`（screen green 0.0、occlusion note 3 row 記録）
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` env-on 実走（別 `--out`・
  `--pixel-artifacts` 付・repo_sha `2e856bca`）— exit 1、同様に
  `fail`。`noredirectionbitmap` が `true→false` に変化し env var
  の GPUI 初期化への実効を HWND レベルで確認。exit 127 未観測
- review-protocol: Reviewer A（cooperative）/ Reviewer B
  （adversarial）の Issue Candidate を Validator/Judge で処理 —
  CONFIRMED P2（cover-vs-skip 記録欠落・discovery doc 事実誤認）
  を修正、P3 群も同時修正、残件は Limitations に記録
- `mise exec -- prek run --all-files` — pass

## Risks

- screen `BitBlt` oracle は観測時点で child 矩形を覆う topmost
  HWND の画素を返す — 本環境では Chrome window が GPUI window
  より上にあり（cover-vs-skip は chain state で判別済み:
  skip 条件非該当のため genuine な cover）、screen=0.0 は
  外部 HWND 遮蔽のみで説明可能。compositor 内部（DComp visual
  が child HWND を隠すか）は screen oracle では判別不能で
  `unresolved` のまま。
- `WindowFromPoint` は DComp visual / flip-model plane を見ない
  — 「HWND 層に遮蔽物無し」は compositor 内部の無遮蔽を意味
  しない。
- z-order walk・probe は観測時点の snapshot — probe 中の
  非同期 destroy/recreate とは race し得る（WindowRow の
  alive/visible/disabled/class_name が probe 時点 state を
  記録するため判別材料は残る）。
- 単一ホスト・単一 GPU・単一 scale 環境での観測。

## Follow-ups

- compositor 内部遮蔽の判別診断（後続 slice 候補）: real-window
  lane で window を foreground 化するか、GPUI root HWND への
  `PrintWindow` 等で window 内部合成を直接観測する経路。
- 方式採否（A/A'/B/C）— 本 slice の「env var は exstyle に実効
  あり・外部 HWND 遮蔽が観測された」を入力に後続 slice で実施。
- `mise.toml` L166-169 の stale コメント（「editor-pixel question
  stays deferred」）は scope 外・未修正、Coordinator 統合時に対応。
- docs index（`docs/*/README.md`、`docs/impl/*/README.md`）は
  Coordinator が統合時に再生成。
- `Cargo.lock` 差分なし（既存 `windows-sys` feature の範囲内）。

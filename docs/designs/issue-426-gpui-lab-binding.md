---
id: "DESIGN-rO8ZT7WxqPYGw0e0jZJx_A"
type: "design"
status: "proposed"
title: "Issue #426 設計 — mechanism の test 実在検査・GPUI lab binding・story matrix"
created: "2026-11-08"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-426-gpui-lab-binding.md"
    - "https://github.com/scarecrow173/ruviform/issues/426"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lab_binding.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lab_story.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/lab_story.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/golden_capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
---

# Issue #426 設計

## 1. mechanism の実在検査（`driver_lab.rs`）

現状の `mise-task` mechanism は task 名の存在だけを検査し、`pass` の根拠となる test
が無くても通ってしまう。`mise-task` に `package` + `filter` を任意指定できるようにし、
指定時は package の `src/` と `tests/` を再帰走査して test 属性付き fn の実在を要求する。

- test 属性判定は `cargo-test` と共通化する（`#[test]` / `#[tokio::test]` / `#[gpui::test]`）。
- `filter` は `cargo test` と同じ substring 規則。
- `filter` のみで `package` が無い指定は検査エラーとする（検査対象が定まらないため）。

検証: 実在しない filter を一時的に指定すると `test-ui-driver-contract` が失敗することを確認済み。

## 2. GPUI lab binding（`ruviform-ui-gpui`）

GPUI 側の test は `#[gpui::test]`（test-support）で実行し、実経路を通す。

| case | 配置 | 検証内容 |
| --- | --- | --- |
| UI-DVR-01 | `lab_binding.rs` | GPUI window 作成 → Created → Ready |
| UI-DVR-03 | `render.rs` tests | window deactivate による drag 解放・cancel 後の遅延 preview 拒否・seq 逆転の OutOfOrder 拒否 |
| UI-DVR-04 | `render.rs` tests | consumer override が ViewPalette に反映、未宣言 mode は resolver が拒否 |
| UI-DVR-07 | `lab_binding.rs` | 宣言 capability のうち Supported は実操作が成功、ime は Unsupported、native_surface_slot は OS 別宣言 |
| UI-DVR-08 | `lab_binding.rs` | GPUI window の Destroyed 後、close policy の `cancel_window_jobs` で job が Cancelled に至る |

`lab_binding.rs` は `gpui-test` 時のみコンパイルする（`lifecycle_fixture` と同じ gate）。
cancel 検証は `RvfView::mounted`（本番の購読経路）で行う。`RvfView::new` は
window activation 購読を持たないため、テストで使うと cancel 経路を通さない。

gesture の cancel は `CommandIntent::Cancel` を発行する
（`intent_from_gesture` が `InteractionEvent::Cancel` を写像する。
第 2 段の story oracle が unmount 時の cancel intent を実測で確認済み）。
UI-DVR-03 の検証は committed 値の不変と drag 解放で行い、intent 列は
対象にしていない。

## 3. Story matrix（`lab_story.rs` 共有 fixture + 両 adapter）

story の semantic 遷移（idle → focus → drag → commit → stale → invalid → teardown）を
`SemanticGuiTest` port 経由で歩く。第 2 段では story 定義を
`crates/ruviform-ui-test/src/lab_story.rs` に集約し、Fake / GPUI-simulated の
両 adapter が**同一の canonical `ViewSpec`・同一の `ComponentKey`・同一の
expected oracle**（`expected_lab_story_trace()`）を共有する。

- `StoryFixture` は manifest id・宣言 theme・mount spec・walk 対象 key・
  teardown spec を持つ。`lab_story_walk` は adapter-neutral で、teardown の
  spec 差し替えだけを呼び出し側の closure に委ねる。
- `StoryStep` は intent kind 列・committed/display・phase・mounted・
  observe error kind を構造化して記録する（文字列 trace ではない）。
- teardown で walk 対象 key が unmount される際、開いたままの gesture は
  detach policy により cancel され `cancel` intent が mint される — これが
  実際の `ViewHost` 挙動であり、oracle はそれをそのまま期待値にする。
- `synvelis-dark` / `audiolens-dark` × scale 1.0 / 2.0 の 4 組合せで
  trace が一致することを Fake（`scenario_contract.rs`）と GPUI
  （`tests/lab_story.rs`、`GpuiGuiTest`/`GpuiObserver` 経由の実 GPUI
  window）で検証する。`component_view.rs` の matrix は同じ fixture を
  直接 `ViewHost` に mount する第 3 の確認経路。
- `ViewHost` は theme・scale を持たない。scale 軸は `ResolvedTheme::with_scale` の
  `ScaleContext` で与え、tokens が不変であることを同時に確認する。
  GPUI 層では `ViewMount.theme` に resolved theme を渡すが、semantic/draw
  経路は scale hint を消費しない — scale の実測差は window pixel の話であり
  Visual/real-os lane の責務。
- GPUI 層では `GpuiObserver::drawn()` の draw plan が行列全体で同一である
  ことも確認する。draw plan は geometry/text/値の一致であり pixel ではない
  （色は palette 側にあり plan に含まれない）。

## 3b. GPUI story golden link（`tests/lab_story.rs` / `tests/golden_capture.rs`）

- gpui-test（`GpuiGuiTest`）には pixel readback が無いため、story-keyed
  `GoldenKey` に対する `golden_check` は `Unsupported` を返す。これは
  capability の正直な報告であり、`record_golden_step` で story
  id/theme/state/driver_class を含む evidence 行を scenario report に残す。
  Fake adapter も同じ link shape（`scenario_contract.rs`）。
- Win32 実窓 backend（`GpuiDriver::capture_window`、PrintWindow 主・
  BitBlt fallback）は #314 slice 3 で実装済み。`tests/golden_capture.rs` の
  e2e を共通化し、canonical story spec を宣言 theme 付きで mount した
  story-keyed capture（`idle` state、scale factor は frame 実測値）を
  2 story 分追加した。ignored test（desktop session 必須）で、
  ローカル Windows で pass を実測済み。
- 実窓 capture の scale factor は display が報告する実測値であり、
  story manifest の `scaleFactor: [1.0, 2.0]` 全網羅は display 依存。
  window pixel の scale 1.0 再現は gpui test platform の制約上不能で、
  real-os/Visual lane の残件とする。

## 4. 共通層の修正（`ruviform-ui/src/view.rs`）

story matrix の実行で、`ViewHost::update` の `Removed` op 順序が `HashMap` の
反復順（hash seed 依存）になっていることが判明した。同一の spec 遷移で op 順序が
毎回変わるのは契約上の問題のため、削除キーを `sort` して決定化した。
これは consumer が依存しうる出力順の決定化であり、semantic の変更ではない。

## 5. Pixel Golden の扱い

- #314（RF-GT-05 GPUI Native Semantic/Visual Golden）の Golden Utility
  （headless core・Scenario 能力接続・Win32 capture backend）は main に
  存在する。本 Issue はその consumer として story-keyed link を接続する。
- 層の分離: semantic oracle（Fake/GPUI-simulated の `StoryStep` trace）、
  draw plan（`GpuiObserver::drawn`）、pixel（`BaselineStore` compare）の
  3 つは別証拠として記録し、混同しない。
- gpui-test の `VisualCapture` は Unsupported（pixel readback なし）。
  `golden_check` の `Unsupported` verdict と story-keyed evidence 行が
  正直な記録となる。
- 実窓 pixel は `tests/golden_capture.rs` の ignored test が担う
  （desktop session 必須・CI 対象外）。
- ruviform-ui-theme の golden は正規化 JSON の golden であり、pixel golden ではない（混同しない）。

## 6. Consumer GPUI 型不使用検査

既存の consumer mise task を実行する（consumer repo は変更しない）。

- audio-lens: `check-cli-headless`（headless crate の依存木に GUI/GPU/WebView framework が無いことを検査）
- synvelis: `check-headless-deps`（同等）
- 検査は依存木ベース。ruviform は consumer の `Cargo.lock` が pin する revision で評価される。
  本 PR の HEAD は consumer 側では評価されない（consumer 変更は範囲外）。

## 7. 非目標・残課題

- window pixel の scale 1.0（gpui test platform が `window.scale_factor` を
  2.0 固定。実測 scale の全網羅は real-os/Visual lane・#313/#314 の残件）
- story の real-os driver binding（OS 実入力は #313 owner のまま blocked）
- UI-DVR-03 の GPUI 側 Cancel は window deactivate 経路のみ（IME・実 OS focus は #313/#389）
- `golden_capture.rs` の story-keyed test は ignored（desktop session 必須）。
  CI/ヘッドレス lane では実行されず、実窓 pixel の継続的 gate は #314 側の
  lane 設計に従う

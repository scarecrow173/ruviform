---
id: "DESIGN-rO8ZT7WxqPYGw0e0jZJx_A"
type: "design"
status: "proposed"
title: "Issue #426 設計 — mechanism の test 実在検査・GPUI lab binding・story matrix"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-426-gpui-lab-binding.md"
    - "https://github.com/scarecrow173/ruviform/issues/426"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lab_binding.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
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

cancel は `CommandIntent` を発行しない（Begin/Preview/Commit のみ）ため、
「commit 非発行」は committed 値の不変で示す。

## 3. Story matrix（`component_view.rs`）

story の semantic 遷移（idle → focus → drag → commit → stale → invalid → teardown）を
`ViewHost` で歩き、trace を文字列化する。`synvelis-dark` / `audiolens-dark` × scale
1.0 / 2.0 の 4 組合せで trace が一致することを検証する。

- `ViewHost` は theme・scale を持たない。scale 軸は `ResolvedTheme::with_scale` の
  `ScaleContext` で与え、tokens が不変であることを同時に確認する。
- 各 state の期待マーカー（例: stale は `StaleGesture`、invalid は `NonFiniteInput`）を
  別途検査し、trace が空回りしないことを保証する。

## 4. 共通層の修正（`ruviform-ui/src/view.rs`）

story matrix の実行で、`ViewHost::update` の `Removed` op 順序が `HashMap` の
反復順（hash seed 依存）になっていることが判明した。同一の spec 遷移で op 順序が
毎回変わるのは契約上の問題のため、削除キーを `sort` して決定化した。
これは consumer が依存しうる出力順の決定化であり、semantic の変更ではない。

## 5. Pixel Golden の扱い

- #314（RF-GT-05 GPUI Native Semantic/Visual Golden）は OPEN。Golden Utility は repo に存在しない。
- `ruviform-ui-test` の `TestIo::screenshot` は headless 実装のみで、GPUI driver は pixel 取得を実装していない。
- したがって本 PR では pixel 取得・比較を行わず NotRun とし、rerun 条件を IR に記録する。
- ruviform-ui-theme の golden は正規化 JSON の golden であり、pixel golden ではない（混同しない）。

## 6. Consumer GPUI 型不使用検査

既存の consumer mise task を実行する（consumer repo は変更しない）。

- audio-lens: `check-cli-headless`（headless crate の依存木に GUI/GPU/WebView framework が無いことを検査）
- synvelis: `check-headless-deps`（同等）
- 検査は依存木ベース。ruviform は consumer の `Cargo.lock` が pin する revision で評価される。
  本 PR の HEAD は consumer 側では評価されない（consumer 変更は範囲外）。

## 7. 非目標・残課題

- GPUI story fixture の GPUI layer（scale 1.0 再現不可・Dock の GPUI 接続が未実装）
- Pixel Golden（#314）
- UI-DVR-03 の GPUI 側 Cancel は window deactivate 経路のみ（IME・実 OS focus は #313/#389）

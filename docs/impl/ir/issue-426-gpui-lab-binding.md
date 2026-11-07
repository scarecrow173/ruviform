---
id: "IMPL-_exo5tf9F8bfLQSN05ZaeA"
type: "impl"
status: "in-progress"
title: "Issue #426 実装記録 — GPUI lab binding・story matrix・mechanism 実在検査・Golden 境界・Consumer 検査"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-426-gpui-lab-binding.md"
    - "docs/plans/issue-426-gpui-lab-binding.md"
    - "docs/specs/issue-426-gpui-lab-binding.md"
    - "https://github.com/scarecrow173/ruviform/issues/426"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lab_binding.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
---

# Issue #426 実装記録

## 実装内容

| 領域 | 内容 |
| --- | --- |
| mechanism 検査 | `mise-task` に `package`+`filter` を任意指定可能にし、package の src/tests に test fn の実在を要求（`driver_lab.rs`）。負例（存在しない filter）で失敗することを確認。 |
| GPUI binding | UI-DVR-01/07/08 を `lab_binding.rs`、UI-DVR-03（cancel・seq 逆転）と UI-DVR-04（override・mode）を `render.rs` に追加。UI-DVR-02/05/06/09 の GPUI mechanism に filter を付与。 |
| Story matrix | LAB-SV-FADER-DOCK / LAB-AL-WAVEFORM-SEEK について、2 theme × 2 scale × 7 semantic state の trace 一致を fake layer で検証（`component_view.rs`）。 |
| 共通層 | `ViewHost::update` の Removed op 順序を sort で決定化（`crates/ruviform-ui/src/view.rs`）。story matrix の実行で hash seed 依存の不安定を検出したための修正。 |
| manifest | cases.json の GPUI binding 9 件（UI-DVR-01/02/03/04/05/06/07/08/09 のうち GPUI 関係）、stories.json の fake layer を pass に。GPUI story layer は not_run（理由・rerun 明記）。 |

## 検証結果（実行済み・ローカル Windows 11 / Git Bash）

| タスク | 結果 |
| --- | --- |
| `mise run test-ui-driver-contract` | pass（7 件。manifest 整合・mechanism 実在検査を含む） |
| 負例: UI-DVR-07 gpui の filter を存在しない名前に変更 | 失敗（期待どおり）。fixture は復元済み |
| `mise run test-ui-component-view` | pass（17 件。story matrix 2 件を含む） |
| `mise run test-ui-gpui` | pass（lib 46 件 = baseline 40 + 新規 6、加えて #429 の統合 test 4 件） |
| `mise run test-ui-parity` | pass（3 件） |
| `mise run test-ui-native-slot` | pass（13 件） |
| `mise run test-ui-test-msrv`（rust 1.89.0） | pass |
| `mise run check-ui-driver-boundary` | pass（dep-graph / signature / allowlist すべて pass） |
| `mise run check-ui-gpui` | pass |
| `mise run lint-ui-gpui` | pass（warning は第三者 crate の future-incompat 通知のみ） |
| `mise run lint` | pass |
| `mise run fmt-check` | pass（`mise run fmt` 適用後） |
| `mise run test`（workspace） | pass（exit 0。`test result` に失敗 0 件を確認） |

最終検証は `origin/main`（`c31dc67c`, Issue #429 観測 seam）を merge した HEAD `21be9695` で実施した。
merge は競合なし（`lib.rs` / `render.rs` は自動マージ）。merge 後の全項目が pass であることを確認している。
本 IR の更新コミットは検証内容を変えない文書のみ。

## Consumer 検査（read-only、consumer 変更なし）

| repo | タスク | 結果 |
| --- | --- | --- |
| audio-lens | `mise run check-cli-headless` | pass（headless crate 10 件に GUI framework 依存なし） |
| synvelis | `mise run check-headless-deps` | pass（同 10 件） |
| audio-lens | `mise run check-ui-gpui`（GPUI feature 型チェック） | pass（※ consumer の Cargo.lock が pin する ruviform revision で評価。本 PR の HEAD ではない） |

注意: 上記 consumer 検査は依存木ベースで、ruviform は consumer の pinned revision で評価される。
本 PR の HEAD を consumer で評価するには consumer 側の ruviform 依存更新が必要で、これは範囲外。

## NotRun / 未達

| 項目 | 状態 | 理由・rerun 条件 |
| --- | --- | --- |
| Pixel Golden（A3） | NotRun | #314（RF-GT-05）は OPEN。Golden Utility・GPUI pixel 取得は repo に存在しない。`TestIo::screenshot` は headless 実装のみ。#314 実装後に GPUI 描画の取得・比較を接続する。 |
| Story GPUI layer（A2 の GPUI 部分） | not_run | gpui-0.2.2 の test platform は `window.scale_factor` を 2.0 固定（`src/platform/test/window.rs`）。scale 1.0 を GPUI で再現できない。Story の GPUI 接続（Dock 含む）も未実装。 |
| UI-DVR-03 の実 OS 側 | not_run（既存） | 実 OS focus 喪失・IME は #313/#389 owner。 |
| 実 OS lane（#313/#388/#389/#262） | 対象外 | 本 Issue は参照・記録のみ。実機 Gate は代替しない。 |

## レビュー（review-protocol）

独立した read-only サブエージェント（新規コンテキスト）で diff をレビュー。
判定: **BLOCKER なし**。指摘は以下の 2 件（既知の限界として記録）。

- MINOR: `has_test_fn` は構文解析ではなく「直前 6 行の属性文字列」によるヒューリスティック。短い test 直後の非 test fn や コメント内の属性文字列を誤検出しうる。現行 fixture の filter はすべて正しい test を指すことを確認済み。
- MINOR: story matrix の theme 軸は `ViewHost` が theme/scale を受け取らないため構造上自明。実質的な保証は (a) `with_scale` が tokens を不変に保つこと、(b) 7 state の期待マーカー検査。設計書にその旨を記載。

## 作業中に発見した事項

- `RvfView::new`（テスト用 helper）は window activation 購読を持たず、cancel 経路を通らない。cancel 検証は本番の `RvfView::mounted` 経路で行う必要があった。
- gpui の `deactivate_window` はプラットフォームの active window が対象の時のみ効く。先に `window.activate_window()` が必要。
- `CommandIntent` は Begin/Preview/Commit のみで、cancel は intent を発行しない（設計どおり）。
- `ViewHost::update` の Removed op 順序が HashMap 反復順に依存していた（sort で修正済み）。
- `ResolvedTheme` の override は同一 profile id の文書を拒否する（`DuplicateSource`）。consumer override は別 id の product 文書で表現する。

## 範囲外・残課題（報告のみ）

- `mise run check-ui-webview` / `test-ui-webview` は未実行（`ruviform-ui` の変更は view.rs の sort 1 行で、webview API への影響は無い想定だが未検証）。
- docs の managed index（specs / designs / plans / tasks / impl ir の README）へ本 Issue の行を手で追加した。doc-maintenance による再生成と doc-status による検査は未実施。
- `ruviform-ui-test` の mechanism 検査ヒューリスティックの精度向上（構文解析化）は別 Issue 候補。

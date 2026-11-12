---
id: "IMPL-_exo5tf9F8bfLQSN05ZaeA"
type: "impl"
status: "in-progress"
title: "Issue #426 実装記録 — GPUI lab binding・story matrix・mechanism 実在検査・Golden 境界・Consumer 検査"
created: "2026-11-08"
updated: "2026-11-09"
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
      - type: "file"
        path: "crates/ruviform-ui-test/src/lab_story.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/lab_story.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/golden_capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #426 実装記録 — GPUI lab binding・story matrix・mechanism 実在検査・Golden 境界・Consumer 検査

## Summary

driver-lab の mechanism 実在検査（`package`+`filter` で test fn の
実在を要求）・GPUI lab binding（UI-DVR-01/07/08 を `lab_binding.rs`、
UI-DVR-03/04 を `render.rs`）・story matrix（2 theme × 2 scale ×
7 semantic state の fake 層 trace 一致）・Consumer 検査を実施した。
第 2 段（2026-11-09）では共有 story fixture
（`ruviform-ui-test/src/lab_story.rs`）・GPUI story matrix・
story-keyed 実窓 pixel capture（#314 Golden Utility の consumer
接続）を追加し、前段の残件（A2 GPUI layer・A3 Pixel Golden link）
を解消した。window pixel scale 1.0・実窓 capture の CI 化・
real-os lane は NotRun/blocked のまま（status: in-progress）。

## Context

- 実施タスク: [../../tasks/issue-426-gpui-lab-binding.md](../../tasks/issue-426-gpui-lab-binding.md)
- plan: [../../plans/issue-426-gpui-lab-binding.md](../../plans/issue-426-gpui-lab-binding.md)
- spec: [../../specs/issue-426-gpui-lab-binding.md](../../specs/issue-426-gpui-lab-binding.md)
- source: <https://github.com/scarecrow173/ruviform/issues/426>

## Implementation

### Changes（第 1 段）

| 領域 | 内容 |
| --- | --- |
| mechanism 検査 | `mise-task` に `package`+`filter` を任意指定可能にし、package の src/tests に test fn の実在を要求（`driver_lab.rs`）。負例（存在しない filter）で失敗することを確認。 |
| GPUI binding | UI-DVR-01/07/08 を `lab_binding.rs`、UI-DVR-03（cancel・seq 逆転）と UI-DVR-04（override・mode）を `render.rs` に追加。UI-DVR-02/05/06/09 の GPUI mechanism に filter を付与。 |
| Story matrix | LAB-SV-FADER-DOCK / LAB-AL-WAVEFORM-SEEK について、2 theme × 2 scale × 7 semantic state の trace 一致を fake layer で検証（`component_view.rs`）。 |
| 共通層 | `ViewHost::update` の Removed op 順序を sort で決定化（`crates/ruviform-ui/src/view.rs`）。story matrix の実行で hash seed 依存の不安定を検出したための修正。 |
| manifest | cases.json の GPUI binding 9 件（UI-DVR-01/02/03/04/05/06/07/08/09 のうち GPUI 関係）、stories.json の fake layer を pass に。GPUI story layer は not_run（理由・rerun 明記）。 |

### Changes（第 2 段・2026-11-09 — 共有 story fixture・GPUI story matrix・Pixel Golden 連携）

前段の残件（A2 GPUI layer・A3 Pixel Golden link）を、共有 fixture と
#314 Golden Utility の consumer 接続で解消した。

| 領域 | 内容 |
| --- | --- |
| 共有 fixture | `ruviform-ui-test/src/lab_story.rs` を新設。`StoryFixture`（manifest id・宣言 theme・canonical `ViewSpec`・walk key・teardown spec）、構造化 `StoryStep`、単一 oracle `expected_lab_story_trace()`、adapter-neutral `lab_story_walk`（teardown の spec 差し替えのみ呼び出し側 closure）。`lib.rs` から re-export。 |
| Fake adapter | `tests/scenario_contract.rs` に story walk 2 件 + story-keyed `golden_check` Unsupported link 1 件を追加。`FakeGuiTest` が canonical spec を mount し共有 oracle と一致することを検証。 |
| component_view | story spec 構築を `lab_story` fixture に委譲（`wave_view` 等の重複定義を解消）。matrix は fixture 経由に統一。 |
| GPUI adapter | `ruviform-ui-gpui/tests/lab_story.rs` を新設。`GpuiGuiTest`/`GpuiObserver` 経由で canonical spec を実 GPUI window に mount し、2 theme × 2 scale context × 7 state で共有 oracle と一致を検証。`GpuiObserver::drawn()` の draw plan が行列全体で同一であることも確認（draw plan ≠ pixel）。story-keyed `golden_check` は `Unsupported` を正直に返し `record_golden_step` で evidence 行を残す。 |
| 実窓 pixel | `tests/golden_capture.rs` の e2e を `run_capture_e2e` に共通化し、canonical story spec + 宣言 theme での story-keyed capture を 2 story 分追加（`idle` state、scale は frame 実測）。併せて、step 失敗後に queued `Tick` が `key_env` 未設定のまま次 step を実行し window procedure（unwind 不可）内で panic する潜在 bug を `fatal` guard で修正。 |
| manifest | `stories.json` の両 story の gpui binding を `pass` に更新。mechanism = `mise-task test-ui-gpui` + filter。note に scale/pixel の境界（window scale 1.0 は test platform 制約で対象外、pixel は別 lane）を明記。 |

### Decisions During Implementation

第 2 段の設計上の判断:

- **scale 軸の解釈**: story manifest の `scaleFactor: [1.0, 2.0]` は、Fake 層の確立した解釈と同じく `ResolvedTheme::with_scale` の `ScaleContext` として与える。tokens が scale で不変であることを各 cell で assert する。gpui-0.2.2 test platform は `window.scale_factor()` が 2.0 固定のため、window pixel の scale 1.0 は本層では再現不能 — semantic/draw 層の pass と pixel 層の残件を分離した。
- **theme 適用の検証範囲**: GPUI 層では `ViewMount.theme` に resolved theme を渡す。semantic trace と draw plan は theme 非依存（色は palette にあり plan/semantic に現れない）ため、theme 軸の実効的な差は pixel 層（実窓 capture の story-keyed baseline）で初めて観測される。この境界を note と IR に明記した。
- **teardown の cancel intent**: `invalid` state が開いた gesture `s2` は commit されず残る。unmount 時に detach policy が cancel し `cancel` intent を mint するのが実際の `ViewHost` 挙動であり、oracle はそれを期待値とする（製品コードは変更しない）。
- **draw plan の theme/scale 不変**: `DrawnPlan` は geometry・text・値のみを持ち色を含まないため、行列全体で同一になるのは契約どおり。pixel 差は `BaselineStore` 側の責務。

### 作業中に発見した事項

- `RvfView::new`（テスト用 helper）は window activation 購読を持たず、cancel 経路を通らない。cancel 検証は本番の `RvfView::mounted` 経路で行う必要があった。
- gpui の `deactivate_window` はプラットフォームの active window が対象の時のみ効く。先に `window.activate_window()` が必要。
- `CommandIntent::Cancel` は存在し、gesture cancel（detach/unmount を含む）は intent を発行する（`intent_from_gesture` の `InteractionEvent::Cancel` 写像。第 2 段の story oracle が teardown の `cancel` intent を実測）。第 1 段の「cancel は intent を発行しない」という記述は誤りで、第 2 段で訂正した。
- `ViewHost::update` の Removed op 順序が HashMap 反復順に依存していた（sort で修正済み）。
- `ResolvedTheme` の override は同一 profile id の文書を拒否する（`DuplicateSource`）。consumer override は別 id の product 文書で表現する。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

### 検証結果（第 1 段・実行済み・ローカル Windows 11 / Git Bash）

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

### Consumer 検査（read-only、consumer 変更なし）

| repo | タスク | 結果 |
| --- | --- | --- |
| audio-lens | `mise run check-cli-headless` | pass（headless crate 10 件に GUI framework 依存なし） |
| synvelis | `mise run check-headless-deps` | pass（同 10 件） |
| audio-lens | `mise run check-ui-gpui`（GPUI feature 型チェック） | pass（※ consumer の Cargo.lock が pin する ruviform revision で評価。本 PR の HEAD ではない） |

注意: 上記 consumer 検査は依存木ベースで、ruviform は consumer の pinned revision で評価される。
本 PR の HEAD を consumer で評価するには consumer 側の ruviform 依存更新が必要で、これは範囲外。

### レビュー（review-protocol・第 1 段）

独立した read-only サブエージェント（新規コンテキスト）で diff をレビュー。
判定: **BLOCKER なし**。MINOR 指摘 2 件は Risks 節
「既知の限界（レビュー MINOR 指摘・第 1 段）」に記録。

### 検証結果（第 2 段・ローカル Windows 11 / Git Bash）

| タスク | 結果 |
| --- | --- |
| `cargo test -p ruviform-ui-test --test component_view`（補助証拠） | pass（17 件。共有 fixture matrix を含む） |
| `cargo test -p ruviform-ui-test --test scenario_contract`（補助証拠） | pass（15 件。story oracle 2 + golden link 1 を含む） |
| `cargo test -p ruviform-ui-test --test driver_contract`（補助証拠） | pass（7 件。stories.json の gpui pass + mechanism 実在を含む） |
| `cargo test -p ruviform-ui-gpui --features gpui-test --test lab_story`（補助証拠） | pass（4 件。2 story × matrix + 2 story × golden link） |
| `cargo test -p ruviform-ui-gpui --features gpui --test golden_capture -- --ignored`（補助証拠・desktop session） | pass（3 件。probe + story-keyed 2 件。実窓 PrintWindow/BitBlt → compare → promote → compare → pass/match を実測） |

canonical な mise タスクでの再検証は最終検証節に記録する。

## Risks

### NotRun / 未達（第 1 段時点）

| 項目 | 状態 | 理由・rerun 条件 |
| --- | --- | --- |
| Pixel Golden（A3） | NotRun | #314（RF-GT-05）は OPEN。Golden Utility・GPUI pixel 取得は repo に存在しない。`TestIo::screenshot` は headless 実装のみ。#314 実装後に GPUI 描画の取得・比較を接続する。 |
| Story GPUI layer（A2 の GPUI 部分） | not_run | gpui-0.2.2 の test platform は `window.scale_factor` を 2.0 固定（`src/platform/test/window.rs`）。scale 1.0 を GPUI で再現できない。Story の GPUI 接続（Dock 含む）も未実装。 |
| UI-DVR-03 の実 OS 側 | not_run（既存） | 実 OS focus 喪失・IME は #313/#389 owner。 |
| 実 OS lane（#313/#388/#389/#262） | 対象外 | 本 Issue は参照・記録のみ。実機 Gate は代替しない。 |

### NotRun / 未達（第 2 段時点）

| 項目 | 状態 | 理由・rerun 条件 |
| --- | --- | --- |
| window pixel の scale 1.0 | NotRun（層外） | gpui-0.2.2 test platform が `window.scale_factor` を 2.0 固定。semantic/draw 層は ScaleContext で 1.0/2.0 を網羅済み。pixel 実測の scale 1.0 は real-os/Visual lane（#313/#314）で display に応じて検証。 |
| story-keyed 実窓 capture の CI 化 | NotRun | `golden_capture.rs` は ignored（desktop session 必須）。継続的 pixel gate の lane 設計は #314 側。rerun: desktop session で `--features gpui --test golden_capture -- --ignored`。 |
| story real-os driver binding | blocked（既存） | OS 実入力は #313 owner。`stories.json` の real-os binding は blocked + owner + rerun のまま。 |
| UI-DVR-03 の実 OS focus/IME | not_run（既存） | #313/#389 owner。 |

### 既知の限界（レビュー MINOR 指摘・第 1 段）

- MINOR: `has_test_fn` は構文解析ではなく「直前 6 行の属性文字列」によるヒューリスティック。短い test 直後の非 test fn やコメント内の属性文字列を誤検出しうる。現行 fixture の filter はすべて正しい test を指すことを確認済み。
- MINOR: story matrix の theme 軸は `ViewHost` が theme/scale を受け取らないため構造上自明。実質的な保証は (a) `with_scale` が tokens を不変に保つこと、(b) 7 state の期待マーカー検査。設計書にその旨を記載。

## Follow-ups

### 範囲外・残課題（報告のみ）

- `mise run check-ui-webview` / `test-ui-webview` は未実行（`ruviform-ui` の変更は view.rs の sort 1 行で、webview API への影響は無い想定だが未検証）。
- docs の managed index（specs / designs / plans / tasks / impl ir の README）へ本 Issue の行を手で追加した。doc-maintenance による再生成と doc-status による検査は未実施。
- `ruviform-ui-test` の mechanism 検査ヒューリスティックの精度向上（構文解析化）は別 Issue 候補。

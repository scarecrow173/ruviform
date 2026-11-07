---
id: "PLAN-Qx5MnBv8TcHs3RdLe6WaKp"
type: "plan"
status: "in-progress"
title: "Issue #429 — Consumer 観測 seam の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/429"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lifecycle_fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-429-consumer-observation-seam.md"
    - "docs/designs/issue-429-consumer-observation-seam.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
---

# Issue #429 — 実装計画

## 1. ファイル対応表

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs` | 新規。`GpuiObserver` / `ViewObservation` / `ComponentObservation`（`gpui-test` 限定） |
| `crates/ruviform-ui-gpui/src/lib.rs` | raw parent gate を `feature = "gpui-test"` へ、観測型の再公開、内部 test hook の定義を `observe.rs` へ移動 |
| `crates/ruviform-ui-gpui/src/render.rs` | `RvfView::host()`（`gpui-test` 限定）、`input` を `pub(crate)` |
| `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs` | `raw_parent_of` の gate を `feature = "gpui-test"` へ |
| `crates/ruviform-ui-gpui/src/lifecycle_fixture.rs` | `GpuiObserver` 経由に移行、`#[test]` 化 |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | 新規。外部 crate の consumer 相当 test（4 件） |
| `crates/ruviform-ui-gpui/Cargo.toml` | `[[test]] consumer_observation`（`required-features = ["gpui-test"]`） |
| `mise.toml` | `test-ui-consumer-observation` 新設 |
| `docs/development-environment.md` | 検証表に `test-ui-consumer-observation` を追加 |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-429-consumer-observation-seam.md` | 文書 chain |

## 2. 作業順序（依存グラフ）

```
T1 gate 修正（raw parent）──┐
T2 view 参照の保持 ─────────┼─> T3 GpuiObserver ──> T4 lifecycle 移行 ──> T5 consumer test ──> T6 検証
T2a RvfView::host/input ────┘                                              └─> T7 mise task / docs
```

1. T1: `raw_parent_of` と `capture_raw_parent` の gate を `feature = "gpui-test"` に揃える（外部 crate test の panic の根本原因）。
2. T2: `view_of` で window の root（`AppRoot`）から mount 済み view を引く。window slot には状態を足さず、`open_window` は変更しない。
3. T2a: `RvfView::host()`（読み取り専用）と `input` の可視性を `pub(crate)` にする。
4. T3: `gpui_driver/observe.rs` に `GpuiObserver` を実装する。`settle` は intent を記録してから handler へ渡す。
5. T4: `lifecycle_fixture` を `GpuiObserver` 経由に移す。`*_for_test` hook は `observe.rs` に置き、`settle_turn` / `inbox_len` / `view_of` への wrapper にする（upstream の render.rs test を変更しない）。
6. T5: 外部 crate test `consumer_observation.rs` を追加する。`Cargo.toml` に `[[test]]` を登録する。
7. T6: 検証（下記 §4）。
8. T7: `mise.toml` / `docs/development-environment.md` の更新。

## 3. リスクと対策

| リスク | 影響 | 対策 |
| --- | --- | --- |
| 外部 crate test で `cfg(test)` の分岐が効かない | consumer test が gpui test platform の panic で落ちる | T1 で gate を feature に揃えた。外部 test が通ることを T5 で確認 |
| `open_window` の行数上限（100 行） | clippy `too_many_lines` で lint 失敗 | window slot に状態を足さず、既存の root から view を引く（`open_window` は gate 修正以外変更しない） |
| `gpui-test` 有効 build で surface 親が未捕捉 | `native_surface_slot` を使う consumer が fail-closed になる | test build 限定と文書化（設計の限界節）。通常ビルドは従来どおり |
| `settle` を呼ばない間の intent は観測されない | consumer が取りこぼす可能性 | 仕様として明記（設計 D4）。`settle` 後に観測する手順をテストで固定 |
| `gpui-test` clippy（unit test 側）に既存の lint 違反 | `-D warnings` で落ちる | 既存の `lint-ui-gpui` は `gpui` feature のみで対象外。本 PR では未修正として報告 |

## 4. 検証計画

| 段階 | コマンド | 目的 |
| --- | --- | --- |
| 単体 | `mise run test-ui-consumer-observation` | seam の 4 件 |
| crate | `mise run test-ui-gpui` | gpui-test 全体（lifecycle 含む） |
| 境界 | `mise run check-ui-driver-boundary` | 公開面の gpui 非露出 |
| 形式 | `mise run fmt-check` / `mise run lint-ui-gpui` / `mise run lint` | 整形と lint |
| 回帰 | `mise run test-ui-parity` / `mise run test-ui-driver-contract` / `mise run check-ui-headless` | 既存 fixture・manifest の非回帰 |

最終検証は merge 後 HEAD で行う。

## 5. ロールバック

- seam は `gpui-test` 限定で、通常ビルドの型・関数を変えない。revert は `observe.rs` の削除、`lib.rs` / `render.rs` / `lifecycle_fixture.rs` / `surface.rs` の差分の戻し、`consumer_observation.rs` と `[[test]]` の削除で完了する。
- raw parent gate の変更（T1）は gpui-test build の挙動のみに影響し、単独で revert できる。

---
id: "DESIGN-Fn7RtQk2VbXp9LwCs4HgJu"
type: "design"
status: "proposed"
title: "Issue #429 — Consumer 観測 seam（GpuiObserver）の設計"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-429-consumer-observation-seam.md"
  depends-on:
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
  related:
    - "docs/designs/issue-390-driver-contract-lab.md"
---

# Issue #429 — Consumer 観測 seam（GpuiObserver）の設計

## 概要

#421 の設計 D5 は「観測 DTO と feature 境界を follow-up で設計する」としていた。本設計はその判断を確定する。consumer test が GPUI driver の projection と intent を観測するための公開 seam として、`ruviform-ui-gpui` の `GpuiObserver` を置く。公開面は `ruviform-ui` の契約型だけで構成し、gpui 型は crate 内に閉じる。

## D1: seam の置き場所と feature 境界

- 置き場所: `ruviform-ui-gpui` の `gpui_driver::observe`（crate 直下の `GpuiObserver` / `ViewObservation` / `ComponentObservation` として再公開）。
- feature: 既存の `gpui-test` を用いる。`GpuiObserver` は gpui の test-support（`TestAppContext`）を必要とし、それは `gpui-test` が既に有効化するものと同一である。新 feature を足すと Cargo.toml と boundary guard（allowlist の `viaFeatures`）の双方に変更が及ぶため、必要性が示されるまで足さない。
- 同一 crate の `gpui_driver` 子モジュールに置くことで、Driver 内部の `Inner` / `drain` / `flush_events` を公開せずに使える。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| `settle_for_test` の `pub(crate)` を `pub` にし、`TestAppContext` を引数に取る | consumer の公開 signature に `gpui::TestAppContext` が出る。consumer に gpui test-support の依存と型を強制する |
| `ruviform-ui-test` に adapter を置く | `ruviform-ui-test` は gpui 依存を持てない（boundary guard の `forbiddenDepRoots` / allowlist は `ruviform-ui-gpui` のみ）。依存方向も逆転する |
| 新 feature（例: `consumer-observe`）を追加する | `gpui-test` と要件が同一。Cargo.toml と allowlist の変更コストに見合う根拠がない |

## D2: 観測 DTO は契約型だけで構成する

公開する型は次の 2 つ（`#[non_exhaustive]`）と、既存の契約型である。

- `ViewObservation { window, generation, components, inbox_len }`: mount された view の観測結果。`generation` は `ViewHost` の世代、`inbox_len` は window inbox の未消費件数。
- `ComponentObservation { key, snapshot }`: `ComponentKey` と `ruviform_ui::view::ComponentSnapshot`（display / committed / phase / observed / mounted）。

intent は `ruviform_ui::Intent`（`rvf.ui.command.v1` の envelope）を返す。command の `kind` は `params.intent.kind` に入る。consumer は必要な型へ自分で decode する（seam は decode を担わない）。

## D3: projection は描画中の host を直接読む

`RvfView` は `ViewHost` を所有する。driver は window slot に mount 済みの `Entity<RvfView>` を保持し、観測時に `Entity::read` で host を読む（`mounted_keys` / `observe` / `generation`）。

- 複製した snapshot を持たないため、host 状態との乖離（stale）が構造上起きない。
- 保持は `gpui-test` 時のみ（`#[cfg(feature = "gpui-test")]`）。通常ビルドのフィールド・関数は存在しない。
- `RvfView::host()` は `pub(crate)` の読み取り専用アクセサで、`gpui-test` 時のみ存在する。

## D4: intent は settle の戻り経路で記録する

`IntentSink` は view 由来の intent を driver の event queue（`WindowLifecycle::Action`）へ積む。`GpuiObserver::settle` は handler を差し替え、`Action` を受けたら `intents` に記録してから consumer の handler へ渡す。

- 記録は settle の戻り経路だけに依存する。settle を呼ばない間の intent は観測されない（仕様）。
- consumer の handler と同一の配送経路を通るため、観測と処理の順序は変わらない。

## D5: 入力の注入は pointer 経路と同じ入口を通す

`GpuiObserver::dispatch_input` は `RvfView::input`（pointer 経路と同じ）を呼ぶ。host が拒否した入力は pointer 経路と同様に落とす。結果は `observe` と `intents` で確認する。未知の window には `UiError::Host` を返す。

## D6: raw parent 捕捉の gate を `gpui-test` に揃える（欠陥修正）

`open_window` は window ごとに OS 親ハンドルを捕捉する（`surface::raw_parent_of`）。従来は `#[cfg(all(test, feature = "gpui-test"))]` で捕捉を外していた。しかし外部 crate の test（consumer test を含む）では lib は `cfg(test)` でビルドされないため、捕捉が走り、gpui の test platform の `window_handle()` が `unimplemented!` で panic した。既存コメントの意図（「gpui-test の test build は親を未設定に保つ」）と一致するよう、gate を `feature = "gpui-test"` に変更する。通常ビルド（feature なし）の挙動は変わらない。

## D7: 内部フックを撤去し、lifecycle fixture を seam に移す

`settle_for_test` / `inbox_len_for_test` は撤去する。`lifecycle_fixture` は `GpuiObserver` を通して実行し、同じ経路を consumer と共有する（単一の実装）。`GpuiObserver` が自前の test context を持つため、fixture は `#[gpui::test]` ではなく通常の `#[test]` で動く。

## 影響範囲

- 公開 API: 追加 `GpuiObserver`（`gpui-test` 限定）、`ViewObservation`、`ComponentObservation`。削除なし（撤去したのは `pub(crate)` の test hook のみ）。
- 既存の通常ビルド（feature なし・`gpui` のみ）: 挙動変更なし。`set_view` と `view` フィールド、`host()` は `gpui-test` 時のみ。
- 永続データ形式・RT: 変更なし（非 RT の UI thread のみ）。

## 境界

- `check-ui-driver-boundary`（`gui-boundary-guard`）が pass する。公開 signature に `gpui` 型はない。
- consumer 側の依存は `ruviform-ui-gpui` の `gpui-test` feature（dev-dependency）のみ。gpui を直接参照しない。

## 限界（seam の仕様として明示する）

- gpui の test platform 上でのみ動く。実 OS window は生成されず、`settle` は 1 turn ずつ進む。実機の描画・IME・focus は manual verification。
- `observe` の `components` は mount 順ではなく key 順（`mounted_keys` の仕様）。
- 通常ビルドでは surface の親ハンドルは捕捉される。gpui-test 有効の build では捕捉されず、`native_surface_slot` を使う consumer は fail-closed になる（test build 限定）。

## 検証

| 観点 | 検証 | 実行方法 |
| --- | --- | --- |
| projection の観測（mount・mounted keys・default committed・inbox） | `observes_mounted_projection_of_parity_view` | `mise run test-ui-consumer-observation` |
| knob の gesture intent と committed | `observes_knob_gesture_intents_through_seam` | `mise run test-ui-consumer-observation` |
| toggle の atomic set と committed | `observes_toggle_atomic_set_through_seam` | `mise run test-ui-consumer-observation` |
| 未知 window の扱い | `unknown_window_has_no_view_to_observe_or_drive` | `mise run test-ui-consumer-observation` |
| lifecycle trace の seam 経由実行 | `gpui_driver_matches_shared_lifecycle_trace` | `mise run test-ui-gpui` |
| 公開面の gpui 非露出・依存の境界 | `gui-boundary-guard`（dep-graph / signature / allowlist） | `mise run check-ui-driver-boundary` |

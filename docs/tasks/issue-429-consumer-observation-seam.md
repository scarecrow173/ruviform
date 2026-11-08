---
id: "TASK-Hk8LsWq3RnVc6YbTd1PeMz"
type: "task"
status: "in-progress"
title: "Issue #429 — Consumer 観測 seam の実装タスク"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/429"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-429-consumer-observation-seam.md"
      - type: "file"
        path: "docs/designs/issue-429-consumer-observation-seam.md"
      - type: "file"
        path: "docs/plans/issue-429-consumer-observation-seam.md"
      - type: "file"
        path: "docs/test-specs/issue-429-consumer-observation-seam.md"
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
    - "docs/plans/issue-429-consumer-observation-seam.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #429 — 実装タスク

## 完了済み（本 PR）

- [x] T1: raw parent 捕捉の gate を `feature = "gpui-test"` に揃える（外部 crate test の panic を解消）
- [x] T2: mount 済み view を window の root（`AppRoot`）の content から引く（`view_of`）。window slot に新たな状態は足さない
- [x] T2a: `RvfView::host()`（読み取り専用）と `input` の `pub(crate)` 化
- [x] T3: `GpuiObserver`（`observe` / `intents` / `dispatch_input` / `settle` / `inbox_len` / `driver` / `driver_mut`）の実装
- [x] T4: `lifecycle_fixture` を `GpuiObserver` 経由に移す。`*_for_test` hook は seam と同じ helper への wrapper に一本化（upstream の render.rs test が引き続き利用）
- [x] T5: 外部 crate test `tests/consumer_observation.rs`（4 件）と `[[test]]` 登録
- [x] T7: `mise.toml` に `test-ui-consumer-observation` を新設し、検証表に追記
- [x] 文書 chain（spec / design / plan / task / test-spec）

## フォローアップ完了（2026-11-09、branch `feature/429-seam-dod`）

- [x] F1: `NumericInput` / `Transport` の consumer 需要を棚卸しし、composition で充足（新 kind 不要）と判断 — 下記「再評価の結論」
- [x] F2: bound 近傍 payload・多チャンネル・viewport culling の seam test 追加（Waveform `MAX_WAVEFORM_BINS`・Spectrogram `MAX_SPECTROGRAM_CELLS`・Meter `MAX_METER_BINS` ちょうど）
- [x] F3: gesture 中の `Notify` projection・`update_view` による spec 差し替え・f64 precision の seam test 追加
- [x] F4: `ViewObservation.generation` の doc comment を実装（mount/detach でのみ進む mutation guard）に合わせて修正
- [x] F5: `cases.json` の gpui `pass` と `mise run test-ui-gpui` / `test-ui-consumer-observation` の実測が一致することを確認（UI-DVR-01〜09）

## NumericInput/Transport 再評価の結論（2026-11-09）

consumer 需要を棚卸しした結果、**共通層への新 kind 追加は不要（composition で充足）** と判断した。`crates/ruviform-ui` は変更していない。

- **Transport**: synvelis#112 の transport は製品側 target（`transport` / `transport/position`）・`TransportProjection`（applied/observed truth）・action intent（play/pause/resume/stop）で既に構成されている。seek は既存 `Seek` control、位置・状態表示は `Label` と projection で足りる。再生状態や適用/観測の区別は製品ドメインの意味であり、driver-neutral な共通契約に載せる必然性が示されなかった。
- **NumericInput**: `ValueDescriptor`（min/max/step/fine_step/mapping/unit/read_only）と `ValueInput`（Plain/Normalized/Steps/Extreme）、`ControlInput`（Begin/Preview/Commit/Cancel/Set/Adjust/Reset）が数値入力の契約を既にカバーする。text 入力の編集セマンティクス（caret・IME・途中入力）は #389 の IME 所有権・driver 責務の領域であり、consumer が要求した形跡もない。
- **audio-lens**: `al-gui/tests/gpui_parity.rs` は Seek/Waveform/TimeAxis/Meter/Spectrogram/Label/Table のみを使用し、NumericInput/Transport kind を必要としない。
- **#387 との関係**: 「NumericInput/Transport は未実装…kind を追加する際は schema 拡張」とあるが、今回の棚卸しで schema 拡張を正当化する consumer 需要は見つからなかった。将来 consumer が共通の semantic gap を示した場合に限り spec を起票して再検討する。

## 残 DoD の棚卸し（2026-11-09 更新）

| 項目 | 状態 | 根拠 |
| --- | --- | --- |
| `NumericInput` / `Transport` Control | 再評価済み — 新 kind 不要 | 上記「再評価の結論」。#387 Non-goals の前提条件（consumer 必要性の再評価）を満たした |
| Waveform/Meter/Spectrum の長尺・RT snapshot・gap/NaN precision・bounded memory/cancel/stale・GPU release の GPUI 検証 | 一部達成 | F2/F3 で bound 近傍 payload・多チャンネル・viewport culling・gesture 中 projection・f64 precision・spec 更新（`update_view`）を seam 経由で検証。gap/NaN・bound 拒否・cancel/stale/out-of-order・teardown は既存 lane（`render.rs` unit test・parity fixture・`test-ui-gpui`）でカバー済み。実 GPU release は platform 制約で **NotRun**（#313 帰属） |
| driver-lab manifest の UI-DVR-03/07/08 GPUI | 達成（実測一致） | `ui_dvr_03_gpui_*`・`ui_dvr_07_gpui_*`・`ui_dvr_08_gpui_*`・`lab_binding::ui_dvr_*` が `mise run test-ui-gpui` で pass。`cases.json` の gpui `pass` と一致（PR #432/#433/#444 で統合済み） |
| `LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK` の 2 scale factor × 2 theme × 状態遷移の GPUI 実行 | 未達（not_run のまま・本タスク対象外） | story GPUI lane は owner = #426。`stories.json` は変更しない |
| 実 OS window・real GPU release（#313）・headless-ts（#294）・pixel golden（#314）・共通 Scenario 契約（#464） | 未達（NotRun・他 Issue 帰属） | platform/owner 制約により本タスク範囲外 |

## 既知の問題（本 PR の範囲外）

- `cargo clippy -p ruviform-ui-gpui --all-targets --features gpui-test -- -D warnings` は、既存の unit test 側に 2 件の違反（`render.rs` の doc comment の backtick 欠落、`rvf_view_native_slot_public_api_cannot_starve_pushes` の行数超過）で失敗する。`lint-ui-gpui` は `gpui` feature のみを対象とするため、mise の検証ゲートでは検出されない。本 PR では触れていない。

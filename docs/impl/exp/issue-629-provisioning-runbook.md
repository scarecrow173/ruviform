# Issue #629 provisioning runbook — 実装実験記録（2026-11-17）

`docs/tasks/issue-629-provisioning-runbook.md` の closeout-prep
slice（AC6 MIDI virtual lane 分離 + provisioning runbook +
capability matrix 確定 + AC7 consumer 照合の headless 範囲）
実装で試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-629-provisioning-runbook.md`](../ir/issue-629-provisioning-runbook.md)（`IMPL-034YIlnrAlTVIKDGSVNxwW`）、
イベントログは
[`issue-629-provisioning-runbook-exp.jsonl`](issue-629-provisioning-runbook-exp.jsonl)
を参照。

## 試行 1: MIDI 仮想 lane の収集範囲決定

- **目的**: `test-midi-virtual-endpoint` の `run` が収集する
  テスト範囲を決める。
- **観測**: `loopback` test target には
  `loopback_round_trip`（仮想 pair verifier・AC2 証拠経路）と
  `real_device_silent_output_smoke`（GS Wavetable 依存の実
  device smoke）が同居する。name filter で絞る案と target
  全体収集の案があったが、Coordinator の第一案は target 全体
  収集 + description で両テストの依存を明記する方針。
  `loopback.rs` には `cfg(feature)` ゲートがないため
  `--features` 指定は不要（audio 側 `--features real-device`
  と異なる点）。
- **結論**: `run = "cargo test -p ruviform-midi-device --test
  loopback -- --ignored --test-threads=1"` で target 全体を
  収集。silent output smoke は「仮想 loopback ではなく実
  WinMM output port への silent 送信 smoke」であり、
  `virtual-os-endpoint` の証拠にはならないことを description・
  design・本記録で明記した。test target 分割・関数移動のような
  構造変更は行わない。

## 試行 2: `test-midi-real-device` の意味論維持

- **目的**: 既存包括 lane と新設仮想 lane の棲み分けを決める。
- **観測**: `test-midi-real-device` は `-p ruviform-midi-
  device -p ruviform-audio-plugin-bridge -- --ignored
  --test-threads=1` で両 crate の全 `#[ignore]` を収集する
  包括 lane であり、loopback target も含む。run を変えると
  既存運用の意味が変わるため、Coordinator 第一案どおり run
  不変 + description/注記で「物理 port 系 + 全 `#[ignore]`
  収集の包括 lane、仮想 pair 専用は `test-midi-virtual-
  endpoint`」と明記する方針を採用。
- **結論**: `run` は無変更。description を更新し、
  `docs/development-environment.md` の lane 表に
  `test-midi-real-device`（従来未収録の doc gap）と新 lane を
  併記した。

## 試行 3: self-run evidence の verdict 採取

- **目的**: canonical lane の self-run を fail-closed に
  記録する。
- **観測**: `mise run test-midi-virtual-endpoint` は exit 0・
  2 passed だが、canonical `run` に `--nocapture` は含めない
  ため verdict 行は libtest capture 内。`--nocapture` 付き
  直接実行で `loopback_round_trip` が
  `verdict: NotRun — NOT_PROVISIONED`（0 input ports・
  1 output port）、`real_device_silent_output_smoke` が
  `verdict: Pass`（GS Wavetable Synth — 実 WinMM port であり
  仮想証明ではない）を確認。
- **結論**: evidence は canonical lane ログ + `--nocapture`
  補助証拠の 2 部構成（5th slice と同じ libtest capture の
  注意）。`evidence/rf629/midi_virtual_endpoint_slice6_
  selfrun-windows-local.txt` に記録し、Pass は主張しない。

## 試行 4: survey doc Evidence 節の append-only 維持

- **目的**: 2026-11-17 の新規 dated entry を追記する。
- **観測**: 「never edit/reorder/delete existing」規約に従い
  既存 entry は不変としたが、追記位置の anchor にした
  5th slice 最終 bullet の直後に未読の末尾行（self-run 記録）
  があり、insert でその bullet が新 section の後に孤立する
  並びになった。一時ファイルで末尾を確認して修正し、
  5th slice section 内の正しい位置へ戻した。
- **結論**: append-only 規約と既存 entry の完全性を維持。
  大きな文書への追記は anchor 周辺の末尾まで読んでから行う
  べきことを確認した。

## 試行 5: docs chain の slug・ID 採番

- **目的**: `issue-629-provisioning-runbook` slug で
  spec→design→plan→task→test-spec→impl ir/exp(+jsonl) の
  chain を作成する。
- **観測**: 既存 docs との ID 衝突を確認したうえで
  SPEC-9bPUGYlLtIw7dXbS3LAyex・DESIGN-E2nW52u7DBe6EqaWvY1PRi・
  PLAN-lbYjQBxlWJgpBMrj1gEUoI・TASK-W4O6BsG8WBPnXtsyy5hX8Y・
  TSPEC-byCAXVcNUJp9U972Dd16XL を採番。IMPL ID は
  `new_impl_record.js` が `IMPL-034YIlnrAlTVIKDGSVNxwW` を
  自動採番した。exp jsonl は `new_experiment_log.js` で
  `issue-629-provisioning-runbook-exp` タイトルから生成し、
  イベントは `append_experiment_event.js` で逐次追記した
  （手編集なし）。
- **結論**: chain は既存 slice（5th slice 等）と同じ front
  matter・relations 構造で作成。docs index（`docs/**/
  README.md`）は生成物のため本 slice では再生成せず、
  Coordinator 統合時に再生成される前提を PR 本文に明記する。

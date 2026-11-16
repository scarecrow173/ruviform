# Issue #629 virtual endpoint lane registration — 実装実験記録（2026-11-16）

`docs/tasks/issue-629-virtual-endpoint-lane.md` の 5th slice
（`test-audio-virtual-endpoint` opt-in lane 登録 + stale 参照
同期 + #628 Test Runtime 結線の headless 検証）実装で試行・
観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-629-virtual-endpoint-lane.md`](../ir/issue-629-virtual-endpoint-lane.md)、
イベントログは
[`issue-629-virtual-endpoint-lane-exp.jsonl`](issue-629-virtual-endpoint-lane-exp.jsonl)
を参照。

## 試行 1: lane 登録の構造決定

- **目的**: `test-audio-virtual-endpoint` を既存 opt-in lane の
  構造に合わせて登録する。
- **観測**: `test-audio-real-input`（`--test real_input_e2e`
  固定）と `test-midi-real-device`（`-p ... -- --ignored
  --test-threads=1` 集約）が先例。4th slice design の AC6 lane
  設計記録は `cargo test -p ruviform-audio-device --features
  real-device --test virtual_endpoint -- --ignored
  --test-threads=1` を指定済み — `test-audio-real-input` と
  同じ `--test` 固定形で `lint-audio-real-input` 直後に配置。
- **結論**: 専用 lint lane は追加しない（Coordinator 確定 —
  `lint-audio-real-input --all-targets --features real-device`
  が新 test target を coverage）。`mise.toml` は task
  ブロック追加のみで既存 task 不変。

## 試行 2: `EnvironmentKind` 語彙照合と Coordinator 判断

- **目的**: Case Schema v1 closed set（5 値）と typed enum
  （3 variant）の差分を確定し、補完方針を決める。
- **観測**: `virtual-os-endpoint`・`physical-device` が欠落。
  さらに既存 `RealAbiTestPlugin` は `real-abi-test-plugin` を
  出力するが schema 語彙は `test-plugin` — 綴り差は serde
  語彙の破壊的変更（`ruviform-plugin-dock-test` が `as_str()`
  を evidence JSONL に出力中、AL/SV が dev-dep 消費）。
- **結論**: Coordinator 回答 (A) — 2 variant のみ additive
  追加し綴り差は既知 gap として docs 記録。`as_str`・doc
  comment・serde round-trip + schema 語彙 literal pin テストを
  整合。`RealAbiTestPlugin` の spelling は不変。

## 試行 3: self-run evidence の verdict 採取

- **目的**: canonical lane `mise run test-audio-virtual-
  endpoint` の self-run を fail-closed に記録する。
- **観測**: canonical `run`（`--nocapture` なし）では
  `test render_capture_round_trip ... ok`（1 passed）・exit 0
  だが、verdict 行は libtest の capture 内で出力に現れない
  （4th slice discovery の補足どおり）。`--nocapture` 付き
  直接実行で `verdict: NotRun — no input devices on this
  host` を確認 — 4th slice 時点の `NOT_PROVISIONED` とは
  理由文字列が異なる（本ホストは input device 0 件）が、
  fail-closed NotRun の判定は同じ。
- **結論**: evidence は canonical lane ログ + `--nocapture`
  補助証拠の 2 部構成とし、canonical `run` には
  `--nocapture` を含めない（設計記録どおりの起動式を維持）。

## 試行 4: stale 参照の同期範囲

- **目的**: 「mise lane 未登録」記述を canonical lane 参照に
  更新する。
- **観測**: `virtual_endpoint.rs` module doc・`endpoint_probe.rs`
  verdict・wasapi-endpoint design の 3 箇所・survey doc Evidence
  節の 1 箇所。survey doc Evidence 節は「never edit/reorder/
  delete existing」の append-only 規約があるため、既存 entry
  は編集せず新規 dated entry で lane 登録・self-run を記録し
  旧記述の解消を明記した。
- **結論**: probe は文言変更のため Windows 実機で再実行し
  `endpoint_probe-windows-local.txt` を更新（exit 0・
  `provisioning: NOT_PROVISIONED` 維持・best-effort 契約不変）。

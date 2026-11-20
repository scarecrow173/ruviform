---
id: "SPEC-630clapOutputDrain01"
type: "spec"
status: "approved"
title: "Issue #842 CLAP output-drain slice — plugin→host 出力イベント drain（Issue #630 残りスコープ）"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/842"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/specs/issue-630-fault-modes.md"
  changes:
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
    added:
      - type: "file"
        path: "docs/specs/issue-630-clap-output-drain.md"
      - type: "file"
        path: "docs/designs/issue-630-clap-output-drain.md"
      - type: "file"
        path: "docs/test-specs/issue-630-clap-output-drain.md"
      - type: "file"
        path: "docs/discovery/issue-630-clap-output-drain.md"
      - type: "file"
        path: "evidence/rf630/clap-output-drain-verification.md"
  implemented-by:
    - "docs/designs/issue-630-clap-output-drain.md"
    - "docs/test-specs/issue-630-clap-output-drain.md"
  depends-on:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/specs/issue-630-fault-modes.md"
  related:
    - "docs/designs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-630-fault-modes.md"
    - "docs/discovery/issue-630-clap-output-drain.md"
    - "evidence/rf630/clap-output-drain-verification.md"
---

# Issue #842 CLAP output-drain slice — spec

## 目的

CLAP ホスト bridge（`ruviform-plugin-clap` feature `host`）の
plugin→host 出力イベント面を実装する — Issue #630 の残りスコープ
（本 slice は #842 が追跡し、#630 は他受入基準が残るため close
しない）。現状 `ClapPlugin::process` はプラグインの
`events.output` に `OutputEvents::void()` を渡し、プラグインが
発行する出力イベントを全て捨てている。VST3 側は既に
`drain_output_reports`（Issue #253 レポート契約）を実装済みで、
本 slice は CLAP 側の parity を取る。

## スコープ

- `process()` の出力イベント sink を `OutputEvents::void()` から
  永続・構造的有界なバッファへ置き換える。
- プラグイン発行 CLAP 出力イベントを中立 `Event` へ翻訳し、
  `EventKind::ParamValue` / `EventKind::Midi` として有界
  `pending_reports` に蓄積する。
- `PluginProcessor::drain_output_reports` を override し、非 RT の
  drain 動詞で消費者 `Vec` へ引き渡す。
- 有界バッファ溢れ・不正/未サポート/非有限イベントは defined
  skip/drop として統計計数する — 黙って捨てない。
- 変形 fixture 出力（`bad_time` の `u32::MAX` スタンプ、`invalid`
  の NaN）がホスト観測可能であることを L0/L1 テストで証明する。
- wire / plugin-core / export / composite の既存機構は zero-diff
  （`DrainOutputReports` 動詞・`WireReply::Reports` チャンク・
  `ErasedPlugin` 経路は既存のまま — override だけで全経路が
  生きる）。

## 翻訳・観測法則

- 表現可能な値は忠実に運ぶ: `u32::MAX` の block 外スタンプは
  `frame_offset`（`u32`）が表現可能なためそのままレポートへ届く
  — clamp も drop もしない（fault fixture の観測対象そのもの
  であり、Issue #253 契約はホストが offset を再解釈しない）。
- 表現不能な値は defined skip: NaN/無限値のパラメータ値・速度は
  canonical `ParamValue`/`MidiValue` が表現不能なため skip とし、
  `0.0` 等の捏造値は絶対に返さない。
- 未知パラメータ・未サポート kind/イベント空間・ワイルドカード
  対象の構造 note は defined skip。
- 計数は正直: `translated`（翻訳成功）、`skipped`（skip/drop）、
  `refused`（構造バウンドでの `try_push` 拒否）を累積する。

## 受入基準（本 slice）

- `ClapPlugin::process` が `OutputEvents::void()` を使わず、
  プラグイン出力が bounded バッファ経由で `pending_reports` に
  届く。
- `drain_output_reports` override が pending を caller `Vec` へ
  移し空にする — 2 回目の drain は空を返す。
- L0（in-process 実 ABI host_path）: `bad_time` armed block の
  `u32::MAX` スタンプ `ParamValue`（id 0、normalized 0.0）が
  drain 経由で観測できる; `invalid` armed block の NaN イベントが
  defined skip であること（drain が live であることを `bad_time`
  で証明した上で空を返す）。
- L1（実 runner、opt-in lane）: `bad_time` 2 ブロック分のレポートが
  `DrainOutputReports` 経由で届き、各 `frame_offset == u32::MAX`・
  `EventKind::ParamValue`・id 0・値 0.0; `invalid` armed block
  群の後の drain が空; lifecycle は `Processing` のまま。
- RT 契約: 出力バッファは `prepare` で事前確保し `process` パスは
  再確保しない（`rt_` レーンが `assert_no_alloc` で検証）。
- lint/fmt/単体テスト/L1 opt-in レーン/prek が実行・合格する
  （実行した結果のみを evidence に記録）。

## 非スコープ

- Issue #630 の close（他受入基準が残る; 本 PR は `Closes #842` +
  `Refs #630`）。
- AC4 real dock/DComp、popup HWND harness、clap-validator 上流
  依存の作業。
- `crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`（別
  slice が占有）。
- VST3 側の実装変更（既存 parity 参照のみ）。

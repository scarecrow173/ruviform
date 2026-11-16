---
id: "DSC-630abiValidator01"
type: "discovery"
status: "final"
title: "Issue #630 外部 ABI validator slice — provisioning 調査と validator 指摘の診断"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "evidence/rf630/verification.md"
  related:
    - "../specs/issue-630-abi-validator.md"
---

# Issue #630 外部 ABI validator slice — provisioning 調査と診断

Issue #630 AC3（Test Plugin の実 ABI 準拠を Host から独立した権威で検証）
と AC5 残部（artifact digest / SDK・validator version・license 記録）の
ための外部 validator 導入経路と、実走で露呈した fixture 側契約違反の
診断を記録する。

## validator provisioning 調査

### CLAP: clap-validator（free-audio/clap-validator）

- FOSS Rust CLI。GitHub Releases に Windows x86_64 バイナリ zip と
  sha256 digest が公開されている。
- 採用: version `0.4.1` の pinned release binary を
  `%LOCALAPPDATA%/ruviform/plugin-abi-validator/` に配置し、
  `clap-validator.exe --version` で `0.4.1` を確認。
- ライセンス: MIT OR Apache-2.0（リポジトリ LICENSE）。
- workspace 依存にはしない（外部 tool として runner が path/env で解決）。

### VST3: Steinberg VST3 SDK validator

- SDK 同梱の `validator` サンプル（`public.sdk/samples/vst-hosting/validator`）。
- 配布 binary は Steinberg から直接取得できないため、SDK ソースを clone し
  CMake + MSVC（VS2022 BuildTools 同梱 cmake）で `validator` target を
  Release build。`validator.exe -version` は `VST 3.8.1 Plug-in Validator`。
- clone commit: `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`（master HEAD）。
- ライセンス: `vst3sdk/LICENSE.txt`（Steinberg VST3 License — ソース利用・
  再配布条件を遵守。SDK 自体は validator 実行のみに使用し成果物へ同梱しない）。
- `.vst3` bundle layout が必須のため、test-only staging
  `Contents/x86_64-win/<bundle名>.vst3`（inner binary 名 = bundle 名）を
  runner が一時 dir に生成して渡す。

## validator 実走で露呈した fixture 側契約違反（全て fixture crate 内で修正）

### CLAP 側

| 症状 | 根本原因 | 修正 |
| --- | --- | --- |
| param module path が invalid | param info の module が `b"/"` | `b""` に修正 |
| fractional sample rate を拒否 | activation が整数 sample rate を要求 | validator 提供値を受理する範囲に緩和 |
| event 数 overrun / hang 懸念 | event scan bound が小さすぎ | `MAX_EVENTS` を 1024 に引き上げ（bounded 維持） |
| instrument が note 処理で fail | note-on/off イベント処理の欠落 | instrument fixture の event 処理を修正 |
| stateful が state-reproducibility 3 件 fail | `state.load` が値を変えても `clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)` を要求しなかった | `PluginStateImpl` が host handle を保持し load 成功時に rescan を要求 |
| `param-fuzz-sample-accurate` で validator 自体が panic（0xc0000409） | validator 側の sandbox thread panic（tool 側 bug） | canonical run では param-mutating case を fault fixture のみ `--exclude` で除外。unfiltered run は informational leg として記録 |

### VST3 側

| 症状 | 根本原因 | 修正 |
| --- | --- | --- |
| 大量の "Failed to setup test!" / teardown 失敗 | `terminate` が終端状態に遷移し再利用不能、`setupProcessing` の受理 state が狭い、bus info 不一致 | component lifecycle を Created/Active 双方で再 setup 可能な契約に修正、bus info を spec 準拠に修正 |

最終結果: VST3 SDK validator `235 passed, 0 failed`（exit 0）、
clap-validator は 5 plugin 全て canonical leg Passed（0 failed）。

## スコープ境界の確認

- production adapter（`ruviform-plugin-vst3`/`ruviform-plugin-clap` 等）は
  zero-diff を維持。VST3 component の lifecycle 契約は fixture 側でのみ
  修正した。production adapter に同型の疑義がある場合は Coordinator 判断へ。
- `test_fault` の fatal fault mode は param-mutating validator case と
  本質的に相性が悪い（設計上クラッシュする）。除外は「検証を弱める」のではなく
  「fixture の意図的 fault surface を一般 param suite の対象外とする」宣言的
  scoping であり、理由を run.json と本書に記録する。

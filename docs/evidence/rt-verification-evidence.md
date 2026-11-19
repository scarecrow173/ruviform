---
id: "EVI-b2d2SdMkxKWrehAIVjKZXy"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "RT 検証の実行記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/22", "https://github.com/scarecrow173/ruviform/issues/126", "https://github.com/scarecrow173/ruviform/issues/130"]
  informs: []
  explains: []
  supersedes: []
---

# RT 検証の実行記録

本書は、RT（リアルタイム）経路の検証として実施された実行とその結果を記録する。記録の対象は、後に削除された過去の実装記録から抽出したものであり、本書はそれらの記録に依存せず単独で読めるように構成している。各記述には出典の Issue 番号を付記し、観測された事実と解釈を区別して記す。

## 検証結果の概要

RT 経路のアロケーション検証は、対象の `rt_*` テストを `assert_no_alloc` の測定窓内で実行し、窓内で発生する Rust のヒープアロケーションを検出する手法で行われた。この手法は plugin-clap、plugin-vst3、video-proc、media-core、dsp、audio-analysis の各クレートに適用された (Issue #22, #126, #130)。

検証の結果は領域ごとに次のとおりである。

- plugin-clap / plugin-vst3: `rt_*` テストを測定窓内へ移し、0 アロケーション・全件パスを確認した (Issue #22)。
- video-proc: 全 28 件の `rt_process_*` テストでアロケーション検出はゼロだった (Issue #126)。
- media-core / dsp / audio-analysis: 全対象 `rt_` テストが測定窓内で全件パスし、実装修正は不要だった (Issue #130)。

以下の各節で、検証の手法と対象、観測された結果、そこから導かれた判断を記す。あわせて、検証中に観測された環境起因のフレークも別節で記録する。

## 測定窓の適用範囲と検出可能性の確認

測定窓の適用には一つの設計上の除外がある。thread-check ABI テスト（T1）は測定窓で囲まなかった。`PluginInstance::process` の host 側 marshalling を含むため窓の対象外とし、契約検証に留める判断が記録されている (Issue #22)。

測定窓が実際にアロケーションを検出できることは、意図的なアロケーションを挿入する sanity abort 検証で確認された (Issue #130)。各 crate の測定窓内に一時的に `vec![0_u8; 8]` を挿入して `cargo test -p <crate> rt_` を実行したところ、3 crate とも `memory allocation of 8 bytes failed` で test binary が abort した（exit code `0xc0000409`）。挿入を全て除去して再実行すると全件パスに戻った。この手順は lane が「armed かつ検出可能」であることを機械的に確認する再利用可能な検証手順として記録されており、Issue #126 と同一の手法である。

この検証は「測定窓が機能していること」の確認であり、実装の正しさを直接検証するものではない点に注意が必要である。

## plugin-clap / plugin-vst3 の rt_* テスト

plugin-clap と plugin-vst3 の `rt_*` テストを測定窓内へ移し、現行コードで 0 アロケーション・全件パスを確認した (Issue #22)。対象は次のとおりである。

- plugin-clap: `rt_process_drives_paired_stereo_block_allocation_free`、gain.rs の `rt_process_full_stereo_block`
- plugin-vst3: `rt_prepare_process_finish_round_trip`、component.rs の `rt_` 3 件

検証の結果、deferred コメントは「実測済み」の記述へ更新された。alloc_counter（ruviform-rt クレート）は変更なしで併存している (Issue #22)。

## video-proc の rt_process_* テスト

video-proc では全 28 件の `rt_process_*` テストに測定窓を適用し、アロケーション検出はゼロだった (Issue #126)。内訳は default lane の 26 件と icc feature lane の `rt_process_icc_*` 2 件である。

moxcms の `TransformF32Executor` を含む全経路で、per-process の Rust アロケーションは観測されなかった（any_to_any feature は off のまま）。カーネル実装の修正は不要であり、「alloc-free by construction」という静的論証が実測で裏付けられたと記録されている (Issue #126)。

この結果を受け、#97/#99 の design で deferred とされていた表の 2 行は解決済みの注記へ更新された (Issue #126)。

## media-core / dsp / audio-analysis の rt_* テスト

sanity abort 検証で lane の武装を確認したのち、全対象 `rt_` テストが測定窓内で process/query を実行して全件パスした。実装修正は不要だった (Issue #130)。対象経路は次のとおりである。

- media-core: `TempoMap::tick_to_media` / `media_to_tick` / `musical_position` / `time_info`（binary search + integer math）
- dsp: `BiquadFilter` / `FirFilter` / `Fft` / `LinearResampler` / `Window::fill` の process
- audio-analysis: `LevelMeter::process`、および内部で `Fft::process` を呼ぶ経路を含む `SpectrumAnalyzer::process`

`test-rt` のマッチ件数は media-core 5 件・dsp 1 件・audio-analysis 3 件と記録されている (Issue #130)。

## 検証結果に基づく判断

各領域の検証で観測された結果に基づき、次の判断が記録されている。

- plugin-clap / plugin-vst3 では、deferred コメントを「実測済み」の記述へ更新した。alloc_counter（ruviform-rt クレート）は変更なしで併存する (Issue #22)。
- video-proc では、#97/#99 の design で deferred とされていた表の 2 行を解決済みの注記へ更新した。カーネル実装の修正は不要と判断された (Issue #126)。
- media-core / dsp / audio-analysis では、実装修正は不要だった (Issue #130)。
- 環境起因のフレーク 2 件は、コード由来の問題ではないとして Risks / Validation に記録された (Issue #21, #126)。

## 環境起因のフレーク記録

thread-priority 実装の検証中、`cargo test --workspace`（並列既定）が一度失敗した。原因は環境のコミット上限枯渇（os error 1455）であり、sibling エージェント群のメモリコミットが要因と記録されている。`-j1` および後続の既定並列再実行では全件パスし、コード由来の問題ではない環境要因の flake として Risks に記録された (Issue #21)。

video-proc の変更とは無関係に、`l1_proxy_e2e` の load-sensitive な既知 flake も記録されている (Issue #126)。`mise run test` で `hang_converges_on_command_timeout` と `wedged_child_write_converges_and_teardown_is_bounded` の 2 テストが workspace 並列負荷下で一度 fail したが、同一条件での再実行と単独実行はいずれも全件パスした。頻発する場合は別途調査 Issue の検討対象とされた。

## 記録の範囲と限界

本書に記録された検証のうち、実行されなかった、または先送りされたものを以下に列挙する。

- thread-check ABI テスト（T1）は、host 側 marshalling を含むため測定窓で囲まず、契約検証に留める判断が記録されている (Issue #22)。アロケーション検出の対象外である。
- `l1_proxy_e2e` の load-sensitive flake 2 テストは原因の特定が行われておらず、既知 flake 族として記録されたにとどまる (Issue #126)。
- 上流文書（docs/specs・docs/adr・docs/designs・docs/test-specs 配下）は参照範囲外として未読であり、本書の findings は task/plan/IR 側の記述に基づく。
- Issue #126/#130 の close/merge 状態は文書上「scope 外」と明記されており、文書からは判定不能である。

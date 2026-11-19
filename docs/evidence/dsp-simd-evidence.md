---
id: "EVI-CT8eScFgdJ7FaQoVRYWYYn"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "DSP・SIMD・カーネルの計測・検証記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/54", "https://github.com/scarecrow173/ruviform/issues/56", "https://github.com/scarecrow173/ruviform/issues/75", "https://github.com/scarecrow173/ruviform/issues/99", "https://github.com/scarecrow173/ruviform/issues/156", "https://github.com/scarecrow173/ruviform/issues/157", "https://github.com/scarecrow173/ruviform/issues/167", "https://github.com/scarecrow173/ruviform/issues/169", "https://github.com/scarecrow173/ruviform/issues/183", "https://github.com/scarecrow173/ruviform/issues/646"]
  informs: []
  explains: []
  supersedes: []
---

# DSP・SIMD・カーネルの計測・検証記録

本書は、DSP・SIMD・カーネル周辺の実装で行われた計測と検証の結果を記録する。記録の対象は、後に削除された過去の実装記録から抽出したものであり、本書はそれらの記録に依存せず単独で読めるように構成している。各記述には出典の Issue 番号を付記し、観測された事実と解釈を区別して記す。解釈を含む箇所にはその旨を明記する。

## DSP/SIMD PoC ベンチの分割実行

Windows x64 での全量ベンチは、単一プロセス実行では完走せず、カーネル単位の分割実行で完走した (Issue #19)。単一プロセスでの全量実行は 2 回とも約 82–85% 地点でコミット上限に達し、alloc 失敗による abort となった。これは並行エージェントが多数稼働する環境要因と記録されている。そこでカーネル単位の 8 分割実行に変更し、2 run × 756 ケースを完走させた。各 run は約 25–27 分で、run 間の分散は p50=1.8%、p90=25.7%、max=516.9% だった。分散の大きさは環境負荷に由来すると記録されている。

## DSP カーネルの本番経路検証

本番経路の検証は全てグリーンだった (Issue #53)。forced token × 全 8 カーネル × 複数サイズの単体テストで、ルーティングを迂回して `*_impl` を直接検証した。SIMD 等価性の確認は simd_equiv matrix に `prod_detected`、`prod_avx2`、`prod_sse4_2`、`prod_fallback`、`prod_neon`（probe-gated）の列を追加して行った。リアルタイム経路では `rt_dsp_kernels_no_alloc` が `assert_no_alloc` 窓内で 0 alloc を確認した。閾値境界 ±1 の動作確認テストもグリーンで、fmt-check/check/lint/test/test-rt/bench-smoke と prek、doc 監査が全てパスし、graph exit-audit が complete となった。

## DenormalFlush と ruviform-dsp の検証

DenormalFlush と ruviform-dsp の検証列は全て完了した (Issue #54, #75)。DenormalFlush では、denormal が ±0 にフラッシュされること、normal 値が不変であること、NaN/Inf が透過することを upstream pull の test-spec 4 ケースで確認した。`rt_denormal_flush_no_alloc` もグリーンで、`mise run fmt-check`、`check`、`lint`、`test`、`test-rt` が全てパスし、doc 監査は findings 0、graph は complete だった。ruviform-dsp では 5 モジュールの実装と workspace メンバー登録を行い、test-spec の全ケースと `mise run test`、`test-rt` がパスした。process 0 alloc を実測し、検証列は全てパス、doc 監査チェーン起因の findings は 0 だった。

## SIMD dispatch オーバーヘッドの計測

カーネル dispatch のオーバーヘッドは「混在」と分類された (Issue #56)。低負荷（CPU 1%）での再計測では、(a) `dispatch_noop` が 1.29ns で安定、(b) 小 N で add カーネルに Δ=+6–7ns（run 間 ±0.1–1.5ns）の安定した増加、(c) 大 N で符号が反転する ±20–70ns の散乱、という 3 分類の結果が得られた。これは観測値であり、解釈は Issue #56 自身の分類に従う。前回の @1024 での劣後（add 0.62x、scale 0.66x）は低負荷条件で 0.91–0.97x に回復し、ノイズと確定した。一方、小サイズの回帰（0.48–0.90x）は実在すると記録された。これを受けた示唆として、prepare 固定の正当性、小 N 軽量カーネルのスカラールーティング、dispatched 大サイズの劣化なし、が挙げられている。

## Issue #99 の Full Gate とタスク別検証

Issue #99 では Full Gate の 10 コマンドが全て exit 0 だった (Issue #99)。対象は fmt-check、check、lint、test、test-rt、test-video-proc-icc、lint-video-proc-icc、bench-smoke、prek、doc audit である。test は 49 test binaries で 932 passed・0 failed、test-rt は実一致件数 107（>0）、test-video-proc-icc は feature-on で全 pass（lib 187 + doctest 24）だった。prek は 7 hooks passed・1 skipped（Windows 非対象）、doc audit は blocking 0 findings だった。全ログは `docs/impl/exp/data/issue-99-final-gate-*-20261025.log` に保存された。

タスク別の commit と review も記録されている (Issue #99)。7 タスク全てが完了し、Task1 c88860e（RgbaF32+transfer internals）、Task2 6d27460（working-space kernels）、Task3 d73b8b9（ToneMapSdr）、Task4 c5aa5a6（IccTransform+icc lane）、Task5 878d602（RgbaF32 composite/affine arms）、Task6 6693801（final verification）、Task7 closeout の commit が対応する。独立 review では Task2/3/4/5 で APPROVE または指摘全修正済みで、reviewer B の P1×2 が修正され P2×2 が follow-up として記録された。Task5 では `oa < f32::MIN_POSITIVE` の domain-relative cut を両 reviewer が正当と裁定し、divergence-pin test で固定した。

## viz ベンチの実測値

viz の criterion ベンチでは、波形 LOD 構築が約 21.7 M frames/s（4M フレームを 192 ms、全階層）、ビューポート応答が zoom-out 14 µs・8 s 窓 20 µs・極端 zoom 10 µs、STFT が 1M フレームを 210 ms（fft=1024、hop 256、約 100× 実時間）、512×96 log タイルが 18 ms（複素スペクトル再解析なし）だった。encode は 512×96 で f64 が 284 µs、f32 が 96 µs、u8 が 392 µs（payload 比 1/0.5/0.125）だった。ゲートは全 pass で、ruviform-viz 単体 35 件と結合 viz_acceptance 13 件が確認された (Issue #157)。

## 数値ライブラリの bit-exact 検証

数値ライブラリの検証は構造的一致に依存し、AL との直接比較は未実施だった (Issue #167)。テスト 7 件（doctest と linalg::tests 6 件: 既知解、残差 <1e-12、pivot 回避、複素除算、cond proxy=1e30、特異/不正 shape の None、非有限伝播契約）が全て pass し、workspace 86 スイートで 0 失敗だった。ただし AL 側との直接比較実行は別リポジトリのため行われず、bit-exact 性は同一式・同一順序の逐語移植と fft::cmul1 系の bit-exact 契約に拠っている。AL 移行時に実値での差分確認を行うことが推奨として記録された。これは解釈ではなく、検証方法の限界として記録された事実である。

## spectral 統計の AL 式 oracle 一致

spectral 統計は AL 式の独立 oracle と `to_bits` 一致を確認した (Issue #169)。`spectral_stats_match_al_oracle_bit_exact` が、退化・単要素・空・擬似スペクトルの 6 系列で全統計値の `to_bits` 一致を示した。`spec::tests` 10 件と `rt_spectral_paths_do_not_allocate` に RT-suitable 8 関数を `assert_no_alloc` 配下で追加した（median は除外）。workspace test では ruviform-plugin-host の既知 flaky（IPC タイミング依存 timeout）が 1 回発生したが、単独再実行で pass し本変更と無関係と記録された。fmt/check/lint/build/test-rt は全て pass した。

## band spectrum の bit-exact と並列共有

band spectrum は bit-exact 等価と並列共有を検証した (Issue #183)。`band_spectrum_matches_band_filter_bit_exact`（len×帯域行列で band/band_into と bit-exact）、`band_spectrum_shared_across_threads`（scoped thread での `&` 共有と Send+Sync 静的保証）、`band_scratch_contract`（変換長不一致拒否、同一 n 使い回し、memory_bytes 一致）の 3 契約テストを追加した。`rt_spectral_paths_do_not_allocate` に新経路を追加し、`cargo test -p ruviform-numeric` は 108 unit + 7 doctest が全 pass、workspace は 113 バイナリで 0 fail だった。review-protocol で P1 が 1 件（`BandSpectrum::band` の黙殺）指摘され、除去で解消、verdict は ACCEPT だった。

## bench --no-run の workspace 全体確認

bench のコンパイル確認は workspace 全体で exit 0 だった (Issue #646)。`cargo bench -p ruviform-numeric --no-run` と `cargo bench --workspace --no-run` が共に exit 0 で、他 crate の潜在 bench 破損はなかった。`mise run test ruviform-numeric` は 116 tests pass で、dev profile で AllocDisabler が有効なままだった。check/lint/fmt-check/prek も全 pass した。dev profile で曖昧 assert が顕在化しなかった正確な機序は未解明と記録されているが、bench profile でのみ再現を確認済みで、恒久脆弱性として修正済みである。

## 記録の範囲と限界

本書に記録された検証のうち、実行されなかった、または先送りされたものを以下に列挙する。AL 側との直接比較は Issue #167 で未実施と明記され、bit-exact 性は構造的一致に依存する。AL 移行時の実値差分確認は follow-up として残された。Issue #646 では、dev profile で曖昧 assert が顕在化しなかった正確な機序が未解明と記録されている。Issue #99 のタスク文書では、Work 内チェックボックスが全て未チェックのまま残っているものがあり、実際に未完了か文書更新漏れかは判定不能と記録された。bench profile と dev profile の差異の機序も未解明である。これらは本書の記録範囲外の未解決事項として扱う。

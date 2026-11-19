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
  source: ["https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/54", "https://github.com/scarecrow173/ruviform/issues/56", "https://github.com/scarecrow173/ruviform/issues/75", "https://github.com/scarecrow173/ruviform/issues/98", "https://github.com/scarecrow173/ruviform/issues/99", "https://github.com/scarecrow173/ruviform/issues/150", "https://github.com/scarecrow173/ruviform/issues/152", "https://github.com/scarecrow173/ruviform/issues/153", "https://github.com/scarecrow173/ruviform/issues/156", "https://github.com/scarecrow173/ruviform/issues/157", "https://github.com/scarecrow173/ruviform/issues/167", "https://github.com/scarecrow173/ruviform/issues/169", "https://github.com/scarecrow173/ruviform/issues/183", "https://github.com/scarecrow173/ruviform/issues/255", "https://github.com/scarecrow173/ruviform/issues/265", "https://github.com/scarecrow173/ruviform/issues/646"]
  informs: []
  explains: []
  supersedes: []
---

# DSP・SIMD・カーネルの計測・検証記録

本書は、ruviform の DSP カーネル・SIMD ディスパッチ・数値カーネルに対して行われた検証とベンチマーク計測の結果を記録する。対象は、検証ゲートの実行結果、dispatch オーバーヘッドや各種カーネルの実測値、別リポジトリの数値基盤（al-math / al-analysis）との比較計測、および実機での動作確認である。各記述には出典の Issue 番号を付記し、観測値とそこから導かれた判断を区別して記す。実行されなかった確認と未解決の事項は末尾に列挙する。

## 全量ベンチの分割実行と計測分散

DSP/SIMD PoC の全量ベンチマークは、Windows x64 環境で単一プロセスでは完走せず、カーネル単位の 8 分割実行で 2 run × 756 ケースを完走した (Issue #19)。単一プロセスでの全量実行は 2 回とも約 82–85% 地点でコミット上限逼迫による alloc 失敗 abort となり、並行エージェント多数稼働下の環境要因と記録された。

分割実行への変更後、産物は分割前と同一で、各 run は約 25–27 分で完走した。run 間分散は p50=1.8%・p90=25.7%・max=516.9% であり、分散の大きさは環境負荷に由来すると記録されている。

## DSP カーネル本番経路の検証

本番経路の DSP カーネル検証は一式すべてグリーンだった (Issue #53)。ルーティングを迂回して `*_impl` を直接検証するため、forced token × 全 8 カーネル × 複数サイズの単体テストを用いた。SIMD 等価性は、simd_equiv matrix に `prod_detected`・`prod_avx2`・`prod_sse4_2`・`prod_fallback`・`prod_neon`（probe-gated）の列を追加して確認した。

リアルタイム経路では `rt_dsp_kernels_no_alloc` が `assert_no_alloc` 窓内の 0 alloc を確認し、閾値境界 ±1 の動作確認テストもグリーンだった。最終的に fmt-check/check/lint/test/test-rt/bench-smoke・prek・doc 監査がすべてパスし、graph exit-audit が complete となった。

## dispatch オーバーヘッドの再計測と分類

dispatch の per-call オーバーヘッドは、低負荷環境での再計測により安定して観測できた (Issue #56)。低負荷環境（OS CPU 9–20%）で 3 run 再計測した結果、専用プローブ dispatch_noop（`dispatch!` の no-op 全経路）は 1.291/1.292/1.296 ns と run 間 0.005 ns 以内に安定し、`Level::new()` 単独は測定分解能以下だった。

分類の結論は「混在」だった (Issue #56)。add カーネルの小 N（16–256）では dispatched−avx2 の差が +6–7 ns（run 間 ±0.1–1.5 ns）で安定しており、per-call 固定コストの実在を示した。一方 N≥1023 では run 間で符号が反転する散乱（例: add/1023 で +3.3/−20.1/+34.7 ns）となり、高負荷時に観測された @1024 の劣後（add 0.62x・scale 0.66x）は低負荷では 0.91–0.97x に回復してノイズと確定した。小サイズ帯の回帰（0.48–0.90x）は実在すると判定された。この結果から、Issue #53 の設計示唆だった prepare 固定とトークン繰り上げ方針は小サイズ帯の固定コスト除去を根拠に正当化されると記録され、あわせて小 N の軽量カーネルをスカラーへルーティングする示唆と、dispatched 大サイズでは劣化がないという確認が残された。

## DenormalFlush と ruviform-dsp の検証

DenormalFlush の検証列はすべて完了した (Issue #54)。denormal が ±0 にフラッシュされること、normal 値が不変であること、NaN/Inf が透過することを upstream pull の test-spec 4 ケースで確認し、`rt_denormal_flush_no_alloc` もグリーンだった。mise run の fmt-check/check/lint/test/test-rt はすべてパスし、doc 監査は findings 0、graph は complete だった。

ruviform-dsp でも検証列はすべて完了した (Issue #75)。5 モジュールの実装と workspace メンバー登録を行い、test-spec 全ケースと mise run test/test-rt がパスし、process の 0 alloc を実測で確認した。doc 監査チェーン起因の findings は 0 だった。

## al-math との精度・性能・メモリ比較

ruviform-numeric と al-math の比較は、AudioLens Issue #98 で凍結された baseline の al-math を path 依存で参照する独立 scratch crate で実施した (Issue #150)。精度では、resample が max|diff|=0.0 の bit-exact を示し、FFT forward は 5.7e-14〜3.6e-12（twiddle 構築順と SIMD/scalar 評価順の差に由来）、round-trip/inverse は約 1e-15 に収束した。

性能では、FFT・dot・sum_sq など SIMD 向きカーネルで al-math が 1.2〜3.7 倍高速だったのに対し、resample は ruviform が 1.83〜1.92 倍、least_squares は 1.68 倍高速だった。前者は ruviform の scalar-first 設計意図どおりの結果と記録されている。alloc では al-math の resample が呼出しごとに +6 allocs/770,228 B を要したのに対し、ruviform の resample_into は 0 alloc だった。

## 相関カーネルの al-math 比較

相関カーネルの比較では、al-locate/al-analysis の実運用形状で数値差が 1.1e-16 以下に収まった (Issue #152)。band_gain window・narrow anchor・band/full_ncc・coarse_ncc・reference lag の各形状で NCC/GCC-PHAT/overlap を比較し、全ケースで max|diff| ≤ 1.1e-16（丸め評価順差のみ）かつ argmax が一致した。GCC-PHAT は既知の埋込み lag で真値ピークを確認した。

性能は、scalar 実装の ruviform が al-math の SIMD 実装に対し 1.24〜2.09 倍の遅延に収まった。相関呼出しは FFT 以外の逐次処理が支配的なため、FFT 単体の差（Issue #150 の約 1.5〜1.8 倍）より縮んだと記録されている。direct 経路の大形状（2,205,000）が最大差の 2.09 倍だった。

## stft・bands・xspec の al-analysis/al-math 比較

純ベクトル系カーネルは al-analysis/al-math との比較で bit-exact を示した (Issue #153)。third_octave_bands・band_power・csd・complex_coherence・envelope は同一式・同一評価順の移植で max|diff|=0.0 だった。STFT 本体は FFT カーネル差に由来する 1.1〜2.3e-13 の差だった。

性能比（ruviform/al-math）は、bandpass_many が 0.77 倍（scalar 差の既知トレードオフ）、stft_into（plan 再利用）が 1.16 倍、Welch+H1+coherence が CrossSpectrum の 1 パス融合蓄積により 2.46 倍、band_gain 形状の合成パイプラインが 1.03 倍となり、構造的な回帰なしと判定された。

## ncc 変換長削減の実測

ncc の変換長選択の変更により、実測で変換長が 42–49% 削減され、時間は最大で約 39% 短縮された (Issue #255)。issue255_perf example の release ビルドで forward+inverse ペアを計測した。

形状別の結果は、S=8,640（ncc-valid）で変換長 n が 16,384→8,640（新/旧比 52.7%・時間比約 61%）、S=66,150（anchor-mid）で 131,072→67,500（51.5%・約 66%）、S=2,205,236（band-ncc）で 4,194,304→2,211,840（52.7%・約 90%）、S=2,400,240（anchor-ncc）で 4,194,304→2,430,000（57.9%・約 98%）となり、最悪形状では旧選択とほぼ同等だった。2 冪のセルフチェック（n=65,536）は比率約 1.02 で同等だった。検証は cargo test の 116 unit + 7 doctest がすべてパスし、test-rt で非 2 冪 plan の alloc-free を機械的に確認した。

## NccDirect 維持の校正後確認

校正後のモデルでも回帰形状の NccDirect が維持されることをベンチマークで確認した (Issue #265)。`cargo bench -p ruviform-numeric --bench corr`（release）の結果、ncc_plan 2,400,000/2,400,240（NccDirect）は 134.95 ms で、audio-lens の旧 direct 実測 134.38 ms と一致した。

他の direct 形状は 2,205,000/2,205,236 で 122.70 ms、551,250/551,309 で 8.96 ms、100,000/100,101 で 2.52 ms、FFT 優位形状は 8,192/8,400 で 204.10 µs、4,410/7,938 で 166.56 µs だった。fft_len のペア比較では 4,194,304（2 冪）が 129.02 ms、2,457,600（new_ncc の選択）が 132.19 ms、2,430,000（旧最小 smooth の選択）が 136.17 ms、2,211,840（Issue #255 の headline 形状）が 113.29 ms、4,251,528（2³·3¹²）が 276.26 ms で、モデル推定順序と計測順序が全形状で一致した。

## Issue #99 の Full Gate とタスク別検証

Issue #99 では Full Gate の 10 コマンドがすべて exit 0 だった (Issue #99)。fmt-check/check/lint/test/test-rt/test-video-proc-icc/lint-video-proc-icc/bench-smoke・prek・doc audit を実行し、test は 49 test binaries で 932 passed・0 failed、test-rt は rt_ 名前フィルタで 107 tests に実一致して 0 failed（27 binaries。マッチ件数が 0 でないことを件数で確認）、test-video-proc-icc は feature-on で lib 187 + doctest 24 = 211 passed・1 ignored・0 failed、bench-smoke は ruviform-math 22 tests と ruviform-rt bench smoke が全 Success、prek は 7 hooks passed・1 skipped（Windows 非対象）、doc audit は 303 documents で blocking 0・findings なしだった。

実装は 7 タスクすべてが commit 済みで完了した (Issue #99)。Task1 c88860e（RgbaF32+transfer internals）、Task2 6d27460（working-space kernels）、Task3 d73b8b9（ToneMapSdr）、Task4 c5aa5a6（IccTransform+icc lane）、Task5 878d602（RgbaF32 composite/affine arms）、Task6 6693801（final verification）、Task7 closeout に対応する。独立 review は Task2/3/4/5 で APPROVE または指摘全修正済みで、reviewer B の P1 2 件は修正、P2 2 件は follow-up として記録された。

タスク単位のゲートも個別に記録されている (Issue #99)。Task2（working-space kernels）は lib tests 130/130（working_space.rs 新規 30 = unit 26 + rt_process_* 4）・doctests 19/19 で全ゲート exit 0 だった。レビュー指摘（pts 伝播の tautology 修正・in-loop match の防御的 arm 化・doc wording・doctest unused bindings 修正）をすべて反映し、F16+Bt709 golden・NaN→0 fold 固定・padded src stride の 3 テストと、拒否テストへの dst 不変断言を追加して再 green を確認した。

Task3（ToneMapSdr）は tone_map.rs 新規 1,016 行・17 tests（unit 15 + rt_process_* 2）を含む lib tests 147/147・doctests 21/21 で、独立レビュー 2 件とも APPROVE（P0/P1/P2 なし）だった。ks≥1 passthrough・min=0 時の b≈9.7e-7・pq_eotf_inv の nits 域呼出しという解釈候補はすべて design 整合と裁定され、非有限出力の契約範囲に関する wording 指摘は module doc へ反映後に再 green を確認した。

Task5（composite/affine arms）は lib 171/171（composite.rs 新規 18・affine.rs 新規 6）・doctests 21/21、test-rt で workspace 107 件実選択（video-proc 27 件、新規 rt_process_*_f32_* 6 件を含む）を記録した。`oa < f32::MIN_POSITIVE` の domain-relative cut は両 reviewer が正当と裁定し、divergence-pin test で固定した。あわせて stale な "both"→"every" の掃討、タスク文書の「≤1 ulp」→「≤2 ulp」訂正、q8_weight の doc 整合が適用済み修正として記録されている。

## linalg の構造的一致検証

linalg の検証は AL 側との直接比較を行わず、構造的一致に依存した (Issue #167)。テスト 7 件（doctest と linalg::tests 6 件: 既知解・残差 <1e-12・pivot 回避・複素除算・cond proxy=1e30・特異/不正 shape の None・非有限伝播契約）がすべてパスし、workspace 86 スイートで 0 失敗だった。

AL 側は別リポジトリのため直接比較実行は未実施であり、bit-exact 性は同一式・同一順序の逐語移植と fft::cmul1 系の bit-exact 契約に拠る。AL 移行時に実値での差分確認を行うことが推奨として記録された。

## spectral 統計の AL 式 oracle 一致

spectral 統計は AL の inline 式を独立 oracle として実装し、to_bits 一致を確認した (Issue #169)。`spectral_stats_match_al_oracle_bit_exact` が退化・単要素・空・擬似スペクトルの 6 系列で全統計値の to_bits 一致を示した。spec::tests 10 件に加え、`rt_spectral_paths_do_not_allocate` に RT-suitable 8 関数を `assert_no_alloc` 配下で追加した（median は除外）。

workspace test では ruviform-plugin-host の既知 flaky（IPC タイミング依存の timeout）が 1 回発生したが、単独再実行でパスし、本変更と無関係と記録された。fmt/check/lint/build/test-rt はすべてパスした。

## band spectrum の契約テスト

band spectrum は bit-exact 等価・スレッド間共有・scratch 契約の 3 テストで検証された (Issue #183)。`band_spectrum_matches_band_filter_bit_exact` は len×帯域行列で band/band_into と bit-exact 一致、`band_spectrum_shared_across_threads` は scoped thread での `&` 共有と Send+Sync の静的保証、`band_scratch_contract` は変換長不一致の拒否・同一 n の使い回し・memory_bytes 一致を確認した。

`rt_spectral_paths_do_not_allocate` に新経路を追加し、`cargo test -p ruviform-numeric` は 108 unit + 7 doctest がすべてパス、workspace は 113 バイナリで 0 fail だった。review-protocol では P1 が 1 件（`BandSpectrum::band` の黙殺）指摘され、除去で解消して verdict は ACCEPT だった。

## viz ベンチの実測値

viz の criterion ベンチ（100 samples・warmup 3 s）では、決定的合成 2ch の DemoWaveform を対象に計測した (Issue #157)。波形 LOD 構築は 1M フレーム 48.4 ms・4M フレーム 192.0 ms で約 21.7 M frames/s と線形だった。ビューポート応答は zoom-out（ピラミッド）14.2 µs・zoom-in（ソース直読み）19.9 µs・極端 zoom 9.9 µs で、要求 bin 数のみに比例し総フレーム数に依存せず、60 fps の予算の約 0.1% に相当する。

STFT は 1M フレームを約 210 ms で解析した（fft=1024・hop 256・実時間の約 100 倍速）。FFT サイズを 4 倍にしても総計算量はほぼ同一だった。スペクトログラムタイル取得は 512 列 18.2 ms・2048 列 22.6 ms で、複素スペクトルの再解析は行われていない。

encode は 512×96 で f64 が 284 µs・f32 が 96 µs・u8 が 392 µs（payload 比 1/0.5/0.125）だった。ゲートは全パスで、ruviform-viz 単体 35 件と結合 viz_acceptance 13 件を確認した。

## Windows 11 + WebView2 の実機検証

Windows 11 + WebView2 の実機で全経路の動作を確認し、検証コマンド 7 本がすべて exit 0 だった (Issue #156)。demo_app 実機では、intent→sys.capabilities の結果返却、demo.synthesize(30s) の進捗→完了→demo/peaks pull、cancel、demo.panic でワーカースレッド境界での panic 封じ込め（job failed 返却・ホスト存続）、focus/blur、SetWindowPos resize に伴う viewport resize（dpr=1）、WM_CLOSE からの正常終了を確認した。demo_cli headless も同一 DemoService で synthesize→pull→cancel を完走した。

既知制限として、macOS/Linux 未検証・DPR>1 未実測・rvf://→http 書き換えによる secure context 喪失（WebGPU 制約。上流 `wry` の PR `#1762` は未実装）・evaluate_script の push が WebView2 側で合体されるため pull/sequence latest-wins 設計とした点が記録されている。

## bench --no-run の workspace 全体確認

bench target のコンパイル確認は workspace 全体で exit 0 だった (Issue #646)。Windows 11 x64・rustc 1.99.1 で `cargo bench -p ruviform-numeric --no-run` と `cargo bench --workspace --no-run` がともにパスし、bench profile で lib test target と corr bench がコンパイルされ、他クレートに潜在する bench 破損がないことを確認した。`mise run test ruviform-numeric` は 116 tests + 7 doctests がパスし、check/lint/fmt-check/prek もすべてパスだった。

修正後の検証では、`rt_processing_paths_do_not_allocate` と `rt_spectral_paths_do_not_allocate` が dev profile で実際に収集・実行され、AllocDisabler が有効なままであることを確認した。dev profile で曖昧 assert が顕在化しなかった正確な機序は未解明と記録されているが、bench profile でのみ再現を確認済みで、恒久脆弱性として修正済みである。

## 記録の範囲と限界

本書の記録のうち、未実施・先送り・未解決と明記された項目を以下に列挙する。

- Issue #167: AL 側との直接比較は未実施で、bit-exact 性は構造的一致に依存する。実値での差分確認は AL 移行時の follow-up として残された。
- Issue #150/#152/#153: 比較対象の AudioLens 側 baseline および現行ソースは本ワークスペース外のため直接確認できず、byte-identical の記述は記録どおりに採用した。
- Issue #156: macOS/Linux の実機検証と DPR>1 の実測は未実施である。rvf:// スキームでの secure context 喪失は上流 PR 未実装のため未解決である。
- Issue #157: メモリ使用量の直接計測はスコープ外であり、記録は BoundedCache の予算拒否テストと LOD 常駐サイズの理論値（約 1.6%）に限られる。
- Issue #646: dev profile で曖昧 assert が顕在化しなかった正確な機序（rustc・クレートグラフ上の要因）は未解明である。bench profile でのみ再現を確認し、恒久脆弱性としては修正済みである。
- SIMD 等価性テスト: 完了条件で観測 max ulp 集計値の記録が求められていたが、確認できた範囲の記録に具体的な ulp 数値は残っていない。
- Issue #56: dispatch オーバーヘッド計測の生データ（261 行/run × 3 run）は集計値のみを確認しており、個別セルの検証は行っていない。
- Issue #19: 対応するタスク文書の Work チェックボックスと plan の最終検証フェーズのチェックボックスが未チェックのまま残っており、実際の未完了か文書更新漏れかは判定不能と記録された。

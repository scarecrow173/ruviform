# Issue #153 ruviform-numeric（stft/bands/xspec）↔ AudioLens al-analysis・al-math 比較レポート（2026-10-31）

Issue [#153](https://github.com/scarecrow173/ruviform/issues/153) の受け入れ条件「`al-analysis/src/stft.rs`、`al-math/src/filter.rs` 等との correctness/performance 比較」「band_gain/band_ncc 関連の性能回帰」の実測記録。
比較相手は AudioLens Issue #98 凍結 baseline（`al98-baseline` worktree）の `crates/al-math` と、`al-analysis/src/stft.rs`・`al-analysis/src/reference.rs` の verbatim 移植。凍結 baseline の `stft.rs`/`filter.rs`/`fft.rs`/`vec.rs` は現行 main と `diff -q` で byte-identical を確認済みのため、本記録は「現行カーネル」と「#98 baseline」の双方の突合を兼ねる。

## 環境メタデータ

- OS: Windows 11 x64 / CPU: Intel Core i9-9900K @ 3.60GHz / 16 コア（HT あり）
- rustc 1.99.0（mise pin）/ cargo release profile（`debug = true`）
- 測定ハーネス: 両クレートを `path` 依存だけで参照する独立 scratch crate（`%TEMP%/r153-compare`、各リポジトリ非改変）。ソースは [`data/issue-153-al-compare-main.rs`](data/issue-153-al-compare-main.rs)・[`data/issue-153-al-compare-Cargo.toml`](data/issue-153-al-compare-Cargo.toml) に保存。`al-analysis::stft` は `al_locate::dsp`（=`al-math` の re-export）経由で `al_math::fft::Fft`・`al_math::vec::{mag_sq, scale}` のみを使う自己完結実装のため、`use` 行を差し替えた verbatim 移植を harness 内に置く（#150 の `al-locate::dsp::conv` 移植と同じ扱い）。Welch 蓄積（Pxx/Pyy/Pxy→coherence→H1）は `reference.rs` L520-546 の inline ループを verbatim 移植
- 生出力: [`data/issue-153-al-compare-output.txt`](data/issue-153-al-compare-output.txt)
- 計測方法: `std::time::Instant` の per-call 中央値（warmup 1 回 + 5〜50 rep）。alloc/bytes は counting global allocator で実測。数値差は `max |a − b|`
- 負荷注意: 単発の ad-hoc 計測であり Criterion 統計処理なし。±20% 程度の揺れはあり得るため桁の大小関係のみを読む（`x比率` は al時間/rf時間、>1 で ruviform 側が高速）
- 入力: `al-analysis` テストと同一の決定的 fixture（1 kHz + 0.3×3 kHz 正弦 + LCG 疑似ノイズ）。production 形状は reference `stft_window=4096/hop=2048`、cheap `2048/1024`、帯域 `third_octave_bands(40, min(rate/2, 16000))`（48 kHz で 26 帯域）

## 精度比較（同一入力・max |diff|）

| カーネル | 条件 | max |diff| |
| --- | --- | --- | --- |
| STFT re/im | w=4096 h=2048（reference 形状・4 s） | 2.3e-13 |
| STFT re/im | w=2048 h=1024（cheap 形状） / w=1500 h=500（非 2 冪窓→2048 ゼロ埋め） | 1.1e-13 |
| power / magnitude / psd | 同 3 条件（先頭 8 フレーム） | 3.5e-10 / 2.3e-13 / 2.4e-17 |
| `third_octave_bands` | 40–16000 Hz（26 帯域） | **0.0（bit-exact）** |
| `band_power` | PSD 行×上記帯域 | **0.0（bit-exact）** |
| `csd` | 単一フレーム対 | **0.0（bit-exact）** |
| `complex_coherence`（free fn・mean 入力） | reference 形状 | **0.0（bit-exact）** |
| `envelope` | magnitude 行・bins=32 | **0.0（bit-exact）** |
| Welch 和（`CrossSpectrum` vs mean×frames 復元） | 92 フレーム×4096 bin | 3.7e-9（和 ~1e7 スケールで相対 ~1e-13） |
| coherence（AL mean 入力 vs rf 和入力） | 同上 | 3.3e-16 |
| `transfer_h1`（同上） | 同上 | 2.2e-16 |
| `bandpass_many` / `BandFilter` | 26 帯域×4 s | 8.9e-16 |
| `spectral_envelope` | len=8192/4096・fft=1024・bins=32 | 1.3e-15 |

所見:

- **純ベクトル系は bit-exact**。`third_octave_bands`/`band_power`/`csd`/`complex_coherence`/`envelope` は同一式・同一評価順の移植であり 0.0 を確認。
- STFT 本体の 1.1〜2.3e-13 は FFT カーネル差（al-math pulp SIMD vs ruviform scalar）由来で、#150 で計測した raw FFT の差（n=1024 で 5.7e-14〜n=65536 で 3.6e-12）と同じ帯。窓係数も AL `hann` と `Window::Hann` で同一式。
- power の 3.5e-10 は |z|² 化による絶対差の増幅（|z|~1e4 級→二乗で ~1e8 級。相対 ~3e-18 程度）で、規約差ではない。
- **和 vs mean の形式差は ~1e-16 で一致**。`CrossSpectrum` が和のまま coherence/H1 を出す設計は、AL が mean 化してから出す値と `1/frames` 約分により数学的に一致し、実測でも ulp 級の差のみ。
- `bandpass_many`/`spectral_envelope` も ~1e-15 で、scalar 化による FFT 丸め差が実効的に小さいことを確認（mask で大半の bin が 0 になるため有効演算量が小さい）。
- 意図的差分: rf 側は PSD 分母を `Σw²` 実計算（AL は `0.375·window` ハードコード）。Hann での差は係数和の丸め誤差分のみで、psd 差 2.4e-17 に含まれる。

## 性能比較（median µs/call・release）

| カーネル | 条件 | al | ruviform | 比率（al/rf） |
| --- | --- | --- | --- | --- |
| `stft` one-shot | 4096/2048・4 s（92 フレーム） | 5259.1 | 5625.7 | 0.93 |
| `stft_into`（plan/workspace 再利用） | 同上 | — | 4519.0 | **1.16** |
| `bandpass_many` | 2ch×26 帯域・4 s（band_ncc 波形生成形状） | 125541.1 | 163370.7 | 0.77 |
| `BandFilter` 逐次（band vs band_into ループ） | 同上 | 140852.2 | 152140.9 | 0.93 |
| Welch+H1+coherence | 92 フレーム×4096 bin | 1295.0 | 526.9 | **2.46** |
| `spectral_envelope` | len=8192・fft=1024・bins=32 | 205.6 | 208.8 | 0.98 |
| band_gain 形状合成（stft×2+psd×2+mean+band_power） | 2ch×4 s | 14018.5 | 13558.6 | **1.03** |

所見:

- **帯域フィルタ系は al-math が 1.1〜1.3 倍高速**（IFFT 支配、SIMD 差）。#150 の real FFT 差（0.69〜0.72）より狭いのは mask 後の逆変換が実効的に小さいため。逐次 `BandFilter` 経路では AL の呼出しごとの dispatch+出力確保が入り 0.93 まで縮む。
- **STFT は plan 再利用で rf が 1.16 倍高速**。one-shot 同士では 0.93（scalar FFT 差）だが、AL `stft()` はフレームごとに `Vec` 2 本を確保し `Vec<Vec>` を構築するのに対し、rf `stft_into` は plan+出力を再利用して 0 alloc で回るため、production の「同じ plan で複数信号を処理」経路で逆転する。
- **Welch+H1+coherence は rf が 2.46 倍高速**。AL の inline ループは pxx/pyy/pxy の 3 系列を走査した後、mean 化の 4 回目の走査 + coherence/H1 でさらに走査・確保するのに対し、`CrossSpectrum::accumulate` は 1 パスで 4 系列を融合蓄積し、導出は和のまま追加 2 パスで済む構造差。
- **band_gain 形状の合成パイプラインは rf が 1.03 倍高速 — 構造的な性能回帰なし** を確認。stft+psd+band_power 合成では AL の `Vec<Vec>` 確保・走査が効き、scalar FFT 差を上回る。
- `spectral_envelope` はほぼ同等（0.98）。小さな変換で SIMD 差が出にくい。

## メモリ / alloc 比較（counting allocator 実測）

| 項目 | al | ruviform |
| --- | --- | --- |
| `stft()` 呼出し（92 フレーム×4096） | **+129 allocs** / 3,154,400 B（frame ごと Vec×2 + 外枠） | +31 allocs / 3,148,448 B（plan twiddle ~28 + 係数 + 行列 2 本） |
| `stft_into` 呼出し（バッファ事前確保） | 該当 API なし | **0 allocs / 0 B** |
| `BandFilter::new`（4 s 入力） | +46 allocs / 20,011,456 B | +46 allocs / 20,011,456 B（同一バッファ構造） |
| `band()` 呼出し | +1 alloc / 1,536,000 B（出力 Vec） | `band_into`: **0 / 0** |
| Welch 蓄積 | inline ループ（pxx/ppy/pxy 確保 + 導出でさらに確保） | `CrossSpectrum::new(4096)`: 4 allocs / 131,072 B、`accumulate`/`coherence_into`/`transfer_h1_into`: **0** |

所見:

- **rf の RT-suitable 経路は全て実測 0 alloc**。`stft_into`/`band_into`/`accumulate`/`coherence_into`/`transfer_h1_into`/`power_into`/`magnitude_into`/`psd_into`/`band_power_into` は `rt_spectral_paths_do_not_allocate` テストでも検証済み。
- `BandFilter::new` はバッファ構造が同一（plan+スペクトル×2+mask×2+real）で allocs/bytes が一致。rf の `memory_bytes()` 構造見積もりは 13,984,176 B（実測 20,011,456 B には plan twiddle 内部・Vec ヘッダ等の allocator 側の詳細を含む）。
- STFT は AL が 92+37 allocs を伴うのに対し rf は行列 2 本 + plan で完結。`StftMatrix` の連続レイアウト化は API 上の意図的差分（設計書「カーネル責務表」）。

## band_gain/band_ncc との関係（回帰確認）

- **band_gain**（reference.rs の stft→mean PSD→band_power→`sqrt(cap/src)`）は rf 合成で 1.03 倍（実質同等〜微速）を実測。AL がこのパイプラインで使う全カーネル（stft/psd/band_power/Welch/coherence）が揃っており、`psd_norm()` 公開により mean 化せず和系列から直接 band 積分するさらに軽い合成も可能。
- **band_ncc**（BandFilter 帯域波形→NCC 曲線→argmax）の #153 側の構成費用は「2 回の forward + 帯域ごと IFFT + `dot`/`sum_sq`」で、実測の帯域波形生成部は 0.77〜0.93 倍（SIMD 差の範囲）。NCC カーネル本体は Issue #152 の範囲（この worktree には未マージ）で、`CorrelationPlan::new_ncc(window, window+2·max)` 呼びはそのまま `rf` の `vec::dot`/`sum_sq`（#150 比較済み: dot 0.62・sum_sq 0.27 — SIMD 差）を使う想定。帯域生成+IIFT 側の構造回帰（forward 共有の喪失等）はなし。
- 回帰リスクが残るのは bandpass 系の ~0.77 倍であり、これは scalar-first 参照実装の既知トレードオフ（#150 比較と同じ帯）。SIMD 化は simd-ulp ADR の等価性検証つきで別途対応する Deferred 項目。

## 構造差まとめ（API 契約レベル）

| 側面 | al-analysis/al-math | ruviform-numeric |
| --- | --- | --- |
| STFT 行列 | `Vec<Vec<f64>>`（frame ごと確保） | 連続 `frames×fft_size` flat（`StftMatrix`） |
| 出力先 | 所有 Vec を常に返す | `*_into`（呼出し側バッファ・RT-suitable）と所有版の両系統 |
| Welch 蓄積 | 呼出し側の inline ループ（3 箇所重複） | `CrossSpectrum`（1 パス融合蓄積・和保持） |
| エラー契約 | `assert!` panic / 暗黙の空結果 | panic-free `Result<_, Error>`（`window=0`/`hop=0`/`rate=0`/`fft_size` 非 2 冪/`third_octave_bands` 非正入力を検証拒否） |
| PSD 正規化 | `0.375·window`（Hann 専用ハードコード） | `Σw²` を係数から実計算（任意窓で正しい） |
| 実行バックエンド | pulp x86-v3 SIMD + TLS dispatch | sequential scalar（bit-exact・ISA 非依存） |

## 未解決・後続

- bandpass 系の ~0.77 倍の scalar 差は SIMD 化の余地（Deferred・simd-ulp ADR の等価性検証が前提）。
- `fir_ir`/`fir_model`・phase unwrap/group delay は合成可能な演算のみ移行し、編成ロジックは AL 残留（設計書の責務表）。
- band_ncc の NCC 本体は #152 並行成果物のため、本記録の帯域生成部計測 + #152 側の相関計測をあわせて評価する（`BandFilter.band_into`→`CorrelationPlan` の接合は設計上協調済み）。
- 計測は単発 ad-hoc のため、継続計測が必要になれば Criterion ベンチ化を検討する。

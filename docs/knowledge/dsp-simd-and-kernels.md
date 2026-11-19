---
id: "KNOW-1CyA3ZraJTJAF9Onfy5GJt"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "codebase"
title: "DSP・SIMD・カーネル・数値処理の設計"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/54", "https://github.com/scarecrow173/ruviform/issues/56", "https://github.com/scarecrow173/ruviform/issues/75", "https://github.com/scarecrow173/ruviform/issues/92", "https://github.com/scarecrow173/ruviform/issues/97", "https://github.com/scarecrow173/ruviform/issues/99", "https://github.com/scarecrow173/ruviform/issues/156", "https://github.com/scarecrow173/ruviform/issues/157", "https://github.com/scarecrow173/ruviform/issues/167", "https://github.com/scarecrow173/ruviform/issues/168", "https://github.com/scarecrow173/ruviform/issues/169", "https://github.com/scarecrow173/ruviform/issues/170", "https://github.com/scarecrow173/ruviform/issues/183", "https://github.com/scarecrow173/ruviform/issues/646"]
  informs: []
  explains: []
  supersedes: []
---

# DSP・SIMD・カーネル・数値処理の設計

この文書は、DSP・SIMD・カーネル・数値処理の設計と実装に関する再利用可能な知見を、過去の実装記録から抽出してまとめたものである。記録元の文書は後に削除されたため、本書はそれらに依存せず単独で読めるように記述する。事実は各記述の末尾に Issue 番号で出所を示す。実測値や観測された挙動は事実として記し、解釈が含まれる箇所はその旨を明示する。

## SIMD PoC の依存隔離と公開範囲

SIMD の PoC 実装は、候補クレートを dev-dependencies にのみ追加し、通常の dependencies は空のまま維持する構成で行った。fearless_simd・pulp・multiversion・criterion・approx の各クレートは ruviform-math の dev-dependencies にのみ存在し、公開 API に候補クレートの型を露出させない方針である（Issue #19）。

pub(crate) は dev-dependency 境界で成立しないため、PoC コードは #[doc(hidden)] pub mod として公開し、ベンチとテストが同一のカーネル実装を共有する構成を採った。tests/ 側は #[path] include で同一モジュールを取り込む（Issue #19）。これは設計書の pub(crate) 配置が dev-dep 制約で成立しないという上流文書との決定差異であり、deviation として設計書側に記録し、計画側は設計書の配置を正として採用する運用とした（Issue #19）。

## unsafe を使わない SIMD レベル強制

PoC コードは unsafe 0 を制約とし、assume_supported・new_unchecked・std::arch を使わずに実装した。レベルの強制には Fallback::new()・Level::new().as_*()・pulp::x86::V*::try_new() の Option 返却 API を使い、構成不能なレベルは panic せず skip として記録する（Issue #19）。

unsafe 不使用の検証は grep ではなく lint の unsafe_code ゲートで行う。grep はコメントや doc 内の言及を誤検出するため不採用とした（Issue #19）。

## 境界入力と denormal の生成

denormal のテスト値は f32::from_bits(0x0040_0000) のようなビットパターン指定や f32::MIN_POSITIVE * 0.5 で生成する必要がある。素朴な係数乗算では denormal が +0.0 に丸まり、生成自体ができない（Issue #19）。

境界入力は normal・edge（denormal・±0・NaN・±Inf の混合）・denormal の 3 系統で設計した（Issue #19）。

## multiversion と criterion の制約

multiversion クレートは #[multiversion(targets(...))] による dispatched 経路のみを提供し、個別 clone の強制 API を持たない。これは確定済みのリスクであり、等価性 matrix では dispatched 列のみとして扱い、検証網羅性の構造的差として採否の判断記録に残す方針とした（Issue #19）。

criterion の estimates.json は実行ごとに上書きされるため、「run 実行 → 集約 → 次 run」の順序で計測する。集約 jsonl は LF 改行・末尾改行で生成し、フォーマッタの end-of-file・改行系フックに適合させた。全量約 400〜560 ケースは BenchmarkGroup の warm_up_time・measurement_time・sample_size の削減で約 20〜30 分に短縮する PoC 用設定を導入した（Issue #19）。

## MSRV 検証の限定

dev-dep 各クレートの MSRV（fearless_simd 1.89、multiversion 1.86、criterion 1.86、pulp は宣言なしで推移的に約 1.89）がワークスペース MSRV 1.85 を超えるため、MSRV 検証は cargo check -p ruviform-math --lib --locked に限定した。Cargo.lock は先に 1.99.0 側のツールチェーンで再生成してから 1.85 検証に入る。dev-dep は caret 指定と lockfile を再現性の基準とし、exact pin は不採用とした（Issue #19）。

## 等価性契約と超過時ポリシー

等価性契約は bit 完全一致（to_bits）または ulp 上限 8 とした。観測値が上限を超えた場合は実装バグか契約不適合として先に調査し、数値を合わせ込む事後緩和はしない。biquad で固定上限の超過が正当と判明した場合のみ、代替契約形（先頭 N フレーム ulp + 以降相対誤差、またはフレーム長依存上限）への切替を許容する。構成不能なレベルは panic ではなく skip 記録とする（Issue #19）。

## 計測データの配置規約

実験ログの監査が exp/ 直下の全 jsonl をイベントスキーマで検証するため、criterion の計測データ jsonl は data/ サブディレクトリに配置した。宣言パスからの逸脱として記録されたが、実験ログと計測データが同居する場合の配置規約として再利用可能である（Issue #19）。

## dispatch 層のサイズ閾値ルーティング

公開カーネル 8 本は、トークンを第 1 引数としサイズ閾値でスカラールーティングする。閾値は add<128、scale<64、clamp_unit<256、その他は常時 token 経路という design 確定値で、根拠コメント付きである（Issue #53）。

契約違反は debug_assert と release 非 panic（共通プレフィックス・channels クランプ）に統一した。各関数の doc には精度契約（bit 完全一致、ulp 256、ulp 8 + 有限入力前提 + NaN/Inf 契約外）を明記する。BiquadCoeffs・SineCoeffs・SineState は dsp から pub use で昇格した（Issue #53）。

## dispatch 固定コストの分離計測

fearless_dispatched の軽量カーネルでの劣後を、per-call 固定コストとノイズに分離する手法として 2 系統の証拠を併用した。系統 A は dispatch_probe ベンチ（level_new・dispatch_noop・token_avx2・小サイズ add の 16/32）で、系統 B は Δ(N)=dispatched−avx2 のサイズ依存差分解析である。結論は両者の一致で支持する。計測中に環境負荷の混入があった場合は run を棄却して再計測し、環境負荷をレポートに記録する運用とした（Issue #56）。

## prepare/process 分離と alloc-free

ruviform-dsp では window・biquad・fir・fft・resample の 5 カーネルを実装し、全カーネルを prepare/process 分離 API とした。process が alloc-free であることを assert_no_alloc の計測窓で実測検証する設計である（Issue #75）。

audio-analysis では LevelMeter・SpectrumAnalyzer・WaveformPeaks を実装した。process 入力長は短い場合ゼロ詰め・長い場合先頭切詰めの決定的規則とした。SpectrumAnalyzer::magnitudes() は n/2+1 bin の |z| 振幅を正規化なしで返し、スケールは描画側の責務とした。LevelMeter は加算ではなく直近ブロック値を保持し、バリスティックは非ゴールとした。WaveformPeaks のみ非 RT で Vec を返却する（Issue #92）。

## 合成・変換カーネルの数値設計

CompositeOver・Crossfade では、呼出し毎に opacity/t を is_finite() と [0,1] で検証し、round(v·255) で Q8 量子化する。Rgba8 の合成は sa/b/oa/oc を全て整数 round-half-up で計算し、b は画素毎に 1 回だけ計算して oa/oc が再利用する。oc≤255 が構造的に保証される自己無撞着設計である。RgbaF16 は同一式を f32 ドメインで評価し、RNE の binary16 丸めを行う。w==0/tq==0 は検証通過後・書込み前に early return し、共有 dst 拒否は opacity=0 でも先行する（Issue #97）。

AffineTransform は Rect/RectMap パラメータを取り、map.src の非空・src フレーム内包含を i64 拡幅で検証する（違反は InvalidValue("src rect")）。写像演算（2·offset·map.src.w 等）は i128/u128 で実施し、セル中心最近傍サンプリングを行う。map.dst の空・キャンバス非交差はエラーとせず全画素 transparent black という定義済み結果とし、矩形外も transparent black で充填する。dst メタデータは src から伝播する（Issue #97）。

f16 コーデック関数（f64_to_f16_bits/f16_bits_to_f64）は複製せず pub(crate) へ格上げして共有した。同等コーデックの複製は DRY 違反のため不採用である。依存は ruviform-video-core・ruviform-media-core のみとし、RT クレートや同順位・engine への依存追加は禁止した。assert_no_alloc は本設計で導入せず、rt_process_* の alloc-free はレビュー担保とし、機械検出は後続 Issue とした。process の in-place dst・可変引数形状は二相 API からの確定済み逸脱であり、実装側で再検討しない（Issue #97）。

## denormal 対策の明示ノード方式

denormal flush は DenormalFlush ノードとして実装した。denormal を ±0 に正規化し、normal 値は不変、NaN/Inf は透過、upstream pull 呼出しという 4 ケースをテスト仕様で検証し、assert_no_alloc の計測窓内で 0 alloc を確認した。FTZ/DAZ のグローバル設定ではなく明示ノード方式を採用した（Issue #54）。

## working space と transfer の境界規則

working space は RgbaF32 + {primaries: Bt2020, transfer: Linear, matrix: Identity, range: Full} と定義し、1.0 = 100 cd/m² の絶対輝度で [0,1] 制限を設けず、負値・1 超を lossless に保持する。alpha は straight パススルーとした。Unspecified の primaries/transfer は Bt709 を既定とし（既存の matrix→Bt601・range→Limited の上位拡張規則）、未知 variant は InvalidValue で拒否する（Issue #99）。

transfer decode の境界規則は、Bt709 v=255→1.0（=100 nits）、toe v<0.081→v/4.5（負入力も定義済み）、Pq v=1.0→100.0・Hlg v=1.0→10.0（10000/1000 nits）、Hlg v=0.5 は S=0.5²/3 分岐、Pq/Hlg 信号域外は clamp とした。FromWorking の Rgba8 出力は round(clamp(v,0,1)·255) の round-half-up で境界 ±1 code とする。encode は Pq/Hlg が信号域 clamp、Bt709/Linear は非 clamp である（Issue #99）。

## ToneMapSdr の EETF 実装

ToneMapSdr は HDR→SDR 輝度写像として BT.2390-8 §5.4 EETF を実装した。L_HS の導出優先順位は mastering_display.max_luminance（/10_000→nits・>0）→ content_light_level.max_cll（>0）→ 既定 1000 nits である。L_min は mastering_display.min_luminance が max_luminance 未満のときのみ採用し、不正順序は b=0 とする。L_HS ≤ L_DS（100 nits）なら ks≥1 で全域 passthrough、Y_src≤0 は画素 passthrough（厳密一致）、hdr は出力で None とする。L_DS=100 nits 固定・ターゲットパラメータ化は非ゴールである（Issue #99）。

## IccTransform の feature 隔離と検証レーン

IccTransform/IccIntent は feature icc 配下に閉じ込め、optional dep の moxcms 0.9 を default-features=false + lut + extended_range で使用した。SIMD 系・in_place・options・any_to_any は不採用である。ctor で profile parse と executor を事前構築し（失敗は InvalidValue）、process は per-pixel 1 要素スライス呼出しで alloc-free とする。moxcms の型は公開 API に露出させず Error::InvalidValue に map する。dst.color は {Unspecified×4, Identity, Full, hdr:None} に落とし（ICC 変換で CICP 保証不可のため）、alpha は straight パススルーとする。golden は pin 版 moxcms スカラー executor 出力への bit-exact である（Issue #99）。

feature 経路は別検証レーンが必要である。cargo test --workspace は default feature のみのため feature icc 経路は構造的に検証対象外になる。対策として feature 付きの test・lint タスクを追加し、CI に icc-verify ジョブを設置した。feature-off の依存ツリーに対象クレートが出ないことを確認してベース経路の無退行を担保する（Issue #99）。

## golden oracle と精度契約

working space・tone map の golden/ulp-bound oracle は、定義式を f64 で評価した参照に対する bound 比較とし、厳密一致 golden は使わない。プラットフォームの powf 差を吸収するためである。IccTransform のみ pin 版 moxcms 出力への bit-exact とする。Full Gate は計画の最終段で 1 回のみ実施し全ログを保存する。RT 検査は実一致件数で証明し、0 件一致のスキップを成功と偽らない（Issue #99）。

既存 CompositeOver・Crossfade・AffineTransform への RgbaF32 arm 追加は、RgbaF16 パスと同式を f32 ドメインで評価し binary16 丸めを適用しない（≤2 ulp 契約）。受理集合の拡張のみで既存 Rgba8/RgbaF16 経路は不変とした。PixelFormat::RgbaF32 は non_exhaustive enum への variant 追加で non-breaking、plane_count=1・is_planar=false・bytes_per_sample=16・chroma_subsampling=None である（Issue #99）。

## bit-exact 移植による置換

audio-lens の csolve を ruviform-numeric::linalg::complex_solve として逐語移植した（audio-lens #102 R1 契約に基づく）。pivot 選択（max_by total_cmp・同値は最後の index）・消去順序・失敗判定を移植元と同一に保ち、同一入力で同一の解・条件数 proxy・失敗（None）を返す。複素値は (re,im) タプル、複素演算は非融合の a*b±c*d 形で評価順を固定した。非有限入力は拒否せず伝播する。これは csolve に有限性検査がない移植元の意図的契約であり、least_squares の None 拒否とは異なるため API 精度契約に例外を明記した。条件数 proxy は pivot 絶対値の最大/最小比であり、真のスペクトル条件数ではない。std のみ・num-complex 不採用・non-RT である（Issue #167）。

既存の OverlapCorrPlan/CorrelationPlan は実行時コストモデルで direct/FFT を切替えるため、呼出側が「必ず直算」を契約できず、移植元の直算実装と bit-exact 置換が不能だった。そこで plan/コストモデルを介さない scalar 直算曲線 API（ncc_direct_into・ncc_direct・overlap_ncc_direct_into・overlap_ncc_direct）を新設した。分子・分母とも corr0_parts_seq と同一の逐次評価順で累積し、正規化は Σx²>0 かつ Σy²>0 のときのみ sxy/√(Σx²·Σy²) とする。縮退・NaN エネルギーは 0.0、|lag|≥L は 0.0 である。_into 形は alloc-free で RT-suitable とした。template エネルギーは 1 回だけ評価し、評価順同一で bit-exact を維持する。移植元 cheap.rs の IPD sweep は逆方向のため overlap_ncc_direct(y,x) または index 反転で同値写像とし、テストで固定した（Issue #168）。

spec モジュール（centroid・bandwidth・rolloff・flatness・entropy・flux・ols_slope・median・argmax）を移植元の cheap.rs から verbatim 移植した。移植元の非対称な縮退規則を意図的に維持する。bandwidth のみ total<=0.0 否定形ゲートで NaN 伝播し（他は total>0.0 肯定形で 0.0 写像）、entropy は n=1 かつ total>0 で ln(1)=0 除算の NaN となり、argmax 同率は最後の index（max_by）である。rolloff のコメントは "85% energy" だが実際は振幅行の累積であり、fraction を引数化した（移植元は 0.85 固定）。flux/ols_slope は移植元の zip 黙認から Err("length") 検証へ意図的に変更した。median は確保するため non-RT、bin 域選択・prev 保持・mean_row 蓄積は呼出側ポリシーとした（Issue #169）。

## 永続契約と stable ID

移植元の normalized_pcm 永続契約・cache 鍵が plan 内部パラメータへ直接依存し、内部表現の変更で崩れ得る問題に対し、係数表を含まない正準パラメータ値 ResampleParams と決定的 stable ID を提供した。ResampleParams は source_rate/target_rate/up/down/radius/cutoff_bits を f64::to_bits で保持し bit-exact な Eq/Hash を持つ。ID 形式は rvf-resample/1/<src>/<dst>/<up>/<down>/<radius>/<cutoff-hex16>、CONTRACT_VERSION=1、ALGORITHM_ID="polyphase-blackman-sinc"（移植元 provenance トークンと同一）である。from_stable_id は正準スペルと全フィールド導出整合性を検証する fail-closed とした。ResamplePlan は params+phases 保持へ再構成し、パラメータ導出を ResampleParams に集約して単一正本とした。serde 非依存（std のみ層）のため正準文字列+値型で提供する。params のみの new() は、係数表確保が非現実的な極端有理比でも ID を取得可能にする（Issue #170）。

## 不変共有と排他 scratch による並列化

BandFilter の保持スペクトルを、不変共有体 BandSpectrum（Send+Sync・forward FFT 1 回）と呼出毎の排他スクラッチ BandScratch（&mut）に分離した。audio-lens #71 の 52 task/pair 並列化需要に対応するためである。(channel,band) タスク単位で scratch を供給し、入力全長 FFT の再計算を回避する。BandFilter は BandSpectrum+BandScratch の逐次便宜型として再構成し、mask→IFFT→切戻しを二重化せず bit-exact を構造的に保証する。BandScratch は構築元変換長 n を保持し、不一致を InvalidValue("scratch") で拒否する（公開 API の panic-free 契約維持・同一 fft_size 間で使い回し可）。review で所有版 BandSpectrum::band を除去した。scratch 不一致時にゼロ波形を黙って返して誤用を隠すため YAGNI と判断した（Issue #183）。

## bench profile と assert_no_alloc

bench profile は debug_assertions off のため、assert_no_alloc の disable_release feature が #[global_allocator] AllocDisabler 型を cfg 外にし、参照が解決不能になることがあった。さらに criterion→serde_json 経由の PartialEq<Value> for f64 無条件実装が、assert_eq!(<f64 式>, []) の [] を曖昧化した。これは dev profile では顕在化しない bench profile 固有の依存グラフ由来の問題である。修正は AllocDisabler への #[cfg(debug_assertions)] 付与と、assert を [] as [f64; 0] の型付けである。assert!(is_empty()) 形は clippy assert_is_empty lint の制約で使用不可である（Issue #646）。

bench profile では debug_assertions off により assert_no_alloc が no-op となるのは crate 設計どおりであり、RT 検査の権威は dev/test profile の rt_ lane にある。mise の bench/bench-smoke タスクは実計測・スモーク入口のため、--no-run コンパイル検証は cargo bench --no-run の直接実行が必要である（Issue #646）。

## 計測に基づく閾値設計と描画 tier の分離

GUI は WebView 非依存コアと wry+tao adapter の 2 クレートに分離し、workspace 既定経路は headless のまま維持した。PoC 計測に基づき、小値は JSON・バルクのみデータ pull の閾値分離を採用した（1 読出上限 16 MiB）。計測値は binary pull が 1 MiB で約 94〜115 MB/s・16 MiB で約 88〜146 MB/s、64 KiB の小 pull は約 2.9 MB/s で JSON RTT より遅かった。Canvas2D が約 32k 列で約 8.75 fps まで低下するのに対し WebGL2 は 57.9〜60 fps を維持した計測を受け、DOM/Canvas2D/WebGL2/NativeSurface を描画 tier として分離した（Issue #156）。

viz は内部値を全て f64・NaN=欠測とし、表示丸め（f32/u8 変換）は Quantization/DbRange/ColorMap/encode の display 境界に限定した。PagedBytes は DataSource 形状（len()+read(offset,buf)）と一致させ、薄い adapter でデータ pull 配線を可能にした。BoundedCache は汎用提供のみで tile() との自動連携は YAGNI、WaveformLod はオンデマンド再構築前提（永続化・増分更新なし）とした。依存は media-core/numeric/exec のみ・全 API 非 RT・unsafe なしである（Issue #157）。

## canonical closeout の運用規約

closeout task の共通規約として、実績 IR・coverage・task/plan status は immutable な実 log のレビューからのみ記述し、新規メトリクスの再計算はしない。検証未実施のまま status を進めず、全 task 完了の実 closeout 後に plan を completed へ遷移する。verified-by の test-spec を実結果で confirmed→approved へ更新し、GitHub merge は lead の authoritative な操作のみで行い、文書チェックリストで代行しない（PR merged の主張はしない）。doc 監査 exit 0・findings 0 と graph complete を確認する（複数 Issue 共通）。

## 未確定・保留事項

- SIMD 採否 ADR を作成するタスク文書はチェックボックスが全て未チェックのままだが、親 plan では G-1〜G-3 が完了済みで plan status は completed である。task 文書側の status 更新漏れか、実際に未完了かは文書群からは判定不能である。採否 ADR 本文は今回の対象外である。
- SIMD PoC の計画文書の Phase H-4（最終検証スイート）のチェックボックスも未チェックのままである。plan status は completed で H-4 はオーケストレータ側最終検証に委譲する設計と読めるが、実績の有無は不明である。
- Issue #646 の「dev profile で assert_eq! 曖昧が顕在化しなかった正確な rustc/クレートグラフ上の機序」は未解明と記録されており、bench profile でのみ再現確認済みである。
- 等価性 matrix の Done When は観測 max ulp 集計値の記録を要求するが、具体的な ulp 数値は本知見の対象文書には記載がなく、実験ログ・採否 ADR 側にあると推測される。
- Issue #167 の bit-exact 性は移植元リポジトリとの直接比較が未実施で、構造的一致のみに依存する。実値差分の確認は移植元側移行時の follow-up として記録されている。

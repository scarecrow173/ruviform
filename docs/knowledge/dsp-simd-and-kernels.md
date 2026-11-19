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
  source: ["https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/54", "https://github.com/scarecrow173/ruviform/issues/56", "https://github.com/scarecrow173/ruviform/issues/75", "https://github.com/scarecrow173/ruviform/issues/92", "https://github.com/scarecrow173/ruviform/issues/97", "https://github.com/scarecrow173/ruviform/issues/98", "https://github.com/scarecrow173/ruviform/issues/99", "https://github.com/scarecrow173/ruviform/issues/150", "https://github.com/scarecrow173/ruviform/issues/152", "https://github.com/scarecrow173/ruviform/issues/153", "https://github.com/scarecrow173/ruviform/issues/156", "https://github.com/scarecrow173/ruviform/issues/157", "https://github.com/scarecrow173/ruviform/issues/167", "https://github.com/scarecrow173/ruviform/issues/168", "https://github.com/scarecrow173/ruviform/issues/169", "https://github.com/scarecrow173/ruviform/issues/170", "https://github.com/scarecrow173/ruviform/issues/183", "https://github.com/scarecrow173/ruviform/issues/255", "https://github.com/scarecrow173/ruviform/issues/265", "https://github.com/scarecrow173/ruviform/issues/646"]
  informs: []
  explains: []
  supersedes: []
---

# DSP・SIMD・カーネル・数値処理の設計

この文書は、Ruviform の DSP・SIMD・カーネル・数値処理に関する設計判断と実装知見を、過去の実装記録から抽出してまとめたものである。記録元の文書は後に削除されたため、本書はそれらに依存せず単独で読めるように記述する。事実は各記述の末尾に Issue 番号で出所を示し、実測値や観測された挙動は事実として、解釈を含む箇所はその旨を明示する。

## SIMD PoC の検証設計

SIMD 候補クレートの評価は、本体の依存を汚染しない構成で行った。fearless_simd・pulp・multiversion・criterion・approx は ruviform-math の dev-dependencies にのみ追加し、dependencies は空のまま維持し、候補クレートの型は公開 API に露出させない（Issue #19）。

dev-dependency は pub(crate) の境界を越えられないため、PoC コードは #[doc(hidden)] pub mod として公開し、tests/ 側は #[path] include で同一モジュールを取り込んで、ベンチとテストが同一カーネル実装を共有する構成を採った。この配置は設計書上の pub(crate) 配置からの差異であり、上流文書間で決定が食い違う場合は deviation として設計書に記録し、plan は設計書側を正として従う運用とした（Issue #19）。

PoC コードは unsafe 0 を制約とし、assume_supported・new_unchecked・std::arch を使わずに実装した。レベルの強制には Fallback::new()・Level::new().as_*()・pulp::x86::V*::try_new() の Option 返却 API を使い、構成不能なレベルは panic せず skip として記録する。unsafe 不使用の検証は grep ではなく lint の unsafe_code ゲートで行う。grep はコメントや doc 内の言及を誤検出するためである（Issue #19）。

境界入力は normal・edge（denormal・±0・NaN・±Inf の混合）・denormal の 3 系統で設計した。denormal のテスト値は f32::from_bits(0x0040_0000) のようなビットパターン指定か f32::MIN_POSITIVE * 0.5 で生成する必要がある。素朴な係数乗算では denormal が +0.0 に丸まり、生成自体ができない（Issue #19）。

評価基盤側にはいくつかの制約があった。multiversion クレートは #[multiversion(targets(...))] による dispatched 経路のみを提供し個別 clone の強制 API を持たないため、等価性 matrix では dispatched 列のみとして扱い、検証網羅性の構造的差として採否の判断記録に残す方針とした（Issue #19）。

criterion の計測運用にも順序の規約がある。estimates.json は実行ごとに上書きされるため「run 実行 → 集約 → 次 run」の順序で計測し、集約 jsonl は LF 改行・末尾改行で生成してフォーマッタのフックに適合させた。全量約 400〜560 ケースは warm_up_time・measurement_time・sample_size の削減で約 20〜30 分に短縮する PoC 用設定を導入した（Issue #19）。

実験ログの監査が exp/ 直下の全 jsonl をイベントスキーマで検証するため、計測データの jsonl は data/ サブディレクトリに配置した。宣言パスからの逸脱として記録されたが、実験ログと計測データが同居する場合の配置規約として再利用できる（Issue #19）。

dev-dep 各クレートの MSRV（fearless_simd 1.89、multiversion 1.86、criterion 1.86、pulp は宣言なしで推移的に約 1.89）がワークスペース MSRV 1.85 を超えるため、MSRV 検証は 1.85 ツールチェーンでの cargo check -p ruviform-math --lib --locked に限定した。Cargo.lock は先に通常側のツールチェーン（当時 1.99.0）で再生成してから 1.85 検証に入る。dev-dep は caret 指定と lockfile を再現性の基準とし、exact pin は不採用とした（Issue #19）。

等価性契約は bit 完全一致（to_bits）または ulp 上限 8 とした。観測値が上限を超えた場合は実装バグか契約不適合として先に調査し、数値を合わせ込む事後緩和はしない。biquad で固定上限の超過が正当と判明した場合のみ、代替契約形（先頭 N フレーム ulp ＋以降相対誤差、またはフレーム長依存上限）への切替を許容する（Issue #19）。

## dispatch 層の閾値ルーティングと固定コスト計測

公開カーネル 8 本は、レベルトークンを第 1 引数に取り、サイズ閾値でスカラー経路へルーティングする。閾値は add<128・scale<64・clamp_unit<256・その他は常時トークン経路という確定値で、根拠コメント付きである。契約違反への応答は debug_assert と release 非 panic（共通プレフィックスへの切詰め・channels クランプ）に統一し、各関数の doc に精度契約（bit 完全一致・ulp 256・ulp 8 ＋有限入力前提・NaN/Inf 契約外）を明記した。BiquadCoeffs・SineCoeffs・SineState は dsp から pub use で昇格した（Issue #53）。

dispatched 経路の軽量カーネルでの劣後を固定コストとノイズに分離する計測では、2 系統の証拠の一致で結論を支持する方法を取った。系統 A は dispatch_probe ベンチ（level_new・dispatch_noop・token_avx2・小サイズ add の 16/32）で dispatch の部品コストを直接測り、系統 B は Δ(N)=dispatched−avx2 を入力長 N の関数として解析する。小 N で安定した正の Δ は固定コスト支配、大 N で符号不定の Δ はメモリ帯域律速域の環境ジッター（±10〜30%）によるノイズ支配と解釈できるため、分散緩和に頼らず結論を導ける。実カーネル経路の固定コストは target_feature シャム経由の非インライン呼出しを含み数 ns 規模になる。計測中に環境負荷の混入があった場合は run を棄却して再計測し、環境負荷をレポートに記録する運用とした（Issue #56）。

## DSP カーネルの prepare/process 分離と明示ノード

ruviform-dsp の window・biquad・fir・fft・resample の 5 カーネルは全て prepare/process 分離 API であり、process が alloc-free であることを assert_no_alloc の計測窓で実測検証する設計である（Issue #75）。

ruviform-audio-analysis の LevelMeter・SpectrumAnalyzer・WaveformPeaks は、process の入力長に決定的規則を持つ。短い入力はゼロ詰め、長い入力は先頭切詰めである。SpectrumAnalyzer::magnitudes() は n/2+1 bin の |z| 振幅を正規化なしで返し、スケールは描画側の責務とした。LevelMeter は加算ではなく直近ブロック値を保持し（バリスティックは非ゴール）、WaveformPeaks のみ非 RT で Vec を返却する（Issue #92）。

denormal flush は DenormalFlush という明示ノードとして実装した。FTZ/DAZ のグローバル設定は採用せず、denormal を ±0 に正規化・normal 値は不変・NaN/Inf は透過・upstream pull を呼出すという 4 ケースをテスト仕様で検証し、assert_no_alloc の計測窓内で 0 alloc を確認した（Issue #54）。

## 画素合成・写像カーネルの数値設計

CompositeOver・Crossfade は、呼出し毎に opacity/t を is_finite() と [0,1] で検証し、round(v·255) で Q8 量子化する。Rgba8 の合成は sa/b/oa/oc を全て整数 round-half-up で計算し、b は画素毎に 1 回だけ計算して oa/oc が再利用する。oc≤255 が構造的に保証される自己無撞着設計である。RgbaF16 は同一式を f32 ドメインで評価し、RNE の binary16 丸めを行う。w==0/tq==0 は検証通過後・書込み前に early return し、共有 dst 拒否は opacity=0 でも先行する（Issue #97）。

AffineTransform は Rect/RectMap パラメータを取り、map.src の非空・src フレーム内包含を i64 拡幅で検証する（違反は InvalidValue("src rect")）。写像演算（2·offset·map.src.w 等）は i128/u128 で実施し、セル中心最近傍サンプリングを行う。map.dst の空・キャンバス非交差はエラーとせず全画素 transparent black という定義済み結果とし、矩形外も transparent black で充填する。dst メタデータは src から伝播する（Issue #97）。

f16 コーデック関数（f64_to_f16_bits/f16_bits_to_f64）は複製せず pub(crate) へ格上げして共有した。同等コーデックの複製は DRY 違反のため不採用である。依存は ruviform-video-core・ruviform-media-core のみとし、RT クレートや engine への依存追加は禁止した。assert_no_alloc はこの設計では導入せず、rt_process_* の alloc-free はレビュー担保とし、機械検出は後続 Issue に委ねた。process の in-place dst・可変引数形状は二相 API からの確定済み逸脱であり、実装側で再検討しない（Issue #97）。

## linear working space と transfer 変換

working space は RgbaF32 ＋ {primaries: Bt2020, transfer: Linear, matrix: Identity, range: Full} と定義し、1.0 = 100 cd/m² の絶対輝度で [0,1] 制限を設けず、負値・1 超を lossless に保持する。alpha は straight パススルーとした。Unspecified の primaries/transfer は Bt709 を既定とし（matrix→Bt601・range→Limited の既定規則の上位拡張）、未知 variant は InvalidValue で拒否する。PixelFormat::RgbaF32 は packed RGBA・binary32 little-endian・1 plane・16 B/px で、#[non_exhaustive] enum への variant 追加のため non-breaking であり、RgbaF32 を知らない既存カーネルは _ arm での graceful 拒否を維持する（Issue #99）。

transfer decode/encode の境界規則は確定済みである。decode は Bt709 v=255→1.0（=100 nits）、toe 部 v<0.081→v/4.5（負入力も定義済み）、Pq v=1.0→100.0・Hlg v=1.0→10.0（それぞれ 10000/1000 nits に対応）、Hlg v=0.5 は S=0.5²/3 の分岐、Pq/Hlg の信号域外は clamp である。encode は Pq/Hlg が信号域 clamp、Bt709/Linear は非 clamp で、FromWorking の Rgba8 出力は round(clamp(v,0,1)·255) の round-half-up で量子化境界 ±1 code とする（Issue #99）。

transfer 変換の内部基盤（pub(crate) の transfer モジュール）は、decode/encode 関数群（Linear・Bt709・Smpte170m・Pq・Hlg）・primaries→BT.2020 結合 3×3 f64 定数テーブル・working-space 定数（L_DS=100 nits・HLG nominal 1000 nits・SDR_SCALE）を持つ。dispatch は slot 解決と fn pointer 表で構成し、transfer_slot を prepare 時に 1 回解決して画素ループから分岐を排除した。全ての math は f64 で評価し、参照定義は「f64 評価→binary32 丸め」、最適化 backend は ≤4 ulp の等価契約とする（Issue #99）。

ToWorking は {Rgba8, RgbaF16, RgbaF32}→RgbaF32 の transfer decode と gamut 変換を融合したパス、FromWorking は ctor で dst primaries/transfer を焼付ける RgbaF32→{Rgba8, RgbaF32} である。identity ctor（Bt2020/Linear）は RgbaF32→Rgba8 の純粋量子化を表現し、shuffle 専用カーネルは持たない。src/dst の ColorInfo 一致検査と working-space 記述の強制は意図的に非検証（consumer 責務）と決められており、ToWorking は RgbaF32→RgbaF32 の identity 入力でも no-op early return せず画素パスを通して working-space の tag 記録を行う（Issue #99）。

両カーネルとも、format・dimensions・共有 dst 排他の全検証を書込み前に完了させ、拒否呼出しで dst が不変であることをピクセル・ColorInfo・pts で断言するテストを持つ。メタデータ伝播テストでは、dst が最初から src の pts を保持していると伝播の検証が tautology 化するため、dst の pts を事前に既知値へ scrub してから伝播を観測する。これは独立レビューの指摘で修正された検証設計上の教訓である（Issue #99）。

実装上の選択として、src/dst format の match を row ループ内に保持する per-row dispatch を採用した。format は loop-invariant だが、分岐のホイスティングは per-format に重複する row ループを生むため、参照実装として分岐予測に委ねる KISS を選択した（最適化 backend は ≤4 ulp 契約の内側で後続自由）。in-loop match は明示 arm ＋ _→Err の防御的構成で、将来の #[non_exhaustive] variant 追加でも panic-free を保つ。FromWorking::process は dst.plane_mut(0) の可変借用前に dst.format() を capture する（Issue #99）。

同じく KISS の判断として、working_space と tone_map の両モジュールは pixel codec helper を同形で mod ローカルに重複実装し、mod 間の pub(crate) 共有を作らなかった。tone_map モジュールの許可依存は transfer モジュールのみという依存境界に従うためである。一時的に未消費の内部定数は #[expect(dead_code)] で残置した。expect は使用者が現れた時点で attribute 自体が警告されるため、除去の自然な着火トリガになる（Issue #99）。

NaN・非有限の扱いは仕様として固定する。quantize8 の NaN→0 fold は golden テストで固定し、非有限伝播（0·∞→NaN mixing を含む）を doc に明記する（Issue #99）。

公開 API の doc には API 単位で RT contract ラベルを付け、ctor は非 RT by convention、process は RT-suitable alloc-free・非ブロッキング・有界と明記する。process の行アクセスは Plane::row/PlaneMut::row_mut のみを使い padding を読み書きしない。alloc の機械検出（assert_no_alloc）は後続 Issue へ defer 済みで、rt_process_* テストは repeatable・src 非破壊の観測検証に留まる（Issue #99）。

実装レビューでは 5 件の確定 finding が修正された。encode_pq(+∞) が ∞/∞=NaN を返す経路には saturate guard を追加し、FROM_BT2020 doc の結合式の誤記を訂正し、pq_eotf の NaN 伝播が非自明である点（f64::max は non-NaN operand 規則で NaN を呑むが分母は伝播する）を明文化し、encode_hlg の exp 分岐を覆う golden（≈0.62962）を追加し、往復テストの bound 5e-4 の根拠（H.273 の toe/power 対が knot 周辺で約 2.5·10⁻⁴ の不連続を持つ）を追記した（Issue #99）。

## ToneMapSdr の EETF 実装

ToneMapSdr は HDR→SDR 輝度写像として BT.2390-8 §5.4 の EETF を逐語実装した。BT.2020 luma Y=0.2627R+0.6780G+0.0593B を pq_eotf_inv で信号域へ変換して正規化し、E≤ks は identity・E>ks は Hermite 補間、black lift E''=b(1−E')⁴+E' を適用して、出力は pq_eotf(E''·e_src)/100 で working scale に戻し、RGB を Y_out/Y_src 比率でスケールする。alpha はパススルー、パラメータは src.color().hdr から呼出し毎に導出する（Issue #99）。

L_HS の導出優先順位は mastering_display.max_luminance（/10_000→nits・>0）→ content_light_level.max_cll（>0）→ 既定 1000 nits である。L_min（black lift）は min_luminance が max_luminance 未満の整合値のみ採用し、absent・逆転は b=0 とする。min=0 は整合値として採用され、b=pq_eotf_inv(0)/e_src≈9.7e-7（literal の 0 ではない）となる境界を golden で固定した（Issue #99）。

境界条件の裁定として、ks≥1（L_HS≤L_DS=100 nits の SDR 等価 source）は全画素 passthrough の defined non-compression とした。「peak 内のみ passthrough」という代替解釈は設計文面と非整合として却下されている。no-op early return ではなく画素パスを通して dst の ColorInfo 記録も実行し、宣言 peak を大きく超える画素も verbatim コピーで bit-exact とする。Y_src≤0 は比率スケール不能のため画素を verbatim コピーし、NaN luma は <=0.0 を fail して式経路へ進み伝播する。hdr は post-loop で dst から消費し、下流での二重 tone-map を防ぐ（Issue #99）。

実装上の落とし穴が 2 つ記録されている。1 つは pq_eotf_inv/pq_eotf の係数が nits 域を仮定するため、working scale（1.0 = 100 cd/m²）で直接呼ぶと誤る点で、Y·100 で nits に変換して呼び、出力側は /100 で戻す。もう 1 つは、宣言 L_HS を大きく超える finite 入力でも Hermite spline を非 clamp で外挿するため非有限値を生じ得る点で、契約が bound するのは finite 出力のみと module doc に明記した。src.color().hdr は信用入力であり、誤タグの HDR metadata は誤った L_HS/L_min 導出に直結する。L_DS=100 nits 固定・ターゲットパラメータ化・stage 分割カーネルは非ゴールである（Issue #99）。

## IccTransform の feature 隔離と決定性の境界

IccTransform・IccIntent は feature icc 配下に閉じ込め、optional dep として moxcms 0.9.1 を default-features=false + lut + extended_range で使用する。IccIntent は #[non_exhaustive] の公開 enum で Perceptual・RelativeColorimetric の 2 variant を持つ。ctor で profile parse と executor を事前構築し（失敗は InvalidValue）、process は per-pixel の 1 要素スライス呼出しで alloc-free とし、Rgba8/RgbaF32 の任意の端点組合せを受理する。moxcms の型は公開 API に露出させず Error::InvalidValue に map し、dst.color は {Unspecified×4, Identity, Full, hdr:None} に落とす（ICC 変換では CICP を保証できないため）。alpha は straight パススルーである（Issue #99）。

依存追加の正当化として、moxcms 0.9.1 は pure-Rust の 3 クレートツリー（moxcms + num-traits + pxfm）で、MSRV 1.89 に適合し、default-features=false ではクレート側の #![forbid(unsafe_code)] が有効になる。却下した代替は lcms2（C FFI で unsafe 境界・システム依存）、qcms（機能制約・保守状況）、自前 ICC 実装（mft/mAB LUT や A2B/B2A の再実装はスコープ外）である。in_place・options・SIMD 系は有効化せず、特に any_to_any は選択される executor が transform 毎に alloc するため alloc-free 契約を破るとして明示的に off とした。prefer_fixed_point:false（参照精度を Q2.13 fixed-point に落とさない）・allow_extended_range_rgb_xyz:true（[0,1] 外を保持）・allow_use_cicp_transfer:true を設定する（Issue #99）。

独立レビューでは 2 件の P1 修正があった。1 つは非 RGB color-space profile の受理で、Layout::Rgba と CMYK profile の組合せでは channel 3 が K インクとして消費され alpha-as-K の誤動作を生じ得たため、ctor に color_space==DataColorSpace::Rgb の両端検証を追加し、偽装 fixture の拒否テストを追加した。もう 1 つは等価性契約の文言で、「ホスト非依存・決定的」は虚偽だった。エンジンが aarch64 NEON baseline と target_feature="fma" の x86/x86_64 で fused FMA を選択するため、bit 出力は build 構成間で last-ulp の差を持ち得る。文言を「同一 build 構成内で決定的」へ訂正し、literal golden 4 件を cfg でゲートして非ゲートの behavioral twin を追加した（Issue #99）。

fixture は pin 版エンジンの built-in profile を encode() して生成した 3 件（srgb・display_p3・gray_gamma22）を同梱し、#[ignore] の再生成スクリプトを recipe として残す。決定的 golden に実 profile バイト列が必要であり、.gitattributes で *.icc を binary 指定して行末変換を抑止した。実効 API が flat チャネルスライス &[f32]/&mut [f32] を取るため、設計の &[[f32;4]] スライス記載からの deviation として記録したが、1 quad = 4 channels として意味は同値で、per-pixel 1 呼出し・行バッファなし・alloc-free の構造保証は維持されている（Issue #99）。

feature 経路は別の検証レーンが必要である。cargo test --workspace は default feature のみを対象とするため、feature icc 経路は構造的に検証対象外になる。対策として feature 付きの test・lint タスクを追加し、CI に icc-verify ジョブを設置した。feature-off の依存ツリーに moxcms が出ないことを cargo tree で確認し、ベース経路の無退行を担保する（Issue #99）。

## golden oracle と RgbaF32 拡張の精度契約

working space・tone map の golden/ulp-bound oracle は、定義式を f64 で評価した参照に対する bound 比較とし、厳密一致 golden は使わない。プラットフォームの powf 差を吸収するためである。IccTransform のみ pin 版エンジンのスカラー executor 出力への bit-exact とし、literal golden vector は x86_64 非 FMA baseline（CI の icc-verify lane）に pin して cfg でゲートする。intent の区別は A2B0≠A2B1・B2A0≠B2A1 の mft2 tag を持つ fixture で検証し、mft1/mft2/mAB 各 tag 型を通る fixture で構造を網羅し、欠落 intent tag の拒否も固定した（Issue #99、fixture 拡張は Issue #125）。

等価性契約の数値は、参照実装（f64 評価→binary32 丸め）に対して最適化 backend がチャネル毎 ≤4 ulp（binary32、ToWorking/FromWorking/ToneMapSdr）、composite 拡張は参照 f32 評価と ≤2 ulp、Rgba8 decode 入力は厳密・Rgba8 出力は量子化境界の ±1 code 許容、Y_src≤0 の passthrough 経路は厳密一致である。契約は有限値の出力誤差のみを規定する。テスト helper の assert_ulp は sign-magnitude の bit 距離で ulp を測るため同一符号の比較にのみ有効であり、cross-zero の golden を追加する場合は ulp 計測法の見直しが必要と記録されている（Issue #99）。

既存 CompositeOver・Crossfade・AffineTransform への RgbaF32 arm 追加は、RgbaF16 パスと同式を f32 ドメインで評価し binary16 丸めを適用しない（≤2 ulp 契約）。composite で oa<2⁻¹²⁶（f32::MIN_POSITIVE）を cut としたのは、f16 規約の oa<2⁻¹⁴ cut の domain-relative な対応物である。cut の根拠が「storage domain の subnormal 領域で除算が ill-conditioned」という点だからで、literal 2⁻¹⁴ なら normal binary32 まで blank して表現域を縮めてしまう。divergence window [2⁻¹²⁶, 2⁻¹⁴) を pin するテストで解釈を固定した。AffineTransform の arm は format gate 追加のみで、画素演算を持たない opaque byte 複写のため NaN payload・-0.0・subnormal を含め bit-exact である（Issue #99）。

test-rt の一致件数は test result: 行の passed 合計で計測する（実績として 107 件・27 バイナリ）。cargo test rt_ は substring filter のため rt_ 完全一致 prefix 以外も合法に一致するが、契約どおり workspace 全体の一致数をそのまま記録する。RT 検査は実一致件数で証明し、0 件一致のスキップを成功と偽らない。Full Gate は計画の最終段で 1 回のみ実施し全ログを保存する（Issue #99）。

## AudioLens 比較ハーネスと scalar-first 移植規則

移植元ライブラリ（AudioLens）との性能・構造比較は、再利用可能な手順で行った。比較対象は凍結 baseline の worktree を使い、凍結ソースが現行 main と byte-identical であることを diff -q で確認する。これにより「現行カーネル」と「baseline」の双方の突合を兼ねる。測定は両クレートを path 依存だけで参照する独立した scratch crate（比較対象リポジトリを改変しない）で行い、Instant の per-call 中央値（warmup 1 回 + 5〜200 rep）と counting global allocator で alloc/bytes を実測した。±20% 程度の揺れを許容し桁の大小関係のみを読む caveat 付きで、自己完結関数は use 行差し替えの verbatim 移植で比較した（Issue #150・#152・#153、baseline は AudioLens Issue #98 由来）。

ruviform-numeric は sequential scalar・FMA 非融合・ISA 非依存の bit-exact を参照実装として採用し、エラーは panic-free Result、workspace は全て明示的な *Workspace 型とする契約を持つ。将来の SIMD 化は Deferred 項目であり、実施時は scalar 実装を oracle にした等価性検証を行うことが設計に記録されている。対比として、移植元は pulp x86-v3 SIMD・TLS dispatch・assert! panic 契約である（Issue #150）。

構造的優位は「plan 側への事前確保と明示 workspace」で実現された。移植元が呼出し毎に HashMap 遅延構築・Vec 確保・TLS lazy workspace を行うのに対し、ResamplePlan には全位相の正規化係数表を事前確保し、FFT workspace は TLS 依存を排除した FftWorkspace として明示所有（&mut 排他）する。scalar でも resample が 1.8〜1.9 倍逆転したのはこの構造差によるもので、SIMD 差を上回る alloc/走査削減が性能に直結することが実測で裏付けられた（Issue #150）。

## 相関・スペクトル解析カーネルの構造設計

NCC の direct/FFT ルーティング係数 C=8（L·K ≤ 8·n·log2(n) かつ K ≤ 2048）は移植元の校正値を継承した。scalar 化で交差位置が変わり得るため境界形状で direct 分子ループと FFT 経路を実測比較したところ、C_eff=8.0 の境界形状で t_direct=35.6µs vs t_fft=36.9µs（差約 4%）とほぼ一致し、両経路が同程度に遅くなって比が保たれたため係数は変更しなかった。K ≤ 2048 上限は acc ベクトルを L1 に収める保険として維持し、選択を誤っても両経路の出力契約は同一という冗長性も確認された（Issue #152）。

相関 workspace のメモリ削減は 2 点の変更で実現した。template_squares/signal_squares scratch（L+S 要素）を sum_sq_seq/prefix 二乗和で代替して削除し、phat_energy の確保を Full mode plan のみに限定した（移植元は常に bins 分を確保）。(4410,7938) の形状での差 131,560 B は (L+S)×8 + bins×8 と一致し、direct mode では 52.9MB → 17.6MB（約 1/3）まで縮小した。処理呼出しは両実装とも 0 alloc である（Issue #152）。

CrossSpectrum は 1 パスで 4 系列を融合蓄積し、和のまま保持して coherence/H1 を導出する。移植元は pxx/pyy/pxy の 3 系列走査後に mean 化の 4 回目走査を行い、coherence/H1 でさらに走査・確保する inline ループが 3 箇所重複していた。和と mean の形式差は 1/frames の約分により数学的に一致し、実測でも約 1e-16（ulp 級）の差しかなかった。この設計差が 2.46 倍の性能差を生み、psd_norm() の公開により mean 化せず和系列から直接 band 積分する合成も可能になった（Issue #153）。

StftMatrix は frames×fft_size の連続 flat レイアウトとし、stft_into で plan/出力バッファを再利用して 0 alloc を実現した。移植元の stft() はフレーム毎に Vec 2 本を確保して Vec<Vec> を構築し（92 フレームで +129 allocs）、one-shot 同士では scalar FFT 差で 0.93 倍だが、production の「同じ plan で複数信号を処理」経路では 1.16 倍に逆転した。PSD 正規化は移植元の Hann 専用ハードコード（0.375·window）ではなく、係数から Σw² を実計算して任意窓で正しくした（Issue #153）。

意図的な差分として残した設計がある。エラー契約は assert! panic ではなく panic-free Result とし、window=0/hop=0/rate=0/fft_size 非 2 冪/third_octave_bands 非正入力を検証拒否する。STFT 行列の Vec<Vec>→連続 flat 化は API 上の意図的差分として記録されている。fir_ir/fir_model・phase unwrap/group delay は合成可能な演算のみ移行し、編成ロジックは移植元に残す棲み分けとした（Issue #153）。

## FFT 変換長選択と混合基数

CorrelationPlan・OverlapCorrPlan の変換長選択を next_power_of_two から境界以上の最小 {2,3,5}-smooth 長へ変更し、FftPlan/RealFftPlan に {2,3,5,7}-smooth 合成数を受け入れる混合基数 DIF 経路を追加した。根拠は、2 冪境界を僅かに超えた形状で FFT 演算量がほぼ 2 倍に膨らむ問題（S=8,640→16,384 等）であり、支配形状で変換長 42〜49% 削減を達成した。2 冪入力は従来の radix-4 DIF 経路を維持し bit-exact 不変とする additive 設計である（Issue #255）。

radix-7 は相関層の選択集合からのみ除外した。radix-7 を含む候補（n=2,401,000=2³·5³·7⁴）は変換長が 57% にも関わらず nextpow2 より約 49% 遅かった。汎用の小 DFT 段が律速になるためで、FftPlan 自体は radix-7 を受け入れたまま API 一般性を維持する。radix-5 は Winograd 型専用式（実乗算約 8/group）で専用化し、汎用経路に流した初回実装で生じた 20〜29% の速度回帰を解消した（Issue #255）。

この変更で Full-mode 相関の潜在不具合が見つかった。従来記述の境界 n≥S+L−1 は出力契約を実際には保証しておらず、実装の非負ラグ相関 c[k]=Σᵢt[i]·s[(k+i) mod n] では S≤k のラグ域で wrap 項 s[k+i−n] が実データを指す。全返却ラグ k≤S+L−2 を無汚染にする真の境界は n≥S+2L−2 である。nextpow2 の余剰が偶発的に充足していた形状でも trailing lag が混入していた潜在不具合であり、境界を所有する plan 構築層で修正して regression test を追加した（Issue #255）。

RealFftPlan の forward packing にも潜在不具合を発見・修正した。内部複素 FFT が m=n/2 を要求するため n は偶数の smooth 長のみ受付だが、m が奇数のとき mid=m/2 の self-conjugate bin は存在しない。従来コードは mid=m/2 を無条件に自己共役扱いしていたため n=6 や n=7,938 で破綻していた。n/2 が常に 2 冪＝偶数だった従来領域では発現し得なかったが、非 2 冪 smooth 長の受入で顕在化した（Issue #255）。

変換長選択のコストモデルも校正した。最小 smooth 選択が移植元で回帰を起こした（template_len=2,400,000・signal_len=2,400,240・K=241 で変換長が 4,194,304→2,430,000 に縮小し NccDirect→NccValid に反転、NCC が 134.38ms→218.61ms と +63%）。根本原因は n·log2(n) が混合基数経路の per-element コスト増（radix-3/5 専用 butterfly・入力複写・digit-reversal gather・twiddle/置換表のキャッシュ外トラフィック）を反映しないことで、fft_cost_units(n)=a+3b+3c+6·log2(rem)+mem(n)（radix-3/5 段に実測フィット値、非 2 冪 n>1M に 0→10 飽和の mem 項）へ校正した（Issue #265）。

「新旧選択の推定コスト比較」を別ロジックで書く代わりに、候補集合に nextpow2 を常に含めて argmin する形にした。「新選択 ≤ 旧 pow2 推定コスト」が構造的不変条件として自明に成立し、「旧選択で direct だった形状が新選択で遅い FFT 化される」回帰ガードになる。nextpow2 超過候補はコスト関数上必敗のため n≤nextpow2(v) も不変である（Issue #265）。

計測上の教訓として、NccDirect の直接カーネル計測に公開 scalar 曲線 API（ncc_direct_into）を使うとアクセスパターンが異なり約 731ms と全く別の値になった。plan 内部の実カーネル（axpy 形 real[k]+=template[i]·signal[k+i]、K≤2048 で acc が L1 常駐）は約 117ms で、移植元の 134ms と整合する。ベンチも plan 経路で計測する必要がある（Issue #265）。

## bit-exact 移植の API 設計と安定 ID

complex_solve は移植元の csolve を ruviform-numeric::linalg::complex_solve として逐語移植した。bit-exact 置換を目的とし、pivot 選択（max_by total_cmp・同値は最後の index）・消去順序・失敗判定を移植元と同一に保ち、同一入力で同一の解・条件数 proxy・失敗（None）を返す。複素値は (re,im) タプル、複素演算は非融合の a*b±c*d 形で評価順を固定した。非有限入力は拒否せず伝播する。これは移植元に有限性検査がない意図的契約で、least_squares の None 拒否とは異なるため API 精度契約に例外を明記した。条件数 proxy は pivot 絶対値の最大/最小比であり、真のスペクトル条件数ではない。std のみ・num-complex 不採用・non-RT である（Issue #167）。

既存の OverlapCorrPlan/CorrelationPlan は実行時コストモデルで direct/FFT を切替えるため、呼出側が「必ず直算」を契約できず、移植元の直算実装と bit-exact 置換が不能だった。そこで plan/コストモデルを介さない scalar 直算曲線 API（ncc_direct_into・ncc_direct・overlap_ncc_direct_into・overlap_ncc_direct）を新設した。分子・分母とも corr0_parts_seq と同一の逐次評価順で累積し、正規化は Σx²>0 かつ Σy²>0 のときのみ sxy/√(Σx²·Σy²) とする。縮退・NaN エネルギーは 0.0、|lag|≥L は 0.0 である。_into 形は alloc-free で RT-suitable、template エネルギーは 1 回だけ評価して bit-exact を維持する。移植元の IPD sweep は逆方向のため overlap_ncc_direct(y,x) または index 反転で同値写像とし、テストで固定した（Issue #168）。

spec モジュール（centroid・bandwidth・rolloff・flatness・entropy・flux・ols_slope・median・argmax）は移植元から verbatim 移植し、非対称な縮退規則を意図的に維持する。bandwidth のみ total<=0.0 の否定形ゲートで NaN を伝播し（他は total>0.0 の肯定形で 0.0 写像）、entropy は n=1 かつ total>0 で ln(1)=0 の除算により NaN となり、argmax 同率は最後の index（max_by）を返す。rolloff のコメントは "85% energy" だが実体は振幅行の累積であり、fraction を引数化した（移植元は 0.85 固定）。flux/ols_slope は移植元の zip 黙認から Err("length") 検証へ意図的に変更した。median は確保するため non-RT で、bin 域選択・prev 保持・mean_row 蓄積は呼出側ポリシーとした（Issue #169）。

永続契約について、移植元の normalized_pcm cache 鍵が plan 内部パラメータへ直接依存し内部表現の変更で崩れ得る問題に対し、係数表を含まない正準パラメータ値 ResampleParams と決定的 stable ID を提供した。ResampleParams は source_rate/target_rate/up/down/radius/cutoff_bits を f64::to_bits で保持し bit-exact な Eq/Hash を持つ。ID 形式は rvf-resample/1/<src>/<dst>/<up>/<down>/<radius>/<cutoff-hex16> で、CONTRACT_VERSION=1・ALGORITHM_ID="polyphase-blackman-sinc"（移植元の provenance トークンと同一）である。from_stable_id は正準スペルと全フィールド導出整合性を検証する fail-closed とした。ResamplePlan は params+phases 保持へ再構成し、パラメータ導出を ResampleParams に集約して単一正本とした。serde 非依存（std のみ層）のため正準文字列と値型で提供し、params のみの new() は係数表確保が非現実的な極端有理比でも ID を取得可能にする（Issue #170）。

BandFilter の保持スペクトルは、不変共有体 BandSpectrum（Send+Sync・forward FFT 1 回）と呼出毎の排他スクラッチ BandScratch（&mut）に分離した。移植元での 52 task/pair 並列化需要に対応するためで、(channel,band) タスク単位で scratch を供給して入力全長 FFT の再計算を回避する。BandFilter は BandSpectrum+BandScratch の逐次便宜型として再構成し、mask→IFFT→切戻しを二重化せず bit-exact を構造的に保証する。BandScratch は構築元変換長 n を保持し、不一致を InvalidValue("scratch") で拒否する（公開 API の panic-free 契約維持・同一 fft_size 間で使い回し可）。review では所有版 BandSpectrum::band を除去した。scratch 不一致時にゼロ波形を黙って返して誤用を隠すため YAGNI と判断した（Issue #183）。

## 検証プロファイルと RT 検査の権威

bench profile 固有のコンパイル破損の 1 つは、assert_no_alloc 側の feature 設計に由来する。debug_assertions off の bench profile では、assert_no_alloc の disable_release feature が AllocDisabler 型自体を cfg 外にし（assert_no_alloc() 関数は no-op 化して残る）、#[global_allocator] 宣言の参照が解決不能になる。修正は宣言側への #[cfg(debug_assertions)] 付与のみで、ワークスペース内の先例は全てゲート済みであり、1 箇所だけが外れ値だった（Issue #646）。

もう 1 つは、criterion が dev-dep 経由で引く serde_json の impl PartialEq<Value> for f64 無条件実装に由来する。assert_eq!(<f64 式>, []) の空配列が f64 と serde_json::Value の 2 候補で曖昧化して E0282/E0283 となる問題で、右辺を [] as [f64; 0] と型付けして解消した。assert!(x.is_empty()) 形は clippy::assert_is_empty が発火するため使えない。いずれも dev profile では顕在化しない bench profile 固有の依存グラフ由来の問題である（Issue #646）。

bench profile で assert_no_alloc が no-op となるのは crate の設計どおりであり、RT 検査の権威は dev/test profile の rt_ lane にある。test-spec でも「bench profile は計測用途・dev profile は RT 契約検証用途」という profile 別責務を明記し、AllocDisabler が arm されないことを期待動作として固定した。mise の bench/bench-smoke タスクは実計測・スモーク入口のため、--no-run のコンパイル検証は cargo bench --no-run の直接実行が必要である（Issue #646、関連するリスク記述は Issue #150）。

## 計測に基づく描画・データ転送設計

GUI は WebView 非依存コア（ruviform-ui、DemoService + headless demo_cli）と wry+tao adapter（ruviform-ui-webview、webview feature は非 default）の 2 クレートに分離し、workspace 既定経路は headless のまま維持した（check で wry/tao 非含有を実証）。rvf://app→http://rvf.app の scheme 書き換えと bridge JS 注入で adapter を配線し、コアは exec/rt 系のみに依存する（Issue #156）。

データ転送と描画 tier の分離は計測に基づく。binary pull は 1 MiB で約 94〜115 MB/s・16 MiB で約 88〜146 MB/s だったが、64 KiB の小 pull は約 2.9 MB/s で JSON RTT より遅かったため、小値は JSON・バルクのみデータ pull（1 読出上限 16 MiB）の閾値分離を採用した。Canvas2D が約 32k 列で約 8.75 fps まで低下するのに対し WebGL2 は 57.9〜60 fps を維持した計測を受け、DOM/Canvas2D/WebGL2/NativeSurface を描画 tier として分離した（Issue #156）。

ruviform-viz は内部値を全て f64・NaN=欠測とし、表示丸め（f32/u8 変換）は Quantization/DbRange/ColorMap/encode の display 境界に限定した。PagedBytes は DataSource 形状（len()+read(offset,buf)）と一致させ、薄い adapter でデータ pull 配線を可能にした。BoundedCache は汎用提供のみで tile() との自動連携は YAGNI、WaveformLod はオンデマンド再構築前提である。依存は media-core/numeric/exec のみ・全 API 非 RT・unsafe なしである（Issue #157）。

viz の計測では、zoom-out は LOD ピラミッドから・zoom-in はソース直読みから取得するハイブリッド構成で、応答時間が要求 bin 数のみに比例することを確認した。バイナリ転送（RVST 形式）は f64 が 283.6µs/393KB・f32 が 96.0µs/197KB・u8 が 392.1µs/49KB で、u8 は dB 正規化と DbRange 写像を含み f32 より遅いが payload は 1/8 になる。帯域重視なら u8・CPU 重視なら f32 が有利である。LOD ピラミッドの常駐サイズは O(frames/256 × levels × channels × 8B) で元データの約 1.6% と見積もられた。GPU は描画バックエンドを保持しない設計のため計測対象外である（Issue #157）。

## closeout と実績記述の運用規約

closeout task の共通規約として、実績 IR・coverage・task/plan status は immutable な実 log（commit history・各 task IR・gate log）のレビューからのみ記述し、新規メトリクスの再計算や再検証はしない。検証未実施のまま status を進めず、全 task 完了が commit 済み証跡で確認できた時点でのみ plan を completed へ遷移する。verified-by の test-spec の "confirmed" は、schema の valid status に confirmed が存在しないため approved へ写像する（Issue #99、複数 Issue 共通）。

GitHub の merge は lead の authoritative な操作のみで行い、文書チェックリストで代行せず、記録文書にも PR merged・issue closed の主張を書かない。doc 監査は新規文書作成後にもう一度実行し、gate 時点の監査ログと最終監査を区別して、exit 0・findings 0 と graph complete を確認する（Issue #99、複数 Issue 共通）。

実施中の規約として、task 間では狭い検証のみを行い full suite の繰返しはせず、Full Gate は計画最終段で 1 回のみ実施する。commit は論理ビルド可能単位で 1 substep/1 commit とし、red 専用 commit は置かない（Issue #99）。

## 未確定・保留事項

- SIMD 採否 ADR を作成するタスクのチェックボックスが全て未チェックのままだが、親 plan 側の対応項目は完了済みで plan status は completed である。task 文書の status 更新漏れか実際に未完了かは、入手した記録群からは判定できない（Issue #19）。
- SIMD PoC 計画の最終検証スイートに相当する phase のチェックボックスも未チェックのままである。オーケストレータ側の最終検証への委譲設計と読めるが、実績の有無は不明である（Issue #19）。
- Issue #646 の「dev profile で assert_eq! の曖昧化が顕在化しなかった正確な rustc/クレートグラフ上の機序」は未解明と記録されており、bench profile でのみ再現確認済みである。
- 等価性 matrix の Done When は観測 max ulp 集計値の記録を要求するが、具体的な ulp 数値は本知見の対象文書には記載がなく、実験ログ・採否 ADR 側にあると推測される（Issue #19）。
- Issue #167 の bit-exact 性は移植元リポジトリとの直接比較が未実施で、構造的一致のみに依存する。実値差分の確認は移植元側移行時の follow-up として記録されている。
- mistagged src と hdr が同時存在する組合せは未テストである（tag 非検証と hdr 消費は別テストで各々検証済み）。alpha passthrough は assert_ulp 比較だが、passthrough 契約上は to_bits 厳密 assert の方が忠実という指摘もある（Issue #99）。
- Issue #99 の task 単位の実装記録や plan/task 文書・Issue #125 の LUT fixture coverage 詳細は記録の収集範囲外で未読であり、Issue #99 の記述は IR 経由の間接的な把握に基づく。Issue #255/#265 には対応する plan/task 文書が見つかっておらず、別名で存在する可能性を排除できない。
- AudioLens 側の凍結 baseline worktree と現行 main のソースは本ワークスペース外のため直接確認できず、byte-identical 確認の記述は文書の記載どおりに採用した（Issue #150・#152・#153）。
- dispatched-overhead 計測の生データ（3 run）は集計値のみで個別セルの検証はしていない（Issue #56）。viz のメモリ計測は BoundedCache の予算拒否テストと LOD 常駐サイズの理論値（約 1.6%）のみで、実メモリ使用量の直接計測はスコープ外と明記されている（Issue #157）。

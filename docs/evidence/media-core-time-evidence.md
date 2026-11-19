---
id: "EVI-iop5R6AgPIbe9SNOEByMY0"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "データモデル・時間・トランスポートの実行記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/18", "https://github.com/scarecrow173/ruviform/issues/26", "https://github.com/scarecrow173/ruviform/issues/33", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/58", "https://github.com/scarecrow173/ruviform/issues/59", "https://github.com/scarecrow173/ruviform/issues/90", "https://github.com/scarecrow173/ruviform/issues/91", "https://github.com/scarecrow173/ruviform/issues/96", "https://github.com/scarecrow173/ruviform/issues/97", "https://github.com/scarecrow173/ruviform/issues/99", "https://github.com/scarecrow173/ruviform/issues/121", "https://github.com/scarecrow173/ruviform/issues/171", "https://github.com/scarecrow173/ruviform/issues/173", "https://github.com/scarecrow173/ruviform/issues/189", "https://github.com/scarecrow173/ruviform/issues/190", "https://github.com/scarecrow173/ruviform/issues/195", "https://github.com/scarecrow173/ruviform/issues/196"]
  informs: []
  explains: []
  supersedes: []
---

# データモデル・時間・トランスポートの実行記録

本書は、データモデル・時間表現・トランスポート周辺の実装に対して行われた検証と実行の結果を記録する。各記述には出典の Issue 番号を付記し、記載する数値は実行時に観測されたものである。実行されなかった検査、先送りされた事項、結論が確認できなかった事項は末尾の「記録の範囲と限界」に列挙する。

## media-core・video-core のデータモデル検証

media-core と video-core の実装は、検証スイートの全項目を通過したうえで PR #39 として main にマージされた (Issue #18)。実施環境は Windows x64・Git Bash で、fmt-check・check・lint・test-rt・bench・prek が全て PASS し、`cargo test --workspace` は最終 80 単体テスト + 50 doctest = 130 合格・0 失敗だった（初回計測は 124）。`mise run test-rt` は意図した 10 テストのみを選択した。`cargo metadata --no-deps` では、全 6 パッケージが `publish: []`（公開禁止）であること、依存辺が audio-core→media-core と video-core→media-core のみで cycle がないことを確認した。merge commit は `d041ba9`（feature `bc81838`、Closes #18）である。実装作業はサブエージェントへの分割で行われ、本記録は完了後に遡ってまとめられたものである。

クレート個別の検証値も記録されている (Issue #18)。media-core は `cargo test -p ruviform-media-core` で初回 23 単体 + 22 doctest、レビュー修正後の最終値で 24 単体 + 23 doctest だった。`rt_` フィルタでは `rt_cross_base_ordering`・`rt_convert_exact_and_rounding`・`rt_checked_arithmetic`・`rt_drop_frame_roundtrip` の 4 テストが選択・合格した。video-core は初回 20 単体 + 6 doctest、最終 23 単体 + 6 doctest で、`rt_` 3 テストのみが選択された。video-core では substring フィルタに `view_rejects_short_backing` が誤選択されたため、`view_rejects_undersized_backing` へ改名して解消した。両クレートとも clippy `-D warnings` で 0 warnings、fmt-check 差分なしだった。

独立レビューのサイクルも数値つきで残っている (Issue #18)。Reviewer A（4件）と Reviewer B（5件）の指摘を Validator が検証し（8 CONFIRMED・A4 UNVERIFIED）、Judge が A1/A2/A3 を blocking と判定した。修正 wave 7 件ののち ACCEPT WITH NON-BLOCKING NOTES でマージに至った。blocking 指摘の修正例として、`PlaneMut` 全ゲッタのカバレッジ欠落（A1）、`new_owned` の `isize::MAX` panic（C2）、`stride >= row_bytes` 不変条件の所有レイヤーへの移動（C4）が記録されている。

公開 enum の方針では、allow 属性を撤去したうえで lint 強制が成立することが確認された (Issue #41)。`#![allow(clippy::exhaustive_enums)]` の撤去後も `cargo clippy --workspace --all-targets -- -D warnings` は 0 warnings であり、ポリシーが lint 強制状態にあることを確認した。`cargo test --workspace` は 80 単体 + 50 doctest = 130 合格・0 失敗。grep 監査で `#[non_exhaustive]` が全 11 enum に付与されていること、`exhaustive_enums` の allow が 0 件であること、兄弟クレートに exhaustive `match` が存在しないことを再確認した（`match self` は同一クレート内では合法）。兄弟クレートが non_exhaustive enum に依存したままビルド・テストを通過したことが、wildcard-match 互換の実証として記録されている。

## video-proc の骨格導入と文書同期

video-proc の骨格導入では、追加直後のワークスペース検証が全項目 PASS した (Issue #26)。`mise run check` は exit 0、`cargo test -p ruviform-video-proc` は 2 unit + 1 doctest 合格、clippy は 0 warnings だった。`cargo metadata` で members に video-proc が加わっていること、deps は video-core のみであること、全 7 パッケージが `publish: []` であることを確認し、fmt-check 差分なし、docs/IR 監査 0 findings だった。実施環境は Windows x64・Git Bash である。

設計文書との同期も併せて行われた (Issue #26)。baseline 採否表の「色変換・HDR の対応範囲」行を 要PoC から 決定 へ移し、根拠列に Issue #26 と ADR D2-a/D3-a を記録した。動画データモデル節の 要PoC 箇条から同項目を外したが、個別 codec/container と decoder zero-copy は 要PoC のまま残した。計画文書中の stale 前提（primitive `f16` が MSRV 1.85 で使用可）には、windows-msvc で不成立であることとソフトウェアコーデック採用の実績を注記として追記した。歴史的記録は書き換えず追記のみとする方針で、AGENTS.md を 6 から 7 クレートへ、README.md を 5 箇所同期した。

この同期は doc-only の変更（コード不変）だったが、総合検証が実行され全項目 PASS した (Issue #26)。`mise run test` は 129 unit + 62 doctest = 191 合格・0 失敗、`test-rt` は 18 件を選択（audio-core 3・media-core 4・video-core 3・video-proc 8）し、video-proc の意図した 8 件を含み誤検出はなかった。`cargo metadata` で video-proc の依存は media-core と video-core のみ（`ruviform-rt` や同順位依存はなし）、全 7 パッケージ `publish: []`、prek 全フック Passed、docs audit と IR audit はともに 0 findings（65/24 ファイル）だった。

video-core の契約文書化も doc-only で実施された (Issue #26)。`cargo test -p ruviform-video-core` が 23 unit + 6 doctest 合格、clippy 0 warnings、`cargo doc` は生成成功で 0 warnings（既存の 4 warnings を解消）だった。RT ラベルの grep 監査で借用・共有・確保系 API のラベル網羅を確認し、doc 監査・IR 監査ともに 0 findings だった。introspection API は追加せず、`plane_mut() == None` を観測点とする方針どおりに実施された。

## video-proc カーネル群の検証

YUV↔RGB 変換の実装では、golden vector による検証が行われた (Issue #26)。YUV→RGB 実装時点で video-proc は 15 unit + 3 doctest 合格、`rt_` フィルタで意図した 2 件のみが選択され、clippy は 0 warnings だった。golden vector は 3 matrix × 2 range × 4 画素を H.273 の重みから独立に導出し、導出式をコメントに記載した。T3b 完了時点では 41 unit + 9 doctest が全合格し、`rt_ -- --list` により rgb_to_yuv 2 件 + rgba_f16 4 件 + yuv_to_rgb 2 件 = 8 件のみが選択されることを確認した。`cargo metadata` で video-proc の依存は media-core と video-core のみで `publish: []`、fmt-check・check・lint・test・test-rt・bench・prek・doc 監査の全エントリが PASS した。

コンポジットとアフィン変換では、仕様に対応する網羅的なテストが追加された (Issue #97)。composite.rs に 26 件の inline tests と doctest 4 件を追加し、golden・`oc` 境界の trap vector・`oa == 0` の両フォーマット・`opacity = 0` の byte-identical early return・検証拒否群・denormal/非有限伝播・`rt_process_*` 4 件を対象とした。affine.rs には 18 件の inline tests と doctest 2 件を追加し、恒等写像の bit-exact・sub-rect crop・1→2/2→4 複製・4→2 縮小・負 `dst.x` のクリップ・`map.src` 検証拒否 6 パターン・format/共有 backing 拒否・`rt_process_*` 2 件を対象とした。test-spec の全ケースがパスし、`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス、`doc_status.js audit` findings 0、依存は video-core + media-core のみだった。

RgbaF32 arm の拡張では、受理集合の拡大のみで既存経路を不変に保つことが検証された (Issue #99)。`CompositeOver`・`Crossfade`・`AffineTransform` に `RgbaF32` arm を追加し、既存の `Rgba8`/`RgbaF16` 経路・Q8 量子化・メタデータ規約は不変とした。等価性は参照実装（f64→binary32）に対し ≤ 2 ulp、affine は写像・複写のみで bit-exact を維持した。検証値は、rgba-f32-composite-affine-arms で video-proc lib tests 171/171・clippy clean・`test-rt` の workspace 107 件実選択のうち本タスクの `rt_process_*_f32_*` 6 件を含むこと、working-space-kernels で lib 130/130・doctests 19/19・`rt_process_*` 4 件実選択、tone-map-sdr で lib 147/147・doctests 21/21・`rt_process_tone_map_sdr_*` 2 件実選択であり、いずれも `mise run test`/`lint`/`test-rt` が exit 0 だった。

## SIMD dispatch の検証

SIMD dispatch の導入では、検出結果と公開面の両方を確認した (Issue #53)。`mise run check`/`lint`/`fmt-check` がパスし、`SimdToken::detect()` が本機（Windows x64）で `"avx2"` を返すことを単体テストまたは doctest で確認した。公開 API に fearless_simd の型を露出させないことも、受入条件として確認済みである。

## 時間算術・タイムコード・コールバック時刻

time.rs の算術には早期飽和のバグがあり、回帰テストで再現を確認してから修正した (Issue #41)。修正前に `range_end_delta_wider_than_i64` と `checked_add_sub_delta_wider_than_i64` が予測どおりの誤値（`end()` が -1 を返す等）で FAIL することを確認し、そのうえで修正を適用した。修正後は `cargo test -p ruviform-media-core` で 30 unit + 24 doctest 全 pass、clippy `-D warnings` 0 warnings、fmt-check 差分なし、workspace の check/lint/test 全緑だった。変更した API の呼び出し側が time.rs の外に存在しないことも確認済みである。

Timecode API では、`TimeBase` 引数から `FrameRate` 引数への破壊的置換が行われた (Issue #58)。`from_media_time`/`from_media_time_drop` の `frame_rate` 引数を置き換え、ADR の副次決定により旧シグネチャは併設しない。nominal grid 計算 `(2*num+den)/(2*den)` は `frame_rate.nominal_fps()` の呼び出しに移譲して規則の二重実装を避け、`elapsed_frames` 内部は `numerator()`/`denominator()` を使う厳密な i128/u128 演算と負時刻の Err 拒否を維持した。検証は `cargo test -p ruviform-media-core` 全テスト・doctest 合格、clippy 0 warnings、fmt-check 差分なしだった。

最終タスクでは文書側の整合を行った (Issue #58)。AGENTS.md の型列挙への `FrameRate` 追記と doc コメント整合を実施し、`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全合格、`prek run --all-files` 合格、`audit_docs.js --type all --json` で findings 0 を確認して spec の status を implemented に更新し、PR（Closes #58）を作成した。bench は整数演算のみの変更で計量対象外としてスキップする判断が記録されている。

オーディオコールバックの変換では、チャンク単位の timestamp 合成を実装した (Issue #59)。`convert.rs::write_interleaved` に `sample_rate: u32` 引数を追加し、frame > 0 かつ sample_rate > 0 のとき `offset_ns = frame * 1_000_000_000 / sample_rate` を u128 で計算して `i64::try_from` で飽和させ、`MediaDuration::new(offset_ns, DEVICE_CLOCK_TIMEBASE)` を `info.timestamp.callback`/`playback` に `checked_add` で合成する。`checked_add` 失敗時は元の instant を保持し（playback を None 化しない）、frame == 0 または sample_rate == 0 のときは既存の info をそのまま渡す。検証は `mise run check`/`lint` のパスと T3 テストで行った。

## 音楽時間モデルと ABI

media-core に music モジュールが実装され、テスト仕様の全ケースを通過した (Issue #90)。`crates/ruviform-media-core/src/music.rs` を実装して lib.rs で re-export し、test-spec 全ケースと `mise run test`/`test-rt` のパスを受入条件として達成した。文書クローズのタスクでは、検証列全パス・doc 監査 findings 0・graph complete を受入条件として全て確認してクローズした。

音楽時間を ABI に載せる方式は、先に設計検証で ABI 非破壊であることを確認している (Issue #33)。`TransportInfo`/`TransportSnapshot` の `reserved: [u64; 2]` 拡張予約と `Option<MusicalTimeInfo>` フィールド追加という方式が ABI を破壊しないことを、plugin-core と media-core の `transport.rs` で確認し、CLAP/VST3 への写像規則と丸め損失の許容範囲が設計に規定されていることをもってクローズした。

両スナップショット型への `musical: Option<MusicalTimeInfo>` 追加は、この方式どおりに実装された (Issue #91)。reserved は温存し Copy を維持した。CLAP アダプタでは `beats_to_ticks` と `adapt_transport` の拡張、VST3 では `ppq_to_ticks`/`musical_from_context`/transport 統合を実装し、双方がフラグゲート規則どおり Some/None を返すことを確認した。設計文書の beattime 係数は 65536 から実 ABI の 2³¹ へ修正した。検証は `adapt_transport_maps_musical_fields`/`_gates_and_unset_forms`（CLAP）、`musical_from_context_maps_valid_fields`/`_gates_and_unset_forms`/`transport_combines_media_musical`（VST3）、および `mise run check`/`lint` と該当クレートの test 全パスで行った。クローズ時には検証列（fmt-check/check/lint/test/test-rt）全パス、doc 監査でチェーン起因 findings 0、`prek run --all-files` 全パスを確認した。

music モジュールの内部モデルは、その後、分解能非依存の表現へ移行された (Issue #189)。固定 960 の整数 Tick から、正規化有理数 `BeatPosition` を正本とするモデルへ移行し、`Ppqn` は 1..=u16::MAX の validated 型、`TickPosition` は tick と分解能を束縛する型、`TempoMap` は Beat 基準の正確な有理数変換となった。`MusicalTimeInfo`（24 byte の ABI/wire 表現）と PPQN 定数は不変に保ったため、plugin 系 consumer は無変更でコンパイル・テストを通過した。検証は fmt-check、check（全 29 クレート）、lint、test（music::tests 20 件 + doctest）、test-rt（`rt_queries_no_alloc` を含む）、lint-l1-real-plugin、test-l1-real-plugin（実規格 L1 E2E 3 件 × CLAP/VST3）、plugin-clap lib 47 件（`beats_to_ticks_clap_factor_golden` を含む）が全て PASS した（Rust 1.99.0）。

PPQN golden 対応表では、Ruviform が責任を持つ項目は全て PASS した (Issue #189)。PPQN-01/02/06/08/10 は Synvelis 側の所有（アプリの project/undo/UI/migration）で本リポジトリの範囲外である。Ruviform 該当分の結果は以下のとおりで、`MusicalTimeInfo` が Copy・24 byte で不変であることもあわせて固定された。

- PPQN-03 可変 PPQN 構成：`ppqn_validates`・`variable_ppqn_media_roundtrip` で 4 分解能の tick→media→tick が ≤ 1 tick
- PPQN-04 非格子 1/7 beat の保持
- PPQN-05 異分解能 Tick の誤混合防止：`TickPosition` は Ord 非実装で、跨り比較は InvalidValue
- PPQN-07 Tempo 境界と負 pre-roll の往復
- PPQN-09 CLAP 2³¹ golden：i64 全域で wrap なし
- PPQN-11 旧 960↔新 960 の golden 一致
- PPQN-12 overflow・分母上限・RT no-alloc

## transport span スケジューラの契約検証

transport span スケジューラは、契約の検証・スケジューラ実装・bridge PoC の三段で進められた (Issue #190)。契約タスク（T-1）では `cargo check -p ruviform-media-core`、`mise run lint`（clippy `-D warnings`、全 workspace）、`mise run test`、`mise run test-rt`（`rt_run_callback_no_alloc` を含む全 `rt_` テスト）が全て pass した。Graph の RT generation/plan 所有は Issue #195 の扱いであり、本契約の generation 刻印は plan builder の入力に留まる旨がリスクとして記録されている。

スケジューラ本体（T-2）では、RT 安全な配信規則が実装された (Issue #190)。`TransportScheduler::run_callback`（RT・no-alloc・no-blocking）は、callback 内の due pending apply と loop wrap を境界として半開 span に分割し、`SpanSink::span` へ配信する。Exact モードでは不正境界の apply を Rejected receipt と fault で応答し、wrap を UnrepresentableLoopWrap fault と deliverable=false の span で表す。Degraded モードでは次の合法境界へ遅延して DeferredApply を記録し、budget 超過は ScheduleFault として規定どおり扱う。検証は 15 件の unit tests（lifecycle・loop wrap・多周 occurrence・stable order・stale generation・Exact reject・Degraded defer・preflight 拒否・restart vs seek・pending・span budget・`rt_run_callback_no_alloc`）が全 pass、lint/test/test-rt も pass だった。

bridge と MockDriver PoC（T-3）では、プロセス境界での契約を統合テストで確認した (Issue #190)。`crates/ruviform-plugin-core/src/transport_bridge.rs` に `contract_from_process_config`/`calls_for_span`/`call_transport_info`/`build_call_event_list` を、`tests/transport_scheduler.rs` に MockDriver（SpanSink impl）と RecordingProcessor を実装した。bridge unit tests 5 件（config 写像・offset 位置・loop range・process 相対 offset・窓外 skip）と integration tests 14 件が全 pass した。integration の対象は以下のとおりである。

- TT-008 lifecycle、TT-025 loop wrap（[0,100)+[100,256)）
- TT-026a Exact の min 違反 call 非発行と UnrepresentableLoopWrap、TT-026b Degraded の DeferredApply
- TT-027 capability が Unknown/Declared で非 supported、TT-028 process 相対 offset
- TT-029 多周 occurrence・zero-frame flush・L0/L1、stale generation 非配信
- TT-009 restart vs seek、TT-012 span budget fault、TT-010 stable order、TT-030 Exact の非 quantize と Degraded の明示

`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` と verify-ci（fmt-check/check-ci/lint-ci/test-ci）も全て pass した (Issue #190)。

## edit クレートのタイムライン評価

edit クレートのタイムライン評価は、14 件のテストで仕様を固定した (Issue #96)。eval.rs のテスト対象は、空タイムライン・ギャップの半開境界・オーバーラップ優先規則・disabled・`source_time` 写像・cross-base の `at`・`TimeOverflow`・決定性などで、`evaluate` の doctest とあわせて test-spec 全ケースがパスした。`rt_` テストは非 RT クレートのため設けない判断が記録されている。`mise run fmt-check`/`check`/`lint`/`test` が全パス、`doc_status.js audit` findings 0、依存は media-core のみと確認済みである。

配置の整理も行われた (Issue #96)。crate README を新設し、crates/README.md に「利用側層（engine と同順位・engine ではない）」の節を追加して、索引を 18 から 19 クレートへ更新した。

## plugin-host の診断レコード

plugin-host の診断タスクは status `done` で完了した (Issue #121)。受入条件は、診断レコード・`StderrSnapshot`・`DiagRegistry`・`DiagnosticQuery` の型と単体テスト、ring バッファの不変条件 `total_read = len + dropped` と watermark 3 値識別の単体テスト pin、`crate::` パスを含まないこと、である。検証は `mise run check` と `mise exec -- cargo test -p ruviform-plugin-host`（diag 単体テストを含む）の exit 0 で確認した。

## UI データレジストリと webview の解放確認

UI のデータレジストリでは、解放と上限規則を 9 件の契約テストで固定した (Issue #171)。`crates/ruviform-ui/tests/data_registry.rs` の対象は、unregister 後の再登録、clear、取得済み Arc の unregister 後読了（deferred-free）、既定無制限の後方互換、LRU 追い出しと read による recency 更新、同名再登録の recency 更新、view op の連番パターンを模した 1000 件連続登録が上限 8 に有界化されること、`Some(0)` の即追い出し縮退、`strong_count` による Arc drop の pin である。webview 側では `data_unregister_and_eviction_yield_404` で、解放・追い出し後の `rvf://data` が 404 になることを wire 境界で確認した。fmt/check/lint/test/build と ui-webview 個別ゲートは全て exit 0 だった。merge 時に Issue #173 側と data.rs・README の文書競合が発生し、双方の記述を残して解消し、Cargo.lock は `cargo generate-lockfile` で再生成した。

ui-webview の scheme 変更でも全ゲートが PASS した (Issue #173)。`mise run check-ui-webview`/`lint-ui-webview`/`test-ui-webview`（scheme 6 tests に `offset == total` の空 200 境界と 400/404 status pin を追加）と、workspace の check/lint/test/fmt が全て exit 0 だった。`cargo test --workspace` の初回実行では ruviform-plugin-host の `l1::video::tests` 2 件が `waiter registered` panic で fail したが、単体・クレート単位・全体の再実行では全て pass した。本変更（ui-webview scheme + docs）と無関係な既知 flake 候補として記録されている。

## 記録の範囲と限界

本書に記録した検証のうち、実行されなかった、先送りされた、または結論が確認できなかったものを以下に列挙する。

- Issue #26 のコメント本文の最終確認は、`gh` CLI が利用できない環境で未実施のままである。受入条件の認識は Issue #41 トリアージ由来の二次情報であり、一次確認は未了である (Issue #26)。
- `PixelFormat::Yuv444p.chroma_subsampling()` が `Some((1,1))` を返す統一グリッド解釈は文書化済みだが、`None` が意図の場合は `plane_dimensions` に特殊ケースが必要であり、最終確定は記録されていない (Issue #18)。
- `mise run test-rt` の substring フィルタ精度改善（厳密プレフィックスマッチ化）は別 Issue の検討事項として残っており、対応状況は不明である。
- `FrameRate`/`TimeBase` 分離・`Timecode` 拡張・丸め誤差累積ポリシーを扱う新規 Issue の起票は計画に記録されたが、実際の Issue 番号は記録されていない (Issue #41)。
- bench は整数演算のみの変更で計量対象外としてスキップする判断が記録されており、実行されていない (Issue #58)。
- Cargo.toml のメンバーコメントが不完全（media-core 依存の未列挙）である点は「別途報告事項」として据え置かれ、修正済みかは不明である (Issue #26)。
- `max_sources` の具体値は caller 依存で本クレートは機構のみを提供するため、consumer 側で採用される値は本リポジトリの記録からは確認できない (Issue #171)。
- `InBlockTransportSupport::Verified` 経路の実装状況は、Issue #196 側の記録を参照しないと確定できない (Issue #190)。
- ruviform-plugin-host の `l1::video::tests` の flaky fail は既知 flake 候補として記録されており、原因の特定は行われていない (Issue #173)。
- Issue #189・#190 周辺の個別タスク文書、Issue #59 の先行・後続タスク文書、Issue #190 の計画文書など、収集対象に含まれなかった関連文書は未読であり、それらにのみ記録された検証方針やスコープ決定は本書に反映されていない可能性がある。Issue #190 の実装記録はタスク分割で残されており、統合記録の有無も確認できていない。

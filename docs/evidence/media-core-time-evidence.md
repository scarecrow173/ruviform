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
  source: ["https://github.com/scarecrow173/ruviform/issues/18", "https://github.com/scarecrow173/ruviform/issues/26", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/96", "https://github.com/scarecrow173/ruviform/issues/97", "https://github.com/scarecrow173/ruviform/issues/99", "https://github.com/scarecrow173/ruviform/issues/121", "https://github.com/scarecrow173/ruviform/issues/171", "https://github.com/scarecrow173/ruviform/issues/173", "https://github.com/scarecrow173/ruviform/issues/190", "https://github.com/scarecrow173/ruviform/issues/196"]
  informs: []
  explains: []
  supersedes: []
---

# データモデル・時間・トランスポートの実行記録

本書は、データモデル・時間・トランスポート周辺の実装で行われた検証と実行の結果を記録する。記録の対象は、後に削除された過去の実装記録から抽出したものであり、本書はそれらの記録に依存せず単独で読めるように構成している。各記述には出典の Issue 番号を付記し、観測された事実と解釈を区別して記す。解釈を含む箇所にはその旨を明記する。

## media-core・video-core のデータモデル検証

media-core と video-core の実装は PR #39 として main にマージされ、検証スイートは全てパスした (Issue #18)。Windows x64・Git Bash 環境で fmt-check/check/lint/test-rt/bench/prek が全て PASS し、`cargo test --workspace` は最終 80 単体 + 50 doctest = 130 合格・0 失敗だった（初回計測は 124）。`mise run test-rt` は意図した 10 テストのみを選択した。`cargo metadata --no-deps` で全 6 パッケージが `publish: []`（公開禁止）であること、依存辺が audio-core→media-core・video-core→media-core のみで cycle がないことを確認した。merge commit は `d041ba9`（feature `bc81838`、Closes #18）である。

独立レビューのサイクルも記録されている (Issue #18)。Reviewer A（4件）と Reviewer B（5件）の指摘を Validator が検証し（8 CONFIRMED・A4 UNVERIFIED）、Judge が A1/A2/A3 を blocking と判定した。修正 wave 7 件ののち ACCEPT WITH NON-BLOCKING NOTES でマージに至った。これはレビュー手順の記録であり、各判定の解釈は記録どおりである。

クレート個別の検証値も記録されている (Issue #18)。media-core は `cargo test -p ruviform-media-core` で初回 23 単体 + 22 doctest、修正 wave 後の最終値 24 単体 + 23 doctest だった。`rt_` フィルタでは `rt_cross_base_ordering`・`rt_convert_exact_and_rounding`・`rt_checked_arithmetic`・`rt_drop_frame_roundtrip` の 4 テストが選択・合格した。video-core は初回 20 単体 + 6 doctest、最終 23 単体 + 6 doctest で、`rt_` 3 テストのみが選択された。両者とも clippy `-D warnings` で 0 warnings、fmt-check 差分なしだった。

レビュー指摘に基づく修正例が記録されている (Issue #18)。`PlaneMut` 全ゲッタのカバレッジ欠落（A1）、`new_owned` の `isize::MAX` panic（C2）、`stride >= row_bytes` 不変条件を所有レイヤーへ移動（C4）が挙げられている。また video-core では `view_rejects_short_backing` の誤選択を `view_rejects_undersized_backing` への改名で解消した。

enum の公開方針については、allow 属性の撤去と lint 強制が確認された (Issue #41)。`#![allow(clippy::exhaustive_enums)]` 撤去後も `cargo clippy --workspace --all-targets -- -D warnings` が 0 warnings で、ポリシーが lint 強制状態であることが確認された。`cargo test --workspace` は 80 単体 + 50 doctest = 130 合格・0 失敗だった。grep 監査で `#[non_exhaustive]` が全 11 enum に存在し、`exhaustive_enums` の allow が 0 件、兄弟クレートの exhaustive `match` が非存在であることを再確認した。兄弟クレートが non_exhaustive enum に依存したままビルド・テストを通過したことが、wildcard-match 互換の実証として記録されている。

## video-proc の骨格導入と docs 同期

video-proc 骨格の導入時、`mise run check` が exit 0、`cargo test -p ruviform-video-proc` が 2 unit + 1 doctest 合格、clippy 0 warnings だった (Issue #26)。`cargo metadata` で members に video-proc、deps は video-core のみ、全 7 パッケージ `publish: []`、fmt-check 差分なし、docs/IR 監査 0 findings が確認された。実施環境は Windows x64・Git Bash と記録されている。

docs 同期では、baseline 採否表の「色変換・HDR の対応範囲」行を 要PoC→決定 へ移動し、根拠列に Issue #26 と ADR D2-a/D3-a を記録した (Issue #26)。動画データモデル節の 要PoC 箇条から同項目を外し、個別 codec/container・decoder zero-copy は 要PoC のまま残した。plan D-2・Q3 の stale 前提（primitive `f16` が MSRV 1.85 で使用可）には、windows-msvc 不成立とソフトウェアコーデック採用の実績注記を追記した。歴史的記録は書き換えず追記のみとする方針で、AGENTS.md を 6→7 クレートへ、README.md を 5 箇所同期した。

doc-only 変更（コード不変）に対する総合検証も全 PASS だった (Issue #26)。`mise run test` は 129 unit + 62 doctest = 191 合格 0 失敗、`test-rt` は 18 件選択（audio-core 3・media-core 4・video-core 3・video-proc 8）で video-proc の意図した 8 件を含み誤検出なしだった。`cargo metadata` で video-proc deps = media-core + video-core のみ（`ruviform-rt`・同順位依存なし）、全 7 パッケージ `publish: []`、prek 全フック Passed、docs audit・IR audit とも 0 findings（65/24 files）だった。

video-core の契約文書化も doc-only で実施された (Issue #26)。`cargo test -p ruviform-video-core` が 23 unit + 6 doctest 合格、clippy 0 warnings、`cargo doc` 生成成功で 0 warnings（pre-existing 4 warnings を解消）だった。RT ラベル grep 監査で借用・共有・確保系 API のラベル網羅を確認し、doc 監査・IR 監査とも 0 findings だった。introspection API は追加せず `plane_mut()==None` を観測点とする方針どおりに実施された。

## YUV↔RGB カーネルとコンポジット・アフィンの検証

YUV→RGB 実装時の検証では、video-proc が 15 unit + 3 doctest 合格、`rt_` フィルタで意図した 2 件のみ選択、clippy 0 warnings だった (Issue #26)。golden vector は 3 matrix × 2 range × 4 画素で H.273 重みから独立導出し、導出式をコメントに記載した。T3b 完了時は 41 unit + 9 doctest 全合格で、`rt_ -- --list` により rgb_to_yuv 2 + rgba_f16 4 + yuv_to_rgb 2 = 8 件のみが選択された。`cargo metadata` で video-proc の依存は media-core + video-core のみ・`publish: []` で、全検証エントリ（fmt-check/check/lint/test/test-rt/bench/prek/doc 監査）が PASS だった。

composite/affine の検証では、composite.rs に 26 件の inline tests と doctest 4 件、affine.rs に 18 件の inline tests と doctest 2 件が追加され、test-spec 全ケースがパスした (Issue #97)。composite のテスト対象には golden・`oc` 境界 trap vector・`oa == 0` の両フォーマット・`opacity = 0` の byte-identical early return・検証拒否群・denormal/非有限伝播・`rt_process_*` 4 件が含まれる。affine のテスト対象には恒等写像 bit-exact・sub-rect crop・1→2/2→4 複製・4→2 縮小・負 `dst.x` クリップ・`map.src` 検証拒否 6 パターン・format/共有 backing 拒否・`rt_process_*` 2 件が含まれる。`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が全パス、`doc_status.js audit` findings 0、依存は video-core + media-core のみだった。

RgbaF32 arm の拡張では、`CompositeOver`/`Crossfade`/`AffineTransform` に `RgbaF32` arm を追加し、受理集合の拡張のみで既存 `Rgba8`/`RgbaF16` 経路・Q8 量子化・メタデータ規約は不変とされた (Issue #99)。等価性は参照実装（f64→binary32）に対し ≤ 2 ulp、affine は写像・複写のみで bit-exact を維持した。検証値は、rgba-f32-composite-affine-arms で video-proc lib tests 171/171・clippy clean・`test-rt` の workspace 107 件実選択のうち本 task の `rt_process_*_f32_*` 6 件を含むこと、working-space-kernels で lib 130/130・doctests 19/19・`rt_process_*` 4 件実選択、tone-map-sdr で lib 147/147・doctests 21/21・`rt_process_tone_map_sdr_*` 2 件実選択であり、全て `mise run test`/`lint`/`test-rt` exit 0 だった。

## edit クレートのタイムライン評価

edit クレートの eval.rs には 14 件のテストと `evaluate` の doctest があり、test-spec 全ケースがパスした (Issue #96)。テスト対象は空タイムライン・ギャップ半開境界・オーバーラップ優先規則・disabled・`source_time` 写像・cross-base `at`・`TimeOverflow`・決定性などである。`rt_` テストは非 RT クレートのため設けない判断が記録されている。crate README を新設し、crates/README.md に「利用側層（engine と同順位・engine ではない）」節を追加して索引を 18→19 クレートに更新した。`mise run fmt-check`/`check`/`lint`/`test` が全パス、`doc_status.js audit` findings 0、依存は media-core のみ確認済みだった。

## transport scheduler の契約テスト

transport scheduler の検証では、bridge unit tests 5 件と `tests/transport_scheduler.rs` の integration tests 14 件が全 pass した (Issue #190)。unit tests の対象は config 写像・offset 位置・loop range・process 相対 offset・窓外 skip である。integration tests は TT-008 lifecycle、TT-025 loop wrap、TT-026a/b Exact/Degraded の min 違反 call 非発行と receipt、TT-027 capability 非 supported、TT-028 process 相対 offset、TT-029 多周 occurrence・zero-frame flush・L0/L1、stale generation 非配信、TT-009 restart vs seek、TT-012 span budget fault、TT-010 stable order、TT-030 Exact 非 quantize vs Degraded 明示をカバーした。`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` および verify-ci（fmt-check/check-ci/lint-ci/test-ci）も全 pass だった。

## plugin-host の診断レコード

plugin-host の診断タスク（T5）は status `done` で完了した (Issue #121)。Done When は診断レコード・`StderrSnapshot`・`DiagRegistry`・`DiagnosticQuery` の型と単体テスト、ring 不変条件 `total_read = len + dropped` と watermark 3 値識別の単体テスト pin、`crate::` パス非含有を要求した。Verification は `mise run check` と `mise exec -- cargo test -p ruviform-plugin-host`（diag 単体テスト含む）の exit 0 である。

## UI データレジストリと webview の解放確認

UI のデータレジストリでは、`crates/ruviform-ui/tests/data_registry.rs` に 9 テストが追加され、全ゲートが PASS した (Issue #171)。テスト対象は unregister→再登録、clear、取得済み Arc の unregister 後読了（deferred-free）、既定無制限の後方互換、LRU 追い出しと read による recency 更新、同名再登録の recency 更新、AL GUI の view op 連番パターンを模した 1000 件連続登録が上限 8 に有界化されること、`Some(0)` の即追い出し縮退、`strong_count` による Arc drop pin である。webview 側は `data_unregister_and_eviction_yield_404` で、解放・追い出し後の `rvf://data` が 404 になることを wire 境界で確認した。fmt/check/lint/test/build と ui-webview 個別ゲートは全て exit 0 だった。merge 時に issue-173 と data.rs・README の doc 競合が発生し、双方記述を残して解消し、Cargo.lock は `cargo generate-lockfile` で再生成した。

ui-webview の scheme 変更でも全ゲートが PASS した (Issue #173)。`mise run check-ui-webview`/`lint-ui-webview`/`test-ui-webview`（scheme 6 tests、`offset == total` 空 200 境界と 400/404 status pin を追加）および workspace の check/lint/test/fmt が全て exit 0 だった。`cargo test --workspace` の初回実行では ruviform-plugin-host の `l1::video::tests` 2 件が `waiter registered` panic で flaky fail したが、単体・crate 単位・全体の再実行で全 pass し、本変更（ui-webview scheme + docs）と無関係な既知 flake 候補として記録された。

## SIMD dispatch の検証

SIMD dispatch タスクでは `mise run check`/`lint`/`fmt-check` がパスし、`SimdToken::detect()` が本機（Windows x64）で `"avx2"` を返すことを単体テストまたは doctest で確認した (Issue #53)。公開 API への fearless_simd 型非露出も Done When で確認された。

## 記録の範囲と限界

本書に記録された検証のうち、実行されなかった、または先送りされたものを以下に列挙する。

Issue #26 のコメント本文の最終確認は、gh CLI 不在環境で未実施のままである。AC 認識は #41 トリアージ由来の二次情報であり、一次確認は未了である (Issue #26)。

Issue #18 由来の open question として、`PixelFormat::Yuv444p.chroma_subsampling()` が `Some((1,1))` を返す統一グリッド解釈は文書化済みだが、`None` が意図の場合は `plane_dimensions` の特殊ケースが必要であり、最終確定は未記録である (Issue #18)。

`mise run test-rt` の substring フィルタ精度改善（厳密プレフィックスマッチ化）は、複数文書で別 Issue 検討事項として残っており、対応状況は不明である。

Issue #41 T4 の新規 Issue 起票（`FrameRate`/`TimeBase` 分離・`Timecode` 拡張・丸め誤差累積ポリシー）は、起票番号が plan 実施時に確定する記述だが、実際の番号は記録されていない (Issue #41)。

Issue #26 の Cargo.toml メンバーコメントの不完全さ（media-core 依存未列挙）は「別途報告事項」として据え置かれており、修正済みかは不明である (Issue #26)。

Issue #171 の `max_sources` 具体値は caller 依存で本 crate は機構のみ提供するため、audio-lens #105 側で採用される値は本リポジトリの文書からは不明である (Issue #171)。

Issue #190 の `InBlockTransportSupport::Verified` 経路の実装状況は、Issue #196 側の文書を参照しないと確定できない (Issue #190, #196)。

Issue #173 の ruviform-plugin-host `l1::video::tests` の flaky fail は既知 flake 候補として記録され、原因の特定は行われていない (Issue #173)。

また、関連する計画・タスク文書の一部は収集範囲外として未読であり、本書の記録は抽出済みの実行記録のみに基づく。

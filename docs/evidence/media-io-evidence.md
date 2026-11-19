---
id: "EVI-yAonXqmLLdSA736n2aGV8I"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "メディア I/O・ファイル形式の検証記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/18", "https://github.com/scarecrow173/ruviform/issues/20", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/191", "https://github.com/scarecrow173/ruviform/issues/197", "https://github.com/scarecrow173/ruviform/issues/223", "https://github.com/scarecrow173/ruviform/issues/233", "https://github.com/scarecrow173/ruviform/issues/244", "https://github.com/scarecrow173/ruviform/issues/249", "https://github.com/scarecrow173/ruviform/issues/251", "https://github.com/scarecrow173/ruviform/issues/284", "https://github.com/scarecrow173/ruviform/issues/285", "https://github.com/scarecrow173/ruviform/issues/286", "https://github.com/scarecrow173/ruviform/issues/288", "https://github.com/scarecrow173/ruviform/issues/295", "https://github.com/scarecrow173/ruviform/issues/296", "https://github.com/scarecrow173/ruviform/issues/297", "https://github.com/scarecrow173/ruviform/issues/298", "https://github.com/scarecrow173/ruviform/issues/299", "https://github.com/scarecrow173/ruviform/issues/300", "https://github.com/scarecrow173/ruviform/issues/301", "https://github.com/scarecrow173/ruviform/issues/302", "https://github.com/scarecrow173/ruviform/issues/305", "https://github.com/scarecrow173/ruviform/issues/306", "https://github.com/scarecrow173/ruviform/issues/307", "https://github.com/scarecrow173/ruviform/issues/398", "https://github.com/scarecrow173/ruviform/issues/400", "https://github.com/scarecrow173/ruviform/issues/409", "https://github.com/scarecrow173/ruviform/issues/468", "https://github.com/scarecrow173/ruviform/issues/721", "https://github.com/scarecrow173/ruviform/issues/725"]
  informs: []
  explains: []
  supersedes: []
---

# メディア I/O・ファイル形式の検証記録

本書は、過去の実装・検証記録から抽出した、メディア I/O とファイル形式に関する検証・実行結果をまとめた記録である。元の記録群は後に削除されたため、本書はそれらに依存せず単独で読めるように記述した。各事実は出所の Issue 番号で示す。測定値や観測された挙動などの実測事実と、そこからの解釈は区別し、解釈は「解釈」と明示する。実行されなかった検査は NotRun・Deferred・unverified としてそのまま記録し、Pass へ繰り上げない。

## audio-core のデータモデルとリアルタイム無割当の検証

audio-core のデータモデルは、Windows x64・rust 1.99.0 の環境で fmt・check・clippy（`-D warnings`）・test がすべて green となった。最終値は unit 30 件に doctest 18 件で、`rt_` 接頭辞の 3 テスト（view channel access・interleaved roundtrip via views・SampleIndex↔MediaTime 変換）が `assert_no_alloc` 系で選択され合格した。レビューで指摘されたアクセサ coverage の欠落・`isize::MAX` での panic 境界・`version()` の RT ラベル欠落は修正 wave で解消した (Issue #18)。

## 受入条件の検証写像とドキュメント監査

Issue #20 の検証は、受入条件 AC1–AC11 をコマンドへ写像する形で設計された。型純度は `cargo doc --no-deps` と `symphonia`・`pub use` の grep 監査で、依存分離は `cargo tree -p ruviform-media-io` で、end-to-end は LFS を使わない in-memory の WAV 合成テストで確認する。panic 表現は `unimplemented!`・`unreachable!`・`panic!` の grep で、`unsafe` はコード内 0 件の grep で、公開 enum の拡張余地は `exhaustive_enums` lint ゲートで、パッケージ公開設定は `cargo metadata` による `publish: []` 確認でそれぞれ担保する構成である (Issue #20)。

この写像に沿った検証スイートは全項目 PASS した。media-io は unit 40 件と doctest 19 件、io-symphonia は unit 11 件・e2e 7 件・doctest 5 件、workspace test は 220 件で失敗 0 だった。test-rt は意図した `rt_` テスト 10 件のみを選択し、新規 2 クレートのテストはすべてフィルタアウトされた。bench は no-op を維持した。`cargo metadata` で全 8 パッケージが `publish: []` であり依存辺が設計どおりであること、コード内の unsafe が 0 件であること、公開 enum 4 件がすべて `#[non_exhaustive]` であることを確認した。ドキュメント監査は初回に 10 件の不整合（索引 status 不整合 5・derived-by 相互参照 4・verifies 不正ターゲット 1）を検出し、修正後 0 件となった。修正 wave 後の clippy・test 再実行もすべて pass した (Issue #20)。

## cpal アダプタのコンパイルと依存境界

cpal アダプタは cpal 0.18.2 に解決され、edition 2024・rust-version 1.85 の条件下でコンパイルに成功した（実 toolchain は 1.99.0）。`cargo test -p ruviform-audio-device` で単体 39 件・mock 結合 4 件・doctest 6 件がすべて pass し、`mise run test-rt` では `rt_output_callback_path_allocates_nothing` が `assert_no_alloc` 配下で選択・合格した。これは view 構成・`copy_to_interleaved`・user callback・timestamp 写像の無割当を機械検証したものである。`cargo metadata` で cpal が当該クレートの `[dependencies]` のみに存在し、adapter への逆依存がゼロであることを確認した。実機挙動は後の実機検証へ引き継がれた (Issue #23)。

## WAV バックエンドの境界契約と往復・失敗行列

WAV バックエンドの sink 側境界は、`MediaSink` を `Write+Seek+Send` に `sync()`（fsync 級）を加えた trait として設計し、`File`・`Cursor`・`&mut T`・`Box<T>` に実装した。`StagedOutput` は Windows では `MOVEFILE_REPLACE_EXISTING` 経由の `fs::rename` で上書きする。`ErrorKind` に conflict variant がないため dest-exists は `InvalidValue` へ写像する（文書化済み）。sink を取らない `make_muxer(streams)` は `Unsupported` を返し、実経路は `open_muxer(sink, streams)` とした。encode 側の `native_format` は `Fact::NotDeclared` とした (Issue #191)。

検証は `tests/roundtrip.rs` 9 件と `tests/contract.rs` 19 件で実施し、すべて pass した。往復試験では 44.1k/48k/96k × mono/stereo/5.1 × S16/S24/S32/F32 の write→reopen を `ruviform-io-symphonia` で decode し、長さ・チャンネル数・サンプル値を確認した。契約試験では capability honesty・unsupported codec/container/sample-format/multi-stream の拒否・stream-id/partial-payload 検証・sink の write/finalize error の `Io` 分類・staged の drop/abort/no-overwrite/disk-full/replace で旧宛先が byte 単位で不変であることを確認した。`cargo test -p ruviform-media-io` は unit 53 件と doctest 20 件で、`mise run fmt-check`・`check`・`lint`・`test`・`verify-ci` と `cargo test --workspace --locked` もすべて green だった。最終検証前に `git merge origin/main` を実施して `Cargo.lock` を再生成している (Issue #191)。

## media-reader のスイートと consumer への影響分類

media-reader の suite は 29 件すべて pass した（3 lib・13 prefetch・11 reader・2 rt_fill）。symphonia boundary は 3 件、AVI は 2 件で、既存 suite は変更なしのまますべて green だった。`test-rt` では `rt_consumer_fill_no_alloc` と `rt_consumer_underrun_fill_no_alloc` が `assert_no_alloc` により RT fill 経路の無割当を証明し、`verify-ci`（CI profile）も pass した。consumer impact の分類では、Synvelis への影響は additive API で強制変更を伴わないものと判定された（記録には impacted(positive) と no impact の両表現が残る）。AudioLens はコードレビューで互換と判断したが、consumer repo が read-only で local-path の `cargo check` を実行できなかったため unverified と記録し、RD-10 の compile-equivalence も unverified のまま残した (Issue #197)。

## MSRV lane の検証範囲と監査証跡の取り扱い

MSRV lane は test 実行のみで clippy を含まない。これは test-ofx-msrv・test-ui-test-msrv と同じ範囲であり、MSRV clippy は情報 lane とする方針に従う。この方針の下で実走した io 系 MSRV lane は rust@1.89.0 の 6 クレート一括と deflate 補完で 680 passed/0 failed/1 ignored となり、ignored の 1 件は io-file の os_publish.rs にある既存 gate が spawn する `#[ignore]` helper だった。workspace test は 3580 passed/0 failed/7 ignored で、DAG/RT 監査（`cargo tree -i`・`cargo tree -p`・`cargo metadata` の parse assert）はすべて Pass した。監査証跡は実行時点の snapshot であり、依存 DAG が変わった場合は再実行が必要である (Issue #223)。

## Fault の計上先と assertion の修正

`rd02_reopen_at_seek_failure_faults_then_reanchors` の初回実測で、pre-boundary の新 generation `Fault` は consumer が generation を未 adopt のため `fault_events` ではなく `stale_drops` に計上されることが判明した。これは `step()` semantics と整合する正しい挙動と判断し、実装を変えずに assertion を `stale_drops >= 1` に修正した。review-protocol は 2 reviewer・13 candidate で verdict ACCEPT WITH NON-BLOCKING NOTES（P0/P1 なし）だった (Issue #233)。

## ring 満杯の決定的再現と counter semantic の変更

ring 満杯時に `End` が再 queue される flake は、`block_count:1` と 4 回の失敗 reopen で 9-slot ring を決定的に飽和させて再現した。`SeededSource` は `Arc<AtomicBool>` ゲートで pipeline 構築後にのみ arm し、probe は unarmed のままとした。検証行列 AC1–AC8 は seek・reopen_at・reopen 失敗時の `faults.unseekable`・`fault_events` の観測・`pump` の `Idle` 返却・discard drain 列を対応づけ、意味的なテスト更新は `rd02_reopen_at_seek_failure_faults_then_reanchors` の 1 件に限定した (Issue #244)。

failure path で `stale_drops` が減少し `fault_events`/per-kind `faults` が増加する意図的な変更が行われた。in-repo では `rd02_reopen_at_seek_failure_faults_then_reanchors` の 1 件を更新し、synvelis は quiesce-drain→seek workaround が引き続き有効で no impact、audio-lens は repo 不在で unknown と記録された。review で新規 wart を 1 件検出した。`PumpOutcome::Ended` が `End` marker emit の 1 pump 手前でも返るもので、`fill_block` の EOF arm が `stream_ended` を立てて `Ended` を返すが `End` は次 pump の step 2 で合成される。documented `Idle`/`Ended` まで pump する manual driver が検出時点で停止すると `End{gen}` が strand し、consumer は `eof` を観測できない。spawn `run()` は pump を継続するため実害は manual driver のみで、別 issue 候補として記録した (Issue #244)。

## consumer impact の確定（synvelis・audio-lens）

synvelis は impacted(positive・対応不要) と確定した。`driver.rs:248-257` の `while pump()==Pushed` bounded loop は非 `Pushed` で一律 break するため変更不要で、旧 semantics では終端 cycle で `End` が strand し得た hazard が消える。`Ended` 後の seek で `End{old}` が `Boundary{new}` より先に観測されるが、generation 付き marker で honest に処理される。audio-lens は local worktree で `ruviform-media-reader` 非依存を確認し no impact に確定し、Issue #244 の unknown を解消した。TDD で新規 2 テストが実装修正前に red を確認した後、`cargo test -p ruviform-media-reader`（prefetch 27・reader 11・rt_fill 3）と workspace `mise run test`（115 suite）がすべて green だった (Issue #249)。

## 入力ストリーム構築の実装と headless 検証

`open_input_stream` を placeholder の `Unsupported` から実 cpal input stream 構築へ置き換え、`build_input_stream::<f32>`→interleaved→planar `AudioBufferView`→`InputDataCallback` を RT 契約（open 時一括確保・callback 内無割当/無ブロック）どおりに実装した。`mise run fmt-check`・`check`・`lint`・`test`・`test-rt` はすべて pass し、audio-device は lib 59・integration 12・doctest 6 だった。`rt_input_callback_path_allocates_nothing` が `InputCallbackState::handle` 全経路の無割当を機械検証した。MSRV `rust@1.89.0` でもすべて pass した。この時点で実機の検証行はすべて NotRun として記録した (Issue #251)。

OS 別の CI 証拠では、run `43428172820` の job/log 証拠として verify ubuntu が 59+4+12+0+6 tests、windows が 75+4+12+0+6 tests（`wasapi_input::tests` 16 件を含む）、verify-macos は `coreaudio-rs` link を確認、rt-verify・msrv-verify（audio-device 非対象）・ci-summary verdict=PASS を記録した。`real_input_e2e.rs` は feature 未設定で 0 tests compile されることを log で確認した。OS×Backend×Device×Case 行列で Windows/WASAPI/CI runner は headless Pass となった。macOS/Linux 実機・物理 USB 抜去・実 OS 権限拒否・xrun 誘発は NotRun/Deferred のままであり、Pass に繰り上げていない。過去の PR で得られた Windows/WASAPI/Audient iD4 の実機証拠は、CI headless の証拠と区別して引用した。CI artifact retention 30 日を考慮し、job URL・test 集計値・SHA を記録本文へ転記した (Issue #251)。

## Audient iD4 実機での capture 計測

Windows 11・Audient iD4（`Analogue 1/2`、driver `audientusbaudio_sc`）・WASAPI・44.1kHz F32 Stereo・`BufferSize::Fixed(441)` で `mise run test-audio-real-input` が 4/4 Pass した。capture の計測値は callbacks=192・total frames=84672・frames/callback=441 固定・order violations=0 だった。実効 input latency は min=10 038 500ns・max=10 117 300ns・mean=10 059 264ns、frame_position は `Some` 192/192 で gaps=0、`clock_domain`=MONOTONIC、`input_latency()`=`Some(StreamLatency{frames:0, coverage:Unknown, provenance:Carried})`、xrun=0 だった。duplex では input/output とも `ClockDomainId(2)` で `same_clock_domain`=true、UNKNOWN rejection=false だった。format mismatch 7 行はすべて `Err(UnsupportedConfig)` だった。device loss は物理 USB 抜去の代わりに既定 endpoint 切替（`IPolicyConfig::SetDefaultEndpoint`、管理者権限不要）で `StreamInvalidated`+`ClockDiscontinuity` を実観測した。物理経路は対話式 lane が案内済みで NotRun のままである (Issue #251)。

## output latency の実測と権限拒否の parity

Audient iD4・WASAPI・44.1kHz F32 で `real_output_stream_observes_latency` を新設し、約 1 秒の無音再生で実測した。callbacks=102・total frames=44629・playback timestamp `some=102 none=0` で、実効 output latency（`playback - callback`）は min=0ns・max=31 972 789ns・mean=30 905 917ns（non-positive=3）だった。解釈として、mean 約 30.9ms は WASAPI shared-mode buffer 深さ（約 3 period ≈ 30ms @ 44.1kHz/441 frame period）に整合し、playback が callback より約 3 period 先の DAC instant を指すことを裏付ける。`output_latency()` は pre-start/post-stop とも `Ok(None)` を返し、cpal portable 経路の非 populate に対する正直な応答である。`wasapi_input.rs` の `map_win_error` に `E_ACCESSDENIED`→`Backend("permission_denied")` を追加し cpal 経路と parity を確保した。headless test で両 context を検証したが実機再現は NotRun である。初回 lane で 1 度だけ `UnsupportedConfig` の一過性 open 失敗を観測したが、以後再現せず環境依存事象として記録した (Issue #251)。

## docs-only slice の検証と証跡管理

Issue #284 はドキュメントのみの slice で、関連する consumer 側 Issue の状態を 2026-11-09 時点の snapshot として記録した（AudioLens #139/#143 は CLOSED、Synvelis #82・#25 は CLOSED、#84 は OPEN）。接続表の「役割」記述が将来陳腐化する可能性は test-spec の Risk に明記した。consumer 互換契約の spec は status が `proposed` のため語彙参照の正本として接続するが、本 slice の承認対象にはしなかった。既存の broken relation は範囲外として修正せず、証拠として報告する運用とした (Issue #284)。

Issue #286 もドキュメントのみの slice で、Verification Notes に未監査のまま残っていた 11 項目を、設計正本の各節・アーカイブクレートの実装・試験コード・E2E カバレッジ・マニフェスト・関連 Issue の設計・ゲート行列へ一対一に照合した。11 項目すべてが実在する証拠へ解決できたため完了とし、`tar_reader` 固有の欠落は確認されず試験追加は行わなかった。外部 gate（consumer 互換・consumer 側 Issue・OS 別 lane・性能・fuzz）は NotRun または委譲のまま残し、Issue は close しなかった (Issue #286)。

## 圧縮フォーマットの契約試験の棚卸しと追加

圧縮フォーマットでは、まず src 全 8 ファイルを approved 設計正本へ写像照合し、公開 API・状態機械・error 語彙・budget 意味論に設計不整合がないことを確認した。「既存実装の変更は設計不整合が確認できた最小限」という条件に従い、変更は test と docs に限定した。既存試験を契約へ写像した棚卸しの結果、不足していた失敗経路・flush/finish/Drop の区別・連結 member 境界・高圧縮率 budget の試験を、encoder 系 R1–R7 と decoder 系 R8–R12 として gzip/deflate/zstd の各系に追加した。失敗注入の fixture（`FailAfter`・`FailFlush`・`RecoveringWriter`・`FailingReader`）は試験共通モジュールに集約した。追加した試験の内訳は、encoder 系が header write 失敗・output budget・flush after finish・失敗 encoder の Drop が trailer/final block/epilogue を書かないこと・`finish` の underlying flush 失敗・連結 member・write after finish、decoder 系が `finish` 冪等・chained decoder の member 境界 resume・read error latch・高圧縮率 decoded budget・`finish` mid-output が `NotExhausted` でないこと、である (Issue #285)。

実走では `test-io-compression` の 5 組合せが exit 0・45 test-result ok、`cargo test --all-features` が 149+9=158 pass、`test-io-msrv`（rust@1.89.0）も pass した。docs-chain の完了基準（specs/designs/test-specs/plans/tasks の相互 relations 解決・index 再生成・`fmt-check` 通過・監査で新規文書 error 0・差分が docs のみ・PR が `Refs` を含み close しないこと）もすべて満たした。監査で検出された既存の broken relation は別途記録する運用とした (Issue #285)。

## IO-06 gate matrix の実走と infra flake の記録

IO-06 の gate 検証は、Windows 11 x64・NTFS 環境で全 18 lane を `mise run` で実走した。計測値は io-file 315・io-structured 251・io-compression 330・io-e2e 77・workspace test 3772 passed/10 ignored・test-rt 290・test-io-msrv 742（rust@1.89.0）だった。第 1 回の run で `check`（exit 101）と `lint`（exit 66）が失敗したが、同一 log 内で `cargo test --workspace` が成功していたことから、rustc キャッシュ破損（`only metadata stub found for rlib dependency core`）による infra flake と診断して再実走し、両 run を併記した。flake を PASS に付け替えてはいない。長時間 lane の並列度は `CARGO_BUILD_JOBS=2` で制限し、MSRV lane は `rustc --version` で rust@1.89.0 を事前確認してから実行した。merge はドキュメント簿記のみだったため、実走結果は merge 後 HEAD `1b78607c` に適用可能と判断した (Issue #288)。

consumer 側の検証では、consumer-verify の最新 run（`42937524742`）の candidate Ruviform Revision `9865cd57` が main の祖先であって最新 tip ではないため、revision をそのまま転記し「最新 main での consumer 受入」とは記録しない判断をした。gate matrix の各セルには判定・根拠・残存 NotRun を記録し、matrix の穴 7 件（実 4GiB・Network/Cloud FS・非既定 FS・macOS/Linux 実機・性能比較・consumer 固有の受入残・fault 注入の fixture 化）を列挙した。macOS/Linux 実機・Network/Cloud FS・実 4GiB・consumer 固有受入・`Indeterminate` 系 fixture 化は `not run`/`unsupported` として残した (Issue #288)。

## StagedFile の mock 検証と Windows 実 FS 検証

StagedFile の最初の検証は mock 行列で行った。Windows x64・rustc 1.99.0 で `git merge origin/main`（Already up to date）の後に `mise run fmt-check`・`check`・`lint`・`test` がすべて green となり、ruviform-io-file は unit 26 件と doctest 10 件（compile_fail 2 件を含む）が全合格、clippy は `-D warnings` で警告 0 だった。この時点の `StagedFile::create` は常に `Unsupported` を返すため、実ファイル生成と fsync の達成は次の Issue のゲートに委ねると明記し、本検証では主張しなかった。`verify-ci` は dev profile の 4 ゲートを実施済みのため割愛と記録した (Issue #295)。

実 FS 検証は Windows 11 x64・NTFS で行い、`mise run verify-ci`（fmt-check→check-ci→lint-ci→test-ci、CI profile・CARGO_INCREMENTAL=0）が 4 ゲート全緑・126 test-result ok・0 failed だった。`tests/os_publish.rs` は 15 件 pass と子プロセス入口の `#[ignore]` 1 件で、6 プロセスの `CreateNew` 競合でちょうど 1 勝者・5 件の `DestinationExists`・宛先 byte 不変・temp 全消去を実測した。Windows の排他 share による `Replace` 失敗が `NotPublished`+`Io` で確定的に分類されることも確認した。誘導不能な失敗（rename 後の dir sync 失敗・`Unknown`→`Indeterminate`）は fault-injection mock で網羅済みである。unix 経路（renameat2・renamex_np・link fallback）と readonly dir 誘導の実測は本ホストで不可能なため CI OS matrix へ委譲し、capability matrix の未検証セル（FAT/exFAT・ネットワーク FS・dir sync 非対応 FS）については保証を主張しないと記録した (Issue #296)。

## 構造化フォーマットの契約試験

構造化フォーマット基盤では、`cargo test -p ruviform-io-structured --all-features` で 36 契約試験と 9 doctest が全緑、`--no-default-features` と各 feature 組合せでも契約試験全緑・警告 0 を確認した。`cargo +1.89 check --all-features --all-targets` で MSRV 1.89 toolchain のコンパイル通過を補助確認し、workspace の `mise run fmt`・`check`・`lint`・`test`（34 crates）もすべて green だった。Windows 実 FS での file feature 試験は `tests/structured.rs` で実施済みで、macOS/Linux の実 OS/実 FS 検証は CI lane に委譲した (Issue #297)。

CSV モジュールは契約試験 41 件と file 合成試験を追加し、`--all-features` で 77 契約試験（CSV 41+既存 36）+ 11 doctest が全緑だった。feature matrix（`csv`/`csv,file`/全組合せ/無 feature/`file` 単体）で警告 0・clippy `-D warnings` 全緑を確認したが、feature matrix は workspace test タスクの対象外のため補助証拠として実施し、canonical は CI の default features と明記した。MSRV `rust@1.89.0 --all-features` でも 41 試験 + 5 doctest が pass した。依存 DAG は csv→csv-core→memchr/itoa/ryu/serde_core で既存層外への新規依存はなく、RT 層からの依存非存在も DAG で確認した。license は Unlicense/MIT/Apache-2.0/BSL-1.0 で copyleft はなく、`bench-smoke` で baseline 劣化はなかった (Issue #298)。

XML モジュールは `mise run check-io-structured`・`lint-io-structured`・`test-io-structured` で feature matrix（default・xml・xml,file・all-features）がすべて緑の 117 tests + doctests、clippy `-D warnings` で 0 warnings だった。負荷試験として DTD（internal/external）・未定義 entity・非 UTF-8・各 limit 分類・seeded fuzz 200 doc を `tests/structured.rs` で pin した。リスクとして、`max_event_bytes` は backend 内部 buffering 後の公開時検査であって確保前の厳密 cap ではなく、複数 root/top-level text は backend 忠実に受理される（document 構造検査は責務外）。io-structured 用 CI lane（job と impact map）が存在しない既存の欠落（json/yaml/toml/csv も未カバー）を確認し、mise タスクは追加したが CI job/map の追加はスコープ外として別 Issue 起票予定とした (Issue #299)。

## 圧縮フォーマットの feature matrix と zstd 追加

Issue #300 では Windows 11 x64・rustc 1.99.0 で feature matrix 4 組合せ（default/gzip/deflate/gzip,deflate）の `cargo check`・`clippy`・`test` がすべて PASS、clippy `-D warnings` は 0 warnings、classifier の contract 52 件もすべて緑だった。CPU 時間・backend 内部メモリの hard limit は宣言せず、累積 byte/unit budget のみを強制する方針で、zip-bomb 対策は呼出し側の DecodeLimits に委譲される。compression-verify lane は shared-config 変更を含むため本 PR では Full CI で実行され、selective skip/run は今後の運用観測事項とした。workspace 全体の `mise run fmt`・`check`・`lint`・`test` と `git merge origin/main` 後の最終再検証は、記録執筆時点では「PR 作成直前に実施」という段階にとどまり、完了の有無は本記録からは確認できない (Issue #300)。

zstd 追加では、`zstd_format.rs` で golden round-trip・single/concatenated frame・skippable frame の accept（unit 非計上）と reject（`Unsupported`）・checksum 混在（`IntegritySummary`）と破損（`ChecksumMismatch`）・window 超過（`LimitExceeded`）・corrupt body（`CorruptData`）・truncated/trailing/wrong magic（`InvalidFormat`）・reserved FHD bit・DID≠0（`Unsupported`）・budget 厳密値（ちょうど/+1）・大量小 frame 累積・`compressed_bytes_consumed` の論理消費・`zstd` crate との interop を検証した。`encoder_contract`/`decoder_contract` に `#[cfg(feature="zstd")]` mod を追加し、冪等 finish・Drop・Failed latch・NotExhausted・AtOutputLimit を pin した。`check-io-compression`・`lint-io-compression`・`test-io-compression` の feature matrix を zstd 単独と全有効を含む 5 組合せへ拡張し（lane 遅延を抑えるため 5 組合せに限定）、impact map・CI 定義コメント・クレート README も同期した。実走は 5 組合せすべて exit 0（zstd で 34 format tests + contract/API 共有 suite + doctest）、`verify-ci`・`test-ci-impact` 52/52 も pass した。`zstd-sys` の vendored C build はローカルの MSVC+cc での成功のみが根拠で、ubuntu-latest の gcc での最終証明は compression-verify lane の Required check に委譲した (Issue #301)。

## アーカイブ reader/writer の試験

zip_reader は `cargo test -p ruviform-io-archive` で `tests/zip_reader.rs` の 24 件がすべて pass した（truncated/malformed/inconsistent/overlap/unsupported/budget/CRC/partial-drop/verify_all を網羅）。clippy `--all-targets -D warnings` も pass した。zip_writer.rs は 10 件すべて pass し（round-trip/poison/finish 分離/limit/measured size）、workspace 全体の fmt・check・lint・test がすべて exit 0 だった（130 test binary 全緑、io-archive は最終的に zip_reader 32・zip_writer 10・doctest 7）。並行 worker が同時期に新規クレートを追加しており Cargo.toml・crates/README.md・Cargo.lock で merge 競合が予想されたため、自クレート追加分を保持して解消し、Cargo.lock は再解決で再生成する運用とした。index/README 類は `doc_maintenance apply --force-index` で機械再生成し、手編集 drift を避けた (Issue #302)。

## media-io cutover と WAV golden/fault 試験

WAV golden/fault では `mise run fmt-check`・`check-io-e2e`・`lint-io-e2e`・`test-io-e2e`（77 tests 全 pass）・`check`・`lint`・`test`・`test-rt` を merge 後 HEAD で全実行・全 Pass した。wav_golden 8 件（ZeroFrames 44B・5.1 WAVEFORMATEXTENSIBLE 68B・S16 stereo・U8/S16/S24/S32/F32 encoder edge sample の NaN→0/±Inf/clamp/round-half-away/F32 verbatim・odd payload pad・classic RIFF 4GiB 境界）と wav_fault 9 件（Io 失敗分類・seal 後 abort・Drop best-effort finalize が completion 証拠でないこと・publish outcome classifier・CreateNew 競合・Replace・create 失敗の型付き FileErrorKind::Io）を追加した。ruviform-io-wav の src は無変更で、真の defect は未発見だった (Issue #305)。

cutover では 3 回の merge 後再検証を行い、fmt-check・check・lint・test がすべて PASS した。of01..of06・staged_* はすべて PASS し、旧 API 残存検査（StagedOutput・media-io 内 PublishOptions・mod staged/staged::）は各 0 件だった。merge commit `7ead144` で 166 test-result blocks・0 failed、combined merge `ce8bc99b`（他候補の取込を含む）で 170 blocks・0 failed・test-io-e2e 31 件 pass だった。`cargo tree -p ruviform-media-io` が Domain Core3 依存のみであることも補助証拠として確認した（trybuild など旧 API 非存在のコンパイル時証拠は、新依存追加禁止のため未導入）。review-protocol では実バグを複数検出して修正した。render の doc overclaim（`Err` 時の temp 除去・dest 無変更が `Published`/`Indeterminate` でも成立するかのような記述）、verify hook の `mem::swap` 差し替えによる receipt/publish 先不整合（operation_id 照合の swap-guard を追加）、test-spec が要求する独立 decode E2E の不足、publish 時 `DestinationExists` 写像の未試験、である。独立 Validator（read-only subagent）による cross-diff 検査は APPROVED で、RF305 scope の 41 file のみ・main 新規成果への diff ゼロ・他候補の実装非改変を確認した (Issue #305)。

## 層横断の合成 E2E クレートの新設

合成 E2E の検証では、test-only クレート `ruviform-io-e2e` を新設し、`StagedFile`・Structured・Compression・Archive・`WavMuxer` の層横断境界を 31 件の合成試験で検証した（staged_lifecycle・structured_staged・archive_compression_staged・extraction・wav_staged）。`mise run test-io-e2e` で 31/31 pass、workspace `cargo test`（merge head `a99e953`）で 2773 passed/0 failed/1 ignored だった。直前の cutover 候補 `82256cf` は scratch worktree に checkout し、旧 `StagedOutput`/`PublishOptions` が不在であることの grep 証跡と、`WavMuxer<&mut StagedFile>`+seal+readback+publish の合成再演を確認した。候補 revision 上でも `check-io-e2e`・`lint-io-e2e`・`test-io-e2e` が全 Pass・31/31 tests が同一の緑となった。直接 cargo で取得した補助証跡は正規の mise lane の Pass とは区別して記録した。fixture の実装上、serde_json が非有限 f64 をエラーにせず null へ写すため encode 失敗 fixture は独自 Serialize 実装の失敗に切り替え、WAV の未 finalize drop は backend が best-effort finalize するため truncated bytes の独立 decode 不一致で検証する形にした。独立 review で検出された採用指摘 1 件（ustar version を含む）を反映し、再実走で緑を維持した (Issue #306)。

両 consumer の実 runner はこの時点で Blocked だった（AudioLens 側の修復待ち・Synvelis 候補の旧 API 参照残存が未解消で、coordinator 管理の後続検証待ち）。io-compression/io-structured の feature matrix lane・実 fault 強制再現・macOS/Linux は NotRun として記録した。main の既知 flake（plugin-host 起動 Timeout）は本 run で未発生であり、失敗を観測していないため分類対象とはしなかった (Issue #306)。

## post-seal temp 契約の固定と durability ladder の実 FS probe

staged_lifecycle には 3 Case を追加し、io-e2e を 77→80 tests に補完した。`post_seal_temp_rewrite_is_not_reverified_before_publish` は post-seal の temp rewrite が publish で再検証されず、改竄 bytes が destination へ publish されることを契約として固定した。`post_seal_temp_delete_fails_publish_not_published` は temp delete→`NotPublished`・destination 非作成・recovery cleanup が `Absent` で成功することを確認した。`recovery_cleanup_failure_returns_handle_for_retry` は cleanup 失敗→handle 返却→retry 成功で `TempState::Absent` となることを確認した。post-seal temp 改竄が publish で検出されないのは seal 時点保証の境界であり、io-file の再検証強化は設計判断として別 slice に分けた (Issue #306)。

durability の検証では、`os_fs_probe` が `CreateNew`/`Replace` × `BestEffort`/`FileSync`/`FileAndDirectorySync` の 6 通りを実 FS で probe し、`DurabilityEvidence` が要求 level と一致するか、`PublishFailureKind::Unsupported`+`NotPublished` で正直に拒否されるかのみを assert した（silent downgrade の検出）。Windows 11 x64/NTFS で 6/6 一致を実測した。ruviform-io-e2e にはこの slice で 29 件を追加して計 60 件となり、fmt-check・check-io-e2e・lint-io-e2e・test-io-e2e と workspace の check/lint/test/test-rt はすべて exit 0 だった。macOS/Linux・Network/Cloud FS・性能は NotRun/Unsupported である (Issue #306)。

## fault-injection seam と corpus replay

fault-injection seam では、feature 無しの `check`・`lint`・`test-io-file` で既定 build が不変であることを確認し、`test-io-e2e` で fault_seam 8 件が pass した。8 件は `Indeterminate`・publish 後の DirSync 失敗・`TempState::Unknown`・phase 別 `NotPublished`（Validate/FileSync/Close/Publish）を公開 API のみで決定的に検証する。`corpus_replay` は `#[ignore]` で通常 lane では非実行であり、opt-in の `test-io-e2e-corpus` は固定 seed の 48 cases/format × 9 形式・計 432 cases を Windows で 1 回実行し、panicked=0・elapsed 約 22.5ms だった。io-structured/io-compression 専用 lane は省略した。変更が io-file の cfg/feature 追加のみで公開 API 不変であり、test-io-e2e が fault-injection 有効の io-file を含む下流一式を compile+test 済みであることから、比例検証の方針に従った (Issue #306)。

## G01/G05 の環境依存ゲートと範囲外の既存破損

環境依存ゲート G01（実 4GiB 境界）と G05（性能 baseline）は io-e2e クレートに実装して実走した。G01 は `#[ignore]` opt-in lane で、SIZES=[4GiB−1,4GiB,4GiB+1] × 5 scenario を FNV-1a checksum で検証し、`set_len` sparse probe の preflight と `StorageFull`/`Io` の `NotRun` 分類により Pass/Fail 偽装なしの構成とした。canonical run は Pass 15/15 case・1008.22s・preflight probe OK（required=4,563,402,753 bytes・NTFS temp_dir）だった。G05 は criterion bench（6 cell・`CountingAlloc` peak heap probe・`BatchSize::PerIteration` で fixture を計測窓外）として実装した。canonical の `mise run bench`（workspace 全量）は exit 101 で失敗したが、原因は `ruviform-numeric` の lib test が bench profile でコンパイル不能な既存破損であって本 slice の範囲外だった。`cargo test -p ruviform-numeric --lib` が 116 tests pass であることで bench-profile 固有と確認し、lease 外のため未修正のまま記録した。io_composition 単独の `cargo bench -p ruviform-io-e2e` は全 6 cell 完走・peak_delta 最大約 1.34MiB（1GiB bound 内）だった。self-review で staged_publish cell の destination 不整合（bound dest `publish.bin` と remove 対象 `published-N.bin` の不一致で初回 iteration が panic する潜在 bug）を 1 件 CONFIRMED し、修正後に bench 実走で再検証した (Issue #306)。

## lane 実走証拠（feature matrix・MSRV・OS 別 CI）

feature-matrix lane は検証・証拠のみの slice として、コード・テスト・設定を変更せず、9 本の scoped lane（check/lint/test の io-compression・io-structured・io-e2e 各 3 本）と workspace 最終検証 5 本（`fmt-check`・`check`・`lint`・`test`・`test-rt`）をすべて `mise run` で実走し全 lane Pass した。compression は 5 組合せ合計 264 passed（default 3・gzip 52・deflate 26・zstd 58・gzip,deflate,zstd 125）、structured は 4 組合せ合計 251 passed（default 5・xml 47・xml,file 49・all-features 150）、io-e2e は 77、workspace test は 3696 passed/0 failed/7 ignored、test-rt は 288 passed だった。組合せ別の test 数は log の機械集計（cargo test の起動境界で分割し awk で集計）によるものであり、推測ではない (Issue #306)。

MSRV lane は `test-io-msrv` タスクを `mise.toml` に追加して整備し、`mise exec rust@1.89.0` で io 系 6 クレートに `--features deflate` 補完を加える構成とした。lane 設計の根拠には resolver v2 の feature unify を採用し、io-e2e を同一 `-p` 集合へ含めると io-e2e の dev-deps 経由で io-structured 全 format と io-compression の gzip/zstd が MSRV build に載ることを実測で確認した。deflate は io-e2e が消費しないため別コマンドで補完している (Issue #306)。

OS 別 CI の証拠では、対象 SHA `e93c438a` の Full CI run `43428172820`（completed/success）の生ログを解析した。verify 3 OS（ubuntu/windows/macos）で fmt-check+lint-full+test-full がすべて success で、io-e2e 77/0f・io-file（1 ignored は意図的）・io-archive 172・io-compression 126・io-structured 150・io-wav 38・io-symphonia 50・media-io 66、すべて 0 failed だった。compression-verify は 5 feature 組合せを deflate 含めて実走（ubuntu のみ）、io-structured-verify は 4 組合せ、msrv-verify は rust@1.89.0 で io 系 6 クレート+deflate 補完、rt-verify は `rt_` filter で io binary は compile/link と filter 実行のみだった（io-e2e の一致は `assert_` の誤検出のみ）。deflate は io-e2e が消費しないため verify lane では 0 であり、ubuntu lane のみの実走として分離記録した (Issue #306)。

## install_plan の破壊的変更と fail-closed 化

install_plan の戻り値は `Result<(),QueueFull>` から `Result<(),InstallPlanError>`（`#[non_exhaustive]`・`QueueFull`/`FrameBaseMismatch`）へ変更された。error 型を名指しする downstream は `InstallPlanError::QueueFull` への機械移行が必要で、本 repo に該当はなく crate 内 callers はすべて `.unwrap()` 経由で source-compatible だった。foreign-base plan の install は以前は照合なしで stage されていたが、新たに `FrameBaseMismatch` で拒否される挙動変更（意図的な fail-closed 化）となった。`mise run fmt-check`・`check`・`lint`・`test`・`test-rt` はすべて exit 0、`frame_base.rs` は 6/6 pass、116 test groups 0 failed、doc lint 0 findings だった。実 plugin/実機 OS・consumer 実 repo ビルドは NotRun であり、cross-consumer lane は coordinator/PR 側の責務である (Issue #307)。

## flaky test の切り分けと consumer 側の実績

workspace の `mise run test` で `ruviform-plugin-host` の `l1_proxy_e2e` が高並列下で散発失敗（`SpawnFailed{Timeout}` 系・失敗する test は実行ごとに異なる）することがあったが、`--test-threads=4` で 184/184 pass を確認し、deterministic な回帰ではないと判定した。対象の差分は docs のみで当該クレート無変更であり、baseline でも再実行で全 suite pass した。io 系全 lane は merge 後 head で個別に pass している (Issue #398)。

consumer 側の実績として、Synvelis 側の 2 本の PR（RootedDir 消費と、`with_borrowed_demuxer` 消費に伴う独自 ReadProxy/channel/scoped-thread の撤去）が MERGED し、Synvelis 側の検証記録は l2_e2e 17/17・x_e2e 5/5・pc_b 7/7 pass だった。borrowed 系の回帰検証では `origin/main` `b36f6de2` で fmt-check・check・lint がすべて pass、borrowed 回帰 11/11・borrowed_source 3/3・wav_staged 3/3・media-reader/media-io/io-symphonia の全 test pass・compile_fail doctest を確認した。`mise run test` では既知 flaky の plugin-host プロセス境界 test（`wedged_child_write_converges_and_teardown_is_bounded`）が 1 回時間超過したのみで、単独再実行は pass であり変更とは無関係と判断した。RF↔SV 候補 tuple 互換 gate は coordinator 管理の cross-repo gate で NotRun、unix/macOS runtime の rooted capability 実測も NotRun（check-linux は型/lint のみ）である (Issue #398, Issue #400)。

## scoped enumeration の実走とレビュー指摘の修正

scoped enumeration の変更は `mise run fmt-check`・`check`・`lint`・`test` が pass し、`origin/main` merge 後に最終実測の exit 0 を再確認した。targeted な io-file+archive の回帰と、`check-linux`（x86_64-unknown-linux-gnu の check+clippy）も pass した。独立 review（Reviewer A が cooperative・Reviewer B が adversarial）で報告された blocking findings（unix readdir の errno・Windows per-call FILE_OBJECT・`sb.Information` bounds・anchor 失敗の `Unsupported` 化・mock の node-children 化）は修正を確認済みである。実測した OS の結果のみを記録し、未実測の OS は NotRun とした (Issue #409)。

## canonical premise の pin 試験による test-spec-skip

Issue #468 は挙動変更のない doc comment・契約文書のみの変更であり、canonical 前提を実行可能に pin する試験は既存の実 FS 試験 `absolute_link_alias_spelling_escapes_root` が既に担っているため、新規 test-spec を作成しない `test-spec-skip` を宣言した。検証は `check-io-file`・`test-io-file`（既存 pin 試験が無変更で pass）と `check`・`lint`・`fmt-check`、および route_graph briefing で本タスク起因の broken relation がないことを確認した。main 既存の issue-389/issue-404 の broken relation は残存を許容した (Issue #468)。

## bench-smoke の実測とドキュメント照合

`mise run bench-smoke` が ruviform-io-e2e の `io_composition` を計測なしで 1 回実行することを実測で確認した（exit 0、6 case すべて Testing...Success: staged_write/seal/publish/readback/tar_gzip_staged/tar_zstd_staged）。開発環境ドキュメントの bench 系記述は `mise.toml` の `[tasks.bench]`/`[tasks.bench-smoke]` と一致しており drift はなかった（同ドキュメントは別 Issue が占有していたため read-only の照合に留めた）。fmt-check・ruviform-io-file の check/lint/test・io-e2e の check/lint/test（`post_seal_temp_*` 2 試験を含む既存 suite 全件 ok）はすべて pass した (Issue #721)。

## plugin_dock の fail-closed degrade 経路の実効確認

plugin_dock.rs の fail-closed degrade 設計（:137-149 early-return）に対し、ruviform_plugin_test_vst3.dll が target/debug/deps/ に存在し `artifact::resolve_artifact` で解決されるため、fixture_run_reports_honest_verdicts は degrade 経路ではなく full assert 経路を実行した。:168,:211 の "test-plugin" assert が実効的に検証された（degrade のみの NotRun ではない）。check/lint/test ruviform-audio-test・ruviform-plugin-dock-test（4/4）・test-ci-consumer-verify 84/84・fmt-check・prek run --all-files はすべて exit 0 だった。scenario.rs:280 の `json!(kind.as_str())` は `as_str()` flip に自動追従し編集不要を確認した (Issue #725)。

## 記録の範囲と限界

本書が記録する検証には、実行されなかった項目と後続へ委譲された項目が含まれる。実行環境に依存する未実施項目として、Issue #251 の macOS/Linux 実機（Stage M/L）・物理 USB 抜去・実 OS 権限拒否・xrun 誘発・consumer 側 frame 相関はすべて NotRun/Deferred のままであり、`E_ACCESSDENIED` の実機再現も NotRun である。Issue #295 の `StagedFile::create` は当時 `Unsupported` 固定で、実ファイル生成と fsync は Issue #296 の Windows 検証へ委譲された。Issue #296 の unix 経路（renameat2・renamex_np・link fallback）と readonly dir 誘導、capability matrix の未検証セル（FAT/exFAT・ネットワーク FS・dir sync 非対応 FS）は CI OS matrix への委譲であり、CI での実測結果は収集した記録には含まれない。Issue #297・Issue #299 の macOS/Linux 実 OS/実 FS 検証も CI lane への委譲のままである。Issue #306 では macOS/Linux・Network/Cloud FS・性能・実 fault 強制再現が NotRun/Unsupported で、残存受入行（G02 Network/Cloud FS・G03 特殊 FS・G04 実 4GiB 以外の計測・G06 consumer gate・macOS/Linux 実機）も Deferred のままである。Issue #288 の matrix の穴 7 件は新規 Issue 起票待ちで、解消状況は不明である。Issue #286 の `tar.zst` 合成個別試験と unix backend の実 FS 実行試験（TOCTOU）は未実施のまま残っている（gzip で契約 pin 済み・Linux CI lane 依存）。Issue #305 で列挙された Published/Indeterminate の失敗 effect・DirSync 後失敗・`TempState::Unknown`・実 4GiB publish も実走していない。

consumer・cross-repo に関する未確定項目として、Issue #197・Issue #244 時点の audio-lens consumer impact は `unknown`/`unverified` で、Issue #249 で `no impact` に確定した（audio-lens repo 自体の検証は対象外）。Issue #306 の両 Consumer 実 runner は Blocked（consumer 側の修復と候補の旧 API 参照残存 — coordinator 管理）で、Issue #307 の実 plugin/実機 OS・consumer 実 repo ビルドも NotRun である。Issue #398・Issue #400 の RF↔SV 候補 tuple 互換 gate は coordinator 管理の cross-repo gate で NotRun、unix/macOS runtime の rooted capability 実測も NotRun（check-linux は型/lint のみ）である。Issue #288 の consumer-verify tuple は revision `9865cd57` の記録であり「最新 main での受入」ではない。Issue #244・Issue #249 の consumer impact 最終判定は cross-repo gate の責務であり、実 consumer repo での回帰結果は本記録からは確認できない。

記録自体の限界として、Issue #303（TAR）と Issue #304（Secure Extraction）に対応する実装・検証の実測記録は収集対象の記録群に存在せず、実装の完了状況は本記録からは判断できない。Issue #304 の `NtDeleteFile` が leaf の symlink/junction に対し link 自体を消すか（`OBJ_DONT_REPARSE` の実挙動）は Windows 実測で確定すると記録されていたが、実測結果は記録にない。Issue #300 の workspace 全体最終再検証は「PR 作成直前に実施」の段階で記録が止まり、完了の有無は不明である。Issue #299 の io-structured 用 CI lane 整備は別 Issue 起票予定とされ、起票の有無は不明である。Issue #306 G05 で発見された `ruviform-numeric` lib test の bench-profile コンパイル不能（exit 101）は lease 外で未修正のまま記録され、後続の修正状況は不明である。Issue #223・Issue #306 の DAG/RT 監査証跡は実行時点の snapshot であり、依存 DAG が変わった場合は再実行が必要である。Issue #398・Issue #400 の close 判断（cross-repo gate・実 OS 行の最終判定を含む）は Issue owner の責務であり、本記録は充足証拠の提供のみで、実際の close 状況は不明である。

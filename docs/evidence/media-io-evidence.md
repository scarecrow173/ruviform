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
  source: ["https://github.com/scarecrow173/ruviform/issues/18", "https://github.com/scarecrow173/ruviform/issues/20", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/191", "https://github.com/scarecrow173/ruviform/issues/197", "https://github.com/scarecrow173/ruviform/issues/223", "https://github.com/scarecrow173/ruviform/issues/233", "https://github.com/scarecrow173/ruviform/issues/244", "https://github.com/scarecrow173/ruviform/issues/249", "https://github.com/scarecrow173/ruviform/issues/251", "https://github.com/scarecrow173/ruviform/issues/285", "https://github.com/scarecrow173/ruviform/issues/286", "https://github.com/scarecrow173/ruviform/issues/288", "https://github.com/scarecrow173/ruviform/issues/296", "https://github.com/scarecrow173/ruviform/issues/298", "https://github.com/scarecrow173/ruviform/issues/299", "https://github.com/scarecrow173/ruviform/issues/300", "https://github.com/scarecrow173/ruviform/issues/301", "https://github.com/scarecrow173/ruviform/issues/302", "https://github.com/scarecrow173/ruviform/issues/305", "https://github.com/scarecrow173/ruviform/issues/306", "https://github.com/scarecrow173/ruviform/issues/307", "https://github.com/scarecrow173/ruviform/issues/398", "https://github.com/scarecrow173/ruviform/issues/721", "https://github.com/scarecrow173/ruviform/issues/725"]
  informs: []
  explains: []
  supersedes: []
---

# メディア I/O・ファイル形式の検証記録

本書は、過去の実装記録から抽出したメディア I/O とファイル形式に関する検証・実行結果をまとめたものである。元の記録群は後に削除されたため、本書はそれらに依存せず単独で読めるように記述した。各事実は出所の Issue 番号で示し、測定値や観測された挙動などの実測事実と、そこからの解釈を区別して記す。解釈は「解釈」と明示する。

## audio-core のデータモデルとリアルタイム無割当の検証

audio-core のデータモデルは Windows x64・rust 1.99.0 環境で fmt/check/clippy（`-D warnings`）/test が全て green となった。最終値は unit 30 件に doctest 18 件で、`rt_` 接頭辞の 3 テスト（view channel access・interleaved roundtrip via views・SampleIndex↔MediaTime 変換）が `assert_no_alloc` 系で選択され合格した。レビューで指摘されたアクセサ coverage の欠落・`isize::MAX` での panic 境界・`version()` の RT ラベル欠落は修正 wave で解消された (Issue #18)。

## ワークスペース横断の検証スイートとドキュメント監査

全体の検証スイートは全項目 PASS した。media-io は 40 unit と 19 doctest、io-symphonia は 11 unit・7 e2e・5 doctest、workspace test は 220 件で失敗 0 だった。test-rt は意図した `rt_` テスト 10 件のみを選択し、新規 2 クレートのテストは全てフィルタアウトされた。bench は no-op を維持した。`cargo metadata` で全 8 パッケージが `publish: []` であり依存辺は設計どおりであること、unsafe grep がコード内 0 件であること、pub enum 4 件が全て `#[non_exhaustive]` であることを確認した。ドキュメント監査 `audit_docs.js --type all` は初回に 10 件を検出した（索引 status 不整合 5・derived-by 相互参照 4・verifies 不正ターゲット 1）。修正後は 0 件となり、修正 wave 後の clippy/test 再実行も全て pass した (Issue #20)。

## cpal アダプタのコンパイルと依存境界

cpal アダプタは cpal 0.18.2 に解決され、edition 2024・rust-version 1.85 の条件下でコンパイルに成功した（実 toolchain は 1.99.0）。`cargo test -p ruviform-audio-device` で単体 39・mock 結合 4・doctest 6 が全て pass し、`mise run test-rt` では `rt_output_callback_path_allocates_nothing` が `assert_no_alloc` 配下で選択・合格した。これは view 構成・`copy_to_interleaved`・user callback・timestamp 写像の無割当を機械検証したものである。`cargo metadata` で cpal が当該クレートの `[dependencies]` のみに存在し、adapter への逆依存がゼロであることを確認した。実機挙動は後の実機検証へ引き継がれた (Issue #23)。

## WAV バックエンドの往復・契約試験

WAV バックエンドは `tests/roundtrip.rs` の 9 件が全て pass した。44.1k/48k/96k × mono/stereo/5.1 × S16/S24/S32/F32 の write→reopen を `ruviform-io-symphonia` で decode し、長さ・チャンネル数・サンプル値を検証した。`tests/contract.rs` の 19 件も全て pass し、capability honesty・unsupported codec/container/sample-format/multi-stream の拒否・stream-id/partial-payload 検証・sink の write/finalize error→Io 分類・staged drop/abort/no-overwrite/disk-full/replace での旧宛先 byte 不変を確認した。`cargo test -p ruviform-media-io` は 53 unit と 20 doctest で、workspace の fmt/check/lint/test も全て green だった (Issue #191)。

## media-reader のスイートと consumer への影響分類

media-reader の suite は 29 件全て pass した（3 lib・13 prefetch・11 reader・2 rt_fill）。symphonia boundary は 3 件、AVI は 2 件で、既存 suite は無変更のまま全て green だった。`test-rt` では `rt_consumer_fill_no_alloc` と `rt_consumer_underrun_fill_no_alloc` が `assert_no_alloc` により RT fill 経路の無割当を証明した。`verify-ci`（CI profile）も pass した。consumer impact は、synvelis が additive API のため no impact、audio-lens はコードレビューで互換を確認したものの local-path `cargo check` は consumer repo が read-only のため unverified と記録され、RD-10 compile-equivalence も unverified のまま残った (Issue #197)。

## MSRV lane と監査証跡の再実行要件

`mise run test-io-msrv` で rust@1.89.0 の 6 crate 一括と deflate 補完が 680 passed/0 failed/1 ignored となった。ignored の 1 件は io-file の os_publish.rs にある既存 gate の spawned `#[ignore]` helper である。workspace test は 3580 passed/0 failed/7 ignored で、DAG/RT audit（`cargo tree -i`・`cargo tree -p`・`cargo metadata` の python parse assert）は全て Pass した。MSRV lane は test 実行のみで clippy を含まない。これは test-ofx-msrv/test-ui-test-msrv と同じ範囲であり、MSRV clippy は情報 lane の方針に従う。監査証跡は実行時点のスナップショットであり、依存 DAG が変わった場合は再実行が必要である (Issue #223)。

## Fault の計上先と assertion の修正

`rd02_reopen_at_seek_failure_faults_then_reanchors` の初回実測で、pre-boundary の新 generation `Fault` は consumer が generation を未 adopt のため `fault_events` ではなく `stale_drops` に計上されることが判明した。これは `step()` semantics と整合する正しい挙動と判断し、実装を変えずに assertion を `stale_drops >= 1` に修正した。review-protocol は 2 reviewer・13 candidate で verdict ACCEPT WITH NON-BLOCKING NOTES（P0/P1 なし）だった (Issue #233)。

## counter semantic の変化と consumer impact、未解決の wart

failure path で `stale_drops` が減少し `fault_events`/per-kind `faults` が増加する意図的な変更が行われた。in-repo では `rd02_reopen_at_seek_failure_faults_then_reanchors` の 1 件を更新し、synvelis は quiesce-drain→seek workaround が引き続き有効で no impact、audio-lens は repo 不在で unknown と記録された。review で新規 wart を検出した。`PumpOutcome::Ended` が `End` marker emit の 1 pump 手前でも返る。`fill_block` の EOF arm が `stream_ended` を立てて `Ended` を返すが、`End` は次 pump の step 2 で合成される。documented `Idle`/`Ended` まで pump する manual driver が検出時点で停止すると `End{gen}` が strand し、consumer は `eof` を観測できない。spawn `run()` は pump を継続するため実害は manual driver のみで、別 issue 候補として記録された (Issue #244)。

## consumer impact の確定（synvelis・audio-lens）

synvelis は impacted(positive・対応不要) と確定した。`driver.rs:248-257` の `while pump()==Pushed` bounded loop は非 `Pushed` で一律 break するため変更不要で、旧 semantics では終端 cycle で `End` が strand し得た hazard が消える。`Ended` 後の seek で `End{old}` が `Boundary{new}` より先に観測されるが、generation 付き marker で honest に処理される。audio-lens は local worktree で `ruviform-media-reader` 非依存を確認し no impact に確定し、#244 の unknown を解消した。TDD で新規 2 テストが実装修正前に red を確認した後、`cargo test -p ruviform-media-reader`（prefetch 27・reader 11・rt_fill 3）と workspace `mise run test`（115 suite）が全て green だった (Issue #249)。

## 入力ストリーム構築の実装と OS 別 CI 証拠

`open_input_stream` を placeholder の `Unsupported` から実 cpal input stream 構築へ置き換え、`build_input_stream::<f32>`→interleaved→planar `AudioBufferView`→`InputDataCallback` を RT 契約（open 時一括確保・callback 内無割当/無ブロック）どおりに実装した。`mise run fmt-check/check/lint/test/test-rt` は全て pass し、audio-device は lib 59・integration 12・doctest 6 だった。`rt_input_callback_path_allocates_nothing` が `InputCallbackState::handle` 全経路の無割当を機械検証した。MSRV `rust@1.89.0` でも全て pass した (Issue #251)。

OS 別の CI 証拠では、run `43428172820` の job/log 証拠として verify ubuntu が 59+4+12+0+6 tests、windows が 75+4+12+0+6 tests（`wasapi_input::tests` 16 件を含む）、verify-macos は `coreaudio-rs` link を確認、rt-verify・msrv-verify（audio-device 非対象）・ci-summary verdict=PASS を記録した。`real_input_e2e.rs` は feature 未設定で 0 tests compile されることを log で確認した。OS×Backend×Device×Case 行列で Windows/WASAPI/CI runner は headless Pass となった。macOS/Linux 実機・物理 USB 抜去・実 OS 権限拒否・xrun 誘発は NotRun/Deferred のままであり、Pass に繰り上げていない。PR #568/#581 の Windows/WASAPI/Audient iD4 実機証拠は過去証拠として CI headless と区別して引用した。CI artifact retention 30 日を考慮し、job URL・test 集計値・SHA を本文に転記した (Issue #251)。

## Audient iD4 実機での capture 計測

Windows 11・Audient iD4（`Analogue 1/2`、driver `audientusbaudio_sc`）・WASAPI・44.1kHz F32 Stereo・`BufferSize::Fixed(441)` で `mise run test-audio-real-input` が 4/4 Pass した。capture の計測値は callbacks=192・total frames=84672・frames/callback=441 固定・order violations=0 だった。実効 input latency は min=10 038 500ns・max=10 117 300ns・mean=10 059 264ns、frame_position は `Some` 192/192 で gaps=0、`clock_domain`=MONOTONIC、`input_latency()`=`Some(StreamLatency{frames:0, coverage:Unknown, provenance:Carried})`、xrun=0 だった。duplex では input/output とも `ClockDomainId(2)` で `same_clock_domain`=true、UNKNOWN rejection=false だった。format mismatch 7 行は全て `Err(UnsupportedConfig)` だった。device loss は物理 USB 抜去の代わりに既定 endpoint 切替（`IPolicyConfig::SetDefaultEndpoint`、管理者権限不要）で `StreamInvalidated`+`ClockDiscontinuity` を実観測した。物理経路は対話式 lane が案内済みで NotRun のままである (Issue #251 Stage W)。

## output latency の実測と E_ACCESSDENIED の parity

Audient iD4・WASAPI・44.1kHz F32 で `real_output_stream_observes_latency` を新設し、約 1 秒の無音再生で実測した。callbacks=102・total frames=44629・playback timestamp `some=102 none=0` で、実効 output latency（`playback - callback`）は min=0ns・max=31 972 789ns・mean=30 905 917ns（non-positive=3）だった。解釈として、mean 約 30.9ms は WASAPI shared-mode buffer 深さ（約 3 period ≈ 30ms @ 44.1kHz/441 frame period）に整合し、playback が callback より約 3 period 先の DAC instant を指すことを裏付ける。`output_latency()` は pre-start/post-stop とも `Ok(None)` を返し、cpal portable 経路の非 populate に対する正直な応答である。`wasapi_input.rs` の `map_win_error` に `E_ACCESSDENIED`→`Backend("permission_denied")` を追加し cpal 経路と parity を確保した。headless test で両 context を検証したが実機再現は NotRun である。初回 lane で 1 度だけ `UnsupportedConfig` の一過性 open 失敗を観測したが、以後再現せず環境依存事象として記録した (Issue #251)。

## 圧縮・構造化フォーマットの契約試験

圧縮フォーマットは、設計正本との照合で公開 API・状態機械・error 語彙・budget 意味論の設計不整合が確認されず、task の「既存実装の変更は設計不整合が確認できた最小限」条件に従い test/docs のみを変更した。追加試験は encoder 系（header write 失敗・output budget・flush after finish・failed encoder Drop が trailer/final block/epilogue を書かない・`finish` の underlying flush 失敗・連結 member・write after finish）と decoder 系（`finish` 冪等・chained decoder の member 境界 resume・read error latch・高圧縮率 decoded budget・`finish` mid-output は `NotExhausted` でない）を gzip/deflate/zstd 各系に追加した。`test-io-compression` は 5 組合せで exit 0・45 test-result ok、`cargo test --all-features` は 149+9=158 pass、`test-io-msrv`（rust@1.89.0）も pass した (Issue #285)。

構造化フォーマットでは、XML の feature matrix（default・xml・xml,file・all-features）が全て緑で 117 tests + doctests、clippy `-D warnings` は 0 warnings だった。負荷試験として DTD（internal/external）・未定義 entity・非 UTF-8・各 limit 分類・seeded fuzz 200 doc を `tests/structured.rs` で pin した。リスクとして、max_event_bytes は backend 内部 buffering 後の公開時検査であり確保前の厳密 cap ではなく、複数 root/top-level text は backend 忠実に受理される。document 構造検査は責務外である (Issue #299)。

圧縮の別 slice では、Windows 11 x64・rustc 1.99.0 で feature matrix 4 組合せ（default/gzip/deflate/gzip,deflate）の check/clippy/test が全て PASS、classify.test.mjs は 52 件全て緑だった。CPU 時間・backend 内部メモリの hard limit は宣言せず、累積 byte/unit budget のみを強制する。zip-bomb 対策は呼出し側の DecodeLimits に委譲される。compression-verify lane は shared-config 変更のため本 PR では Full CI で実行され、selective skip/run は今後の運用観測事項とされた。zstd を含む別 slice では check/lint/test-io-compression の 5 feature 組合せが全て exit 0（zstd で 34 format tests + contract/API 共有 suite + doctest）、verify-ci・test-ci-impact 52/52 も pass した。origin/main merge 後の生成 index 衝突は doc_maintenance.js apply の再生成で解決した。ubuntu-latest CI 上の zstd-sys C build はローカル MSVC+cc 成功のみが根拠で、最終証明は compression-verify lane の Required check に委譲された (Issue #300, Issue #301)。

## CSV モジュールの検証

CSV モジュールは契約試験 41 件と file 合成試験を追加し、`--all-features` で 77 契約試験 + 11 doctest が全て緑だった。feature matrix（`csv`/`csv,file`/全組合せ/無 feature/`file` 単体）で警告 0・clippy `-D warnings` 全緑だった。MSRV `rust@1.89.0 --all-features` で 41 試験 + 5 doctest が pass した。依存 DAG は csv→csv-core→memchr/itoa/ryu/serde_core で既存層外への新規依存はなく、license は Unlicense/MIT/Apache-2.0/BSL-1.0 で copyleft はない。`bench-smoke` で baseline 劣化はなかった (Issue #298)。

## アーカイブ reader/writer の試験

zip_reader は `cargo test -p ruviform-io-archive` で `tests/zip_reader.rs` の 24 件が全て pass した。truncated/malformed/inconsistent/overlap/unsupported/budget/CRC/partial-drop/verify_all を網羅し、clippy `--all-targets -D warnings` も pass した。zip_writer.rs は 10 件全て pass し（round-trip/poison/finish 分離/limit/measured size）、T3 で workspace 全体の fmt/check/lint/test が全て exit 0 だった（130 test binary 全緑、io-archive は zip_reader 32 + zip_writer 10 + doctest 7）。並行 worker が同時期に新規クレートを追加しており Cargo.toml/crates/README.md/Cargo.lock で merge 競合が予想されたため、自クレート追加分を保持して解消し、Cargo.lock は再解決で再生成する運用とした。index/README 類は `doc_maintenance apply --force-index` で機械再生成し、手編集 drift を避けた (Issue #302)。

## WAV の golden/fault 試験と旧 API 残存の確認

WAV golden/fault では `mise run fmt-check/check-io-e2e/lint-io-e2e/test-io-e2e`（77 tests 全 pass）/check/lint/test/test-rt を merge 後 HEAD で全実行・全 Pass した。wav_golden 8 件（ZeroFrames 44B・5.1 WAVEFORMATEXTENSIBLE 68B・S16 stereo・U8/S16/S24/S32/F32 encoder edge sample の NaN→0/±Inf/clamp/round-half-away/F32 verbatim・odd payload pad・classic RIFF 4GiB 境界）と wav_fault 9 件（Io 失敗分類・seal 後 abort・Drop best-effort finalize が completion 証拠でないこと・publish outcome classifier・CreateNew 競合・Replace・create 失敗の型付き FileErrorKind::Io）を追加した。ruviform-io-wav の src は無変更で、真の defect は未発見だった (Issue #305)。

cutover では 3 回の merge 後再検証を行い、fmt-check/check/lint/test が全て PASS した。of01..of06・staged_* は全て PASS し、旧 API 残存検査（StagedOutput・media-io 内 PublishOptions・mod staged/staged::）は各 0 件だった。merge commit 7ead144 で 166 test-result blocks・0 failed、combined merge ce8bc99b（RF401+RF402+RF407 候補取込）で 170 blocks・0 failed・test-io-e2e 31 件 pass だった。独立 Validator（read-only subagent）による cross-diff 検査は APPROVED で、RF305 scope 41 file のみ・main 新規成果への diff ゼロ・RF407 実装非改変を確認した (Issue #305)。

## fault-injection seam と corpus replay

fault-injection seam では、feature 無しの check/lint/test-io-file で既定 build 不変を確認し、test-io-e2e で fault_seam 8 件が pass、corpus_replay は `#[ignore]` で通常 lane 非実行であることを確認した。test-io-e2e-corpus の opt-in は 432 cases・panicked=0・elapsed≈22.5ms だった。io-structured/io-compression 専用 lane は省略した。変更は io-file の cfg/feature 追加のみで公開 API 不変であり、test-io-e2e が fault-injection 有効の io-file を含む下流一式を compile+test 済みのため、比例検証の方針に従った (Issue #306 G07/G08)。

## composition-e2e と consumer gate の状態

test-io-e2e は 31/31 pass、workspace test（merge head a99e953）は 2773 passed/0 failed/1 ignored だった。RF #305 候補 82256cf… の scratch worktree run で 31/31 tests が候補 revision 上で同一緑となり、fixture を候補 worktree へ uncommitted 接続して実走した。NotRun は io-compression/io-structured feature matrix lane・両 Consumer 実 runner（Blocked: AL127 修復・SV e815384 候補の旧 API 参照残存 — coordinator 管理）・実 fault 強制再現・macOS/Linux である。main ccf61a2 既知の plugin-host 起動 Timeout flake は本 run 未発生で、失敗未観測のため分類対象はなかった (Issue #306 composition-e2e)。

## OS 別 CI lane の io 系実走証拠

Full CI run 43428172820 の io 系 OS-L1 実走証拠として、対象 SHA e93c438a の Full CI（completed/success）生ログを解析し evidence/rf306/ci-os-l1-report.md に記録した。verify 3 OS（ubuntu/windows/macos）で fmt-check+lint-full+test-full が全て success だった。io-e2e 77/0f・io-file（1 ignored=意図的）・io-archive 172・io-compression 126・io-structured 150・io-wav 38・io-symphonia 50・media-io 66、全て 0 failed だった。compression-verify は 5 feature 組合せで deflate 含め実走（ubuntu のみ）、io-structured-verify は 4 組合せ、msrv-verify は rust@1.89.0 で io 系 6 crate+deflate 補完、rt-verify は rt_ filter で io binary は compile/link・filter 実行だった（io-e2e の一致は assert_ 誤検出のみ）。deflate は io-e2e 非消費のため verify lane では 0 で、ubuntu lane のみ実走として分離記録した (Issue #306 slice V)。

## post-seal temp 契約の pin

staged_lifecycle.rs に 3 Case を追加し io-e2e を 77→80 tests に補完した。post_seal_temp_rewrite_is_not_reverified_before_publish は post-seal temp rewrite が publish で再検証されず改竄 bytes が destination に publish されることを契約固定した。post_seal_temp_delete_fails_publish_not_published は temp delete→NotPublished・destination 非作成・recovery cleanup が Absent で成功することを確認した。recovery_cleanup_failure_returns_handle_for_retry は cleanup 失敗→handle 返却→retry 成功 TempState::Absent を確認した。post-seal temp 改竄は publish で検出されない。これは seal 時点保証の境界であり、io-file の再検証強化は設計判断として別 slice とし、本 slice は現行契約を test で固定した (Issue #306 slice T)。

## durability ladder の実 FS probe

os_fs_probe は CreateNew/Replace × BestEffort/FileSync/FileAndDirectorySync の 6 通りを実 FS で probe し、DurabilityEvidence が要求 level と一致するか、PublishFailureKind::Unsupported+NotPublished で正直に拒否されることのみを assert した（silent downgrade 検出）。Windows 11 x64/NTFS で 6/6 一致を実測した。macOS/Linux・Network/Cloud FS・性能は NotRun/Unsupported である。ruviform-io-e2e に 29 件追加し計 60 件となり、mise fmt-check/check-io-e2e/lint-io-e2e/test-io-e2e と workspace check/lint/test/test-rt は全て exit 0 だった (Issue #306 slice-2)。

## feature-matrix lane の実測値

9 本の scoped feature-matrix lane（check/lint/test-io-compression・io-structured・io-e2e）と workspace 最終検証 5 本を全て mise run で実走し全 lane Pass した。コード・テスト・設定は一切変更しない検証・証拠 slice である。compression test は 5 組合せ合計 264 passed（default 3・gzip 52・deflate 26・zstd 58・gzip,deflate,zstd 125）、structured は 4 組合せ 251 passed（default 5・xml 47・xml,file 49・all-features 150）、io-e2e 77、workspace test 3696 passed/0 failed/7 ignored、test-rt 288 だった。組合せ別 test 数は log の機械集計（cargo test 起動境界で分割+awk）で記録し推測ではない (Issue #306 slice-4)。

## G01/G05 canonical lane の実走と範囲外の既存破損

G01 canonical lane は Pass 15/15 case・1008.22s・preflight probe OK（required=4,563,402,753 bytes、NTFS temp_dir）だった。G05 canonical `mise run bench`（cargo bench --workspace）は Fail exit 101 で、本 slice 範囲外の既存破損だった。ruviform-numeric lib test が bench profile でコンパイル不能であり、`cargo test -p ruviform-numeric --lib` は 116 tests pass で bench-profile 固有と確認した。lease 外のため未修正で、evidence/PR に記録した。G05 補助 `cargo bench -p ruviform-io-e2e --bench io_composition` は Pass で全 6 cell 完走、peak_delta 最大≈1.34MiB（1GiB bound 内）だった。self-review で staged_publish cell の destination 不整合（bound dest publish.bin vs remove 対象 published-N.bin の不一致で初回 iteration panic する潜在 bug）を 1 件 CONFIRMED・修正済みである (Issue #306 G01/G05)。

## install_plan の破壊的変更と検証

install_plan の戻り値は `Result<(),QueueFull>` から `Result<(),InstallPlanError>`（`#[non_exhaustive]`）へ変更された。error 型を名指しする downstream は `InstallPlanError::QueueFull` への機械移行が必要で、本 repo に該当はなく crate 内 callers は全て `.unwrap()` 経由で source-compatible だった。foreign-base plan の install は以前は照合なしで stage されていたが、新たに FrameBaseMismatch で拒否される挙動変更（意図的な fail-closed 化）となった。mise run fmt-check/check/lint/test/test-rt は全て exit 0、frame_base.rs は 6/6 pass、116 test groups 0 failed、doc lint 0 findings だった。実 plugin/実機 OS・consumer 実 repo ビルドは NotRun であり、#219/#294 cross-consumer lane は coordinator/PR 側である (Issue #307)。

## flaky の切り分けと consumer 側の実績

`mise run test`（workspace）の ruviform-plugin-host l1_proxy_e2e flaky は既知の process-boundary timing 問題で、docs のみの差分とは無関係だった。失敗 test は実行ごとに異なり deterministic な回帰ではなく、`--test-threads=4` で 184/184 pass を確認した。consumer 側では Synvelis PR #180（RootedDir 消費）・PR #181（with_borrowed_demuxer 消費+独自 ReadProxy/channel/scoped-thread 撤去）が MERGED し、SV 検証記録は l2_e2e 17/17・x_e2e 5/5・pc_b 7/7 pass だった。RF↔SV 候補 tuple 互換 gate は coordinator 管理の cross-repo gate で NotRun である。unix/macOS runtime の rooted capability 実測は依然 NotRun で、check-linux は型/lint のみだった (Issue #398/#400)。

## bench-smoke の実測とドキュメント照合

`mise run bench-smoke` が ruviform-io-e2e io_composition を計測なし 1 回実行することを実測確認した（exit 0、6 case 全て Testing...Success: staged_write/seal/publish/readback/tar_gzip_staged/tar_zstd_staged）。docs/development-environment.md L20/L21 の bench 系記述は mise.toml `[tasks.bench]`/`[tasks.bench-smoke]` と一致し drift はなかった（同ファイルは #720 占有のため read-only 照合に留めた）。fmt-check・check/lint/test ruviform-io-file・check/lint/test-io-e2e（post_seal_temp_* 2 試験を含む既存 suite 全件 ok）は全て pass した (Issue #721)。

## plugin_dock の fail-closed degrade 経路の実効確認

plugin_dock.rs の fail-closed degrade 設計（:137-149 early-return）に対し、ruviform_plugin_test_vst3.dll が target/debug/deps/ に存在し `artifact::resolve_artifact` で解決されるため、fixture_run_reports_honest_verdicts は degrade 経路ではなく full assert 経路を実行した。:168,:211 の "test-plugin" assert が実効的に検証された（degrade のみの NotRun ではない）。check/lint/test ruviform-audio-test・ruviform-plugin-dock-test（4/4）・test-ci-consumer-verify 84/84・fmt-check・prek run --all-files は全て exit 0 だった。scenario.rs:280 の `json!(kind.as_str())` は `as_str()` flip に自動追従し編集不要を確認した (Issue #725 Phase B)。

## 記録の範囲と限界

本書が記録する検証には、実行されなかった項目と後続へ委譲された項目が含まれる。以下に、素材が NotRun・Deferred・unverified・Blocked と記録した範囲を列挙する。

実機・実行環境に関する未実施項目として、Issue #251 の macOS/Linux 実機（Stage M/L）・物理 USB 抜去・実 OS 権限拒否・xrun 誘発・consumer 側 frame 相関は全て NotRun/Deferred のままである。`E_ACCESSDENIED` の実機再現も NotRun である。Issue #296 では unix（renameat2/renamex_np/link fallback）・readonly dir 誘導の実測が本ホスト不可で CI OS matrix に委譲され、capability matrix の未検証セル（FAT/exFAT・ネットワーク FS・dir sync 非対応 FS）は「保証が効くか」を主張していない。Issue #306 では macOS/Linux・Network/Cloud FS・性能が NotRun/Unsupported で、G02 Network/Cloud FS・G03 特殊 FS・G04 実 4GiB 以外の計測・G06 consumer gate・macOS/Linux 実機の残存受入行も NotRun/Deferred のままである。Issue #288 の matrix の穴 7 件（実 4GiB・Network/Cloud FS・非既定 FS・macOS/Linux 実機・性能比較・consumer 固有受入残・fault fixture 化）は新規 Issue 起票待ちで解消状況は不明である。Issue #286 の `tar.zst` 合成個別試験・unix backend の実 FS 実行試験（TOCTOU）は未実施のまま残っている（gzip で契約 pin 済み・Linux CI lane 依存）。

consumer・cross-repo に関する未確定項目として、Issue #197/#244 時点の audio-lens consumer impact は `unknown`/`unverified` で、Issue #249 で `no impact` に確定した。ただし audio-lens repo 自体の検証は対象外である。Issue #306 composition-e2e の両 Consumer 実 runner は Blocked（AL127 修復・SV e815384 候補の旧 API 参照残存 — coordinator 管理）、実 fault 強制再現・macOS/Linux も NotRun である。Issue #307 の実 plugin/実機 OS・consumer 実 repo ビルドは NotRun で、#219/#294 cross-consumer lane は coordinator/PR 側である。Issue #398/#400 の RF↔SV 候補 tuple 互換 gate は coordinator 管理の cross-repo gate で NotRun、unix/macOS runtime の rooted capability 実測も依然 NotRun（check-linux は型/lint のみ）である。

その他の留保として、Issue #300/#301 では ubuntu-latest CI 上の zstd-sys C build の最終証明は compression-verify lane の Required check に委譲されており、selective skip/run は今後の運用観測事項である。Issue #299 の io-structured 用 CI lane（job+impact map）整備は別 Issue 起票予定とされ、起票の有無は不明である。Issue #306 G01/G05 で発見された ruviform-numeric lib test の bench-profile コンパイル不能（exit 101）は lease 外で未修正のまま後続の修正状況は不明である。Issue #306 の監査証跡は実行時点の snapshot であり、依存 DAG 変更時は再実行が必要である。Issue #398/#400 の Close 判断（cross-repo gate・実 OS 行の最終判定を含む）は Issue owner の責務であり、本記録は充足証拠の提供のみで実際の close 状況は不明である。

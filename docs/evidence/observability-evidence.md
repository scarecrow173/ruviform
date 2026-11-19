---
id: "EVI-aBRXF4U03hPD1uYHzs9ppF"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "Observability・probe の検証記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/56", "https://github.com/scarecrow173/ruviform/issues/104", "https://github.com/scarecrow173/ruviform/issues/138", "https://github.com/scarecrow173/ruviform/issues/185", "https://github.com/scarecrow173/ruviform/issues/275", "https://github.com/scarecrow173/ruviform/issues/276", "https://github.com/scarecrow173/ruviform/issues/278", "https://github.com/scarecrow173/ruviform/issues/280", "https://github.com/scarecrow173/ruviform/issues/281", "https://github.com/scarecrow173/ruviform/issues/289", "https://github.com/scarecrow173/ruviform/issues/290", "https://github.com/scarecrow173/ruviform/issues/291", "https://github.com/scarecrow173/ruviform/issues/292", "https://github.com/scarecrow173/ruviform/issues/293", "https://github.com/scarecrow173/ruviform/issues/308", "https://github.com/scarecrow173/ruviform/issues/309", "https://github.com/scarecrow173/ruviform/issues/429"]
  informs: []
  explains: []
  supersedes: []
---

# Observability・probe の検証記録

本書は、観測基盤（observability）と型付き probe に関する検証・実行の結果を記録する。記録の対象は、後に削除された過去の実装記録から抽出したものであり、本書はそれらの記録に依存せず単独で読めるように構成している。各記述には出典の Issue 番号を付記し、計測値や観測された挙動などの事実と、そこからの解釈を区別して記す。解釈を含む記述は「解釈」と明示する。

## RT スナップショットとバックエンド境界の検証

RT 側の可変状態をスナップショットとして公開する仕組みは、`cargo test -p ruviform-rt` で単体テストと doctest が合格し、`rt_` 接頭辞によるテスト選択が機能することを確認した (Issue #21)。publish 後に `read` が新値を観測すること、guard 解放が RT 側の dealloc を行わず `collect` 経路で解放されること、publish で追い出された `Arc` が drain 経路で解放されることを、それぞれ専用テストで固定した。counting allocator を使った検証では、`read` と guard drop の確保が 0 であることを確認した (Issue #21)。レビュー対応では、retire チャネルの共有カウンタを `SnapshotWriter::retire_overflows()` として非 RT 側の監視経路へ露出し、writer drop 後も `read` が最終公開値を返す endpoint 独立性を `rt_snapshot_reader_outlives_writer` で確定した (Issue #21)。`arc_swap` 型が公開シグネチャに出ないことは `pub` の grep で確認し、`mise run lint` は 0 warnings、`fmt-check` は差分なしだった (Issue #21)。

バックエンド選定の ADR は、検証項目 6 件すべてに証跡を対応付けて `proposed` から `accepted` へ昇格した (Issue #23)。`mise run` の fmt-check・check・lint・test・test-rt・bench がすべて exit 0 で、`prek run --all-files` の全 hook も合格した。`cargo metadata` で cpal が `ruviform-audio-device` の依存にのみ存在することを確認し、`cargo doc -p ruviform-audio-core` の出力に cpal 由来の型・エラー・feature 名が 0 件であることを `#[non_exhaustive]` lint ゲートで確認した (Issue #23)。spec の受入条件 6 件（候補比較の文書化、外部型の非露出、分離クレートへの cpal 閉じ込め、デバイス非依存テストのみの CI、コールバック非 alloc 契約と `assert_no_alloc` 検証）はすべて充足と記録された。ALSA 対 PipeWire、Send/Sync、WASAPI exclusive/ASIO、初回 timestamp=0 という未観測項目は ADR の Confirmation 節へ引き継がれた (Issue #23)。

## RT 観測コストの計測と計測構成の是正

RT 経路に載せる観測機構のコストは、criterion によるベンチマークで計測された。計測環境は Windows 11 x64・i9-9900K・rustc 1.99.0・criterion 0.8.2 で、単一スレッド・非競合のレイテンシ推定値として 3 run の平均を取った (Issue #104)。

計装のコストは、等しいワークロードを全列に課した `counter_equal_work` 構成で計装デルタ +3.99 ns/op だった。`flag/on` は `always` とほぼ同値（5.389 対 5.342 ns/op）だが、`flag/off` は disabled 比で +0.74 ns/op の残存ゲートコストがあり、「ゲートは無料」とは解釈しない結果となった (Issue #104)。`overflow_policy_nonzero` では drop が 1.52 ns/op、count・aggregate・retire_leak が 5.41〜5.43 ns/op で同一コスト級だった。タイムスタンプは `Instant::now` が 29.2 ns、`timestamped_record` が 39.4 ns（clock 読出しが 74%）で、`SystemTime::now` の 15.0 ns は壁時計のため RT 経路には不適と判断された (Issue #104)。run 間の |max Δ%| は p50=0.4%、p90=16.4%、max=38.5% で、小さい絶対値のセルにノイズが集中した (Issue #104)。

初回計測はレビューで計測構成の欠陥 2 件が確認され、該当 2 グループのみ再構成して再計測された。`counter` 群は disabled 列だけが wrapping_add と black_box のワークを持つ非 equal-work 構成であり、`overflow_policy` の aggregate は値 0 の記録を fold して `fetch_add(0)` となり方針コストを判別できなかった (Issue #104)。`counter_equal_work`（全列に同一の base workload を課し計装 step の有無だけを差分にする）と `overflow_policy_nonzero`（非ゼロ値 `POLICY_REC` を記録）へ再構成し、revision `matched-work-nonzero-v2`・harness_sha256 `b0be85ed` で 3 run 再計測した。初回値は初期条件データとして保存し、「+2.5 ns/op の加法計装コスト」「ゲート実質無料」「count 対 aggregate の約 5 倍の非対称」「最悪値約 5.4 ns/op で有界」といった解釈はすべて撤回された (Issue #104)。

計測結果に基づく採否の判断は次のとおりである (Issue #104)。telemetry の prepared Counter は equal-work 上 +3.99 ns/op の synthetic バッチ単一スレッド推定値で、有力候補として維持されたが、採否は trigger Issue の判断に委ねられ、新クレートの新設は consumer・DAG・ADR のトリガまで deferred とされた。rt_bridge は設計どおり drop-on-full + 共有カウンタを支持し、count 方針を選ぶ根拠はコストではなく集約セマンティクスの要否のみと整理された。metrics facade は集約需要が未発生で facade の RT 検証もないため延期、tracy-client 等の profiling は需要なし・RT 検証なし・C++ ツールチェイン要件で延期、logging/tracing は RT 経路に Subscriber コールバックを置けない構造的理由で RT 不採用とされた。タイムスタンプは `Instant` から epoch 相対 nanos を経て `i64` にする方式が採用可能だが、29〜39 ns/stamp は全件スタンプに重いため「呼出し側が epoch を提供し観測点のみスタンプする」方式が推奨された (Issue #104)。

計測の再現性と留保事項も記録されている。バックエンド候補の宣言 MSRV は 1.65〜1.71 だが推移的な実効 MSRV は未検証で、保守状況は暫定評価（レジストリ出典の日付なし）とされた。マイクロコスト値は暫定・非アーカイブの観測値であり、採否の根拠には使わないと明記された。tracy-client-sys は bundled Tracy（BSD-3-Clause）の C++ ツールチェインを要し、0.18.5 で観測した `SymInitialize FAILED` は 0.19.0 で未再現だった (Issue #104)。dev-dep 導入前のコンパイルコスト baseline は検証可能な記録が保持されておらず、before/after 差分は未検証として計測欠落の逸脱を受理した。MSRV 検証は `rustup run 1.89 cargo check -p ruviform-rt --lib --locked` に限定し、dev-dep を `--lib` 範囲外とする封じ込め検証形を取った (Issue #104)。

## 観測基盤の受入照合と証拠の引き継ぎ規則

観測基盤の受入条件は、実装済み Issue の棚卸しと証拠の引き継ぎ規則によって管理された。2026-11-14 時点の `gh issue view` 実測では、実装 Issue #276〜#280・#289〜#293 はすべて CLOSED で、OPEN は検証 Gate の #281 と #294、#219、AL #127、SV #28 だった。未着手の実装 Issue は存在せず、残件はすべて検証・Gate 領域のため新規実装 Issue の起票は不要と結論された (Issue #275)。

受入台帳には証拠の引き継ぎ規則が定められた。`git diff 365c74ac..20d8e7c2` で `ruviform-observability`・`ruviform-rt`・`ruviform-media-core`・`ruviform-audio-core`・`ruviform-video-core` は diff ゼロのため、E-CI-1（run 43439434737 @`365c74ac`、verify 3 OS + rt-verify + msrv-verify 全 success）の証拠を「同一コードへの証拠」として引き継いだ。一方 `ruviform-plugin-host`・`ruviform-plugin-core` は diff があり、OBS-04 の `365c74ac` 証拠は stale と判定された。対象 Revision `20d8e7c2` 自体の CI・consumer-verify run は未発生で、HEAD での統合検証はすべて `NotRun` だった。ローカル実行の結果は Windows・dev toolchain の補助証拠であり、CI lane の Pass や統合受入の Pass を意味しないと明記された (Issue #275)。

台帳の Deferred 表が旧方針の `#313/#314` 系 real-window lane を参照したまま残っていたため、Virtual-first 方針へ再指向する追記が行われた。実 OS GUI 行の再開 Owner を `RF #293 + #631 + #630 + AL#127/SV#28`（方針基盤 #627）へ、実 Device 行を `RF #281 + #629 + #628 + device 利用機能 Owner` へ更新した。#628〜#631 はすべて OPEN で Harness 実装・lane は未完のため、この更新は Owner・Trigger の指し先を現行方針へ合わせるものであり、当該 L2/L3 Case の実行済みを意味しない。未検証は `NotRun` のままとされた (Issue #275)。

## logging adapter の検証

`--features logging` の logging adapter は、`cargo test -p ruviform-observability --features logging` で 13 unit + 19 contract + 3 doctest が全件パスした (Issue #276)。contract test は dual sink の filter routing、target 別 filter、JSONL/Text format、retention（seeded 5 件に対し `max_files=3` で古い順削除）、Required/BestEffort/全滅/空 sinks/不正 filter、guard drop による drain+join、global 未設定、shutdown 後 emit の `channel_disconnected`、snapshot の非再帰、`build_logging_layers` の host registry 合成、multi-thread emit、片 sink の shutdown で他 sink が継続することを網羅した (Issue #276)。

MSRV は `rust@1.89.0` で check/clippy（`-A clippy::float_cmp -A clippy::similar_names`）がパスし、`--no-default-features` で normal dep がゼロであることを確認した。example `logging_host` を `2>/dev/null` で実行して stdout が 0 byte であることを確認した。stdout 非出力は `SinkDestination` に経路が存在しない構造的保証によるものである (Issue #276)。

## tracing-export の検証

`--features tracing-export` は 2 つの slice に分けて検証された。slice1 では `cargo test -p ruviform-observability --features tracing-export` で 156 unit + 8 doctest が全件パスし、default build の `cargo tree -e normal` に `tracing` が含まれないことを確認した (Issue #278)。レビューで CONFIRMED の P1 が 2 件検出・修正された。`tracing::Level` の `Ord` が verbosity 順であり severity 比較に `max` を使うと全 Level が逆転する問題を `min` へ修正し、test-spec R-8 の dedup 未充足（`min_delta=0`/`min_interval=0` で無変化 poll・同一 batch の再 emit が重複発行し得た）に対して無変化観測の抑制 gate を追加した。P2（clippy pedantic、test fixture の lease 落とし）も修正された (Issue #278)。

slice2 では record export と trace_context が追加され、同じテストコマンドで 233 lib + 11 doctest が全件パスした（`tracing_export` 32 tests = slice1 の 15 + record 9 + trace_context unit 8）(Issue #278)。受入照合では受入 8 行をコードと試験で照合し、in-repo 証拠ギャップの 2 件の contract test を追加した。`rt_record_trace_context_is_metadata_not_correlation` は valid な traceparent が unlinked 分類となり span 関係 field の非存在を変えないことを、`rt_record_confidentiality_is_reported_not_enforced` は Internal/Confidential 分類の忠実な報告と値の非秘匿を固定した。`mise run test-observability-msrv`（rust@1.89.0 `--all-features`）で lib 290 + integration 15 + logging 28 + rt_boundary 3 + doctest 15 が全件パスした。実装 API・schema・`event_version` は無変更だった (Issue #278)。

## rt_bridge / record の検証

rt_bridge と record の経路は `cargo test -p ruviform-observability --all-targets` で unit 60 + doctest 6 がパスし、`mise run test-rt` で `rt_bridge::tests` 全件と `rt_try_record_and_counters_allocate_nothing`（`assert_no_alloc` 窓で受理・drop 両パス）がパスした (Issue #280)。pinned 証拠として、`full_ring_drops_and_reports_positional_loss`（位置付き損失）、`bridge_delivers_fifo_with_zero_dropped`、`consumer_lag_is_delay_not_loss`（lag は損失ではない）、`counters_survive_as_registrable_readers`（OBS-02 互換）、`obs02_lifecycle_retire_final_drain_removed`、`event_record_travels_the_bridge`（clock/correlation の輸送）、`recorder_drop_leaves_sink_drainable`、`bridge_cross_thread_delivery`、`zero_capacity_panics`、`endpoints_are_send` が挙げられた。依存方向は observability→rt/media-core の一方向で逆依存はなかった (Issue #280)。

## feature matrix lane の検証

feature 組合せの lane は、Windows 11 x64・dev toolchain で `mise run check-observability`/`lint-observability`/`test-observability` の 6 組合せすべてが exit 0 だった。`--all-features` の test では lib 298・integration 15・logging 28・rt_boundary 3・doctest 15+1 ignored を確認した。`mise run check|lint|test ruviform-observability`・`fmt-check`・`actionlint`・CI 契約の node test（ci-yml-contract 10/10・summary 31/31・consumer-verify 64/64）もすべて Pass だった (Issue #281)。

CI（workflow_dispatch）の実走は `NotRun` である。dispatch-only のため push/PR では起動せず、lane 定義とローカル実走は統合受入の Pass を意味しない。対象 SHA の CI 証拠・consumer tuple・実機 L2/L3 は merge 後の Daily/手動 dispatch が正本とされた。台帳の G0 行は lane 定義済み・CI 未実走として `NotRun` を維持した (Issue #281)。

## Collector / Metrics の検証

Collector と Metrics は `cargo test -p ruviform-observability` で unit 110 + doctest 6 がパスし、Post-Review Fix で終端 drain batch 回帰 6 件が追加された。`mise run test`/`test-rt`/`check`/`lint`/`fmt-check` はすべてパスし、`--features logging` と `--no-default-features` の両 build で collector が logging/tracing 型に依存せず snapshot/health を利用できることを確認した (Issue #289)。pinned 証拠として、Counter の baseline/rate/実測 window、減少時の Discontinuous、Gauge の 0/Unavailable 区別、固定 bucket Histogram、Active→Retiring→FinalDrain→Removed の最終 drain、lease drop の abandoned drain、forced shutdown の `Partial`+`WriterNotQuiesced`、mid-poll の lock-free retire と同 poll drain、tombstone、`max_batch_size` の round-robin、key/cell の重複拒否、bounded history、oversized drop、mutex poison recovery、`PollRunner` の定期 poll が確認された。merge 後に `origin/main`（ad99d18）で全 lane を再実行して Pass した (Issue #289)。

残リスクとして、Linux/macOS は未実測、cross-repo Consumer gate（#281）は OBS-11 の責務で `NotRun`、Histogram bucket 読取は複数 atomic の非同時観測でトランザクション整合を持たない（契約で明示）と記録された (Issue #289)。

## allocation-probe / rt-timing の検証

allocation-probe は `cargo test -p ruviform-observability --features allocation-probe` で 34 profiling tests + doctest がパスし、workspace test は 181 suite で 0 failed だった。allocator 16 + scope 13 + stats 4 + doctest 1 の内訳で、`cargo tree -e normal --features allocation-probe` では新規外部依存がなかった。OBS-09 diagnostics との併存を origin/main merge 後に確認し 175 tests が PASS した。`#[global_allocator]` は install せず host 責務とし、テストは既存 `AllocDisabler` を global allocator に持つ lib test binary の制約から `CountingAllocator` の `GlobalAlloc` メソッドを直接呼び出して計測した。受入照合 audit では RT Timing 行のみ未充足と判定され、別 slice へ回された (Issue #290)。

rt-timing は `src/rt_timing/tests.rs` の 21 試験（assert_no_alloc・drop 帰属・clock anomaly 分離・budget 意味論・generation・Collector→MetricValue::Histogram）を含む mise 5 lane が base 0d07471d で全 PASS した。`mise run test` の初回は `ruviform-ui-test` lib test の rustc が `STATUS_STACK_BUFFER_OVERRUN`（OOM 系 flake、当該 crate は本変更と無関係）で失敗したが、`CARGO_BUILD_JOBS=4` で再実行して全緑になった。review-protocol は CONFIRMED P0/P1 なしで承認扱いだった。`assert_no_alloc` は dev counting allocator 依存であり、実機 jitter の保証を意味しないと明記された (Issue #290)。MSRV 1.89 の直接 build は toolchain 1.99 pin のため未実測で、使用 API はすべて 1.89 以前に安定化しており MSRV lane は #223 の責務とされた (Issue #290)。

## diagnostics の検証

Diagnostics は `src/diagnostics/` 8 ファイル・tests.rs 30→31→32 tests で、mise 5 lane が Windows 11 x64 / rust 1.99.0 で全パスした。2026-11-09 audit（base 0d07471d、PR #504 実装）と 2026-11-10 の残件確認 audit（base 1dc235b9）の 2 回で受入 8 行を照合し、具象 `tracing_export::TracingFindingSink`（PR #507/slice2）の着地を確認した。残件 slice では `EvidenceTime::Clock` verbatim 保存の golden 試験 `evidence_clock_domain_stamp_is_preserved_verbatim` を 1 件追加した (Issue #291)。

review-protocol で CONFIRMED の P1 が 1 件検出・修正された。source generation 変更時に confirmed state が transition なしで Healthy に reset される欠陥を、streak のみの reset に修正し、回帰テスト `generation_change_never_fabricates_healthy_over_confirmed_bad` を追加した。残 P3 non-blocking として finding upsert が source を含まない点が挙げられた。両 Consumer の Evidence/Health ユースケース検証（#281）は cross-repo gate として `NotRun` のままだった (Issue #291)。

## debug inspector の検証

Debug Inspector は `src/debug/` 10 ファイル・tests.rs 35→39→45 tests で、mise 5 lane が全パスした。初回 clippy で 19 件の指摘（unused_self/single_match_else/manual_let_else 等）を全て修正して lint をパスさせた (Issue #292)。review-protocol で CONFIRMED 4 件を検出・全て修正した。(1) P1: `ExportLimits::max_items==0` が文書の「summaries only」に反し無制限収集となる問題を、0 の場合 record 収集自体を skip する形に修正。(2) P1: `DiagnosticReport::source_generation==0`（never evaluated sentinel）を実 generation として比較し偽の Skewed となる問題を、0 は absent 扱いに修正。(3) P2: consistency で `Unknown` が確定済み `Skewed` を上書きし得た問題を `Skewed>Unknown>Aligned` の優先順位に修正。(4) P2: `findings_total` が domain-health entry を含み水増しする問題を、findings のみ計数に修正した (Issue #292)。

2026-11-10 slice で受入関連の未検証要素を固定する試験 6 件を追加した（`batch_generation_skew_is_declared_not_hidden`・`drain_waits_for_quiescence_then_recovers_rt_records`・`draining_rejects_attach_activate_and_prepare`・`report_external_export_permitted_when_allowed`・`report_export_limits_bytes_and_deadline_truncate`・`rt_debug_value_types_allocate_nothing`）。実 `rt_bridge` 上の quiescence 未確認では `drain_final` を実行せず in-flight を保持することを検証した。AudioLens/Synvelis 両 Consumer 互換（#281）と concrete probe adapter（#293/ruviform-audio 側）は `NotRun`/別 Issue として明記された (Issue #292)。

## integration feature / ProbeSource adapter の検証

OBS-11 の integration feature は default off で、`cargo clippy/test -p ruviform-observability --all-features` と `integration`/`integration,logging`/`integration,tracing-export` の各組合せが全てパスした（lib 250 + integration contract 11 + 全 feature test binary）。default off のため canonical lane が feature-gated code をカバーしない点を補うため、これらの組合せを個別に実施した。review-protocol で CONFIRMED P2 を 1 件修正した。Stopped→prepare restart 後に drain 済み logging attachment が残り `attach_logging` が AlreadyAttached で永久拒否される問題を、prepare で attachment をクリアし回帰試験を追加して解消した。判定は ACCEPT WITH NON-BLOCKING NOTES だった (Issue #293)。

後続 slice で `test-observability-msrv`（`mise exec rust@1.89.0 -- cargo test -p ruviform-observability --all-features`、`CARGO_TARGET_DIR=target/msrv-observability`）を test-ofx-msrv 先例に倣って追加し、Rust 1.89 で実走 green（288 unit + 15 integration + 28 logging + doctests）を確認した。adapter 側は `typed_probe_source.rs` 8/8・`integration.rs` 9/9 がパスした。`mise run test` 初回で `ruviform-plugin-host` の `wedged_child_write_converges_and_teardown_is_bounded` が spawn timeout で 1 件失敗したが、単独・workspace 再実行ともパスし、本 slice 無関係の timing flake と判断された (Issue #293)。

## typed probe rebind の検証

probe の rebind atomic 性は `tests/typed_probe.rs` で検証された。origin/main clean merge 後に fmt-check/check/lint/test/test-rt が全 exit 0（Windows 11 x64 / cargo 1.99.0）で、`tests/typed_probe.rs` は 18/18 パス（新規 6 件を含む）だった。`test-rt` は 135 groups で `rt_probe_process_allocates_nothing` の非退行を確認した (Issue #308)。

新規 6 件の内容は次のとおりである。queue 飽和点の 5 パターン（capacity 8 に対し Begin 直前/直後/Stage 途中/Commit 直前/成功）で部分適用ゼロ・`Unbound` 表面化・drain 後 retry で全件 commit を検証した。連続 republish で superseded 世代の record が 0 で `stale_commands` が計上されること、unresolved point の `receipt.unbound`+`RebindDropped`、capture の `StopAtBoundary`/`Restart`、republish+rebind 前後で sink 出力が bit 一致することを確認した (Issue #308)。実デバイス/実 plugin 経路の probe 観測と consumer repo（synvelis/audio-lens）からの `on_publish` 呼出は `NotRun` で、現時点の caller は ruviform 内テストのみだった (Issue #308)。

## probe 時刻整合契約の検証

probe の時刻整合契約（PROBE-TIME）は `tests/typed_probe_time.rs` 23/23 と `tests/typed_probe_time_align.rs` 11/11 でパスし、既存 `typed_probe.rs` 18 件は非退行だった。workspace は 181 test groups で 0 failed、`rt_probe_process_allocates_nothing`・`set_clock_mark_is_alloc_free_on_rt_path`・`rt_typed_process_no_alloc` で RT no-alloc の非退行を確認した (Issue #309)。

align fixture は実 `GraphRuntime`+`RtRuntime`+実遅延 `DelayDsp`+決定的 fixture で、latency 0/64/128 の 3 probe が `Aligned{SharedCursor}`、offset=latency 差、epoch bump→`Incomparable{EpochMismatch}`、generation 跨ぎ→`Incomparable{GenerationSpan}`/緩和 policy で `Conditional{CrossGeneration}` を確認した。PCM は純粋 delay ゆえ `to_bits` で bit-exact、Peak/RMS は f32 累積と f64 独立期待の丸め差を許容した。`LostRecords`/`Unobserved`/`GenerationBoundary` class は実 runtime から非決定的なため contract unit 層で検証済みと棲み分けた (Issue #309)。

fixture 実装では `src/` 契約の不具合ではなく fixture 側の問題を 3 件発見・修正した。(a) 無音判定を cursor block index の `is_silent_block` から「独立 f64 期待 peak が厳密に 0」の latency 補正後観測窓基準に修正（delay 分ずれて無音でない区間を誤判定したため）。(b) telemetry loss fixture の capacity を block 0 の `Installed` status 3 + metric 3 = 6 record に合わせ 6 に設定（capacity 4 では status のみ残り metric 全損で coverage が空になった）。(c) `rt_` prefix test を `test-rt` lane 選択用に追加した (Issue #309)。

残リスクとして、`RecordMeta`/`StatusRecord` への `clock` 追加（16 bytes additive）の帯域影響実測は consumer 接続時課題として残り、device timestamp 未配線の live device では epoch が `UNKNOWN` のまま `Incomparable{UnknownClock}` に倒れる設計どおりの挙動が確認された。consumer repo 接続・実機検証は `NotRun` だった (Issue #309)。

## UI consumer observation の検証

UI 側の consumer observation は、外部 crate test `tests/consumer_observation.rs` 4 件と `[[test]]` 登録、`mise.toml` への `test-ui-consumer-observation` task 新設で検証された。F2/F3 で bound 近傍 payload・多チャンネル・viewport culling・gesture 中 `Notify` projection・`update_view` spec 差替・f64 precision の seam test を追加した。`cases.json` の gpui `pass` と `mise run test-ui-gpui`/`test-ui-consumer-observation` の実測が一致した（UI-DVR-01〜09）(Issue #429)。

既知の問題として、`cargo clippy -p ruviform-ui-gpui --all-targets --features gpui-test -- -D warnings` は既存 unit test 側の 2 件違反（`render.rs` doc comment backtick 欠落・`rvf_view_native_slot_public_api_cannot_starve_pushes` 行数超過）で失敗するが、`lint-ui-gpui` は `gpui` feature のみ対象のため mise 検証ゲートでは検出されない。本 PR 範囲外として未修正のまま記録された。残 DoD の実 GPU release（#313）、pixel golden（#314）、共通 Scenario 契約（#464）は他 Issue 帰属で `NotRun`、2 scale factor×2 theme の story GPUI lane は owner=#426 とされた (Issue #429)。

## SIMD dispatch probe のベンチマーク

数値計算側の dispatch 機構の計測として、`crates/ruviform-math/benches/simd_poc.rs` に `dispatch_probe` グループが追加された。`level_new`/`token_avx2`/`dispatch_noop` の部品プローブと、`add_{scalar,fearless_avx2,fearless_dispatched}` の N=16/32 小サイズ帯（固定コスト支配域の確認用）の計 9 case を計測した (Issue #56)。`dispatch!` は各 arm が異なる opaque `impl Simd` を返すため、`black_box(&simd)` で `()` に統一しつつトークン生成経路を保持した。`fearless_simd::{Level, dispatch}` の use 追加で既存の完全修飾 `Level::new()` が `unused_qualifications` になったため短縮形へ修正し、`cargo bench -- dispatch_probe --test` の完走を確認した (Issue #56)。

## 記録の範囲と限界

本書に記録された検証の多くは Windows 11 x64・dev toolchain 上のローカル実行であり、CI lane の Pass や統合受入の Pass を意味しない。`NotRun` または deferred として記録された主な項目は次のとおりである。

- 実時間経過の rotation・readonly-dir/disk-full・Linux/macOS は未検証で、CI 3-OS matrix と consumer 導入 Issue #138 側に委譲された (Issue #138)。
- ADR の未観測項目（ALSA vs PipeWire・Send/Sync・WASAPI exclusive/ASIO・初回 timestamp=0）は Confirmation 節へ引き継がれた (Issue #23)。
- telemetry の新クレート新設は consumer/DAG/ADR トリガまで deferred、metrics facade・profiling・logging/tracing の RT 経路採用は延期または不採用とされた (Issue #104)。
- 対象 Revision `20d8e7c2` 自体の CI/consumer-verify run は未発生で、HEAD での統合検証はすべて `NotRun`。ローカル実行は補助証拠に留まる (Issue #275)。
- CI（workflow_dispatch）実走は `NotRun` で、merge 後の Daily/手動 dispatch が正本 (Issue #281)。
- cross-repo Consumer gate（#281）と両 Consumer の Evidence/Health ユースケース検証は `NotRun` のまま (Issue #289, #291)。
- allocation-probe の MSRV 1.89 直接 build・実機 overhead 計測・OS/feature matrix・AudioLens/Synvelis 回帰は `NotRun`（MSRV lane は #223 の責務）(Issue #290)。
- `test-msrv`/`test-observability-msrv` は mise.toml に task が存在しない時期があり `NotRun` と記録された（Issue #278, #290, #309）。
- 実デバイス/実 plugin 経路の probe 観測と consumer repo からの `on_publish` 呼出は `NotRun`（Issue #308, #309）。
- UI 側の実 GPU release・pixel golden・共通 Scenario 契約は他 Issue 帰属で `NotRun`（Issue #429）。

また、`assert_no_alloc` は dev counting allocator 依存の検証であり実機 jitter の保証を意味しない点、Histogram bucket 読取は複数 atomic の非同時観測でトランザクション整合を持たない点は、契約または留保事項として明示されている (Issue #289, #290)。

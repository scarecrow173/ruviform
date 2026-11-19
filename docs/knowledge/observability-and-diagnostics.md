---
id: "KNOW-i48Bzxm2EvCJ6bemOgQsBl"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "Observability・probe・diagnostics の設計"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/104", "https://github.com/scarecrow173/ruviform/issues/121", "https://github.com/scarecrow173/ruviform/issues/185", "https://github.com/scarecrow173/ruviform/issues/275", "https://github.com/scarecrow173/ruviform/issues/276", "https://github.com/scarecrow173/ruviform/issues/278", "https://github.com/scarecrow173/ruviform/issues/280", "https://github.com/scarecrow173/ruviform/issues/281", "https://github.com/scarecrow173/ruviform/issues/289", "https://github.com/scarecrow173/ruviform/issues/290", "https://github.com/scarecrow173/ruviform/issues/291", "https://github.com/scarecrow173/ruviform/issues/292", "https://github.com/scarecrow173/ruviform/issues/293", "https://github.com/scarecrow173/ruviform/issues/308", "https://github.com/scarecrow173/ruviform/issues/309", "https://github.com/scarecrow173/ruviform/issues/429"]
  informs: []
  explains: []
  supersedes: []
---

# Observability・probe・diagnostics の設計

本書は、後に削除された実装記録群から抽出した、observability・probe・diagnostics 領域の再利用可能な設計知見をまとめたものである。対象は logging・tracing_export・collector・diagnostics・debug・rt_bridge・rt_timing・allocation-probe・integration を束ねる単一クレート `ruviform-observability` と、その consumer 側 adapter である (Issue #275)。各知見には出典の Issue 番号を付記する。測定値や観測された挙動は事実として記し、判断や推奨はその旨を明示する。

## RT 経路のメモリ解放

RT 経路で snapshot の dealloc が起きないことを保証するには、arc-swap 単体では不十分である。reader 側の last-drop が RT スレッド上で dealloc し得るため、`SnapshotGuard` の `Drop` が内部 `Arc` を inline drop せず `Retirer::retire` へ送出し、`publish` で追い出した旧 `Arc` も `DropCollector::push_local` で同一の drain 経路へ送る構成を採った。dealloc 地点は非 RT 側 collect の単一 drain 経路に統一される (Issue #21)。

`slot<T>` は内部に `arc_swap::ArcSwap<T>` を使い、`SnapshotReader::read` は lock-free load と `Arc` clone で alloc なしに読む。共有 `Arc<T>` の内容には `T: Send+Sync` を API 契約として明示した。`SnapshotGuard` は `Option<Arc<T>>` と `&'a RefCell<Retirer<Arc<T>>>` で構成され、型境界は構造から導出される。guard は `!Send`・`!Sync` となるが、否定 bound は stable では表明できないため doc とテストコメントで明記する規約とした (Issue #21)。

arc-swap の debt node は (スレッド×スロット) の初回 load で `Box::leak` により確保されるため、「read の no-allocation」主張は冷起動では成立しない。このため「RT 区間開始前に RT スレッドで 1 回 read して warm-up する」前提と、「reader は非 RT 側で drop するか writer 退役前に drop する」teardown 契約を module doc と read doc に明文化した。writer 先行 drop 後に RT 側で reader を drop すると live snapshot の `T` 破棄が RT スレッドで起きる。`retire_capacity>0` は debug_assert から release でも有効な `assert!` へ昇格した (Issue #21)。

## RT 契約テストの命名規約

RT 契約テストの関数名は `rt_` プレフィックスで始め、`cargo test --workspace rt_` lane がそれらを選択する。非 RT のテスト・doctest・ヘルパー関数名は `rt_` 部分文字列を含めてはならない。`assert_*`・`start_*` は禁止、`expect_*`・`spawn_*` は許可される。この規約は lane が意図しないテストを拾う事故や必要なテストを拾わない事故を防ぐための既存規約であり、observability の RT 境界テストでも踏襲されている (Issue #21)。

## 計測・ベンチマークの方法論

ナノ秒級の atomic 操作は criterion の分解能と同桁のため、1 iteration あたり 512 op のバッチ反復と `iter_batched_ref` で fixture を計測窓の外に出し、SN 比を確保する。「enabled/disabled」の評価はランタイムフラグではなく、同一操作の計装バリアント列を同一 run 内で比較し、run 内比率で環境ノイズを相殺する形を採った (Issue #104)。

criterion の `estimates.json` は run ごとに上書きされるため、「run 実行→集約→次 run」の順序が必須である。counter 計装コストは non-equal-work 比較では測れず、全列が同一 base workload で計装 step のみ差分の `counter_equal_work` 構成へ再構成した。aggregate fold は値 0 の記録を fold すると count との比較が無意味になるため、非ゼロ値 `POLICY_REC` で計測した (Issue #104)。

alloc_counter の TLS 窓は設置スレッドのみ有効であり、spawned thread の alloc は機構的に不可視である。呼出側スレッドで窓を開いたまま spawned thread が alloc/dealloc しても呼出側 count は 0 のままで、spawned 側で窓を開けば観測できることを実証した。含意として、backend 所有スレッド上の RT 経路 alloc は呼出側の `assert_no_alloc` 型窓では検出不可能であり、窓は計測対象スレッド自身に設置する所有権設計が必要になる (Issue #104)。

バックエンド候補クレートの評価は workspace の `Cargo.toml`/`Cargo.lock` を汚さないよう、`target/` 配下の `[workspace]` 空テーブルで切り離したスクラッチクレートで行い、成果は数値のみレポートへ転記した。PoC scaffolding は計測対象プリミティブを所有するクレートの `#[doc(hidden)] pub mod` と `benches/` に封じ込め、新クレート新設・別クレート同居はしない。実験 API は本番 RT 契約対象外・unsafe 0・pub enum なし・pure std + crate 内依存のみとし、`rt_` テストは追加しない (Issue #104)。

計測データの jsonl は impl-doc の experiment-log 監査が `exp/` 直下のフラット配置をイベントスキーマで検証するため `data/` サブディレクトリに置き、1 行=1 計測点・LF 改行・末尾改行ありで生成して lint hook に適合させた。bench は workspace 全量ではなく対象クレートと bench 名を限定して実行する逸脱を記録した (Issue #104)。

初回計測で見えた count vs aggregate の約 5 倍非対称と false sharing・キャッシュライン配置差の解釈は、ゼロ値 fold 構成の artifact であり撤回された。是正計測では count/aggregate が同一コスト級 (約 5.4 ns/op) と判明した。「最悪値 ~5.4 ns/op で有界」という表現も撤回されており、全計測値は単一スレッド非競合レイテンシの推定値であり、ハード RT 上限や本番ヘッドルームの証明ではない (Issue #104)。

## ADR 昇格の判断枠組み

audio device backend の採否 ADR は、`Verification` 全 6 項目に証跡 (実験ログ・3 IR・assert_no_alloc・モック結合テスト) を対応付けて `proposed→accepted` に昇格した。再評価トリガ (duplex 要件化・cpal メンテナンス再停滞・致命的挙動不良) はいずれも未観測であり、`Fixed(480)` 拒否は「致命的挙動不良」ではなく設計想定内の `UnsupportedConfig` 型表現 (`Default` フォールバックで開通) と判定した。未観測項目 (ALSA vs PipeWire・Send/Sync ばらつき・WASAPI exclusive/ASIO・初回 callback timestamp=0) は ADR `Confirmation` に引き継ぎ、未観測を以て昇格を阻害しない判断根拠 (Windows/WASAPI 実機で主要経路は実証済み) を残した。採用した cpal は `ruviform-audio-device` の依存にのみ存在し、audio-core 公開 API と `cargo doc` に cpal 型は露出しない (Issue #23)。

## クレート構成・feature・CI lane

observability の設計正本は、logging・tracing_export・collector・diagnostics・debug・rt_bridge・rt_timing・integration を 1 クレート `ruviform-observability` に集約する統合設計である。論理依存 DAG・RT/non-RT 境界・Source/Generation/Clock/Loss の非偽装・Consumer (AudioLens/Synvelis) 利用面・受入 Gate 9 項目との証拠対応が定義され、実装は per-issue の CLOSED 群として着地した (Issue #275)。

要件状態 (Required/Deferred/NotApplicable) と実行結果 (Pass/Fail/NotRun/Unsupported) の語彙は分離し、lane 定義の存在から個別 Case の Pass を推論しない。lane 名から Pass を推論して記す案は非昇格原則に反するため棄却され、per-module 公開 API の全列挙案も per-issue 正本との drift リスクで棄却されて代表型のみに留めた。`ci.yml` は `workflow_dispatch` のみで、`daily-ci.yml` が main SHA で dispatch する (Issue #275)。

feature unify の実測では、`integration` のみが `ruviform-audio` dev-dep 経由で `test-full` 系に到達し、`logging`/`tracing-export`/`allocation-probe` を有効化する workspace member は存在しない。帰結として、feature 付き module の compile+test は `msrv-verify`/`test-observability-msrv` の `--all-features` のみで行われ、Win/macOS native の feature 付き build/link/test は NotRun である (Issue #275)。

feature matrix lane は確立パターンの `&&` 連結を踏襲し、default・各 feature 単独・`--all-features` の 6 組合せを check/lint/test の 3 task で連結する。`--no-default-features` は `default=[]` で default と等価のため省略した。CI は `observability-verify` (ubuntu+windows matrix・`fail-fast:false`) と `observability-verify-macos` (別 job) の 3 OS 物理分離規約に従う。pairwise 組合せは `--all-features` が統合面を被覆するため Deferred とした (Issue #281)。

`required-features` ゲートにより default `cargo test` は feature 付き test target 3 本を compile すらせず、`--all-features` 単独は組合せ分離の検証にならない。新 CI lane を `ci-summary.needs` に足すだけでは `ci/summary/summary.mjs` の `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` に未宣言のまま集約され、checkout SHA 証跡の fail-closed 検証対象から外れる。新 lane 追加時はこの宣言同期が必須である (Issue #281)。

`test-observability-msrv` lane は `test-ofx-msrv`/`test-ui-test-msrv` の先例に倣い crate 固有 MSRV lane として追加され、`logging`/`tracing-export`/`allocation-probe`/`integration` の feature 付き module を compile+test する唯一の stable 系 lane となった (Issue #293)。collector/diagnostics/debug-tools の feature 分割は調査の結果、3 subsystem が常時 compile で依存追加ゼロ、opt-in が additive feature に担われているため「always-on core + additive opt-in」が現行設計の意図と判断され、変更しないことが記録された。consumer 要件が固まった時点での再評価は deferred である (Issue #293)。

## ロギング出力

`tracing_appender::non_blocking` は queue-full/partial write/flush 失敗/disconnect を区別計測できないため棄却し、`sync_channel` 有界化 + sink 専用 worker thread + `WriterGuard` の自前 `BoundedWriter` を採用した。Shutdown は in-band marker (`WorkerMsg::Shutdown`) で送信し、guard drop 時に drain→flush→join (5s timeout、超過で `shutdown_abandoned` 計上) する。marker 以後の straggler は `channel_disconnected` に計数される。`EventWriter` は fmt の複数 `write` を Drop で 1 event=1 message にバッファする (Issue #276)。

Host が subscriber を所有し、`build_logging`/`build_logging_layers` は `tracing::Dispatch`・`WriterGuard`・degraded・health を返すが `set_global_default`/`init` を呼ばない。stdout 非出力は `SinkDestination` に stdout 経路が存在しない構造的保証である。sink 分類は Required→`RequiredSinkFailed` で即中止、BestEffort→`SinkDegraded` 記録で継続、全滅→`AllSinksFailed` (全滅を成功扱いしない)、空→`NoSinks`、不正 filter→`InvalidFilter` である (Issue #276)。

`tracing-appender` の癖への対応として、`max_log_files` は 0.2.3+ が必要なため下限を pin した。upstream の `build()` は `prune_old_logs` が `create_dir_all` より先に走り存在しない dir で eprintln するため、adapter 側で先に `create_dir_all` する。rotation 時の `create_writer` 失敗は upstream が eprintln して旧ファイルへ継続する経路であり adapter の counter に到達しない。この caveat は契約として明記された。`Stderr` sink の `queue_capacity` は公開面を増やさず固定値 4096 で吸収し、`SinkMetrics` の hot counter は atomic のみ・状態フラグは Mutex 側に寄せて snapshot コストを最小化した (Issue #276)。

## tracing_export

`tracing-export` feature (`dep:tracing` のみ・既定 OFF・`logging` と独立) で gate し、Host が `tracing::Dispatch` を scoped `with_default` で注入する。Collector/Metrics/Source/Record/rt_bridge/diagnostics は `tracing` を一切 import しない構造保証であり、`cargo tree` で `tracing-subscriber`/`tracing-appender` を含まないことが検証済みである (Issue #278)。

`Instant` は wall-clock 変換不可のため、初回 emit を t0 とする `*_mono_us` 相対 microsecond を使い捏造 wall-clock を避けた。slice-1 で schema を凍結したため、measurement/window event への trace field 付与は `event_version` bump を伴う破壊的変更になるとして断念し、trace field は `rt_record` のみに限定した。`TraceContext` は W3C `traceparent` の構文検証のみ (version ff・全ゼロ id 拒否、v00 は 4 field 固定、他 version は先頭 4 field 前方互換) で認可・信頼判定を持たない。confidentiality は分類の忠実報告のみで値秘匿は host の認可呼出し責務 (Non-goal) である。correlation 分類は cap 判定より先に全 record へ適用し、cap 圧力下でも相関健全性を観測可能にした (Issue #278)。

cap 超過の `cap_dropped` は baseline を進めず次 poll で再評価する。`tracing::Level` の `Ord` は TRACE>DEBUG>INFO>WARN>ERROR の verbosity 順で severity とは逆のため、「高 severity」は `Ord::min` で取る (当初の `max` は全 Level を逆転させる P1 欠陥だった)。`event!` の callsite level は static 必須のため、計算 Level は DEBUG/INFO/WARN (sink は +ERROR) の callsite 分岐で発行する。panic 処理は非対称で、exporter は subscriber panic を catch せず伝播させ (baseline を emit 前に更新し二重報告を防止)、`TracingFindingSink` のみ `catch_unwind` で隔離して `panics()` に計数する。OnIncrease dedup は「signal 増分なし」だけでは不十分で、「signal 不変 かつ validity/issue/lifecycle 不変」の抑制条件が必要だった (Issue #278)。

## rt_bridge の損失と遅延の分離

`rt_bridge` は独自リングを実装せず `ruviform_rt::channel` の SPSC を再利用する (重複実装禁止・ADR の葉プリミティブ再利用方針)。`RtRecorder::try_record` は bounded・非ブロッキング・alloc なしで、満杯時は drop+共有カウンタ加算し、次に受理された record の `dropped_before` で「どの受理位置の前に何件落ちたか」を報告する。`lag()=recorded-drained_total` で遅延と損失を分離する。Record 語彙は固定長 `Copy` で生 PCM/信号バッファを構造的に排除し、Clock 語彙は media-core 製を参照のみで observability に変換権を持たせない (Issue #280)。

RT 観測 PoC の verdict 推奨として、rt_bridge の drop-on-full+共有カウンタは `record`/`retire_channel` と同コスト級で支持された。logging は RT 経路では構造的理由 (Subscriber 側で任意の alloc/blocking が走り得る) で不採用とされ、「425ns だから遅い」は根拠にしない。timestamp は `Instant`→epoch 相対 nanos→i64 が採用可だが 29–39ns/stamp と重く、「呼出し側提供 epoch+観測点ごとの必要箇所のみスタンプ」が推奨された。telemetry の counter pair 形状は有力候補として維持、metrics/profiling は需要なし・RT 検証なしで延期。採否の最終判断は trigger Issue 側の責務である (Issue #104)。

L1 diagnostics の最初の実消費者面として `Counter`/`CounterReader` pair を提供する。`Counter` は writer で `Relaxed` atomic 更新のみの RT-suitable 参照、clone で reader と共有し、`CounterReader` は非 RT snapshot で load のみを行う。複数 counter の逐次 `Relaxed` 読みは同一時点の整合 snapshot を保証しないことを公開 API doc に明記する契約とし、`reserved: [u64; 2]` 規約を維持した (Issue #121)。

## Collector の観測モデル

単一 poll owner は `Collector` の `!Clone` + `poll(&mut self)` でコンパイル時に保証し、runtime flag を持たない。`Measurement.value: Option<MetricValue>` で `Unavailable`/`Unsupported` を `None` 表現し 0 に偽装しない。Counter 減少は巨大 delta を作らず `Discontinuous`+issue で delta/rate を抑止し次回 baseline を再基準化する (wrap/reset を推測しない)。forced/abandoned drain は quiescence 未確認として `Partial`+`WriterNotQuiesced`、domain-retire 済み最終 drain は `Valid` と validity 優先位を分離する。mid-poll の lock-free retire は同 poll 内で再検出して drain し、mutex poison は `PoisonError::into_inner` で復旧して lock-recovery event 単位で計上する。履歴は samples 数+概算 bytes の二重上限、key/cell 重複登録は `KeyConflict`/`SourceReused` で拒否し、key claim は登録意図寿命まで永続する (Issue #289)。

Post-Review で P1 Blocking が検出・修正された。有界履歴は回復経路ではなく終端 batch が正本である。`history_max_samples=0`/`history_budget_bytes=0` の有効な「履歴無効」設定で、最終 drain の Measurement が capacity-0 history への push のみで消失し回収不能だった。修正は `drain_all` を poll と同じ観測 commit 責務へ揃え、終端 drain が新世代を mint して全 measurement・removed・window・snapshot を 1 つの terminal `ObservationBatch` として公開する方式である。drain 対象ゼロでも空の terminal batch が mint され、`Stopped` 後の `latest_batch` が常に定義済み世代を指す。history 強制 ON にはせず回収正本を batch に置いた。教訓として、有界履歴は記録責務であり回収正本は別経路が必要である (Issue #289)。

## allocation-probe と rt-timing

`CountingAllocator` は `fetch_update` が toolchain 1.99 で deprecated、`try_update` が MSRV 1.89 未満のため `compare_exchange_weak` の飽和 CAS ループを自前実装し、peak は `fetch_max` で中間値も正確に記録する。node アドレスは `expose_provenance`/`with_exposed_provenance` で復元し、`as` cast の provenance 喪失と `cast_ptr_alignment` を回避する。計数は inner 呼出「後」に一括実行して partial count を残さず、realloc は分解せず delta bytes で計上する。crate は `#[global_allocator]` を一切 install しない (host 責務) (Issue #290)。

`AllocationScope` は当初 `Send` 想定だったが、`mem::forget(window)` で漏らした scope の node は enter したスレッドの TLS chain に残り、scope を別スレッドへ move して drop すると unlink が誤った TLS を走査して node が dangling (UAF) + `Cell` データ競合になるため、`PhantomData<*const ()>` で型レベルに `!Send`+`!Sync` へ変更した。設計書・risk 表も同期された (Issue #290)。

`TimingRecord` は bridge 内 `TimingRecorder` からのみ構築可能なため foreign-clock record は公開 API 上到達不能とし、sink に防御的 clock 比較を置かない。duration は native tick のまま保持し tick→秒変換は domain の責務。`budget_ticks==0` は「budget 未 assert」で `max_utilization_permille` は 0% ではなく `None`。`end<start` の clock anomaly は histogram/scalar duration から除外するが generation accounting には含める。drop-on-full の `dropped` は drain された poll window に帰属させ、`dropped==0` は完全性を証明しない (in-flight carrier が後で損失を帰属し得る) ため権威累積は `TimingSnapshot.dropped_total`。quantile は固定 bucket に基づく区間推定で厳密 percentile を装わない (Issue #290)。

## diagnostics の証拠モデル

`EvidencePayload` は closed enum (Metric 値は typed 参照ではなく所有コピー、Claim は `&'static str`、None) で任意バイト列・path・token・media・prompt・raw stderr を構造的に排除する。Health の `Unknown` は「正常でも異常でもない」状態として既知の bad 状態を上書きしない。counter delta/rate が `None` の場合は `RuleInput::Unevaluable`→`Unknown` で 0 扱いや warning 化をしない。finding upsert は worst severity・highest certainty・strictest confidentiality を保持し first_seen を維持する。Report 認可は runtime 依存を持たず `max_export_disclosure`+`ReportRequest.disclosure` の policy parameter でモデル化し、超過 item は redact ではなく entry 単位で除外して `withheld_confidential` で計数する。`SnapshotConsistency::IndependentReads` を型レベルで明示し transactional snapshot でないことを保証する。`DebugHealth`/`DiagnosticsHealth` は固定カウンタのみで source 登録せず自己観測の再帰を防ぐ (Issue #291)。

実装レビューで CONFIRMED P1 が検出・修正された。source generation 変更時に confirmed state が transition なしで `Healthy` に reset される欠陥があり、streak のみ reset に修正して回帰テスト `generation_change_never_fabricates_healthy_over_confirmed_bad` を追加した。教訓として、世代境界での状態リセットは「確認済みの悪い状態を消さない」規則 (Unknown は known-bad を上書きしない) と同型の fail-closed 原則を適用すべきである (Issue #291)。

## probe と debug inspector

`typed::probe` は `Off`/`Monitor`/`Capture` の capability 宣言契約で、preallocated bounded PCM capture を pooled chunk channel で RT→非 RT に handoff し、POD telemetry は別の drop-newest channel に載せる。telemetry flood が capture を枯渇させない分離である。capture budget は pool size で cap せず `max_total_frames` が総配信量を制限し、pool は outstanding in-flight chunk のみを制限する。rebind command は発行時の plan generation を持ち stale binding は `Unbound`。OFF mode は tap site が arena slot に触れず record も出さない cheap scan である。loss-tolerant probe 契約は lossless Recording Writer とは別契約で再利用不可 (Issue #185)。

probe 依存は `ProbeSource` object-safe port trait のみで再宣言し、concrete adapter は `ruviform-audio`/#293 側が実装する (`observability→audio` 依存は循環のため置かない)。section truncation は snapshot-global ledger の delta で計上し先行 section の drop を後続に誤計上しない。deadline 到達の probe 収集は収集済み部分 deck を破棄せず `Truncated` 保持。`max_items==0` は意図的 no-entry mode で budget drop と区別。missing input は空の成功 section ではなく `Unavailable{MissingInput}`。consistency は section 成功と独立に raw generation/clock stamp から計算し probe clock mark 無しは fail-closed `Unknown`、優先順位は `Skewed>Unknown>Aligned` (`Unknown` が確定済み `Skewed` を消さない。OBS-09 と同規則)。`SessionPermit` は Arc token capability で暗号学的認可ではなく host 認可は #293。drain timeout は `deadline_exceeded` を報告するが `Stopped` を強制しない (quiesce しない drain を偽って完了扱いしない) (Issue #292)。

レビューで 4 件の CONFIRMED 欠陥 (P1×2+P2×2) が検出・修正された。`ExportLimits::max_items==0` が文書の「summaries only」に反して無制限収集していた件、`source_generation==0` (never evaluated sentinel) を実 generation として比較し偽の `Skewed` を生成していた件、consistency で `Unknown` が確定済み `Skewed` を上書きし得た件、`findings_total` が domain-health entry を含んで水増ししていた件である。各々回帰テスト付き (Issue #292)。

`on_publish` の逐次 `ClearAll`+個別 `Bind` は bounded queue 飽和時に部分適用が残り得たため、`RebindBegin`/`RebindStage`/`RebindCommit` の 3 相プロトコル + generation/staging epoch の fail-closed guard + `mem::swap` による不可分 commit を採用した。棄却した代替案は `spsc.rs` への batch push 追加と `ProbeCommand` への巨大 batch variant で、SPSC slot の固定サイズ前提を崩し command サイズが `max_probes` に比例して bounded でなくなるためである。enqueue 失敗時は Commit 無しの inert prefix のみ残り live bindings は不変で、drain 後 retry で全件 commit 可能。未解決 point は `receipt.unbound`+`StatusKind::RebindDropped` で明示される。staged は bindings と同じ `max_probes` capacity の事前確保 Vec で、commit は `mem::swap`、他は clear/push のみで RT no-alloc 契約を維持する。generation/staging epoch 不一致の stage/commit は stale として drop + `stale_commands` 計上。`CapturePolicy::StopAtBoundary` (既定・`StopReason::GenerationBoundary`・budget 破棄)/`Restart` (新規 full budget segment) は resolve 時に消費され command 化しない (Issue #308)。

破壊的 API 変更 (`on_publish` 戻り値を `Result<RebindReceipt, ProbeError>` 化・`ClearAll` 廃止) は pre-release で呼出側がリポジトリ内テストのみのため許容と判断した。`RebindDropped` status が telemetry overflow で欠落する場合は `lost_records` counter 依存で許容とし仕様として記録した (Issue #308)。

`ProbeClockMark` は `Option` ではなく `ClockDomainId::UNKNOWN` sentinel の copyable 値型とし、`RecordMeta`/`StatusRecord` への additive 拡張で全 emit 経路の `Option` 分岐を回避する。mint は `ProbeBroker`/`pair()` のみが probe 専用 namespace (`0x5256_4650_0000_0000`、media-core well-known ID より上位で衝突回避) から atomic 採番し、RT deck は `SetClockMark` command で copy を受けるだけである (`set_clock_mark_is_alloc_free_on_rt_path` で検証)。`bump_epoch` は channel full 時に broker/deck mark を一時 diverge させ得るため caller の retry を要求する契約である。`align_observation_points` は `Aligned`/`Conditional`/`Incomparable` の三値を entry fail-closed→basis 解決→format/rate 検査→latency_term→evidence_conditions→coverage_phase→error budget の phase 分割で返し、各段が `IncomparableReason` を返す。rate 変換は `allow_rate_conversion` 明示時のみ `NearestEven`+residue を `error_budget` に計上し、resample・PCM 相関・信号内容推論は non-goal。`ObservationCoverage::reconstruct` は event sweep で測定無音と未観測を区別し、status は data に優先・未説明 gap は `Unobserved`・`lost_records>0` は `Unknown` へ降格・測定無音は `Valid` に留める。clock 語彙・frame 変換は media-core を再利用し新しい clock 正本は作らない (Issue #309)。

fixture の教訓として、既存 mock DSP は `latency()` を宣言するが PCM を実際には遅延しないため、latency 宣言値と process の実遅延が一致する test-local `DelayDsp` を ground truth として新設した。無音判定は cursor block index ではなく latency 補正後の観測窓で行い「独立 f64 期待 peak が厳密に 0」の区間を測定無音と定義した (pre-roll の delay line 出力 0 は Valid coverage)。telemetry loss fixture は capacity 6 で block 0 が `Installed` status 3 件+metric 3 件をちょうど収め、block 1–2 の overflow で `lost_records>0` を決定的に発生させた。PCM は offset 補正後 `to_bits` で bit-exact、Peak/RMS は f32 実測と f64 独立期待の丸め差を許容。`F64Native` 境界は実 span の `precision` field の単一 field mutation で検証し、consumer 側の native f64 source は consumer 責務と明記した。`LostRecords`/`Unobserved`/`GenerationBoundary` class は実 runtime からは status 非送出または非決定的なため contract unit 層で検証する分担とした (Issue #309)。

concrete `ProbeSource` adapter は `ruviform-audio` 側 (`typed/probe/source.rs`) に置き、依存方向は `ruviform-audio → ruviform-observability` の一方向を維持する。`Arc<Mutex<SharedState>>` の Clone 可能 handle が `ProbeBroker`+`ProbeCollector` を束ね、collector handle と同じ non-RT 共有パターンである。`decks()` は telemetry channel のみ drain し `chunks_out` の capture PCM は絶対に消費しない (inspection が record stream を横取りしない)。capture PCM は `drain_events()` 経由でのみ host が受け取る。`dropped`/`lost_blocks` は probe 帰属不能な deck-global 値を全 reading にミラーし分割偽装しない。generation は broker が最後に staging した plan generation で、未解決 probe は旧 generation を保持して skew を inspector に見せる。`applied_mode` は command 受理≠適用を区別するため status stream reconcile による RT 確定値を報告し、`bound_generation`/`availability`/`last_status`/`recorded` は `on_publish` の Begin/Stage/Commit 全 enqueue 成功後にのみ確定する (rebind 途中失敗で新旧が混ざらない)。edge 強化の contract test として reconfigure の `non_migrated` 報告・secret non-exposure・GUI pull 境界が固定された (Issue #293)。

## ObservationRunner と Host 駆動モデル

`ObservationRunner` は `Unconfigured→Prepared→Ready→Running→Draining→Stopped` のライフサイクルを持ち、`poll_once` を Host が呼ぶ設計でスレッドもタイマーも所有しない。RT/非 RT 境界を Host に残すためである。動的設定は `Validate→Prepare→Activate→Retire` で、reconfigure 時は新 config を validate+完全構築してから live state を swap し旧 component set を retire/drain する。source 登録・diagnostics retention・debug session・exporter baseline は移行せず `ReconfigureReport.non_migrated` で明示し、「完全保存」を主張しない契約である。`finish_shutdown` は Collector 最終 poll→`Collector::shutdown` terminal drain→terminal batch export→logging guard drop/flush/close の固定順序で、logging health は guard drop 後も attachment として保持され shutdown 後も読める。`RunnerHealth` は atomic/値コピーのみの非再帰 snapshot で tracing emit・sink アクセスを行わない。`ObservabilityConfig` は subsystem 独立設定で Logging は Host 注入の `attach_logging` のみ、`validate()` は各 subsystem の既存 validator に委譲して新ルールを発明しない。export は `tracing-export` feature 時のみ存在する cfg 構成で fake type を置かない。`ObservationRunner` は `Send` を要求せず Host thread 所有で、`Arc<DebugAuthority>` は Host が permit 発行用に保持できる (Issue #293)。

GUI/consumer 向けには push `Subscription`/fan-out/callback が設計上存在せず、正本は `collector_handle()`/`snapshot()`/`latest_batch()`/`history()`/`series()` の pull API のみである。OBS-07 no-fan-out 規約により欠落機能ではないことを runner/README に明文化した (Issue #293)。

## consumer 側の観測 seam

外部 crate の consumer 相当 test では `cfg(test)` 分岐が効かないため、raw parent 捕捉の gate を `feature = "gpui-test"` に揃えて panic を解消した。`GpuiObserver` の `settle` は intent を記録してから handler へ渡す。mount 済み view は window slot に状態を足さず window の root (`AppRoot`) の content から引く。`NumericInput`/`Transport` は consumer 需要棚卸しの結果「共通層への新 kind 追加は不要 (composition で充足)」と判断された。`ValueDescriptor`/`ValueInput`/`ControlInput`/`TransportProjection` で既にカバーされ、schema 拡張を正当化する consumer 需要が見つからなかった。kind 追加前に consumer 需要を棚卸しすることが教訓であり、将来共通 semantic gap が示された場合に限り再検討する。`ViewObservation.generation` は mount/detach でのみ進む mutation guard として doc を実装に合わせて修正した (Issue #429)。

## 未確定・保留事項

- 設計の一次正本 (spec/design/test-spec/ADR 文書・計画文書・evidence 台帳の個別受入行) は本書の記録範囲外であり、却下理由の詳細や OBS-01〜11 個別受入の詳細は本書に未反映である (Issue #276/#278/#289/#290/#291/#292/#275)。
- Issue #104 の verdict は推奨であり、telemetry counter pair の採否・timestamp 方式の採否など最終判断は trigger Issue 側の責務として未確定である。metrics/profiling は需要なし・RT 検証なしで延期、logging は RT 経路不採用。
- Issue #275 の Deferred 4 項目 (実外部 Provider・実 OS GUI・実 Device・長時間 Profiler) は再開 Owner と Activation Trigger が owner map に記録されている。pairwise feature 組合せも Deferred。
- Win/macOS native の feature 付き build/link/test は NotRun であり、feature 付き module の検証は MSRV lane のみが担う。
- live device では device timestamp 未配線のため clock mark の epoch が `UNKNOWN` のまま残り得るが、`Incomparable{UnknownClock}` で保守的に倒れる設計どおりの挙動である (Issue #309)。
- collector/diagnostics/debug-tools の feature 分割再評価は consumer 要件が固まるまで deferred (Issue #293)。`NumericInput`/`Transport` の新 kind 追加も consumer 需要が示されるまで再検討しない (Issue #429)。
- Issue #429 には実装 IR の記録が存在せず、実装完了の検証は task の完了欄と残 DoD 棚卸しが正本である。
- 本書の全計測値は単一スレッド非競合レイテンシの推定値であり、本番 RT 上限やヘッドルームの証明ではない (Issue #104)。

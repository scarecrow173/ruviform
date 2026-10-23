# RT 観測コスト計測 PoC 計測レポート（2026-10-23）

Issue [#104](https://github.com/scarecrow173/ruviform/issues/104) / task T2（`docs/tasks/rt-observability-poc-run-and-report.md`）の計測実績。#103 設計 [foundation-observability-minimal-api](../../designs/foundation-observability-minimal-api.md) の「要 PoC」判定項目（telemetry / rt_bridge / metrics / profiling / logging）への判断材料。

生データ（初回 3 run × 26 セル・初期条件データ）: [`data/rt-observability-poc-20261023-01.jsonl`](data/rt-observability-poc-20261023-01.jsonl)・[`data/rt-observability-poc-20261023-02.jsonl`](data/rt-observability-poc-20261023-02.jsonl)・[`data/rt-observability-poc-20261023-03.jsonl`](data/rt-observability-poc-20261023-03.jsonl)（1 行 = 1 計測点、26 行/run。run・group・column・variant・elems・median_ns・mean_ns・stddev_ns・CI95・ns_per_op）。イベントログ: [`rt-observability-poc.jsonl`](rt-observability-poc.jsonl)（seq 1–11 は初回計測当時の記録、seq 12–13 が本レポートの是正を反映）。

**是正計測（2026-10-24・レビュー起因）**: 初回計測の `counter` / `overflow_policy` 両グループは計測構成上の欠陥（非 equal-work 比較・ゼロ値 fold）がレビューで確認され、`counter_equal_work` / `overflow_policy_nonzero` に再構成して当該 2 グループのみ 3 run 再計測した。是正データ: [`data/rt-observability-poc-20261024-corrected-01.jsonl`](data/rt-observability-poc-20261024-corrected-01.jsonl)・[`data/rt-observability-poc-20261024-corrected-02.jsonl`](data/rt-observability-poc-20261024-corrected-02.jsonl)・[`data/rt-observability-poc-20261024-corrected-03.jsonl`](data/rt-observability-poc-20261024-corrected-03.jsonl)（8 行/run）・集約 [`data/rt-observability-poc-20261024-summary.json`](data/rt-observability-poc-20261024-summary.json)（revision `matched-work-nonzero-v2`・harness_sha256 `b0be85ed`）・集約スクリプト [`data/rt-observability-capture.cjs`](data/rt-observability-capture.cjs)。バックエンド依存メタデータ: [`data/rt-observability-backend-metadata-20261024.json`](data/rt-observability-backend-metadata-20261024.json)。初回 3×26 セルは初期条件データとして保存し、本レポートの counter / overflow_policy 両節は是正値を正・初回値は superseded 注記とする。

**数値の位置づけ**: 本レポートの全計測値は **単一スレッド・非競合時のレイテンシ推定値**であり、ハード RT 上限・最悪値・本番ヘッドルームの証明ではない。スレッド間競合（reader/observer との同時アクセス）・エンドツーエンドのコールバック予算は未検証。

## 環境メタデータ

- OS: Windows 11 x64 / CPU: Intel Core i9-9900K @ 3.60GHz / 16 コア（HT あり）
- rustc 1.99.0 (b940084d7 2026-09-28) / criterion 0.8.2（`default-features = false`）/ ruviform-rt 0.1.0
- 計測列（26）: `counter`×4 → 是正後 `counter_equal_work`×4（disabled / flag off / flag on / always、全列同一 base workload）、`rt_bridge`×6（record not_full/full、drain empty/half/full、overflows read）、`overflow_policy`×4 → 是正後 `overflow_policy_nonzero`×4（drop / count / aggregate / retire_leak 基準、aggregate は非ゼロ値 fold）、`rt_primitives`×8（channel try_push/try_pop、retire/overflows、clock tick/publish_tick、slot publish/read）、`timestamp`×4（instant_now / instant_to_epoch_nanos / timestamped_record / systemtime_now 対照）
- 時間設定: warm-up 300ms / measurement 1s / sample 20（dsp-simd-poc と同一の削減予算形）。バッチ反復 `BATCH = 512` op/計測 iteration（ns 級操作の SN 比確保）、fixture は `iter_batched_ref` + `BatchSize::NumIterations(64)` で窓外
- 実測所要時間: 約 2–3 分/run（plan 目安 10–20 分を大幅に下回る。criterion 追加分析フェーズ込み）
- 負荷状況: 計測中も並行エージェントプロセスが多数稼働（~458 プロセス、システムコミット ~35.5/64 GB）。分散緩和は 3 run 実施＋median 採用＋run 内比率評価で対処
- Windows criterion quirks: なし。3 run とも全 26 列完走・abort・異常終了なし

## 実施記録

- `mise exec -- cargo bench -p ruviform-rt --bench rt_observability_poc` を 3 run 連続実行。各 run 後に `target/criterion/{counter,rt_bridge,overflow_policy,rt_primitives,timestamp}/**/new/estimates.json` から median/mean/stddev/CI95 を抽出し `data/` jsonl へ集約（run 間の上書きを防止 — test-spec「run → 集約 → 次 run」順序どおり）
- `cargo bench --workspace` 全量ではなく `-p ruviform-rt` のみ実行（ruviform-math の simd_poc は Issue #19 で計測済み・今回の対象外。`mise run bench` は両方を含む全量入口であり、そのままでは ~30 分規模になるため対象クレートに限定した。26 列のハーネス自体は `cargo bench --workspace` 経由でも同一の産物を出す）
- 是正計測（revision `matched-work-nonzero-v2`・該当 2 グループのみ 3 run）: `mise exec -- cargo bench --offline -p ruviform-rt --bench rt_observability_poc -- 'counter_equal_work|overflow_policy_nonzero'` を実行し、各 run 直後に `mise exec -- node docs/impl/exp/data/rt-observability-capture.cjs --capture <1..3> docs/impl/exp/data/rt-observability-poc-20261024-corrected-<NN>.jsonl` で run ごとの `estimates.json` を退避（run → capture → next。`estimates.json` は run ごとに上書きされるため）
- バックエンド評価: `target/rt-observability-backend-probe/`（git 管理外・`[workspace]` 空テーブルで切離）にスクラッチクレートを作成し、依存木・ライセンス・宣言 MSRV・マイクロコストプローブを実測。workspace の `Cargo.toml`/`Cargo.lock` へは一切追加していない

## 分散（run 間再現性）

- ns_per_op の run 間 |max Δ%|: p50 = 0.4%、p90 = 16.4%、max = 38.5%
- 最大偏位は `rt_bridge/drain/empty`（O(1) の空 drain コールで ~2 ns/回の小絶対値にノイズが乗る）と `rt_bridge/record/not_full`（4.35–6.68 ns/op — 原因未分離、fixture 再確保・システム負荷等の可能性を含む仮説）
- 判定には **run 内比較**（同一 run のバリアント列・基準列との比率）と 3 run 平均を併用し、run 間絶対値差を相殺（dsp-simd-poc と同じ評価形）

## 計測マトリクス判定表（3 run 平均 ns/op・単一スレッド非競合レイテンシ推定値）

### counter_equal_work — prepared counter の計装コスト（是正計測・equal-work 比較）

| 列 | ns/op | vs disabled（paired delta） | 備考 |
|---|---|---|---|
| disabled（計装なし・base workload のみ） | 1.356 | — | 全列共通の `black_box(value).wrapping_add(1)` ベース |
| flag/off（ゲート閉・インクリメント不実行） | 2.097 | +0.742 | フラグ読出し（`black_box` 経由 Relaxed load）+ 分岐の残存コスト — 「無料」ではない |
| flag/on（ゲート開・常にインクリメント） | 5.389 | +4.034 | always と同コスト級（+0.9%） |
| always（無条件 `fetch_add` Relaxed） | 5.342 | +3.987 | base workload + 1 increment |

- **計装デルタ（equal-work）**: always − disabled ≈ **+3.99 ns/op**（paired delta 3 run 平均 +3.986841）。全列が同一 base workload を行う構成で、計装 step の有無のみを差分として評価 — synthetic equal-work 上の推定値であり、本番パスへのヘッドルーム証明ではない
- **フラグ経路**: flag/on ≈ always（+0.05 ns/op 差）— ゲート開時は無条件計装と同コスト。flag/off は disabled に対し **+0.74 ns/op** の残存ゲートコストが残る — 計装を off にする経路は「実質無料」ではなく、常時計装の単純さとの比較材料
- **superseded（初回計測・初期条件データ）**: 初回 `counter` 群（disabled 1.361 / off 0.262 / on 3.895 / always 3.872）は disabled 列のみが `wrapping_add`+`black_box` のワークを持つ非 equal-work 構成であり、「+2.5 ns/op の加法計装コスト」「ゲート実質無料」としての解釈は撤回（レビュー指摘 P1/L1）。初回値は履歴として `data/` jsonl に残る

### rt_bridge — レコード受け渡しと観測アクセサ

| 列 | ns/op | 備考 |
|---|---|---|
| record/not_full | 5.74（r1 6.19 / r2 4.35 / r3 6.68） | rtrb push。run 間 24% の偏位あり（fixture 由来ノイズとして記録） |
| record/full（drop+count） | 5.41 | not_full と同桁 — 満杯時の破棄+計数パスは push 成功パスと同コスト |
| drain/empty | 2.45（38% 分散） | O(1) 空呼出し。小絶対値にノイズが支配 |
| drain/half（256 records） | 4.07/record | bounded drain、消費レコードあたり |
| drain/full（512 records） | 4.07/record | half と一致 — per-record コストが線形・有界 |
| overflows/read | 0.45 | Relaxed load（fetch_add より安い読出しのみパス） |

### overflow_policy_nonzero — 同一満杯シナリオの方針別コスト（是正計測・非ゼロ fold 値）

| 方針 | ns/op | 備考 |
|---|---|---|
| drop（沈黙破棄） | 1.525 | 最安。bookkeeping なしの満杯チェックのみ |
| count（破棄 + 共有カウンタ） | 5.417 | record/full・retire_leak と同コスト級（≈5.4 ns/op） |
| aggregate（破棄 + u64 fold 集約・非ゼロ値） | 5.428 | **count と同コスト級** — 非ゼロ値 `POLICY_REC = ObsRecord::new(1, 0, 1)` で計測 |
| retire_leak（既存 leak-on-full 基準） | 5.413 | count/aggregate と同コスト級 — 既存 `retire_channel` 先例と一致 |

- **方針別コストの読み方（是正後）**: count・aggregate・retire_leak はいずれも ≈5.4 ns/op の同一コスト級。「push 失敗 → 共有 atomic へ fetch_add」の bookkeeping コストは方針によらずほぼ同じ — 有意な差は silent drop（~1.5 ns/op）との差であり、「破棄を観測可能にする」対価として ~3.9 ns/op と読める。aggregate を選ぶ根拠はコストではなく集約セマンティクスの要否のみ
- **superseded（初回計測・初期条件データ）**: 初回 `overflow_policy` 群（drop 0.894 / count 5.404 / aggregate 1.092 / retire_leak 5.403）は aggregate 列が値 0 の記録を fold する構成（`fetch_add(0)`）であり、「count vs aggregate の ~5x 非対称」「false sharing / キャッシュライン配置差」という解釈は判別力のないデータに基づくものとして撤回（レビュー指摘 P1/L2）。「最悪値 ~5.4 ns/op で有界」という表現も撤回 — 単一スレッド非競合の推定値であり最悪値・上限としては読めない。初回値は履歴として `data/` jsonl に残る

### rt_primitives — 既存 RT プリミティブの素コスト（計装なし基準列）

| 列 | ns/op | 列 | ns/op |
|---|---|---|---|
| channel/try_push | 1.79 | clock/tick | 0.88 |
| channel/try_pop | 1.12 | clock/publish_tick | 1.08 |
| retire/retire | 1.29 | slot/publish | 83.5 |
| retire/overflows | 0.44 | slot/read | 18.4 |

- **計装余裕の評価（参考値）**: 是正計測の +3.99 ns/op（equal-work・単一スレッド推定値）を `try_push` 級操作（~1.8 ns）と比べると比率では ~2x だが、これは synthetic バッチ上の比較であり実コールバック予算への適合性は未検証。`slot/publish`（83.5 ns）は非 RT 経路であり RT 純度比較の対象外（計測コストに pending Vec 成長の alloc を含む — test-spec「測定窓の純粋性」の除外規定どおり）

### timestamp — タイムスタンプ取得コスト

| 列 | ns/op | 備考 |
|---|---|---|
| Instant::now() | 29.2 | Windows QPC 経由 — 計測したタイムスタンプ取得列の中では最も高い単発コスト |
| instant_to_epoch_nanos | 34.6 | now + saturating delta + u128→i64 変換（+5.4 ns） |
| timestamped_record | 39.4 | now + 変換 + `ObsRecord` 構築（`instant_to_media_time` 同形の代理計測） |
| SystemTime::now()（対照列） | 15.0 | Windows では QPC より安いが壁時計 — RT タイムスタンプ源として不適（設計の前提どおり） |

- **判定に効く結論**: クロック読出し（29.2 ns）がタイムスタンプ付きレコード経路（39.4 ns）の 74% を占める。レコード全件へのスタンプ添付は高頻度経路では支配コストになる — 「呼出し側提供 epoch + 疎な観測点のみスタンプ」が妥当。実機経路は `ruviform-audio-device` の `StreamInstant::as_nanos`（コールバック提供時刻で取得コスト自体は発生しない）が既存実装

## カバレッジ隙間の実証（alloc-gap）

`alloc_counter`（crate 既存の thread-local 測定窓 — `assert_no_alloc` と同一機構）による決定的実証:

- **呼出側窓 = 0**: 呼出側スレッドで窓を開いたまま spawned thread が alloc/dealloc しても、呼出側 `count()` は 0 のまま（隙間）
- **spawned 側窓 = 26（>0）**: spawned thread が自身で窓を開けばイベントは観測できる（窓は設置スレッドのみ有効）
- 実測出力: `alloc-gap demo: caller_window=0 spawned_window=26 spawned_observed=true`（`observability_poc::tests::alloc_window_covers_only_owning_thread`）
- **含意**: cpal 等のバックエンド所有スレッド上の RT 経路 alloc は、呼出側スレッドに置いた `assert_no_alloc` 型窓では機構的に検出不可能。窓は「計測対象スレッド自身」に設置する所有権設計が必要（backend callback 内で窓を開く・process-wide 計数アロケータ `CountingAlloc` 型の併用等）。RT 検証レーンの限界記録として留保

## バックエンド評価（workspace 依存へ非混入）

依存メタデータはスナップショット [`data/rt-observability-backend-metadata-20261024.json`](data/rt-observability-backend-metadata-20261024.json)（正本 — `cargo metadata` 由来・Windows ターゲット・デフォルト feature 統一での normal/build 依存閉包・**自分自身を含む**）。マイクロコストは `target/rt-observability-backend-probe/`（`[workspace]` 空テーブルで切離・git 管理外）のスクラッチプローブによる **暫定観測値** — 生サンプル・記録済み stdout は保存されていないため、アーカイブ済み・再現可能な証拠ではなく、採否の根拠データとしては使わない。

| 候補 | 解決版 | 依存閉包（自身含む） | ライセンス | 宣言 MSRV | 保守状況 | マイクロコスト（暫定観測・非アーカイブ） |
|---|---|---|---|---|---|---|
| `tracing` | 0.1.44 | 9 pkgs | MIT | 1.65 | 暫定評価（日付付きレジストリ出典なし） | `info!` 無 Subscriber **0.27 ns/op**・fmt→io::sink **425 ns/op** |
| `tracing-subscriber` | 0.3.23 | 25 pkgs | MIT | 1.65 | 同上 | （subscriber 側・アプリ所有。emit 側コストは上記） |
| `metrics` | 0.24.6 | 3 pkgs | MIT | 1.71.1 | 同上 | `counter!` 無 Recorder **4.27 ns/op**（key hash + グローバル recorder 解決） |
| `tracy-client` | 0.19.0 | 8 pkgs | MIT/Apache-2.0（`-sys` は bundled Tracy C++ で BSD-3-Clause 併記） | 1.70 | 同上 | `span!` **~26 ns/op**（client 未開始・開始済みで同値） |

- 表の MSRV は **宣言値のみ**（1.65–1.71、いずれも workspace 1.89 以下）。推移的依存の実効 MSRV は未検証 — 採用時に検証が前提
- `tracy-client-sys` は Tracy profiler の C++ クライアントをバンドルし C++ ツールチェインでビルドする（build-time 要件）。0.18.5 初回実行時に Windows で `SymInitialize FAILED (code 87)` の警告を出した（0.19.0 では未再現）
- プローブ参照列は `fetch_add` 3.86 ns/op・`Instant::now()` 29.3 ns/op（同じく暫定・非アーカイブ観測値。本ハーネスの計測系と同桁のプラットフォーム定数を示す参考値）
- **emit 経路の要点（暫定観測のみ）**: 各候補の emit 経路の挙動・コストは未設定時・設定時ともにアプリ側構成（Subscriber / Recorder / client 状態）に依存し、RT 適性は未検証。暫定・非アーカイブのプローブ観測値から設計特性を断定しない

## #103「要 PoC」項目への verdict 対応表

設計の「要 PoC」据置項目に対し、計測データが示す推奨方向を記録する（採否の最終判断は trigger Issue 側の責務。結論先行で数値を寄せていない）。

| 項目 | 設計の現判定 | 計測が示す推奨 | 根拠データ | 未解決点 |
|---|---|---|---|---|
| `telemetry`（prepared Counter） | 構成候補として維持・要 PoC | **形状は有力候補として維持（採否は trigger Issue 判断）**: `Arc<AtomicUsize>` 計数ペアのスケッチ形状で、equal-work 上の計装デルタは +3.99 ns/op（synthetic バッチの単一スレッド推定値 — 本番ヘッドルームの証明ではない）。フラグゲートは on で無条件計装と同コスト・off で +0.74 ns/op の残存コスト — 常時計装の単純さが有利な方向は変わらない。新クレート新設・既存 API への counter 適用は consumer / DAG / ADR トリガまで引き続き deferred | counter_equal_work グループ 4 列 | 2 つ目のドメイン計測点需要は引き続きトリガ条件（実需要の確認は PoC 管轄外） |
| `rt_bridge` 形状・オーバーフロー方針 | 構成候補・形状確定済み（要 PoC） | **設計確定どおり drop-on-full + 共有カウンタを支持（取りこぼしを観測できる最小信号）**。count 方針は record パス・retire_channel 先例と同コスト級（単一スレッド推定値 ≈5.4 ns/op）。aggregate（非ゼロ fold）も同一コスト級 — 選ぶ根拠はコスト差ではなく集約セマンティクスの要否のみ（撤回済みの 5x 差を根拠にしない） | rt_bridge・overflow_policy_nonzero グループ | 初回計測の非対称解釈（5x・false sharing）は撤回。残課題は silent drop と計測方針の ~3.9 ns 差の意味づけ（観測可能性の対価） |
| `metrics` バックエンド | 保留（#104 PoC 結果待ち） | **延期（現時点で採用しない）**: 集約需要が未発生・facade の RT 検証なし — コスト推定値ではなく需要・検証欠如が延期理由 | metrics 行（メタデータスナップショット） | 最初のメトリクス集約需要（xrun/飽和レポート等）出現時に再評価 |
| `profiling`（tracy-client 等） | 保留（#104 PoC 結果待ち） | **延期**: 需要なし・RT 検証なし。C++ ツールチェイン要件と `unsafe` 方針の整合も別途検討が必要 | tracy-client 行 | 需要出現時は feature flag 付きの opt-in 構成のみ検討（RT クレートの強制依存にはしない） |
| `logging`（tracing/log） | 保留（現時点では不要） | **RT 経路では採用しない**: Subscriber 側で任意の alloc/blocking コードが走り得るため、RT 経路に logging コールバックを置けない（設計の「RT 経路に未検証 Subscriber コールバックを置かない」条件どおり）。「425 ns だから遅い」は根拠として使わない — 構造的理由による不採用 | tracing 行 | 非 RT 構造化ログ需要が出た場合に限り再評価（ライブラリはグローバル Subscriber を初期化しない原則を維持） |
| タイムスタンプ取得方法（rt_bridge レコード schema 内の委譲項） | #104 PoC へ委譲 | **`Instant` → epoch 相対 nanos → `i64` を採用可**だが 29–39 ns/stamp は全件スタンプに重い。**呼出し側提供 epoch + 観測点ごとの必要箇所のみスタンプ**を推奨。`SystemTime` は安い（15 ns）が壁時計で不適。実機経路は `StreamInstant` → `MediaTime` 継続 | timestamp グループ 4 列 | 実機 `StreamInstant` 経路の実測は cpal 実コールバックが必要（本 PoC 範囲外、同形数学の代理計測で代替済み） |
| alloc 検出カバレッジ隙間 | （RT 検証レーン設計の限界記録） | **機構的限界として確定記録**: thread-local 窓は設置スレッドのみ観測可能（caller=0/spawned=26>0 を決定的に実証）。backend 所有スレッドの計測は窓を当該スレッドに設置する設計が前提 | alloc_gap 実証 | 将来の窓設置所有権設計（callback 内窓設置・process-wide 計数器併用）の検討材料として留保 |

## 補助データ

- コンパイルコスト: `criterion` dev-dep は ruviform-math の simd_poc と同一 pin 方針（`0.8.2`・`default-features = false` で rayon/plotters を除外）を既存先例・ライセンス理由で採用。推移的依存数の具体的なパッケージ数は未検証のため記録しない。dev-dep 導入前後のコンパイルコスト比較は有効な基準測定が保持されていない — 記録されていた値は導入後ビルドのみで、before/after としては成立しない（計測欠落として受理済みの逸脱）
- ハーネス健全性: PoC scaffolding・ベンチとも `unsafe` 0（`unsafe_code` lint ゲート通過）。self-consistency テストが測定窓内 alloc/dealloc = 0 を `alloc_counter` で機械検証済み（`mise run test` グリーン）
- MSRV: `rustup run 1.89 cargo check -p ruviform-rt --lib --locked` は dev-dep 封じ込め（`--lib` 範囲）で検証 — 候補クレートは workspace 依存に含まれないため MSRV リスクなし
- 混同行列注記: `rt_bridge/drain/half` の elems=256（`Throughput::Elements(fill)`）で、他全列は elems=512（`BATCH`）。`ns_per_op` はこの elems で正規化済み

## 未計測・残課題

- 実機 `StreamInstant` タイムスタンプ経路（cpal 実コールバック上でのみ取得可能）は代理計測のみ。実機計測は `ruviform-audio-device` 側の別検証で扱う
- `rt_bridge/record/not_full` の run 間偏位（4.35–6.68 ns/op）と `drain/empty` の高分散は原因未分離の計測ノイズとして記録（fixture 再確保・システム負荷・小絶対値等の可能性を含む仮説） — 低負荷環境での再計測が必要なら別途実施
- 本計測は Windows x64 単一環境・単一スレッド非競合のレイテンシ推定値のみ。スレッド間競合（reader/observer 同時アクセス）・エンドツーエンドのコールバック予算は未検証 — ハード RT 上限の根拠としては使用不可。Linux/macOS での `Instant::now()`・atomic コストもプラットフォーム別に再計測が必要（設計の「対象プラットフォーム別に検証」条件どおり）
- tracy-client 0.18.5 で観測した `SymInitialize FAILED` 警告は 0.19.0 で未再現。採用時は解決版の再確認が前提
- バックエンド候補のマイクロコストは暫定・非アーカイブ観測値 — 採否判断に使う場合はアーカイブ済み再計測（生サンプル + 記録済み出力）が前提

---
id: "KNOW-CWIX9f5dlGyyMPRRkYHgnB"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "GUI アーキテクチャ・GPUI・dock・ウィンドウ"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/28", "https://github.com/scarecrow173/ruviform/issues/181", "https://github.com/scarecrow173/ruviform/issues/182", "https://github.com/scarecrow173/ruviform/issues/243", "https://github.com/scarecrow173/ruviform/issues/257", "https://github.com/scarecrow173/ruviform/issues/258", "https://github.com/scarecrow173/ruviform/issues/259", "https://github.com/scarecrow173/ruviform/issues/260", "https://github.com/scarecrow173/ruviform/issues/262", "https://github.com/scarecrow173/ruviform/issues/263", "https://github.com/scarecrow173/ruviform/issues/310", "https://github.com/scarecrow173/ruviform/issues/311", "https://github.com/scarecrow173/ruviform/issues/313", "https://github.com/scarecrow173/ruviform/issues/386", "https://github.com/scarecrow173/ruviform/issues/387", "https://github.com/scarecrow173/ruviform/issues/388", "https://github.com/scarecrow173/ruviform/issues/389", "https://github.com/scarecrow173/ruviform/issues/390", "https://github.com/scarecrow173/ruviform/issues/404", "https://github.com/scarecrow173/ruviform/issues/423", "https://github.com/scarecrow173/ruviform/issues/426", "https://github.com/scarecrow173/ruviform/issues/431", "https://github.com/scarecrow173/ruviform/issues/465", "https://github.com/scarecrow173/ruviform/issues/532", "https://github.com/scarecrow173/ruviform/issues/552", "https://github.com/scarecrow173/ruviform/issues/631"]
  informs: []
  explains: []
  supersedes: []
---

# GUI アーキテクチャ・GPUI・dock・ウィンドウ

この文書は、Ruviform の GUI アーキテクチャ・GPUI・dock・ウィンドウに関する設計・実装の知見を Issue 番号の帰属付きでまとめたものである。実測値や観測された挙動は事実として記し、解釈や仮説を含む記述にはその旨を明記する。

## PoC ハーネスの隔離構造と計測規約

`poc/ui/` は独立 cargo ワークスペースとした。ルート `Cargo.toml` の `members` が glob なしの明示リストであるため PoC は吸収されず `exclude` は不要と検証した。`ui-poc` 単一バイナリに `--experiment` で実験を起動時選択させ、wry WebView が 1 イベントループしか持てない制約に合わせて実験間はプロセス分離とした。外部型（wry/tao/winreg/windows-rs）は `host.rs`/`assets.rs`/`check.rs` に封じ込め、`exp/*` と `metrics.rs` は内部 host I/F のみに依存させる。この境界維持を、将来のクレート分割のリハーサルとして評価軸に記録した（Issue #28）。

計測の客観性は規約で担保する。`poc-metric.v1` スキーマに `env` を同梱し、生サンプルは `out/` で再生成可能、結果文書への再現コマンド記載を必須とした。ヘッドレス環境で実行不能な GUI/GPU 実験は `status:"skipped"`・非空 `reason`・exit 0 で分離し、CI が呼ぶのは `--check` のみ。計測値の撤回は行削除でなく訂正行の追記で行う。未ビルド時は `reason: frontend not built` で skip し、`package-lock.json` コミット・`dist/` gitignore・`npm ci && npm run build` を標準前提とした（Issue #28）。

wgpu の MSRV 1.87 が宣言 `rust-version = "1.85"` と衝突する確定リスクには、独立ワークスペースと pin toolchain で先行検証し、衝突の事実を記録して MSRV 見直し専用タスクへ連携した（引き上げ自体は決めない）。wry の上流 PR 未 merge で真の DOM 内合成が公式には作れないため、隣接構成（`adjacent-parent`/`adjacent-child`）を主経路とし、DOM 内合成は未 merge の上流エビデンスとして記録し、fork スパイクは探索的付録に隔離した。Linux 配布は文書評価止まりのため依存パッケージ・API バージョン表で定量化し、「文書評価」ラベルを test-spec で強制した（Issue #28）。

## Window/Job 契約と寿命分離

window 契約型は `WindowId`（NonZeroU64 ベース、`new(u64)->Option<Self>`）と `RoutedEvent{window: Option<WindowId>}` で構成し、`None` は ownerless job/event を表す。`CloseDecision{Approve,Veto,Defer}` と `WindowHandle` trait（eval/resize/focus/create_window/request_close/respond_close/destroy_window/windows/monitors/exit）を定義し、全公開 enum/struct に `#[non_exhaustive]`・全 API doc・RT 非該当明記の規約を敷いた。全変更は additive で、`HostEvent` には `Moved`/`Destroyed` を追加し、単一 Window 経路の `CloseRequested` は従来どおり維持した（Issue #181）。

Job owner は寿命ではなく routing のみを担う。spawn Job に `owner=Some(w)` を付与し、`CloseRequested` は宛先 Window に返すが Job は残す。`cancel_window_jobs(window)` は owner Window 所有 Job のみ opt-in cancel とし、既存 `handle`/`dispatch_op` 経路は `owner=None` の非回帰経路として残した。Window 破棄後の宛先 event は silent drop（可観測 drop counter は deferred）、未回答 `CloseRequested` の既定は Window 単位の `Approve` でループ終了には繋げない。runtime に project 概念がないため「App/Project-owned Job」は window 非紐付け job（`owner: None`）に対応し、consumer 側の policy 区別は close-policy テストで被覆する（Issue #181, Issue #243）。

`WindowHandle` を実装する FakeHost（HashMap<WindowId, FakeWindow>・close 決定機構・bounded pending drop-oldest・eval 配送記録）を adapter 契約セマンティクスの参照実装とし、2 Window イベント分離・id 衝突 routing・Approve/Veto/Defer・未回答既定 Approve・破棄済み宛先 silent drop・pre-Ready bounded 保留・create_window 失敗で Window を残さない・Window Close で Job 非取消・cancel_window_jobs opt-in・全 Window 破棄でも auto-exit しない、の 11 シナリオを headless で検証する構成とした（Issue #181）。

WebView2 は UDF シングルトン制約があるため、`run_multi` は共有 `WebContext` を 1 基だけ持ち（data_dir 既定 `rvf-udf-multi-<pid>`）、全 WebView が `new_with_web_context` で共有し、`WebContext` は全 WebView より長命（run_multi ローカル）にして drop 順序を構造で担保する。pre-Ready eval は bounded pending（cap 64・drop-oldest・Ready flush）、不明/破棄済み宛先は silent drop、Window 全滅でも終了せず `exit()` が `want_exit`→`MainEventsCleared` で Exit する last-window policy を採った。tao/wry の動的 Window 生成は型検証までで、WebKitGTK/WKWebView の実機差異は `unverified` と記録した（Issue #181）。

## Dock モデルの不変条件と永続化

`ViewInstanceId` は consumer 採番の NonZeroU64（serde transparent・Display `v{n}`）。`DockTree::new` は normalize+validate を通し、normalize は空 Tabs 除去・2 未満の子 Split 畳み込み・active clamp のボトムアップ再帰、validate は ratio 有限 (0,1)・葉 views≥1・split children≥2・tree 内重複 View なしを拒否する。`DockNode` は `#[non_exhaustive]` で `tabs`/`split` コンストラクタを持ち、serde 表現は `#[serde(tag="type")]`。TDD で構築拒否 EmptyTabs/InvalidChildren/InvalidRatio を固定した（Issue #182）。

View 移動は two-phase である。`begin_move`（検証+pending 登録・構造不変）→`commit_move`（target 再検証、消失なら `MoveAborted`+`Err(TargetUnavailable)` で自動 rollback、成功は detach→attach→normalize→`MoveCommitted`）→`abort_move` の順。`remove_window` は policy 必須引数（既定なし）で `WindowCloseDisposition{DetachViews,RelocateTo,ReleaseViews}` を返し、Job cancel/Project 語彙を持たない。`move_view` one-shot が menu-move 経路（drag 機構なし）として提供される（Issue #182）。

serde restore は `#[non_exhaustive]` 構築子を迂回して素の形を復元するため、`DockTree::restore` が normalize+validate を再適用して不変条件を再保証する。永続化は `LayoutSnapshot{schema,windows,detached}`（`LAYOUT_SCHEMA_VERSION=1`）の JSON round-trip で、restore は schema 一致（不一致→`UnsupportedSchema`）・各 tree の clone→normalize→validate・全 window+detached 横断の View 重複拒否・w/h>0（`InvalidSnapshot`）を検証し、破損時は `restore_or_default` の安全な空 layout に落ちる（Issue #182）。

`MonitorHint{name,width,height,scale_factor}` は signature のみで永続 display ID を仮定しない。`correct_bounds` は rect center が現モニター内なら採用、外なら signature 一致時に該当モニター・不一致なら先頭モニターへ中央配置（サイズクランプ）、monitors 空なら rect 保持とする。`WindowConfig.position: Option<(i32,i32)>` は `Some` 時のみ `with_position` を適用する additive 契約だが、単位は物理 px と logical px の記述が分かれており未確定である。フィールド追加は外部構築リテラルを破壊しうるため `..Default::default()` 必須を文書化し、`DockPath` の構造変更無効化が consumer を躓かせる点は op を view 基底・path API を補助と明記して緩和した（Issue #182）。

## driver 中立 WindowDriver 契約

`ruviform-ui::driver` に `LogicalSize`/`WindowSpec`/`WindowLifecycle`/`DriverEvent`/`WindowCapabilities`/`WindowDriver` を新設し、`UiError::Unsupported{capability,message}` と `UiRuntime::dispatch_window_intent` を追加した。`ruviform-ui-gpui` は非既定 `gpui` feature ゲートとし、`cargo check --workspace` が GPUI なしで通ることを契約（R1）とした（Issue #386）。

FakeDriver は FakeHost と同じ close-veto/Destroyed/drop-oldest 意味論を `WindowDriver` port で実装し、inspection API（delivered/windows/post_event inbox）を持つ。`#[gpui::test]` suite は後続 Issue へ defer し、semantic 契約検証を fake_driver fixture と runtime dispatch parity に集約した（`test-ui-gpui` の実行 test 数 0 は design deviation として記録）。実 OS 検証（実 window・DPI・IME・OS focus・実 close・GPU device lost・実 plugin Native Dock）は全て NotRun で、手動 entry は `example-ui-gpui-minimal`（Issue #386）。

共通 lifecycle fixture として `ruviform-ui-test::lifecycle` の `run_lifecycle`+`expected_trace()` を追加し、2 Window・2-phase move commit/abort・detach/reattach・focus・late post_event・close veto/defer/approve・unanswered close 既定・id 非再利用・tear-down を中立 `WindowDriver`+Dock だけで実行し、Fake と GPUI の両 driver で同一正規化 trace に一致させた。gpui test platform の window は OS handle を持たず `window_handle()` が panic するため、`capture_raw_parent` を `cfg(test)`+`gpui-test` で None を返す seam とし、`settle_for_test` も `pub(crate)` seam とした。公開 signature に gpui 型を出さない設計で、本番・非 test ビルドの挙動は不変（Issue #386）。

`GpuiDriver` は `focus` を Supported と宣言するが gpui 0.2.2 の制約で `focus(_, false)` は `UiError::Unsupported` を返す。fixture が「Ok か Unsupported のどちらか（Host error ではない）」を要求するため、headless trace の一致は capability 宣言の正しさを保証しない。この教訓から `WindowCapabilities::focus` の文言を「取得（`focused = true`）」に限定し、release は操作単位で `Unsupported` を返しうると明記して宣言と実装を一致させた（Issue #386, Issue #423）。

WebView adapter は `session.rs` を neutral WindowDriver へ移行し `host.rs` を page load/IPC decode 専用に縮小し、公開面を `run_multi`/`SessionConfig`/`WebViewWindowConfig` のみへ限定した。旧 `run`/`HostConfig`/`SessionConfig::single`/`dispatch_script` 再 export は削除し、design からの逸脱として adapter 内に閉じた。IPC decode 失敗は `Event::error` を origin window へ返し、late receipt は `commit_on_host` を抽出して driver 経由の描画投影まで検証した（Issue #423）。

## driver-neutral 契約の正本統合と境界 guard

蓄積済み driver-neutral UI 契約（方針・Theme・Window/Host・Component/View・Lab/Guard・Scenario・補遷契約）は 1 つの正本設計書へ参照統合され、領域別索引・受入項目ごとの証拠対応表・歴史的参照の区別・未確定事項を含む。参照先の文書と型が main merge 済みであることは `git log` とクレート実在走査で確認した。GPUI 採否は未確定であり、この正本が採用確定の根拠にならないことを明記した docs-only の変更である（Issue #257）。

依存境界の guard は dep-graph と公開 signature の 2 面を検査する。dep-graph 検査は workspace member 起点で direct/feature/dep.features/transitive/conditional/dep-activation/allowlist 整合を見るが、cargo metadata の resolve graph は未 activation optional dep を省略するため、conditional 到達は manifest 宣言（`dep.path` 再帰走査）で補完する。optional/dev/rename 宣言による検査迂回は不可とした。signature 検査は syn+proc-macro2 の exposure-aware 2-pass 走査で crate 外露出 item（pub use chain・alias・公開 surface を持つ impl の header/assoc item）のみを対象とし、cfg-gated item は保守的に全走査する（Issue #390）。

guard verdict=pass は「依存・公開 signature 境界が clean」だけを意味し、semantic pass ではない。consumer 側の契約検証行は boundary lane（ruleset 適合）と semantic fixture lane（fake layer の実行判定）の 2 つの独立した実判定を持ち、fake layer の pass は GPUI・実 OS・Native Dock の受入証拠にならない。status 語彙の正直化として、契約が main に存在するのに `blocked` と表記されていた fixture は実態へ修正し、oracle を覆っていない pass は `not_run`+rerun 条件へ修正した。`ruviform-ui-gpui` の allowlist grant は crate が workspace member として実在するため required:true へ更新し、protectedPackages（`ruviform-ui`・`ruviform-ui-theme`）を ruleset で名指し pin した（Issue #390）。

検査の限界も記録されている。glob re-export の全展開は名解決なしには不可のため private-mod の pub item 全走査で塞ぐ保守設計とし、macro 生成コードは未走査、bare OS handle 型名（import 後の裸名）は path root 検査の対象外で対応 OS/FFI crate 依存は dep-graph 側で捕捉する。dep 宣言先の package 解決は name 一致ベースで同名 multi-version 共存時に誤同定の可能性があり、registry dep の未 activation target は `--no-deps` で解決不能のため skip とした（Issue #390）。

## behavior・view・viz の実装契約

`ui/core` は DOM/WebView/React 等のランタイム依存を持たない vanilla TypeScript の ESM パッケージ `@ruviform/ui-core`。「初期実装は TS module で良い、Rust crate は実需要まで不要」の判断に従い、Node 26 の erasable TypeScript 直実行（`node --test`）でビルド工程を持たず、型検査は `tsc --noEmit`（DOM lib 非含有）、devDeps は typescript+@types/node のみとした（Issue #258）。

`adjustNormalized` は normalized 近似量子でなく宣言 step ドメインで整数 step 演算する。plain step 宣言で normalized 入力が snap を bypass する問題を実装中に確認し、`resolvePlain`/`resolveNormalized` の双方向 resolve で宣言ドメインに再 snap する形に修正した。非線形写像（log）では normalized 側近似だと double conversion 誤差が生じるため、step 演算は常に宣言 `stepDomain` で行う（Issue #258）。

レビューで修正した 5 件を記録する。(a) detach/focus-lost が seq drop で取りこぼされジェスチャが残存 → ライフサイクル入力を seq 免除+gestureId キー化、(b) custom 写像の範囲外出力が committed 値に流入 → `invalid-mapping` 化、(c) 宣言ドメイン再 snap、(d) ポートコールバック throw の伝播 → degrade 正規化、(e) `decibelFormatter` の非有限入力が −∞ を詐称 → `String(plain)` フォールバック。`constant` degenerate は atomic commit を許す（自動化可能な固定パラメータを塞がない）（Issue #258）。

controlled value は `value`/`checked`/`valueLow`/`valueHigh` の書込みを consumer の authoritative echo として `atomic` meta で抑制し、active gesture 中・unavailable 中・connect 前は `pendingExternal_` に stage して gesture 終了時に着地させる。`descriptor` swap は新 default に reset、`ports`/identity swap は committed value を保持（dock-transfer）、in-flight gesture は必ず `detach` で終端する（Issue #259）。

gesture id は一意 mint が必須（`ptr-<id>-<seq>` 等）。終端済み id は `already-terminated` で拒否されるため再利用すると 2 回目の drag/wheel/edit が壊れる。`meta.sequence` は controller rebuild（descriptor/ports/control-id 変更）で consumer dedupe を壊さないよう element で rebase する。RangeSelector の edge-crossing 防止は presentation-level clamp でなく `preprocess` の input-dispatch 境界に置き、pointer/keyboard/wheel/programmatic 全経路で観測させた（Issue #259）。

token は 3 層（foundation/semantic/role）解決で、解決順は `reference defaults → base (light|dark) → density → scope overrides → instance tokens → semantic state CSS`。失敗 token は `TokenDiagnostic`（unknown-token/type-mismatch/non-finite/unknown-ref/circular-ref）として報告し CSS literal fallback を適用する。`<rvf-theme>` は全 `--rvf-*` var を自身の inline style に設定するため双方向に漏洩しない。NumericInput は compositionstart..end で全 dispatch を suspend（IME-safe）し、default parser は末尾 unit suffix を strip・空文字を reject する（`Number("")===0` で cleared field が 0 commit するのを防止）（Issue #259）。

paint/data 分離は構造的に保つ。`ResolvedVizStyle` は paint 値のみで cache key・query state・subscription は style を観測しないため、`appearance`/`--rvf-viz-*` 変更は `styleDirty`+repaint のみ。LOD budget（`buckets`）は element width でなく laid-out plot width を使う（初回 query は実 layout `lastLayout_` を待たないと stale key で cache される、review で発見）。deferred-first-query は `invalidateQueued_`+in-flight check で requery を 1 件に bound する（無いと rAF 毎に query が再発行される無限ループ、実測で発見）。stale view は issue 時と resolve 時の両方で revision 一致を要求し、NaN/非有限は gap（補間・ゼロ埋めしない）。snapshot pull 系は sequence gap→`dropped`（visualize）・age→`stale`（hatch+status text で色だけに依存しない）を surface する。meter の availability は dB-mapped value でなく raw amplitude で判定し、有限 `0` は silence（空 bar）、`NaN`/`±Inf` は unavailable（hatch）とする。`setSnapshotSource` で `lastSeq`/`dropped`/`stale` を reset する（異 source の sequence は比較不能）。`linearTicks`/`axisTicks` は非有限・degenerate 入力を reject し反復上限 4096（Issue #260）。

Rust 側 `ruviform-ui::behavior` は `ui/core/{value,interaction}.ts` と `ui/binding/src/contract.ts` の意味論を port し、TS 不変条件（safe Begin/terminal 後 noop/detach→cancel/availability→cancel/receipt rank/projection revision guard）を保持する。`CommandBinding` の ack timeout は時計を持たず `on_tick(now: Instant)`/deadline 指定で caller 駆動とし headless deterministic を維持する。drag 中 pointer が element 外へ出ても window 全 mouse move listener+`on_mouse_up_out` で Commit する（Issue #387）。

view 層は ComponentSpec/Panel/Tabs/Fader/Seek/Waveform/ViewHost で、reconcile は key map diff、generation は `ViewInstanceId` 毎に採番して stale を拒否する。`WaveformBin` 中立 payload 型は bounded（`MAX_WAVEFORM_BINS`）で `ruviform-viz` 依存を追加せず、spec payload 型と `ruviform-viz::ViewBin` の field 意味 drift 対策として片方向変換を consumer 側に集約し field 名を `ViewBin`/`LodBucket` に一致させた（Issue #387）。

## wire validator・fixture・oracle の独立性

`ui/guitest` に private package を新設し、`fixtures/README.md` に形式 v1・バージョン規則・denylist/allowlist 語彙を正本化した。fixture は `ipc/{intents,events,data}.v1.json`・`binding/messages.v1.json`・`scenarios/{binding,host}/*.v1.json` の 3 層 15 fixture で、denylist field で実 media/prompt/grant/token/binary 非含有を保証する。`fixtureload.ts` は `v`/`suite`/case 構造を fail-closed で検査する。byte 精度の boundary は `bodyGen: pad-intent` で生成する方式にし巨大文字列を fixture にハードコードしない。query string の parse 契約は `rawQuery` で文字列のまま pin、scenario receipt 相関は `ref` 語彙を `last`/`first`/`dead-epoch`/`unknown`/literal の 5 値に統一した。recordings fixture は見送り recorder 往復を inline JSONL で検証し、`test/env.ts` hook は Node 24+ が .ts を直接実行するため不要と判断した。Windows パス区切り対策として fixture に絶対パス・区切り文字を書かず両側とも path API で組み立てる（Issue #311）。

`ipcwire.ts`/`bindingwire.ts` は zod 等不使用・依存ゼロの手書き validator で、`rvf.ipc.v1` の Intent parse を Rust `parse_intent` と同一判定（サイズ上限・`v`/`kind`/`op` 形・`id` 型）で行い、Event envelope decode・`rvf://data` の dataUrl 構築往復・応答契約モデル（`bodyLen ≤ min(len,total-offset,maxChunk)`・`X-Rvf-Total`）を実装する。拒否語彙は `missing-field`/`bad-kind`/`bad-type`/`out-of-range` で Rust 側 error code と 1:1。mirror は wire 意味のみを共有し実装コードからの import は型のみに限定する。契約拡張時は fixture `v` バンプと validator 更新がセットになる運用制約がある（Issue #311）。

ContractModel oracle は「文書化契約規則のみの観測モデル」（receipt rank 分類・projection revision 単調性・generation/detach 規則等）で、実装 src とコード共有なし（型 import のみ）とし、内部構造（queue/tombstone/tracked map）を再実装しないことで維持負荷を抑える。fixture `expect` pin は oracle 生成ではなく手書きに留めた。`IpcRecorder` は既定 `enabled:false` no-op・`maxEntries`/`maxBytes` bounded・最初の overflow で `sys/dropped` marker 1 回・文字列 128 文字 truncate・機微 field は denylist で構造的に収集しない。`seed.ts` は mulberry32 PRNG で fault 種別・順序・遅延を seed から導出し、`ScriptedDawBackend`/`ScriptedAnalysisBackend` は契約形状を写すが製品語彙を持ち込まない。記録済み応答の再生は PASS 条件に含めない。`replay.ts` は実 binding に注入して trace を収集し `assertDeterministic` で 2 回実行の一致を検証する（Issue #311）。

replay 実装で修正した設計ミスを記録する。dispatch/subscribe/query/cancelQuery は全て binding→consumer の `out`、reply のみ `in` と方向意味を修正した（初期実装は逆で録画 diff レビューで発見）。entries-bound overflow 時に `sys/dropped` marker 自体が drop される問題を marker 枠 1 つ予約で解消した。`checkExpects` は exact equality ではなく ordered-subsequence pin 照合とし interleaved 中間 event に耐性を持たせた。Rust lane は `Event` が Serialize-only+`#[non_exhaustive]` で deserialize 不可のため「公開 constructor が pin wire shape を出す」検証に限定し、decode/拒否面は TS lane が担う。destroyed window の event は `drain_jobs_routed` が error でなく生産し続けるため、runner が `alive` set で delivery-layer drop を再現し実 wire 到達意味のみ trace に残す。`waitJobs` は job 実行系の性質上実時間 poll（timeoutMs bounded）とした（Issue #311）。

receipt routing は `commandId` でなく `intent.controlId` を使う。`cmd-N` は binding 毎に mint され衝突するため、`gateway.deliver` を fixture 毎に 1 回だけ配線し `(receivedRecord, receipt)` で受け、全 receipt method が `DemoReceivedCommand` を取る設計に改めた。`addTarget` は duplicate 宣言で throw（shared-gateway cross-mount bleed を fail loud に）。`domFingerprint` は semantic fingerprint（tag+part/role/name/slot/tabindex/type/aria-*/data-*）で `<style>`/`<script>`・classes・inline styles・private nesting を skin internals として除外し、deferred cell は `data-deferred` の honest placeholder とする（fake element や空 canvas を置かない）。dependency audit は `UI_PACKAGES` を filesystem から発見し peer/optional deps が空であることを assert し、import specifier scanner は comment を strip して `require("x")` も match する（Issue #263）。

## Native Dock の境界配置と session 状態機械

plugin↔ui 相互依存禁止の境界配置として、architecture baseline の「plugin 系と ui 系の相互依存禁止・consumer 側で結線」に従い、共有 opaque 語彙（`PluginEditorSessionId{slot,generation}`/`ParentSlotLease`）は `ruviform-plugin-core::boundary`、slot 契約（`SurfaceSlotId`/`SlotLease`/`SlotSpec`/`SurfaceSlotRegistry`/`NativeSurfaceSlot`）は `ruviform-ui::native_slot`、session 状態機械（`EditorSessionState`/`EditorSessionOps`/`InProcessEditorOps`/`L1EditorOps`/`PluginEditorSessions`）は `ruviform-plugin-host::editor_session` に配置した。lease→raw OS token の償還は composition/adapter 側に限定し、`InProcessEditorOps::with_redeemer` の `LeaseRedeemer` 注入 seam で実現する。型上の強制は弱いため、dep-graph guard で composition-root のみ adapter 依存を許可する構成で補強した（Issue #388, Issue #389）。

capability は bool 群でなく `#[non_exhaustive]` enum `Support` で表現し「既定は全 Unsupported（能力未実証）」の honest default とした（bool 群は clippy `struct_excessive_bools` にも抵触）。`EditorSessionError::ResizeRejected` は session を終端へ送らず observable な拒否として返す。`L1EditorOps` は全 op `EditorOpError::Unsupported` の defined refusal で wire 非送信・L0 への暗黙退化なし（Issue #388, Issue #389）。

`concluded` flag で born-`Closed` と lifecycle 収束 `Closed` を分離し instance 毎 ≤1 live session を強制した。full workspace test が `Closed` 初期状態を収束済みと誤認し second live session を通す回帰を検出・修正した。独立 review で 2 件の P1（concluded session の再 open 経路・live session の end_session 除去）を確認・修正し、副次修正として lease seq の registry-global u32 化・slot generation tombstone・capability 未 enforcement・sync/async reattach イベント不整合・L0 floating open の暗黙 docked 偽装・born-Closed session への terminate による再 open 孔・`NativeSurfaceSlot` trait の release_lease/validate_lease 欠落・`PluginEditorSessionId` generation の u16→u32 拡幅等が挙がった。残 residual として `PluginHandle.generation` は wire 互換のため u16 のままである（Issue #388, Issue #389）。

focus の責務境界は、keyboard focus の要求・ルーティングを `NativeSurfaceSlot::request_focus`/`WindowDriver::focus`（presentation/slot 層）が所有し、dock focus の session 側意味論は `Attached ↔ Hidden`（`set_visibility`）が担うとした。session port の ops 動詞は capabilities/open/close/set_size/set_visibility/reattach の 6 つで focus は含まれず、focus verb を足すと常時 `Unsupported` の動詞と責務重複を生むため追加しない。「Generic Editor」は host 側 fallback UI の語彙で session `AttachMode`（Docked/Floating のみ）に存在せず、本契約が固定する degrade は Floating への明示遷移（`Degraded` event・`OpenOutcome::Degraded`）までとした（Issue #389）。

raw `GuiParentToken` を公開 API から完全除去し、generation 付き不透明 `ParentLease{generation:u32,payload:u64}` へ移行した。`Plugin::open_gui`/`ErasedPlugin`/`HostCommand::OpenGui`/`wire::Command::OpenGui`/`LeaseRedeemer` 戻り値が `ParentLease` を運び、OS handle は adapter 内部に閉じ込める。`ParentLease` は同一プロセス trust boundary の opaque 語彙で認証ではなく、stale/revoked/forged 拒否は redeem seam の ledger 検証が権威である。`OpenGui` payload 変更の codec 規約（payload field-set 変更は常に version bump）どおり `WIRE_VERSION` を 4→5 に bump した（L1 GUI 未実装のため live consumer 影響なし）。`ParentSlotLease` の payload は packed `SlotLease{index,seq}` であり、redeemer は `adapter.redeem(lease.raw())` の token と `seq` から generation を復元して `ParentLease` を mint するため、opaque u64 shape は不変である（Issue #389）。

session hardening として `SessionDeadlinePolicy{timeout, Box<dyn SessionClock>}`+`MonotonicSessionClock`+`DEFAULT_REQUEST_TIMEOUT`（10s・暫定値）を導入し、request の唯一の採番点 `next_request` で全 in-flight request に deadline を arm、driver 主導 `tick`/`tick_all` が期限超過を `Failed{Timeout}`+単一 `Failed` event に収束させる。`set_deadline_policy(None)` で disarm、`request_deadline` で driver が次回 tick を schedule 可能。sync convenience path（`open`/`close`/`recover`/`reattach`）は ops を inline 呼出しするため preempt せず、timeout が意味を持つのは `begin_*`/`complete_*` の async 経路のみ。`Closing` timeout は `complete_close` が error を運ばないため `mark_failed` 経路で `Failed{Timeout}` に落ち、`editor_live` は維持されるため後で adapter が close を完了しても `complete_close` は `StaleRequest` となり terminal event の重複は起きない。monitoring は driver 責務であり、`tick`/`tick_all` を呼ばなければ timeout は発火しない（Issue #389）。

degrade は `open_or_degrade`/`begin_degrade` が preferred mode の `Unsupported` のみを条件に fallback を同一 request id で 1 回だけ retry し、成功時は `OpenOutcome::Degraded{mode}`+`Degraded{mode}` event のみを発火する（`Attached`/`Recovered`/`Reattached` は発火しない）。`preferred==fallback` は retry 不能のため plain `open` に退化する。この retry は `ops.open` を 2 回呼ぶため「`Unsupported` を返す open は editor を生成していない」という前提に依存し doc 明記した。`SessionFailCause::GraphicsLost`/`EditorOpError::GraphicsLost` を additive 追加し `notify_terminated(GraphicsLost)` も `Terminated{GraphicsLost}` を発火できる。`PendingAttach` enum（`None`/`Reattach`/`Degrade`）が排他な 2 bool を置換した（API 経路上排他であり clippy `struct_excessive_bools` 警告の解消でもある）（Issue #389）。

`complete_close` に error channel がないため、`ops.close` 拒否時に `complete_close` を配送しないと request は `Closing` のまま deadline で `Failed{Timeout}` に落ち、後続 `close` が adapter 経路で再収束するのが定義済み挙動である（hang 経由と refusal 経由の両方を fixture で固定し、contract 変更は別 Issue 候補とした）。`reattach` は新しい `ParentSlotLease` を受けるが plugin へ渡す `ParentLease` は issuer 側 seam でしか作れないため、open で使った `LeaseRedeemer` を再利用し redeem を `host.execute` の前に置く redeem-first 順序を採用した（後置だと拒否済み lease が plugin へ到達しうる）。composite は `l0_only_gui_verb` predicate を `Slot::OutOfProcess` 分岐の先頭に置き、`to_wire`（`unreachable!` を持つ）到達前に defined `HostError::Unsupported` を返す（Issue #389）。

FakeHostDriver fixture（`PluginEditorSessions`+`ScriptedOps`+共有 `FakeClock`+pending completion queue+driver 側帳簿）は public API（`begin_*`/`complete_*`/`tick`/`tick_all`/`notify_terminated`/`end_session`）のみを駆動し session 内部を覗かない。`live_editors` 帳簿は attach 受理でなく adapter 真実（`ops.open` が `Ok` を返した時点で真・`ops.close` の `Ok` 配送/termination で偽）を追う。これは review で、stale だが成功した `ops.open` が残す adapter orphan を不可視にする当初モデルの欠陥が指摘されて修正された点である。pending 配送は session 存在確認を ops 呼出より先に行い、registry から消えた session 宛は adapter を呼ばず `stale_drops` に落とす。`hung` op kind は配送 skip で queue に残し、deadline timeout で `Failed{Timeout}` に収束させる（Issue #389）。

「contract headless Pass」と実 Native Dock PASS を区別する。contract headless Pass は driver-neutral 契約と状態機械の headless 検証で、実 Dock PASS には GPUI 実 dock layout（tab/clip/dock 管理 widget）+実 plugin binary が必要であり、window 内 region での実 native attach は dock layout 未実装のため Dock PASS ではないと注記した。OS/format/L0-L1 要求行マトリクスでは、Windows/GPUI main dock・VST3/CLAP・L0 が contract headless Pass+実 GPUI window 内実 native attach Pass（window 内 region）/実 plugin binary NotRun、L1 は Unsupported、macOS/Linux X11/XWayland/Wayland は NotRun/Unsupported（backend 未実装→明示 Unsupported）。第二 increment で Windows 実 GPUI window 内への実 native editor attach を実機観測済みである。引き渡し文面は「L1 完了可否」（OS-L1 は 3 OS で Pass）と「実 Native Dock 非保証」（DComp 画素問題未解決・実 Plugin Dock は Deferred）と consumer 側境界を分離し、Floating/Generic/Mock を実 Native Dock Pass として扱わない（Issue #388, Issue #389）。

## VST3・CLAP adapter の実機発見

VST3 distributed plugin（`single_component=false`）は `IConnectionPoint` 双方向接続が必須である。Vinyl 1.12.0 の実機観測では、未接続では `createView` が null→`Unsupported` であり、接続時の `notify` が `IMessage` を `IHostApplication::createInstance` に要求し、`kNotImplemented` を返すと plugin 内部で null 参照 crash した。`IMessage`/`IAttributeList` の host class 実装（`BridgeMessage`+`BridgeAttributeList`+`BridgeHostApplication::createInstance` の CID dispatch）が実 plugin 互換の前提条件だった。`BridgePlugFrame::resizeView` は defined refusal `kResultFalse` とした（container 幾何は slot 側所有）（Issue #389）。

CLAP `open_gui` は `get_extension::<PluginGui>()` → `GuiConfiguration{WIN32, is_floating=false}` negotiation → `create` → `set_parent(Window::from_win32_hwnd(parent.payload()))` → `show` の順で、dormant（instance なし）・gui 既存の二重 open は `PluginFailed`、extension 不在や `is_api_supported` false は `Unsupported` を返す。`set_parent`/`from_win32_hwnd` は unsafe を adapter 境界内に SAFETY コメント付きで閉じ込め、`create` 成功後の失敗は `destroy` で対称に巻き戻し、`GuiHandle` は attach 成功時のみ mint（`gui_seq` wrapping counter）する。`close_gui` は hide 失敗を許容して destroy を必ず呼ぶ best-effort 対称 teardown。platform gate は `EDITOR_GUI_API: Option<GuiApiType>` で Windows のみ `Some(WIN32)`（VST3 `EDITOR_PLATFORM_TYPE` パターン踏襲）。size negotiation は `open_gui` に含めず container 幾何は slot 側所有。Drop は gui teardown を processor into_stopped/deactivate より前に実行する（Issue #389）。

IME verbs の format 差として、`IPlugView` に hide/show verb は存在せず、hide=`removed()`（detach・plugin UI state 喪失）、show=`setFrame`+`attached` で同じ parent/frame に再構成（新 generation の HWND）する。CLAP は `hide`/`show` の真の verb があるため、意味差を維持したまま同じ session 遷移（`Hidden`/`Shown` event）へ写像した。VST3 `set_gui_size` は `canResize` gate→`checkSizeConstraint`→`onSize` で logical:pixel 1:1 の既存 documented deviation を踏襲し、`set_gui_parent` は remove→新 parent attach・失敗時 old parent best-effort restore、`set_size` の plugin refusal は session 非致命（`Attached`/`Hidden` 維持）、`set_visibility` は adapter 成功後のみ遷移し no-op は event を発火しない（Issue #389）。

HWND 値は destroy 後に OS が再利用しうるため、「旧 child が死んだ・新 child が生まれた」を HWND 値だけでは証明できない。fixture の attach 毎に window title へ `rvf-test-editor-attach-{n}` を刻印し generation 前進を title 比較で観測する方式を採用し、`ChildWindowFacts` に `width`/`height`/`title` を追加して `GetWindowRect`/`GetWindowTextW` を query-only で観測、`IsWindow` で teardown を証明した。resize oracle は全 child の rect 一致を要求する。lane verdict の Pass は accepted+lifecycle verified+`effect_confirmed` の 3 条件を要求し、refused→`Unsupported`・accepted+未確認→`NotRun`・契約不一致→`Fail` の写像を維持した（Issue #389）。

lifecycle 規範を実 `real-abi-test-plugin` での観測として記録する。`Destroy` は `Activated` から illegal lifecycle transition で拒否され `Deactivate`→`Destroy` の順が必要である。born-`Closed`（open 未実施）の session に `notify_terminated` しても `Terminated` event は発火しない（`Closed` からの遷移のみ event を発する規範）ため、host-terminate 系の検証は refused open で `Failed{Unsupported}` に落ちた live session を使う経路を採る。recovery の起点は、`open_gui` が view 既存で `PluginFailed` を返すため live editor からの `Failed` 遷移では recover を試せず、`open(Floating)` が非 Docked を `Unsupported` 拒否され `Failed{Unsupported}`+`editor_live=false` に落ちる点である（実走で events `[Failed{Unsupported}, Recovered{Docked}, Closed]` を観測）。timeout は `SessionDeadlinePolicy{timeout: Duration::ZERO}`+`begin_detach`+`tick_all()` で deterministic に `Failed{Timeout}` へ収束させる（Issue #389）。

Terminated exactly-once oracle は、event 列がちょうど `[Attached{Docked}, Terminated{HostLost}]` であることと、2 回目 `notify_terminated` 後の drain が空であることの 2 段で断言する。`notify_terminated(HostLost)` は by_instance binding を除去してから session 通知するため 2 回目は lookup miss の no-op になる。`open_or_degrade` の double-refusal（test_gain は Docked/Floating 両拒否）は `Err(EditorSessionError::Ops(Unsupported))` を返し `Degraded`/`Attached`/`Recovered`/`Reattached`/`Shown` event 0 件であり、拒否を degrade 成功と偽らない契約を実 binary で観測した。test_gain の `createView` null により open は必ず `Ops(Unsupported)` に落ちるため、session case の Pass は registry 契約（SessionBusy・refused-open 収束・Terminated exactly-once・degrade 非発火）の証拠に留まり、実 editor attach・画素の証拠ではない。crash/hang fault は in-process lane では実行不能（fixture crash は hosting process を殺し、hang は lane を永久に塞ぐ・out-of-process plugin host 非存在）のため `required` row として catalog に残し、body が architectural boundary を説明して `Unsupported` を返す — `NotRun` 偽装・fault 実行・行欠落のいずれも行わない（Issue #389）。

case 固有の session drive は `AttachDrive{drive_open, observe_open}` seam に一般化した（degraded=`open_or_degrade`、recovery=refused open→recover、timeout=attach 後 `begin_detach`+deadline）。`drive_open_docked` が従来の `session.open(Docked)` step row を逐語再現し、既存 case の evidence shape を維持する。real lane catalog と executable entry（`real_entries()`）の pairwise 1:1 同期は専用 test が gate する。前提バイナリ依存で `ruviform-plugin-l1-runner` 未ビルド時に e2e が環境要因 fail し、build 後に全件 Pass となった例（変更とは無関係のテスト前提依存）も記録する（Issue #389）。

## GPUI の実装特性

GPUI は invalidation 駆動である。`bind_native_slot`/`unbind_native_slot` は bindings map 更新後に mounted view entity へ `cx.notify()` が必要で、notify が無いと静的 view が二度と paint せず `project_slot_frame` が走らず slot が永久に Hidden のままになる（実走で `project_geometry` fail として観測→修正後 pass）。状態変更系 API は notify を伴う必要がある（Issue #631）。

GPUI は最終 window destroy で platform loop が終了し UiJob pump が死ぬ。実 window lifecycle test で「scenario command timed out — gpui app thread unresponsive」が serial/parallel 両方で再現し、並列干渉説を棄却した真因は、最終 window 破棄で GPUI loop が終了し、破棄後の `settle_until`/`window_geometry` が reply を得られないことだった。sentinel window を残す構成は flaky 回避ではなく last-window-exit アーキテクチャへの構造的必須条件として恒久記録された（Issue #631, Issue #243）。

close 応答は armed queue 設計を採る。`WindowDriver::request_close` は `CloseRequested` emit と同じ同期 flush 内で unanswered request を default `Approve` するため、caller 側が `UiJob` で後追い `respond_close` しても必ず負ける。`arm_close_response(window, decision)` で per-window FIFO の `CloseDecision` queue を caller thread から直接 push し（`&self` で共有 `Arc<Mutex>` のみ触り UI round-trip 不要）、app thread の event handler が `CloseRequested` 分岐で同期消費・`respond_close` を呼ぶ設計にした。armed queue が `CloseRequested` flush 内でのみ消費されることを test が直接検証する（veto で window 生存・defer で close-pending・その後の approve で destroy）（Issue #631）。

`ViewHost::update` の Removed op 順序は HashMap 反復順依存であることを story matrix 実行で検出し、削除キーの sort で決定化した。`CommandIntent::Cancel` は gesture cancel（detach/unmount 含む）で intent を発行するのが実際の `ViewHost` 挙動であり、「cancel は intent を発行しない」という当初記述は誤りとして訂正した。`ResolvedTheme` の override は同一 profile id の文書を `DuplicateSource` で拒否するため、consumer override は別 id の product 文書で表現する（Issue #426）。

story manifest の `scaleFactor` は window pixel scale ではなく `ResolvedTheme::with_scale` の `ScaleContext` として与える解釈を採用した。gpui-0.2.2 test platform は `window.scale_factor` が 2.0 固定で scale 1.0 pixel を再現不能なため、semantic/draw 層と pixel 層を分離した。gpui の `deactivate_window` は platform の active window が対象 window の時のみ有効で先に `activate_window()` が必要であり、テスト用 `RvfView::new` は window activation 購読を持たず cancel 経路を通らないため、cancel 検証は本番の `RvfView::mounted` 経路で行う必要があった（Issue #426）。

共有 story fixture `ruviform-ui-test/src/lab_story.rs`（StoryFixture/StoryStep/単一 oracle `expected_lab_story_trace`/adapter-neutral walk）を新設し、Fake と GPUI の両 adapter が同一 oracle を消費する。teardown の spec 差し替えのみ呼び出し側 closure に委ねる設計である。driver-lab の mechanism 実在検査は `package`+`filter` で test fn 実在を要求する方式を導入したが、`has_test_fn` は「直前 6 行の属性文字列」ヒューリスティックで誤検出の限界がある。story matrix の theme 軸は `ViewHost` が theme/scale を受け取らないため構造上自明であり、実質的保証は `with_scale` の tokens 不変性と state の期待マーカー検査にある（Issue #426）。

GPUI window の GUI dogfooding では、UIA が GPUI content を露出しない（Zed::Window・descendants 0・title-bar controls のみ）。実 window に対し Win32 `PostMessage(WM_LBUTTON*)` で client 座標 click を送る手法を採用し、lParam が client 座標を要求するため client-origin offset の減算が必要だった。screenshot 画素解析でボタン帯を同定し、select/run/run-all/real-row reject/quit を実走確認した（Issue #631）。

## Native slot 投影と DComp 画素問題

slot_clip の投影規則として、`project_slot_frame` が `window.content_mask().bounds`（ancestor overflow mask の累積値）との交差を slot clip として投影する。mask が狭めない場合は `None`（clip 未適用）、完全に mask された場合は zero-size clip（→ adapter 側で空 HRGN → child 非表示・hit-test 不可）、負の交差寸法は 0 に clamp する。既知の制限として、`SharedSurface::set_clip` の外部呼出しは renderer の projection ledger を更新しないため、投影 clip が `None` のままだと dedupe で port が再 assert されず stale region が残りうる（PoC は明示 clear で回避し、単一 writer 前提の契約は要検討）。clip 証拠は window viewport mask であり、中間 ancestor の `overflow:hidden` clip は未発生である（Issue #388）。

Dock scenario は canonical surface を使う。既存 floating probe は `GpuiSurfaceAdapter::new` の独立 ledger を使うが renderer の投影は canonical surface にしか届かないため、Dock scenario は canonical surface を使用し floating probe 側は変更しない（影響範囲最小化）。tab 切替は pointer 入力自動化が範囲外のため `ViewHost` の既存 reconcile 経路を通す host 駆動で確認し、renderer の inactive-tab 抑止と同じ経路を実機確認した。slot が window 端まで伸びる layout では editor の overhang が画面に出ないため clip は Pass 根拠がなく NotRun とした。`GpuiDriver::update_view` は拒否された spec の理由を呼び出し元へ返さない（`ViewHost::update` 契約で前の木を残すのみ）ため、result channel 追加が follow-up 課題である（Issue #388）。

DComp による Native Dock の画素問題が実測で確認されている。GPUI 0.2.2 `platform/windows/directx_renderer.rs:793` が `CreateTargetForHwnd(hwnd, topmost=true)` を呼ぶため DComp swapchain が window の全 child HWND を覆い、plugin editor と対照 probe HWND 双方の画素が画面に一切合成されない（実測 Unsupported）。早期の観測で「Vinyl 背景」と解釈された `38,45,54` は実は GPUI `control_track`（`0x262d36`）であり plugin 画素ではなかった。HWND 機構（parentage/geometry/visibility/clip/hit-test/focus/key/lifetime/re-dock）は全て実測 Pass だが、Native Dock PASS は画素表示未達のため不成立である（Issue #388, Issue #389, Issue #532）。

vendored gpui 0.2.2 の DComp 監査結果を記録する。`CreateTargetForHwnd(hwnd, true)` は `directx_renderer.rs:793` の 1 箇所のみ、`topmost` literal は crate 全体で 1 token であり patch 面積は最小である。`Platform`/`DirectXRenderer`/`DirectComposition` 等は全て `pub(crate)` 以下で、downstream から到達可能な合成経路スイッチは `GPUI_DISABLE_DIRECT_COMPOSITION` env var のみ（受理値 "true"/"1"、process-wide 起動時確定）。gpui の DComp object は再利用不能のため patch/vendor 方式を OS interposition（env var/`SetWindowRgn`/第二 DComp target）として再定義し、env var を patch 不要の代替経路として比較枠組みに加えた。crates.io 最新 publishable は 0.2.2 で、zed repo 現行は `gpui_macros` の `publish=false` 継承により publish 不能である（Issue #532）。

`GPUI_DISABLE_DIRECT_COMPOSITION=1` 経路は GPUI 側 crash（exit 127）で未達である。gpui Windows 経路に `process::exit`/`abort` の明示呼出がなく非 DComp 経路の失敗は全て `anyhow::Result` Err 伝播（panic なら通常 exit 101）のため、コードパスからは一意に特定不能と判断した。H1（wrapper/launcher の exit code 混同）〜H5（device-lost は回復機構に吸収）の仮説を列挙し、実 OS での stderr/exit code 捕捉再現が必要として open のまま記録した。コード読解だけで原因を断定しない方針である（Issue #532）。

遮蔽診断は query-only の観測で行う。`WindowFromPoint` は HWND manager z-order のみを見て DComp visual・flip-model plane は hit-test に現れないため、「HWND 層に遮蔽物無し」は compositor 内部の無遮蔽を意味しない。cover-vs-skip 判別のため probe 時点の chain 状態（alive/visible/disabled/class_name）を記録する。foreground-probe は GPUI root の z-order/foreground を段階的に変異させ外部 HWND stacking と compositor 内部遮蔽を二分し、`SetWindowPos` の自 window root への拒否は anomalous→fail、`SetForegroundWindow` の foreground-lock 拒否は spec 期待の note で verdict 入力にしない。restore が保証するのは topmost band のみで z-order/foreground は best-effort とし、`AttachThreadInput` 等の lock 回避手法は不採用と確定した。presentation-probe は変異なし観測専用で、verdict は evaluability・family 完了・lifecycle 収束のみに依存し capture/hit-test/fingerprint は verdict 入力にしない。`IsHungAppWindow` な foreign window への `PrintWindow`（`WM_PRINT` で wedge し得る）は skip、座標系を `bounds_space:"window"`/`rect_space:"client"` で明示した（Issue #532）。

PrintWindow の限界を観測事実として記録する。`PrintWindow` flags ∈ {0, PW_CLIENTONLY, PW_RENDERFULLCONTENT, both} の 4 variants で capture 経路を分離したところ、flags=0/PW_CLIENTONLY は child で green=1.0（WM_PRINT 需要駆動で描画可能）だが RENDERFULLCONTENT 系は全 target で flat dark raster だった。5bit/channel fingerprint は match_fraction=0.0 で「screen ≠ 既知 suspect raster」までしか示せず、外部 compositor 相手には無力で、pixel artifact PNG の histogram 解析が実質的代替証拠となった。`PW_RENDERFULLCONTENT` が GPUI root の DComp content を実際に捕捉するかは実装依存で、実走データが答えるとした（Issue #532）。

host 境界修復として、`Container::set_clip` を `Result<(), SlotError>` 化し `None` で `SetWindowRgn(NULL)` を発行（旧来は clip 解除で region が残る欠陥）、`CreateRectRgn`/`SetWindowRgn` 失敗を `Backend` で返すようにした。`set_geometry` に clip 再投影を追加し、`lease_parent` re-apply 失敗を rollback+`Err` にした。owner HWND `DestroyWindow` 後の slot/binding/lease を収束させる `binding_dead`/`sweep_dead` を追加し、`Container::create` に `IsWindow(parent)` 検査を追加した。変更は `gpui_driver/surface.rs` に閉じ、非 Windows は stub backend で `#[cfg(windows)]` test 未実行 = Unsupported 記録とした（他 OS 実機未検証を Pass に転用しない）（Issue #404）。

## 実 OS 入力注入と query-only 観測

SendInput は「受理≠実効」であり、戻り値が期待数でも実効ゼロの環境障害がある。`os_input::preflight` は注入の実効を独立 OS API の read-back で検証する。pointer probe は相対移動を pointer acceleration の影響で不採用とし `MOUSEEVENTF_ABSOLUTE|VIRTUALDESK` の絶対移動 16px→`GetCursorPos` を 25ms×80 poll（±8px）、keyboard probe は未割当 `VK_F24` key-down→`GetAsyncKeyState` high bit を poll（`GetKeyState` は呼出スレッドの message queue 依存で不採用）。key-up は read-back 失敗時も必ず送信・復帰も試行する。アルゴリズムを `Probe` trait でプラットフォーム非依存にし mock で unit test とした。不通時は verdict `environment_blocked`・exit 3（新設区分）で lane 本体を走らせない（Issue #552）。

`OsInputOutcome`（Injected/Unsupported/EnvironmentBlocked/NotRun）で環境差異と契約失敗を分離し、`os_press_key`/`os_pointer_move_to`/`os_mouse_down`/`os_mouse_up` は共通 prelude で target pin（epoch/generation）と component 実在を再検査し、family 単位 preflight+focus 観測を gate する。`os_input_preflight(scope)` は family 毎に 1 回だけ probe する session 内 cache で、非対応/非実効/未 focus は `Unsupported`/`EnvironmentBlocked`/`NotRun` を返して注入しない。実機 lane は対話 desktop session が必要で、非対話なら evidence は NotRun とする（Issue #313, Issue #552）。

SendInput 全域無効化事象の記録として、Riot Vanguard `vgk.sys` の armed 状態が最有力仮説である。`sc query vgk` の STOPPED は service 停止≠kernel driver unload であり得るため否定材料にならず、時系列相関ベースの記録であり因果は未確証と明記する。`os_input::preflight` read-back が再発時に exit 3 `environment_blocked` で fail-closed するため、verdict 誤記録リスクは残らないと判断した（解釈を含む記録）（Issue #243）。

`PostMessageW(WM_CLOSE)` は close ボタンと同じメッセージだが物理入力ではないため、この区別を証拠上維持する運用とした。`monitors()` の scale factor は driver 既知の近似値（先頭 window の観測値）であり精度限界を証拠に明記する。focus/幾何イベントはタイミング依存のため lifecycle kind と window id の対応を正本とし、checkpoint で ~5s の settle を置く probe 設計とした（Issue #243, Issue #431）。

click oracle は「注入した」ではなく「カーソルが実際にそこへ行った」の証拠を取る必要がある。初版は `move_to` error を握り潰し `WindowFromPoint` の hit-test math のみを信用していたため実カーソル未到達を検出できなかった。`os_input::cursor_position()`（`GetCursorPos`）read-back と `move_to` error 記録、foreground title 記録を追加し、これが環境ブロック検出を可能にした（Issue #243）。

実 OS multiwindow smoke は約 1900 行の phase machine fixture で、3 実 Window を SendInput 実入力で駆動して focus/close/detach/reattach/move/regenerate/job policy を Windows 実機で実測する計画として設計された。cursor read-back を独立 oracle 化し retry 上限後は fail+診断情報（`fg_log`/`failures`）を JSON に残し、oracle を弱めず、teardown は verdict 書き込み後・失敗時にも実行する。最後の window が予期せず close されると GPUI が quit する問題を、close/regenerate の順序を「生存 window を残す」不変条件に修正して解消した。`EvalMove` で Win+Down による minimize と geometry 消失を記録し、driver `focus`（activate 経路）で restore する順序に修正した（`activate_window()` が iconic window を restore することを利用）。Alt+F4 2 回目 click の flaky 対策として activation click に bounded retry を追加した（Issue #243）。

query-only Win32 観測 module を整備した。`oschild`（`EnumChildWindows`/`GetParent`/`IsWindow`/`IsWindowVisible`/`GetWindowThreadProcessId`/`GetCurrentThreadId`）、`osoccl`（extended style・z-order・`WindowFromPoint` probe）、`ospix`（`window_screen_rect` 再利用・`PrintWindow` flag variants・window-DC・screen BitBlt・quantized fingerprint）、`osmut`（staged mutation: `HWND_TOP` raise・`SetForegroundWindow`・topmost band）、`ospres`（top-level window inventory `EnumWindows` cap 256+truncation flag・presentation attributes）はいずれも query API のみで window 生成・移動・style/z-order/region 変更・message 送信を行わず、unsafe/Win32 型は module 内に閉じる。`osmut` は `u64` HWND+plain 値 record のみ公開し、`SetLastError(0)`→call→`GetLastError` の対で refused call と正当な 0 を区別し、`GW_HWNDPREV` walk は cap で打ち切り `truncated` flag を記録する（Issue #532, Issue #631, Issue #389）。

`real-editor-attach` case は `test_editor` fixture の実 child HWND を leased container HWND 配下で query-only Win32 観測して honest Pass とした。descriptor 選択を `d.name=="test_editor"` に限定し `.or_else(first())` fallback を撤去したため、`test_editor` 不在は present-but-broken として Fail になる。open Ok 時のみ child HWND の存在・parent・visibility・detach 後 `IsWindow==0`・events=`[Attached{Docked},Closed]` を観測し、全 check 合意のみ Pass とする。thread-affinity 逸脱（caller thread≠GPUI app thread）は verdict 入力にせず観測記録のみとし、`IsWindowVisible` ancestor-chain false negative は masking retry せず Fail として記録する（Issue #631）。

`wm-close.ps1` の運用知見として、WebView2 の子プロセスが stdio pipe を継承し読み取りが終わらないため pipe を使わず `cmd /c` のファイルリダイレクトにした。`Start-Process` の `ExitCode` が取得できないため `System.Diagnostics.Process` で `cmd` を起動し子 exe の exit code を `cmd` 経由で得る。PowerShell 5.1 が UTF-8 BOM なしを誤読するため ASCII のみとし、タイトルの em dash は照合せず部分一致の ASCII 語を使う（Issue #431）。

## verdict・証拠の記録規約

verdict 語彙の分離として、`Unsupported`（宣言済み capability 欠落として追跡）/`NotRun`（未実験・未測定・環境阻害）/`Fail`（実測不一致）を requirement/result 列で分離し、Deferred 要求の NotRun は Pass にしない。Gate D では HWND 機構層 Pass・画素層 Fail+Blocker・Dock PASS 未達を別 Case に分離し、機構 Pass を Dock PASS に昇格させなかった（Issue #262）。

SHA 照合規則として、ancestor SHA の CI 証拠を最新 SHA の Pass に読み替えない（`git merge-base` で ancestor か確認し ancestor SHA の証拠として記録）。HEAD run 未実施は `NotRun`（Pending）として明記する。CI 証拠 run と監査 HEAD の SHA が一致しない場合は `git diff` で契約層ファイルの無変更を確認した上で「同一 SHA の新規 run は未取得」と明記して誤読を防ぐ。verdict の決裁（accepted/conditional/rejected・Issue 投稿・ADR status 変更）は Coordinator 権限で、worker は推奨 verdict+根拠表のみ記録する。証拠 run の選定は同一 SHA の唯一の completed run を `gh run watch` で完了確認後に採用した先例がある（Issue #262, Issue #532, Issue #388）。

Fake/Headless の Pass を上位 gate の Pass と主張しない方針を採用した。実 Window 起動・実入力・動的 DPI/IME は GUI-L2 gate、Native Plugin Dock は EXT-L3 gate として NotRun と明記した。consumer 実機非回帰（audio-lens/synvelis）は cross-repo・別 Owner として範囲外に分離した（Issue #243）。

verdict 正直性が一貫原則である。定義済み拒否（`createView` null→`HostError::Unsupported`→`EditorOpError::Unsupported`）は Unsupported と記録し、artifact 未解決は NotRun/Fail で fail-closed（silent skip しない）。実観測できない能力は Unsupported、効果 oracle のない受理は NotRun、環境阻害は NotRun+`observed.environment_blocked`。観測失敗 key は `null` で正直記録し捏造しない。実走での OS 挙動差（geometry 値・イベント timing）は honest Fail/NotRun で記録し、判定を緩めて辻褄を合わせない。`ScenarioReport` は意図的に非 Serialize で、JSONL case 行は出力境界で public field から `serde_json::Map` 組立し adapter 内部 ID（targets）は evidence 面に出さない（Issue #631）。

OpLanding 3 値として、resize/redock の着地点判定に `OpLanding{Refused,Accepted,Other}` enum を導入した（`struct_excessive_bools` lint と verdict 写像の明示性を両立）。定義済み Unsupported は lifecycle/event/子 HWND 収束を検証した上で Unsupported、受理されても非画素 effect oracle が無ければ NotRun（Fail/Pass にしない）。close/DPI/IME は公開 API 非存在・capability 未 claim を理由に Unsupported。`--blocked-probe` は session を開始せず全 catalog 行に `NotRun`+`observed.environment_blocked`+`environment_kind=deterministic-backend` の report を生成し違反を exit 1 で検出する fail-closed 実証 lane である（Issue #631）。

catalog flip の前提として、`real-window-close`/`real-window-dpi` を Unsupported stub から実判定へ flip する際、close case は window B/C を対象にする（GPUI は最後の window が閉じると loop が終了するため window A sentinel を後続 case のために残す）。veto/defer/deferred-approve/default-approve の 4 経路を `arm_close_response`/`request_close`/`respond_close`+`window_events` の `CloseRequested`→`Destroyed` 遷移で検証する。DPI case は `window_geometry` を poll して物理 px/scale-factor の安定読み・resize/scale 遷移・幾何 digest を記録し、DPI 変更 verb が存在しないため `dpi_change_driven:false` を正直に記録して観測のみで pass とする。close case の window 破壊は catalog 順の位置で安全であることが前提である（Issue #631）。

「stale WindowId 宛 terminal routing」と「delivery 層の dead-window drop」は別々の不変条件として観測する。Harness は delivery 前の `RoutedEvent` を journal 記録するため、runtime が owner 破棄後も stale `WindowId` 宛に terminal job event/command receipt を産出し続けることと、delivery 層（`post_event`/`delivered`）が dead-window 宛を drop し sibling へ leak しないことを独立に assert できる。既存の delivery drop 固定だけでは terminal routing の stale-id 維持は証明されなかった（Issue #243）。

## scenario catalog・console・evidence fields

scenario catalog は executor でなく投影である。`ScenarioEntry<C>`/`DriverBinding`/`ScenarioMatrix`/`EntryProjection`+`run_entry`/`run_entries`/`run_matrix` を `ruviform-ui-test` に新設し、headless 8 case（`HEADLESS_MATRIX`・'static）と real-dock 4 case（session-scoped `real_entries()`）を同一表へ移した。catalog は `run_scenario` 呼び出しの列挙・投影であり verdict 権威を持たない（各 entry の `run` body が既存 case ロジックを保持する「runner は 1 関数」制約）。`blocked_reports` も `real_entries()` から導出するため live/blocked 経路の drift が構造的に排除される。`ScenarioEntry<C>` の `Copy`/`Clone` は手動 impl が必要である（derive だと `C: Copy` bound が付き、非 Copy context 向け entry を static 表から move できず E0507 で実測）。`real_entries()` は session-scoped で `&'static` 不可のため `run_entries`（flat slice）で対応する。`DriverBinding.capabilities` は宣言の写しであり、live `capabilities()` との drift 防止は consumer test の責務である（Issue #631）。

selector の fail-closed として、`plugin-dock-console` bin は `--list`（capability projection のみ・verdict 語彙を含まず実行を主張しない）と `--run`（`--case`/`--scenario`/`--driver` selector、flag 内 OR・flag 間 AND）を提供する。selector 検証は real-lane 認識→各値 unknown→空 intersection の順で fail-closed し（real case 名を先に判定しないと「unknown case」と誤案内される）、`--include-real` は非 real-dock build で行を静かに落とす代わりに exit 2 で拒否する。real lane metadata は ctx を持たない `real_catalog` module（RUN_ID/SCENARIO/LANE/DRIVER/REAL_CATALOG）に分離し全 build で compile する。console の JSONL `lane` は `plugin-dock-console` で run id を共有するが、絞込み実行が full lane 証拠と混同されないよう provenance を分離した。`HeadlessContext` は fields private のため `scenario::run_selected` を `pub` で追加し selector 述語のみを外に出した（Issue #631）。

Command→Receipt link の 2 段構造として、`Event::Result`（status:applied+revision）だけでは binding は `CommandPhase::Applied` に留まり、`Event::Notify`（control_id/revision/value）の authoritative projection で初めて `Observed` へ settle する。stale revision guard があるため projection は実 commit revision+1 で送る必要がある。rejected 経路は `Event::result_err` で `CommandPhase::Rejected` を観測する。`command_id` 相関は minted `MintedIntent` の値を `Event::Result.id` に verbatim 使用し display text・配列位置・合成値を使わない。receipt は `post_event` した実 `Event` を `CommandReceipt::from_result` で decode する（stand-in ではない）。command binding を持たない dock spec のため、第 4 window D（`sv_fader_dock_fixture`）を新規 mount し、既存 window（sentinel/close/focus target）の前提を壊さない構成とした（Issue #631）。

evidence fields として、window identity は `window_id`+mount title+session 開始時 geometry の canonical 文字列を `Sha256::digest` で environment に stamp し（`window_geometry` 失敗時は `sha256: null`）、`capture_sha256` は全 pixel capture row の実 RGBA 画素バイト digest、`save_pixel_artifact` は `(width,height,rgba)`→`{path,sha256}` を返す。semantic tree は `ViewHost::validate_spec` と同じ walk（Panel 再帰+Tabs alias/canonical 両 key）で全量化し verdict 入力ではない。canonical `--pixel-artifacts` 化は PNG binary の churn を理由に不採用で、opt-in artifacts+canonical digest を維持した。`case_seed`（case-scoped deterministic・sha2）と `receipts` field は additive 追加した（Issue #631）。

real-window native-slot dock lane は、`real_window.rs` に slot ops（declare/bind/unbind/lease/validate/release/redeem）を `UiJob` seam 経由で追加し、`ruviform-plugin-dock-test` に `real-dock` feature+optional `windows-sys` dep+gated bin で実現した。optional dep により default lane は不変で、headless は NotRun 記録で fail-closed。capability gating は `surface::BACKEND_AVAILABLE && !cfg!(feature="gpui-test")` の predicate で、native capture stub が real container HWND を mint できない `gpui-test` feature が capability を名乗れないようにした。slot ops の戻り値は `Result<T, ScenarioError>` で `SlotError` を ``native slot `{op}`: {error}`` prefix で `ScenarioError::Driver` に写像し、`private_interfaces` lint（`gpui_driver` module が private）を回避して consumer が `ruviform-ui` の型のみ扱う境界を維持した。`ParentSlotLease` の opaque `u64` payload を偽造（`payload ^ 0xFF` 等）して `redeem_native_lease` に流すと canonical `SharedSurface` ledger が拒否し `InProcessEditorOps` redeem も `HostError` に落ちることを実測し、拒否自体を evidence detail に記録して fake success を主張しない（Issue #631）。

test-lab GUI は `lab-gui` feature の `plugin-dock-lab` bin（`dep:ruviform-ui-gpui`+`gpui` のみで scenario/test adapter 面は引かない）で、`GpuiDriver::run` の event callback を直接駆動し app thread+ready-channel で `WindowLifecycle::Ready` handshake を行う。catalog は `HEADLESS_MATRIX.project()`+`REAL_CATALOG` の投影のみで case 定義を複写せず、headless cell は window 内 worker thread から `run_selected` で実行し、real row は実行せず委譲文言を返す fail-closed。非対話環境では全 row を NotRun+environment_blocked で記録して exit 0。`emit_blocked` が `fs::write` で `--out` を truncate し先行 evidence を失い得る欠陥を self-review で検出し append mode に修正した。unit test は window を開かない pure 関数（catalog row 写像・spec 組立・blocked report・arg parse）のみである（Issue #631）。

`real-os-input` の aim 対象として、`plugin.dock` panel は canvas を paint しないため `component_bounds` に現れず pointer_move が NotRun（no painted bounds）になった。実 native slot を declare→bind→project→lease した `plugin.editor` の painted bounds を aim 対象に変更し、pointer move（cursor read-back 付き）・click・Tab が全て Injected で Pass を観測した。`real-os-input` の Pass は transport-level（注入が実 window に届いた）証拠のみで semantic effect oracle は claim しない（Issue #631）。

headless command-receipt の helper（`step`/`steps_outcome`/`component_snapshot_json`/`commit_for_gesture`/`commit_control_id`）は `real_dock.rs` が `#[cfg(feature="real-dock")]` 配下のため default build の `scenario.rs` から参照できず、共有化は lease 外ファイル編集が必要なため `scenario.rs` に同型 helper を局所実装した。3 つ目の lane が必要になった時点で cfg 非依存 module への集約を検討する残件である。`--blocked-probe` が再生成した lease 外 JSONL は差分が `repo.sha` のみと確認し `git checkout` で復元した（Issue #631）。

共有 fixture 拡張を避ける選択として、`UiRuntime`+`FakeDriver` を consumer 配線（Action→`dispatch_window_intent`→`deliver_routed` 等）で合成する Harness をテスト内に作り、`window-lifecycle.v1.json` 等の共有資産を変更しなかった。fixture 拡張は共有資産の変更を伴うため別 Issue 候補とした（Issue #243）。

clippy 運用として、`run_editor_attach` の `too_many_lines`（148/100）は `fake_dock_slot_spec()`/`attempt_editor_attach()` への分割で挙動不変に解消し、部分失敗時の cleanup も helper 内で完結させた。`SampleRate`/`ProcessConfig` の fallible 変換は `test_gain_config()` helper に集約した（Issue #631）。

lease/PR 運用として、docs index は Worker が編集せず Coordinator が統合時に doc-maintenance で再生成する。派生 index を直接直すのではなく front matter の status を修正して `doc_maintenance apply` で再生成するのが正しい境界。PR 本文は `Refs #631` のみで auto-close 禁止。検証コマンドが lease 外ファイルを再生成した場合は差分が `repo.sha` のみなら `git checkout` で復元し、検証自体は実走済みとして記録する。catalog↔entry の協調変更は catalog 整合 test が両者を検査するため分割すると中間状態で test が落ち、1 commit 単位が自然である（Issue #631, Issue #243）。

## WebView legacy 資産の扱い

legacy WebView-first 前提の文書・API 記述は legacy/preview に分類し、棚卸し（discovery）を正本として spec/design/ADR/plan/task の doc chain を作成した。関連 spec/design/test-spec に legacy banner を挿入（本文は改訂しない）、開発環境文書の lane 注記、`crates/ruviform-ui` の README+module doc コメントに legacy 分類と移行先を明記した。挙動変更なしのため新規 test-spec は作らず、既存 driver 境界 lane（`check-ui-driver-boundary`/`test-ui-driver-contract`）と cargo test が回帰を担保する（Issue #310, Issue #465）。

撤去台帳の分類単位は「削除で壊れる契約」である。`ui/guitest` や `poc/` のように共有部分と legacy 部分が同居する対象を package 単位で分類すると `ui/guitest/fixtures` を読む Rust conformance test が壊れるため、`ui/guitest/fixtures`=reuse、`ui/guitest/src`=provisional、`poc/*` コード=remove_candidate、RESULTS/ADR=exception に分割した。`ipc_conformance.rs` は schema 部と `rvf://data` 部が同居し、撤去時に検証対象単位での再分割が必要である（Issue #465）。

`crates/ruviform-ui-webview` には `#[deprecated]` を付与せず doc コメント/README の legacy/preview 注記のみに留めた。crate は現行 consumer が使用中で API 属性の変更は「挙動変更」に近くスコープ外のため。P2 物理撤去は GPUI adoption 決定と consumer 退役 owner の受入待ちで、`remove_candidate` entries はその入力となる（Issue #465）。

consumer 参照列挙の手法として、`gh api search/code` は全 query で `total_count: 0`・`incomplete_results: true` を返し信頼できなかったため、origin/main HEAD の clean checkout（read-only）を正本とする手法に切替え、pin SHA・feature/bin/lane/asset/vendor 構造・Issue 状態を file-level で列挙した。手法と制約を台帳に明記し、consumer repo への変更はない（Issue #465）。

legacy `HostHandle`/`WindowHandle` trait の Phase 2 削除は ADR（status: proposed）で保留とした。Phase 2 gate（consumer の native 最小 slice・`WindowLifecycle` 直接消費・両 consumer の実 OS 合格）が未達であり、`HostEvent`・`WindowHandle` は runtime・テスト・consumer の経路上にあり、`HostHandle` は実装者 0 件だが公開 API の再 export で残る。コード変更なしの文書判断のみの slice である（Issue #431）。

## 未確定・保留事項

GPUI 採否は未確定であり、driver-neutral 契約の正本は採用確定の根拠にならない（Issue #257）。

`GpuiDriver` の `focus` capability 宣言と実装の不整合（gpui 0.2.2 で `focus(_, false)` が `Unsupported`）は、WebView 側の文言精緻化は済んだが driver 宣言全般の精緻化は後続 Issue 送りである（Issue #386, Issue #423）。

`SharedSurface::set_clip` の外部呼出しが renderer projection ledger を更新しない制約は、単一 writer 前提の契約として要検討である。中間 ancestor の `overflow:hidden` clip は未発生で将来課題（Issue #388）。

DComp 画素問題は未解決である。exit-127 の原因は H1-H5 仮説のまま PENDING、`PW_RENDERFULLCONTENT` の GPUI root 捕捉可否・`WindowFromPoint` の GPU 合成下決定性・`topmost=false` の実画素効果は未検証で、実 OS 再現は L2/L3 再開時である（Issue #532）。

crash/hang fault は in-process lane で実行不能（out-of-process plugin host 非存在）のため executable `Unsupported` row として catalog に残る（Issue #389）。

`complete_close` の error channel 不在による refusal→timeout 収束は定義済み挙動として contract 変更せず別 Issue 候補（Issue #389）。

`PluginHandle.generation` は wire 互換のため u16 のまま残る（Issue #389）。

`WindowConfig.position` の単位は物理 px / logical px の記述が分かれており未確定（Issue #182）。

`GpuiDriver::update_view` が拒否された spec の理由を呼び出し元へ返さない件は result channel 追加が follow-up（Issue #388）。

SendInput keyboard probe は `GetAsyncKeyState` が注入キーの down 状態を反映する Windows の既知挙動の実機裏付けが必要で、反映されない場合 keyboard probe が常に blocked になる（Issue #552）。

GPU 情報取得は `windows-sys` の `Win32_Graphics_Dxgi` feature 追加（manifest 変更=要承認）または不存在の GPUI renderer-info API が必要で missing_issue 候補（Issue #631）。

production host の UI thread attach は別課題で、thread-affinity deviation は観測記録のみ（Issue #631）。

legacy `HostHandle`/`WindowHandle` trait の Phase 2 削除は ADR（status: proposed）で保留であり、Phase 2 gate が未達（Issue #431）。

headless helper の cfg 非依存 module 集約は 3 つ目の lane が必要になった時点で検討（Issue #631）。

VST3 `set_gui_size` の logical:pixel 1:1 は既存 documented deviation として踏襲（Issue #389）。

macOS/Linux backend 未実装→Unsupported/NotRun、cross-repo consumer（audio-lens/synvelis）追従は別 Owner（Issue #243, Issue #389）。

`DEFAULT_REQUEST_TIMEOUT`=10s は暫定値（Issue #389）。

`ruviform-ui-webview` の P2 物理撤去は GPUI adoption 決定と consumer 退役 owner の受入待ち（Issue #465）。

`has_test_fn` の mechanism 実在検査は属性文字列ヒューリスティックで誤検出の限界があり、story matrix の theme 軸は構造上自明で pixel 層の scale 1.0 再現は real-os/Visual lane の残件である（Issue #426）。

gpui test platform の `window.scale_factor` 2.0 固定により pixel scale 1.0 は headless では再現不能（Issue #426）。

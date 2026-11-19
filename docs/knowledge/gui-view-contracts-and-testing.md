---
id: "KNOW-zH1AsgygRDxY67ESy3cZec"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "codebase"
title: "GUI view 契約・ウィジェット・テスト基盤"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/29", "https://github.com/scarecrow173/ruviform/issues/125", "https://github.com/scarecrow173/ruviform/issues/243", "https://github.com/scarecrow173/ruviform/issues/310", "https://github.com/scarecrow173/ruviform/issues/311", "https://github.com/scarecrow173/ruviform/issues/313", "https://github.com/scarecrow173/ruviform/issues/314", "https://github.com/scarecrow173/ruviform/issues/377", "https://github.com/scarecrow173/ruviform/issues/410", "https://github.com/scarecrow173/ruviform/issues/414", "https://github.com/scarecrow173/ruviform/issues/418", "https://github.com/scarecrow173/ruviform/issues/431", "https://github.com/scarecrow173/ruviform/issues/436", "https://github.com/scarecrow173/ruviform/issues/442", "https://github.com/scarecrow173/ruviform/issues/451", "https://github.com/scarecrow173/ruviform/issues/453", "https://github.com/scarecrow173/ruviform/issues/461", "https://github.com/scarecrow173/ruviform/issues/463", "https://github.com/scarecrow173/ruviform/issues/464", "https://github.com/scarecrow173/ruviform/issues/465", "https://github.com/scarecrow173/ruviform/issues/478", "https://github.com/scarecrow173/ruviform/issues/479", "https://github.com/scarecrow173/ruviform/issues/490", "https://github.com/scarecrow173/ruviform/issues/532", "https://github.com/scarecrow173/ruviform/issues/631"]
  informs: []
  explains: []
  supersedes: []
---

# GUI view 契約・ウィジェット・テスト基盤

この文書は、GUI view 契約・ウィジェット・scenario/oracle テスト基盤に関する設計と実装の知見を記録する。元になった実装記録は後に削除されており、この文書はそれらに依存せず単独で読めるように書かれている。各記述には出典の Issue 番号を (Issue #NNN) の形で付記した。記述は主に [K]（知識）とタグ付けされた実装記録に基づく。測定値や観測された挙動は事実として、解釈を含む部分はその旨を明示して記す。

## 借用 view の型設計と契約

channel-major の連続ストレージを前提とする既存の `AudioBuffer` を拡張せず、独立 per-channel planar バッファへゼロコピーで参照する借用 view を新設した (Issue #29)。配置先は `ruviform-audio-core` とし、`AudioBufferView` 系と同じ「音声バッファ借用形状」の責務にまとめた。`Plane`/`PlaneMut` による video-core の先例と整合し、plugin 以外の非連続バッファ供給にも再利用できる。型形状は `&'a mut [&'a mut [f32]]` 相当で、ライフタイムを process コールバックに束縛する。

view 構築自体は alloc しない。コンストラクタはアダプタが activate/prepare 時に事前確保するチャンネルリスト用スクラッチを受け取る形状にした (Issue #29)。in-place 処理（CLAP `in_place_pair` / clack `ChannelPair::InPlace` 相当）では、`&[f32]` と `&mut [f32]` を同一領域に同時構築しない。ポインタ一致検出で専用 view 分岐かスクラッチ経路かを選ぶ方式は、方式 ADR の判断材料として評価する。f64 型形状は予約し、借用のスコープ脱退は `compile_fail` doctest（borrow-escapes-scope 系）でコンパイル時に固定した。

## driver-neutral Scenario Contract と decode 正本

`ruviform-ui-test` に driver-neutral な `SemanticGuiTest` trait・Target・Capability・Result/Report を置き、Fake adapter と GPUI adapter が同一契約で同じ Scenario を実行できるようにした (Issue #464)。依存方向は `ruviform-ui-test` が `ruviform-ui-gpui` に依存しない形で固定し、逆転を禁じる。GPUI adapter は `gpui-test` feature 限定とした。

wire `Event` を `ViewHost` の既存 event 処理へ decode する正本は `ruviform-ui` の `deliver_event` であり、両 adapter がこれを共有することで decode の二重化を防いだ (Issue #464)。`GpuiObserver` の `!Send` 制約は維持する。

scenario runner は executor 化せず、`run_scenario` の body を `FnOnce` とし、scenario は普通の Rust 関数のままにした (Issue #464)。`ScenarioReport` への `timeout`/`cleanup` field の追加は additive に行い、timeout 矯正は Pass→Fail のみに限定して他の verdict を書き換えない。`settle_until` の `ready` 述語は `&G` 経由の read-only 観測のみを許し、状態を書き換える待機条件は契約上書けない。

## ViewLink 世代管理と projection の受理規則

`ViewLink.attach(windowId, generation?)` は「検証→冪等判定→必要時 detach→採用/既存 increment→伝搬」の順で generation を採用する (Issue #125 系ではなく ViewLink 世代管理)。同一 window・同一 epoch の呼び出しは冪等で再購読せず、異 epoch では旧 subscription/gesture/query を失効させて再購読する。負数・非整数・NaN・非 safe integer の無効 epoch は状態を変えず、generation 省略時は local increment を維持し、late-created binding も authoritative epoch を採用する。

oracle 側の `ContractModel.addQuery` は SDK `ViewLink.registerQuery` と同じ epoch 採用契約に整合させた (Issue #490)。`linkAttached` 時に `attach(linkGeneration)` を追加し、`addBinding` と同型の先例に倣った。テストは「発行 generation が link epoch と一致する」「live-epoch echo reply が stale でない」という契約条件に限定し、内部手順に依存しない形にした。

projection の受理規則は revision の高水位で管理する。`ViewHost::projection_floors`（control_id ごとの受理済み revision 高水位）を host レベルに置いた (Issue #410)。binding は `unmount` で破棄されるため、epoch を跨ぐ状態の所有者は host が正しい境界という判断による。revision advance 規則は `revision_advances` helper に集約して binding 内 guard と共有し、floor は受理時のみ更新する。棄却・detached・binding 不在では巻き戻さず先取りもしない。

受理した projection は `apply_projection`→`apply_external` の経路で controlled display（renderer 消費値 `snapshot.display`）へ authoritative 値を反映する (Issue #414)。payload decode は厳格化し、有限 JSON number・`{"plain":n}`・`{"normalized":n}` のみ受理し、業務 JSON を無条件に f64 解釈しない。gesture 中は `pending_external` に stage し、`end_gesture` で flush して最終 authoritative 値のみ着地させ、intent/gesture event は発行しない。`register_control` では生存 binding の `last_observed` を mapper 経由で復元適用し、detach→reattach で display が default に留まらないようにした。

## 有界保持とコマンド index の eviction

`command_index` には `MAX_COMMAND_INDEX=512` を導入し、liveness class（Dead→Settled→LiveNonExpect→LiveExpect、同 class 内は command id 辞書順）で決定的 eviction を行う (Issue #410)。全件 LiveExpect の場合は yield して in-flight を stranded にしない。outbox は 64 で最古を drop し、`push_event` は coalescible 優先、droppable がなければ最古を drop する。non-droppable も失われ得ることは正直に明記した。drop された Intent の receipt routing は `retire_undelivered_intent`/`expire_command` で delivery-lost にする。

`validate_spec` で canonical mount の command owner id 重複を reject し、`invoke`/`emit_selection` の binding key を `owner_key_for` で canonical owner id に正規化して tab alias 経由の mint 穴を塞いだ。u64 カウンタは `checked_add().expect()` 化した (Issue #410)。

## binding lifecycle と descriptor 更新

`unregister_control` を `detach_control`（gesture のみ解除）と release 用に分離し、contract change（descriptor 更新）には `detach_control` を使う設計を採用した (Issue #451)。descriptor 更新で in-flight receipt の routing が切れない一方、`update_reregister_keeps_binding_epoch` と `remount_absorbs_stale_receipts` の既存テストで stale receipt 保護が弱まらないことを確認する。

Tabs の選択モデル（`TabsSpec.selection`/`select_tab`/`emit_selection` の共有化）も Table と同じ経路に揃え、非選択 Tabs は常に `Some(active)` を返すと明記した (Issue #451)。

## ウィジェットのレイアウト契約

各 paint canvas の prepaint で `record_bounds` により bounds を記録する監査テストを先に失敗させて実測値を取り、Waveform/Meter/Spectrogram の slot を `viz_slot` に統一し、Waveform の余分な wrapper を除去、TimeAxis slot に `flex_shrink_0` を付けた (Issue #453)。残課題として Fader/Knob の `h_full` による兄弟要素の押し出し（0 サイズ canvas の再発要因）が記録されている。

Panel 子としての Fader/Knob の縦寸法について、現状維持・固定高さ・viz_slot 型 min_h+grow・ViewSpec 契約追加の選択肢を評価し、ADR Option C（`flex_grow` + `min_h` slot）を採用した (Issue #478)。`element_for` の Fader|Knob 分岐と `control_element` 非 Seek 経路を grow+min_h slot 契約に改め、`grow_slot` helper に集約して `viz_slot` と `min_h(px(120))` を一本化した。`h_full` ラップは除去し `.size_full()` は維持する。高さ所有権は slot 側にあり、canvas bounds = slot bounds で #450 契約を維持する。min_h は viz_slot と同じ 120px に一本化し、control 専用の別値は同一ポリシーの重複になるため置かない。Seek 経路 h(24px)・#479 キーボード規約・driver-neutral 契約は不変。

値制御のキーボード操作では、PageUp/PageDown の移動量を descriptor の絶対量ではなく effective step の 10 倍という相対量で一貫させた (Issue #479)。絶対量方式は renderer が `effective_step` を複製する必要があるため却下した。値制御は一律 tab stop とし、`press_control_key` は mounted spec に control がある場合のみ focus handle を作り、なければ `UiError::Host` を返す。契約（`ruviform-ui`）は変更せず GPUI 描画層と seam に閉じた。

## paint 実装の意味論と既知の制約

gpui 0.2.2 の `paint_layer` は layer bounds を `bounds ∩ content_mask` に clip し、primitive の content_mask を変更しないため、従来の clip 範囲と差が出ないことを確認した (Issue #461)。layer の order 共有下でも `Scene::finish` の stable sort が同一 order 内の挿入順を維持するため、cell の 1px clamp の重なり順が保存される。TimeAxis の repaint 約 0.8s は tick ごとの label `div` + text 描画が支配的で canvas quad の問題ではないため budget 対象外とし、element tree 規模の別問題として残課題に記録した（後に issue-461-timeaxis-label-cost で対応）。

Spectrogram cell の描画色は `viz.waveform.opacity(intensity)` の `surface.bg` 上への不透明事前合成（`cell_paint_color` helper、Rgba source-over → 不透明 Rgba）に変更した (Issue #463)。原因は列 frame 共有による alpha 累積飽和であり、`paint_layer` 案は棄却した。既知のトレードオフとして cell 重なり時の blend が「後勝ち上書き」に変わるが、これは期待色と一致させる意図した修正である。実窓 capture は gpui-test が headless で pixel を返さないため、専用 example 実行 + OS 画面 capture（PowerShell 等）で代替する手法を記録した。

## theme 解決パイプライン

`ruviform-ui-theme` crate で deterministic な DTCG→typed `ResolvedTheme` パイプラインを実装した (RF384/RF385)。serde/serde_json のみ依存し、12 fixtures + 18 contract tests で検証した。GPUI 技術採用の最終承認は #262/#386/#388/#389 の実機証拠後とし、ADR に逆反映欄を確保した。`ResolveError::Document` は将来の複合 helper 用に保持する。GPUI mapper 消費契約の確定形は #386 に委譲した。

Issue #418 では `ResolvedTheme::new` が `pub(crate)` のため、生成コードを同一 crate 内 module（`src/generated.rs`）として emit する必要があり、in-crate bin `theme-codegen` を選択した（新規 workspace crate は task 制約で不可）。生成テキストは rustfmt 子プロセスで正規化し、手計算再現は脆弱・rustfmt 不在は明示エラーでフォールバックなしとした。未知 `ResolvedValue` variant・identifier 衝突・予約語は決定的エラーで fail-closed とし、`rvf-primitive` fixture を追加して全 8 variant・alias・mode・rounding を網羅検証した。resolve 規則の第二実装は持たず、正本は runtime pipeline のみとする。

## テスト基盤: browser-test ハーネス

`ui/browser-test`（@ruviform/ui-browser-test, private）は `page.route` による hermetic 静的 serve（origin `http://rvf.test`、`/pages`・`/showcase/dist`・`/browser-test/dist` マッピング、MIME・`..` 拒否）と、ページ内 `window.__rvf` protocol（events/commands/receipts/drain/queries/drawAll/stats/prop/setProp/mount/unmount/clear/ready）で構成する (Issue #310)。Playwright config は chromium 既定・固定 viewport {960,640}+deviceScaleFactor:1・trace retain-on-failure・screenshot only-on-failure で決定性を確保し、import map で `@rvf/showcase`→showcase dist、`@rvf/browser-test-page`→browser-test dist を解決する。

Node 側 utility として `waitFor`（expect.poll 相当・固定 sleep を内部で禁止）、`expectGestureSequence`（begin→preview{0..N}→終端 exactly-one・同一 gestureId・sequence 単調）、`expectComposedInit`（detail/bubbles/composed 検査）、receipt 遷移検査、`canvasFillProbe`（fill ratio 行列）・`expectGapRegion`（NaN gap 中央帯 fill 低下）・`expectBackendRecorded`、`test.extend` fixture（page.clock install + harness open）を提供する (Issue #310)。Consumer が通常の Playwright API と併用する設計。

## テスト基盤: OS 実入力注入

`os_input.rs` に `OsInputOutcome`（`Injected`/`Unsupported`/`EnvironmentBlocked`/`NotRun`、public `#[non_exhaustive]`）を追加し、`supported_capabilities()` に `os_input::supported()` 条件で `RealOsPointerOrKeyboard` を出す (Issue #313)。`os_preflight` cache + `window_focused`（Focused(active) を `Arc<Mutex<BTreeMap<WindowId,bool>>>` に記録）+ `request_focus`/`os_press_key`/`os_pointer_move_to`/`os_mouse_down`/`os_mouse_up` を実装した。Windows は preflight→focus→`run_scenario` で keyboard/pointer ops→intents→receipt→observe→verdict JSON（`RUVIFORM_REAL_OS_INPUT_VERDICT`）+ driver_lab emit、非 Windows は `unsupported` 証拠を出す設計。

「Build/Link 成功 ≠ Driver 機能対応」の規律として、compile+link のみの lane を Pass にせず NotRun とした (Issue #313)。CI の `windowed_uitest` env-gated no-op や macOS の cfg 空 test target は compile/link のみで実行されないため、capability 記録では Pass にせず NotRun とした。`WindowedIo::JOB_TIMEOUT` は `ruviform-ui-gpui` 側で編集範囲外のため対象外とし IR/report に明記した。Timeout 検証は driver 実装を変更せず既存 public API（`call`/`wait_exit`/`spawn_tcp`/`kill`/`try_exit_status`）のみで行った。

`--stall before-serve|after-serve` の非終了 fixture と `test.shutdown` を `call(SHUTDOWN)` 経由にした理由は、`before-serve` が `serve_from_env` 前、`after-serve` が serve 終了後（全 outcome で）に永久 park し、TCP `CONNECT_TIMEOUT` accept loop と `wait_exit` の non-exit 契約をそれぞれ実演するためである (Issue #313)。`d.shutdown()` は内部で固定 `EXIT_TIMEOUT`(15s) を待ち report を捨てるため、`d.call(ops::SHUTDOWN)` で `bye:true`+`cleanup.exited:true` を確認後 caller bound の `wait_exit(500ms)` で同一契約を短時間 assert した。未知 phase・引数不足は exit 2 で fail-closed。

OS 注入 op は `RealOsGuiTest` の並行 port（`os_*`）に集約し、`SemanticGuiTest`/`GuiAction` を汚さない設計を採用した (Issue #313)。semantic dispatch（`ViewHost::dispatch_input`）・toolkit simulated input（`dispatch_keystroke`/`simulate_*`）・OS automation injection（`SendInput`）を証拠上分離するため、OS op は全て新しい `os_*` メソッド群に集約し戻り値を `OsInputOutcome` 語彙で返す（`Err(ScenarioError)` は contract 失敗のみ）。capability claim は `os_input::supported()`（現 slice Windows のみ）に限定し、`SendInput` は OS automation injection であり物理手動入力でないことを verdict JSON の `input_method`/`input_method_note` に明記した。

OS 照準は `screen = origin + (logical + frac*size) * scale` で計測値のみから解決し、receipt は head record（最新の receipt-expecting commit）へ送る (Issue #313)。`exec_resolve_os_aim` は計測済み論理矩形（`component_bounds`）+物理 window geometry（`window_geometry`）+scale factor から実スクリーン座標を解決し、bounds 欠損/degenerate は `NotRun`。pointer move は注入後 `cursor_position` で read-back し commanded/measured 両点を evidence 化した。実装期バグとして receipt leg が最初の `expect_receipt` commit（keyboard 側）へ receipt を投げていたのを、`phase()` が head record を読む契約に合わせ `.rev().find` で最新の pointer-click commit へ修正した。

`rvf.uitest.v1` closed op set を実 GPUI window に配線するため、`WindowedIo` が `UiJob` seam で control/UI thread を橋渡しする (Issue #313)。`GpuiDriver` は `Rc<RefCell>` で非 `Send` のため agent serve loop（control thread）は driver に触れられず、全 op は `UiJob`+oneshot reply で app thread へ跨ぐ。action-job+confirm-job の決定論的順序付けにより confirm job は action を apply した drain の次に着地し、`flush_events` で derived event 配送・journal・notice が caller 復帰前に完了する。`emit` は `Tick`/`Action`/`Focused`/`CloseRequested` のみ honest 注入、他 lifecycle は `emit.unsupported` notice で observably drop。`ipc_disconnect` は per-window 限定（`window=None` は `invalid_params`）。

notice 観測は `TestDriver::notifications()`（buffered `Notify` payload）経由とする (Issue #313)。agent は毎コマンド後に `io.notices()` を drain するため `test.notices` op は常に `[]` を返し、lane の notice 検証は `TestDriver::notifications()` で行う（`e2e_process.rs` の `pending_notices` と同型）。初回実走で `d.notices()` が常に空を返し `notice: false` と誤判定したことから修正した。

注入は UI thread の外から行う (Issue #313)。`GpuiDriver` は `Rc<RefCell>` で Send ではないため injector thread から `os_input::press_key` を呼ぶ。UI thread は `Ready`+`Focused(true)` を観測してから `Go` を送り、イベント記録と verdict 判定のみ行う境界分離。`source` は intent に載らない（`CommandIntent` Begin/Commit は `source` フィールドを持たず `InputSource::KEYBOARD` は `ControlInput`/`InteractionEvent` meta まで）ため、keyboard 由来の保証は「OS キーイベントのみがこの経路に到達する」構造的事実+`atomic: true` の begin/commit pair で代替した。Abort で注入停止し別 window へのキー漏れを防ぐ。

pointer の照準は `component_bounds`（paint 記録の論理 px）+`window_geometry`（`GetClientRect`+`ClientToScreen` 実測物理 px）+scale factor の実行時計測値のみを使う (Issue #313)。座標ハードコード・capture pixel 逆算は却下した。ポインタ注入の固有問題は照準であり、製品 Scenario DSL への座標固定は scope 外。`os_input` の move は `MOUSEEVENTF_MOVE|ABSOLUTE|VIRTUALDESK` を `GetSystemMetrics` virtual screen で正規化（負原点対応）、`send` 共通ヘルパーで受付数不足を `UiError::Host`（部分注入を成功にしない）。Preview は coalescible で正当に 0 件になり得るため合格条件にせず Begin+Commit と `gesture_id` 一致を要求した。

`UiJob` seam は `Send` クロージャを `mpsc` 経由で UI スレッドの `App` context 内で実行し、GPUI 型を UI スレッドに固定する (Issue #314)。`RealOsGuiTest`（caller スレッド façade+UI スレッド job pump）を実装するため `GpuiDriver` に `UiJob` seam を追加し、`Send` クロージャを `mpsc` 経由で `Inner.ui_jobs` に積み、`drain` が window ops 適用後に `App` context 内で `drain_ui_jobs` として実行する。`gpui/test-support` は `real-scenario` feature に含めない（test platform は OS capture を stub するため pixel 証拠を名乗れない）。capability は `SemanticAction`+`MultiWindow` 常時・`VisualCapture` は Windows のみ claim、`RealOsPointerOrKeyboard` は claim しない（OS-level input は #313 scope）。

`cargo test` は環境不通を通常の nonzero exit と区別できないため、`environment_blocked` は JSONL `observed` フィールド+panic `environment_blocked:` prefix の 2 点で記録する (Issue #314)。real-scenario lane の session 起動時 `UiError::Host`/`BackendUnavailable` のみ driver-lab evidence に `observed="environment_blocked"` を記録し panic tag で区別する。`Internal`/`Unsupported`/`ScenarioError`/assertion は通常の test failure のまま。headless 環境で実行しても「未実行を pass と見せない」境界を維持する設計（#552 `os_input::preflight` の verdict 分離と同思想だが本 lane は OS input を注入しないため preflight 自体は適用しない）。

scale は mapper-side hint（`ResolvedTheme::with_scale`）で token 不変を cell 毎に assert し、`GoldenKey.scale_factor` は要求値でなく `CaptureMeta` 実測値を使用する (Issue #314)。{theme × scale} 4 cell matrix（`synvelis-dark`/`audiolens-dark` × 1.0/2.0）で cell 毎に独立した real-window session を起こす。theme/scale は `ViewMount.theme` に `ResolvedTheme::with_scale(ScaleContext{scale_factor, rem_base_px:16.0})` を渡し、scale は mapper-side hint で token 不変を assert（Fake/GPUI-simulated matrix と同一規則）。`#![cfg(all(windows, feature="real-scenario"))]` は維持し非 Windows の vacuous pass を作り込まない。

## テスト基盤: golden capture と consumer 観測

consumer 型テストの置き場は外部 test crate とした (Issue #436)。受入「consumer 側 test から drawnBars/drawnMarks を観測」は、AudioLens 実施（pin 更新を伴う別作業）ではなく ruviform 内の consumer 型テスト `crates/ruviform-ui-gpui/tests/consumer_observation.rs` で代替検証した。同ファイルは外部 test crate として公開 `GpuiObserver` seam と契約型のみを使い gpui 型・crate 内 hook を参照しない。`ruviform-ui-test` への GPUI dev-dep 追加は driverless build（UI-DVR-01）と MSRV 1.89 lane を壊すため見送り、cases.json への binding 追加も manifest `requiredCases` 完全性 pin に波及するため範囲外とした。

実窓 pointer 検証はフォーカス API ではなく OS 実ポインタ（mouse_event）を使う (Issue #451)。Windows のフォーカス制約で実窓クリックが失敗し得るため、OS 実ポインタでクリックする方式を採用し、フォーカス API には頼らない判断をした。実窓証拠は手動で再現性がないため、自動化された pointer テスト（TestAppContext）で同じ経路を検査するものを正本とし、実窓は補助証拠と明記する運用知見が記録されている。

## 残存 Unsupported・対象外とスコープ判断

Issue #442/#443/#435 では、platform a11y（OS accessibility tree 接続）は Unsupported のまま（drawn grid と contract a11y 木の一致が代替証拠）、大量行仮想化はスコープ外とした。`columns.is_empty()` 拒否・列見出し重複検証の hardening は既存契約拡張でスコープ超過のため follow-up 候補に留めた。`mise run test-viz` は mise.toml に存在せず ruviform-viz は workspace `test` でカバーされる。docs audit は repo 全体に多数の既存 finding があり、対象 3 chain のみ修復し全体 clean とは主張しない方針。

## 後継 ADR と依存境界

Driver-neutral GUI contract の後継 ADR（RF384/RF385）では、Driver-neutral 構造正本（GPUI driver + webview-first）を確立し旧 2 ADR は superseded・履歴保持とした (RF384/RF385)。DTCG interpreter の境界は `ruviform-ui-theme` crate に閉じ、serde/serde_json のみ依存する deterministic なパイプラインとした。GPUI 技術採用の最終承認は #262/#386/#388/#389 の実機証拠後とし ADR に逆反映欄を確保した。

## 未確定・保留事項

- Issue #532 の compositor/presentation 内部の第二境界（hit-test が自 child HWND に解決しても screen green=0）の残余機構は未断定のまま。settle 付き topmost 再観測・uncovered baseline 環境が残課題として open (Issue #532)。
- Issue #631 command-receipt の `test-ui-plugin-dock-lab` interactive lane は operator の Run all→Quit 操作が必要で本 worktree では確認不可と記録されており、実走 JSONL 追記は残件 (Issue #631)。
- Issue #631 AC4 残 field（GPU 情報）は windows-sys feature 追加か GPUI renderer-info API が必要で missing_issue 候補のまま (Issue #631)。
- AudioLens/Synvelis consumer 側の実 test lane 実行は各 repo の ruviform pin 更新後の作業であり、本記録群ではソース確認または pinned revision 評価による代替証拠に留まる (Issue #436)。
- Issue #314 の `WindowedIo::JOB_TIMEOUT` は `ruviform-ui-gpui` 側で編集範囲外のため対象外とし IR/report に明記したまま (Issue #313)。
- Issue #453 の Fader/Knob `h_full` による兄弟要素の押し出し（0 サイズ canvas の再発要因）は残課題として記録されているが、Issue #478 で `grow_slot` helper による一本化が行われた後の最終状態は本資料からは確定できない。
- Issue #461 の TimeAxis repaint 約 0.8s は element tree 規模の別問題として残課題に記録され、後に issue-461-timeaxis-label-cost で対応されたとあるが、本資料にはその対応内容の詳細は含まれない。
- Issue #442 の `columns.is_empty()` 拒否・列見出し重複検証の hardening は follow-up 候補に留められており、実施状況は本資料からは不明。

---
id: "EVI-5KFoh3CPq68NT7RIBAlL7B"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "GUI・GPUI・dock・テスト基盤の検証記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/29", "https://github.com/scarecrow173/ruviform/issues/125", "https://github.com/scarecrow173/ruviform/issues/181", "https://github.com/scarecrow173/ruviform/issues/182", "https://github.com/scarecrow173/ruviform/issues/243", "https://github.com/scarecrow173/ruviform/issues/260", "https://github.com/scarecrow173/ruviform/issues/262", "https://github.com/scarecrow173/ruviform/issues/263", "https://github.com/scarecrow173/ruviform/issues/310", "https://github.com/scarecrow173/ruviform/issues/311", "https://github.com/scarecrow173/ruviform/issues/313", "https://github.com/scarecrow173/ruviform/issues/314", "https://github.com/scarecrow173/ruviform/issues/386", "https://github.com/scarecrow173/ruviform/issues/388", "https://github.com/scarecrow173/ruviform/issues/389", "https://github.com/scarecrow173/ruviform/issues/410", "https://github.com/scarecrow173/ruviform/issues/414", "https://github.com/scarecrow173/ruviform/issues/418", "https://github.com/scarecrow173/ruviform/issues/423", "https://github.com/scarecrow173/ruviform/issues/426", "https://github.com/scarecrow173/ruviform/issues/431", "https://github.com/scarecrow173/ruviform/issues/436", "https://github.com/scarecrow173/ruviform/issues/442", "https://github.com/scarecrow173/ruviform/issues/461", "https://github.com/scarecrow173/ruviform/issues/465", "https://github.com/scarecrow173/ruviform/issues/478", "https://github.com/scarecrow173/ruviform/issues/480", "https://github.com/scarecrow173/ruviform/issues/532", "https://github.com/scarecrow173/ruviform/issues/552", "https://github.com/scarecrow173/ruviform/issues/631"]
  informs: []
  explains: []
  supersedes: []
---

# GUI・GPUI・dock・テスト基盤の検証記録

この文書は、GUI・GPUI・dock・テスト基盤に関する検証と実行の結果を記録したものである。内容は後に削除された過去の実装記録から抽出した証拠に基づき、この文書はそれらなしで読めるよう自己完結させている。各事実には由来する Issue 番号を付記する。実測値や観測された動作は事実として記し、記録上の評価や解釈はその旨を明示して区別する。

## headless テスト基盤：window_job_headless と UiRuntime 合成

GUI の Window・Job 不変条件を実 OS なしで検証するため、`window_job_headless.rs` に `UiRuntime` と `FakeDriver` を本番 consumer 配線で合成する in-process integration test を追加した (Issue #243)。未被覆だった不変条件として、cancel scoping・close policy・orphan job terminal routing・stale id cleanup の 4 シナリオを実装し Harness 化した (Issue #243)。`mise run fmt-check`/`check`/`lint`/`test ruviform-ui-test` は全て exit 0、`cargo test --test window_job_headless` は 4 tests/0 failed だった。`git merge origin/main`（71445679→3d981b75）後の merged tree でも同一 4 コマンドを再実行し全 pass を再確認した (Issue #243)。`evidence/rf243/` に被覆対応表・実行記録・NotRun 残件を作成し、PR は `Refs #243` で Issue は Open 継続と記録された (Issue #243)。

同じ Issue #243 で、借用 planar view の全検証は `cargo test -p ruviform-audio-core`・`clippy --all-targets -D warnings`・doctest の compile_fail 確認が全て exit 0 だった (Issue #29)。独立チャンネル構築・読み書き・`rt_` no-alloc 契約テストを含み、局所許可のない unsafe は混入していないと記録された (Issue #29)。

## dock 統合テストと detach→reattach の ID・状態保持

`tests/dock.rs` で fake host（`multi_window.rs` の FakeHost パターンを再利用）による end-to-end を検証した (Issue #182)。detach（menu-move one-shot と begin+webview-ready+commit の両経路）から reattach までで、論理 View ID と `MoveCommitted` handoff（consumer 側 view state 実演）が保持されることを確認した。create_window 失敗・Ready 未到達時の abort、commit 時の dest Destroyed rollback、close 3 ポリシー、snapshot save/restore も同じテストで検証された (Issue #182)。`demo_dock.rs` は headless script デモで detach→reattach を print 実行する (Issue #182)。

`cargo test -p ruviform-ui dock` は persistence テスト（roundtrip・schema/構造/重複の reject・off-screen/missing-monitor 補正・dirty tree normalize）込みで green だった。`check-ui-webview`/`lint-ui-webview`/`test-ui-webview` も `position` 配線を含めて green だった (Issue #182)。リスクとして、`WindowConfig.width/height` は logical、snapshot 側は physical px で consumer が scale_factor 換算する責務があり、`WindowPlacement` doc に明記された（混合単位は snapshot 内で完結する）(Issue #182)。

Issue #181 では `demo_multi.rs`（SessionConfig で main+sub の 2 Window、handle_window/drain_jobs_routed→eval の consumer 配線例、last-window policy 例）を追加し、fmt-check/check/lint/test/test-ui-webview 系が全て green だった (Issue #181)。実機検証は実施範囲のみ impl record に記録し、未検証 OS・異 DPI・モニター切断は `unverified` と明示した。consumer 影響は synvelis=impacted、audio-lens=no impact（単一 Window baseline 不変）と分類した (Issue #181)。

## GPUI 採用 Gate の verdict matrix と証拠の再実走

Issue #262 では、A〜E Gate × OS × capability を CaseID/expected/observed/verdict/証拠で記録する verdict matrix・証拠 report・#419 引き渡し文面（「L1 のみ評価済み、L2/L3 非保証」+ #465 独立判断境界）・検証記録を `evidence/rf262-gpui-verdict/` に作成した (Issue #262)。Fail/NotRun/Unsupported を Pass に繰り上げず、証拠再実走（test-ui-scenario-contract/check-ui-driver-boundary/test-ui-native-slot/test-ui-parity/test-ui-gpui）を HEAD 上で実行して verification.md に記録した。verdict 決裁宣言・Issue 投稿・ADR status 変更は Coordinator 権限として未実施だった (Issue #262)。

Gate D では HWND 機構と画素を別 Case に分離した (Issue #262)。GPUI 0.2.2 の `directx_renderer.rs:793` の `CreateTargetForHwnd(topmost=true)` が child HWND を覆い、`WS_EX_NOREDIRECTIONBITMAP` で GDI redirection fallback がないことを確認し、#532 を open Blocker として併記した。verdict-matrix では GD-D1 機構層 Pass / GD-D2 画素層 Fail / GD-D3 Dock PASS 未達を別 Case に分離し、各 report の「不成立/Partial」明記をそのまま転記した。Gate E は `consumer-verify.yml` が workflow_dispatch 専用で対象 SHA run がなく NotRun に固定された (Issue #262)。

HEAD `7aab6302` 上で `mise run test-ui-scenario-contract`/`check-ui-driver-boundary`/`test-ui-native-slot`/`test-ui-parity`/`test-ui-gpui` を再実走し、scenario-contract fake 16+gpui 7、boundary verdict: pass、native-slot 13、parity 14、gpui lib 77+各 test target が全て Pass だった (Issue #262)。これは L1 契約層の補強証拠であり、実 GUI/実 plugin の Pass ではないと明記された (Issue #262)。

## Windows 実機 multiwindow smoke と SendInput 環境退行

`real_os_multiwindow_smoke` fixture（3 window・focus/keyboard/Dock/move/close/regenerate/job policy・OS oracle・cursor 復元）を実装し、Windows 実機で verdict=pass（17 checks）を 1 回実測した (Issue #243)。事後 run で SendInput が kernel driver に drop される環境退行を検出したため、cursor read-back oracle を強化（commanded point・cursor 実位置・hit-test・foreground を記録）して診断可能化した。capability matrix は Pass（前回実測）と NotRun（再検証・環境ブロック）を誠実に分類し、健全 host での再実行を follow-up に記録した (Issue #243)。

`mise run example-ui-gpui-real-os-multiwindow-smoke` を対話 desktop・Windows 11・revision `4eea9115` で再実走し exit 0・verdict=pass を確認し、canonical verdict JSON を v2 schema（preflight 付き）で再生成した (Issue #243)。併せて rf313 capability matrix の stale 行（NotRun・environment-blocked 記述）を実測に整合させ、関連 task/spec/test-spec/plan の status を done/implemented に更新した。Issue #243 全体は Open 継続で、macOS/Linux・動的 DPI・Native Dock・consumer 非回帰は別 Gate と記録された (Issue #243)。

clippy 対応 refactor 後の再実行では、`per_window_focus_real_click` 等 4 check が fail した (Issue #243)。commanded (1140,152) に対し cursor read-back は (586,1079)、foreground は無関係な `タスク マネージャー` だった。host-level 切り分けで、SendInput は受付を返すが注入イベントが一切効果を持たない system-wide ブロックと判定し、`mouse_event`/`keybd_event`/`SetCursorPos`/`WScript SendKeys`・Task Scheduler context・`vgk`/`vgc`/`vgtray` 停止・LG HUB/GameInput proxy kill・class filter・BlockInput・RDP・IL 差異の否定を記録した。「SendInput 受理 ≠ カーソル到達」を fail として正直に記録し、fixture/backend は変更せず環境要因として記録に留めた (Issue #243)。

Issue #552 では、実機 lane（対話 Windows desktop）で `real_os_smoke`（Keyboard）・`real_os_pointer_smoke`（Pointer）・`real_os_multiwindow_smoke`（PointerAndKeyboard）の 3 件とも verdict pass・exit 0・`preflight.effective=true` を schema v2 で記録した (Issue #552)。`mise run fmt-check`/`check`/`lint`/`test`・test-ui-gpui（os_input mock 8 件含む 77+22+4+7）等全 lane pass だった。review-protocol で空 scope の vacuous-true（CONFIRMED）を修正した。preflight 不通の実環境再現は自然発生待ちで強制障害注入は行わないと記録された (Issue #552)。

## window_lifecycle_probe の実機検証

観測用 example `window_lifecycle_probe` を追加し、Windows 実機で probe 既定モードを `wm-close.ps1 -Steps "veto,veto,veto,main"` で実行して `[probe] PASS`+exit 0 を確認した (Issue #243)。`--degenerate-create` は受理+clamp を分類記録し、`real_window_lifecycle`（`inspector,inspector,main`）と `minimal_window`（`minimal`）も exit 0 だった。fmt-check/check/lint-ui-gpui/test-ui-gpui 全て exit 0 で impl record に証拠・再現手順を記録した (Issue #243)。

## 実キー・pointer 注入の実測（Issue #313）

`test-ui-real-os-scenario` が Windows 実機で pass した (Issue #313)。2 コントロール view を real window に mount し、`Tab→Right→Tab→Left` の実キー列（atomic Begin+Commit）→計測矩形上 pointer drag（non-atomic・gesture_id 一致）→click→`drain_intents` で IPC intent 検証→head record へ `Event::result_ok` で `Applied` observe→`capture_frame` で実ピクセル取得を行い、verdict `pass` を `evidence/rf313-os-capability-matrix/verdict-real-os-input-windows.json` に記録した。既存 real lane・headless lane 全て非回帰だった。teardown の `RefCell already borrowed`/`HRESULT 0x80070578` は main 由来の既存ノイズとして記録のみとした (Issue #313)。

stdio transport の frame cap `MAX_INTENT_BYTES`（1 MiB）を超える screenshot reply が session を `DriverGone` で殺すことを実走で発見した (Issue #313)。800×600 級 RGBA→base64 reply は cap 超過で `write_frame` 失敗→serve が `DriverGone`→AUT 死亡→後続 op 全て `Disconnected`/`BrokenPipe`（初回実走で観測）だった。adapter は encode 後に cap を pre-flight し超過時は `service_error` で op のみ失敗させ channel を生かすよう対処した。lane は fixture の `width`/`height` で 320×240 window を capture 対象にし、pixel は常に実 capture・量だけを transport に合わせた。wire 側 chunked reply は closed op set 不変の制約で未着手と記録された (Issue #313)。

`RVF_WINDOWED_UITEST=1` gate の manual lane で全 11 op を実走 pass した (Issue #313)。hello（backend `gpui-windowed`・screenshot claim・input_injection/dom_access denied）、windows（real `Ready`）、fault.set（`ipc_down`+`suppressed` receipt+`ipc.suppressed` notice）、fault.window_crash（対象 window のみ `Destroyed`+`window.destroyed` notice）、request_close（veto 維持→未応答 default-approve で destroy）、shutdown（`exited`・AUT は `ServeOutcome::Shutdown` で code 0）を `verdict-windowed-uitest.json` に記録した。env gate なしでは no-op skip で verdict を書かない設計で、未実走を pass と偽装しない。UI loop の死は `process::exit(3)` で fail-closed とした (Issue #313)。

`real_os_smoke` example が実 GPUI window で `SendInput(Tab,Right,Tab,Left)`→WindowLifecycle::Action commit intent を実測 pass した (Issue #313)。`Ready`+`Focused(true)` 観測後に injector thread が `SendInput` でキーを投入し、+step/−step commit が異なる control_id で両方観測されることを合格条件にした（Tab 巡回順は tab stop spec 順依存のため順序非依存 assert）。verdict=pass exit 0・`verdict-windows.json` 採取（Windows 11 Pro x64・revision `4aa92c1`）。非 Windows は `unsupported`/exit 2 で「未実行」を「失敗」と偽らない設計だった (Issue #313)。

`real_os_pointer_smoke` は Fader ドラッグ+Seek クリックを `SendInput` で注入し非 atomic gesture intent を実測 pass した (Issue #313)。cursor read-back ±4px を証拠化し、「注入した」ではなく「カーソルが実際にそこへ行った」の証拠とした。`Ready`+`Focused(true)`+geometry/bounds 揃いをゲートに、Fader ドラッグ（Begin≈0.25/Commit≈0.75）と Seek クリック（≈0.8）を注入した。2 回実行とも pass・intent 値 0.25/0.75/0.8 が一致した。起動時カーソル位置を保存し終了時に復元（best-effort）した (Issue #313)。

Issue #313 の OS-L1 slice では、対象 SHA `e93c438a` の Full CI run `43428172820`（conclusion=success）から lane/test 単位で証拠照合し、capability matrix に #313 scoped 19 行を追加した (Issue #313)。`gh run view --log` で `verify-{ubuntu,windows,macos}` の `test-full`、`ui-gpui-verify` 3 OS、`ui-gpui-offscreen-verify`（real_window 3/3 pass・`real_os_input_scenario` は Linux で `verdict=unsupported` を正直に記録）を照合した。stale 行 `scenario-driver-realos-adapter`（「adapter 未実装」）を `RealOsGuiTest` が `real-scenario` feature で実装済みの実態に訂正した。新規 timeout e2e 3 test は merge 後の 3 OS verify lane が初回実走予定として記録された (Issue #313)。

`test-ui-real-scenario` では 2 story × 4 cell = 8 session 全て pass し、golden verdict は cell 毎に `blocked/baseline_missing`→`pass/match` となった (Issue #314)。Windows 11 desktop session で `mise run test-ui-real-scenario` exit 0・2 tests passed/8 real-window sessions。driver-lab evidence 8 行全て `observed=pass`、`evidence/issue-314-real-scenario/png/` に 16 PNG+8 meta.jsonl を記録した。`stories.json` の `drivers.real-os` を `mise-task` mechanism に更新した (Issue #314)。

canonical story を実 window に mount し `golden_check`（`driver_class=real-os`・`capture=RealWindow`・実測 scale/backend）で baseline_missing→pass/match（compared=1,024,000, mismatched=0）となった (Issue #314)。`cargo test -p ruviform-ui-gpui --features real-scenario --test real_window` で 2 passed。idle/teardown の実 pixel capture（1280×800, `win32-printwindow`, sf1.0）と driver-lab/golden JSONL・PNG 4 枚を `evidence/issue-314-real-window/` に記録した。`dispatch_input` は semantic dispatch で pointer/keyboard 証拠を名乗らない境界を明記した (Issue #314)。

## 実 window lifecycle の OS close 検証（Issue #431）

`e2e_smoke`（WebView2 往復）・`demo_app`/`demo_multi` の OS close（wm-close.ps1）・`real_window_lifecycle`（Dock 2-phase・defer/approve の OS close）を実機で PASS させ、陰性対照で trace 照合が失敗すること・`minimal_window` の OS close（報告書 M1）も確認した (Issue #431)。legacy trait Phase 2 の判断 ADR は status: proposed で作成、mise.toml に manual タスク 2 件追加、fmt/lint/test/境界 guard 記録・報告書・独立レビュー（blocker なし）まで done だった。T12 の commit/push/PR は pending と記録された (Issue #431)。

## GPUI driver の capability 宣言と story matrix

`GpuiDriver` の capability 宣言不整合を発見した (Issue #386)。`focus` を Supported 宣言するが `focus(_, false)` は gpui 0.2.2 制約で `UiError::Unsupported` を返す。fixture は「Ok か Unsupported のどちらか（Host error ではない）」を要求しており、headless trace の一致は capability 宣言の正しさを保証しないことが判明した。宣言の精緻化は後続 Issue 候補として記録された。DoD 対応表では「common public interfaces が GPUI/JS/WebView/OS Handle なしで build」「Fake/GPUI 同一 meaning fixture 合格（headless trace）」は達成、「WebView 側写像・実 OS 検証」は未達と正直に分類した (Issue #386)。

Issue #426 では、fake 層で 2 theme×2 scale×7 state の trace 一致、GPUI 層で `lab_story` 4 test pass、`golden_capture` の story-keyed 実窓 capture（PrintWindow/BitBlt→compare→promote→pass）を desktop session で実測 pass した (Issue #426)。consumer 検査（audio-lens `check-cli-headless`/`check-ui-gpui`、synvelis `check-headless-deps`）は pass だが、consumer の Cargo.lock が pin する ruviform revision で評価されるため本 PR HEAD の評価ではない点に注意と記録された。window pixel scale 1.0・実窓 capture の CI 化・real-os lane は NotRun/blocked のままだった (Issue #426)。

Issue #423 では、A1（旧 API が adapter 内に閉じる・rg で一致なし）・A2（focus 宣言一致）・A4（late receipt が view 描画投影まで到達）・A5（driver boundary verdict pass）・A6（non_exhaustive/MSRV/lint）達成、A3 は実 OS 起動で Created/Ready/Focused/Resized/Moved を確認したが close・Dock 2-phase・detach は手順のみで部分達成だった (Issue #423)。全 mise lane pass（test-ui-gpui 36 件等）。WebView2 上の end-to-end と実 OS lane は NotRun と記録された (Issue #423)。

Issue #414 では、修復前に「外部非ゼロ projection を idle で受けても display.normalized は初期値のまま」という反例を gpui/component_view test で再現（RED）し、修復後に全 lane で GREEN を確認した (Issue #414)。fmt/check/lint/test-ui-* 系・MSRV 1.89 lane 全て pass、driver-lab に `UI-DVR-09` required case を追加（required_cases 8→9）。macOS/Linux 実機は NotRun、SV 側 workaround は追加せず RF 所有境界で修復した (Issue #414)。

Issue #410 では、`mise run fmt-check`/`check`/`lint`/`test`（ruviform-ui unit 99 tests）/test-ui-* 系 12 lane すべて exit 0 だった (Issue #410)。review-protocol の Issue Candidate で test-spec Rule「floor は control_id ごとに独立」に対応 test が無いことが CONFIRMED P1 となり、`projection_floors_are_independent_per_control` を追加して解消した。`update` 経路は binding 存続のため floor の clear/非 clear は観測不能（P3・記録のみ）と判定した (Issue #410)。

Issue #418 では、`codegen-theme`/`codegen-theme-check`（byte 比較）・fmt/check/lint/test が origin/main merge 後に全て成功した（test は 181 test-result 群・失敗 0）(Issue #418)。`tests/codegen.rs` 7 test（byte-identical 再生成・committed 鮮度・generated/runtime `to_normalized_json()` 一致・invalid manifest で非 0 終了かつ部分出力なし等）全 pass、clippy `-D warnings` clean。review-protocol で CONFIRMED P0/P1 なしだった (Issue #418)。

Issue #478 では、新規テストを旧実装（h_full）に先に実行し RED を確認（Fader=936px/Waveform=120px で Fader が残り高を独占、grow slot 合計 1038.5px vs 期待 1032px）してから修正した (Issue #478)。gpui-test 3 件追加（`rvf_view_fader_slot_shares_height_with_siblings`、`rvf_view_fader_in_tabs_fills_body_remainder`、`rvf_view_canvas_bounds_nonzero_for_parity_kinds` へ assert 追加）で赤→緑を確認し、既存 UI lane 全 pass だった。全 mise lane pass（test-ui-gpui 65+20+7、parity 12、consumer-observation 20 等）。`component_showcase` 実窓で Fader≈299px の body 残り充足・drag gesture の Begin→Preview×13→Commit intent 列を観測した。`mise run test` 初回に無関係の flaky（l1_proxy_e2e）を 1 度観測したが単独・全体再実行で pass した (Issue #478)。

Issue #480 では、`RvfView::flush_events` で `Removed`/`Unmounted` op を受けたとき `tab_focus` の対応 entry を解放し、strip の FocusHandle が spec の存続期間を超えて残らないようにした (Issue #480)。gpui-test に解放検証テスト（Removed・kind-swap・Unmounted・sibling 保持・Updated 保持）を追加し RED 3 fail → GREEN 3 pass を確認した。op が flush まで滞留する経路は production に存在しない（全 mutation path が同期 flush）ことも確認済みだった。`control_focus`（#479 FU-c）の解放は範囲外として deferred と記録された (Issue #480)。

Issue #263 では、review で `rvf-range-selector` が per-edge control id（`<id>-lo`/`-hi`）を single `<id>` binding の filter が捨てていた silent unwire を発見・修正した (Issue #263)。range-selector は edge bindings+targets+`bindRangeElement` reflect path に修正し regression test で `range-lo` commit の gateway 到達を assert した。`examples/showcase.html` の receipt drain が microtask flush 前に `received` を splice して commit が 1 gesture 遅れ最後の gesture が timeout `invalid` になる race を `flushWith` 同期 mount で修正した。theme parity の vacuous assertion（非空 assert 追加）・a11y coverage 拡張・`DemoAnalysisBackend.currentRevision` の stale reply 配線も実施した (Issue #263)。

Issue #260 では、review-protocol pass で確認した blocking findings を修正した (Issue #260)。(1) GL hatch は意図的に rect をはみ出す segment を描くため Canvas2D の `ctx.clip` に依存していたが WebGL2 には per-op scissor がなく meter gap-hatch が canvas 全体に streak した → `pushClip`/`popClip` 追加。(2) context-pinning canvas では fallback と `webglcontextlost` recovery が dead end → `replacePlotCanvas` で新規 plot canvas node に差替えてから Canvas2D retry（context-pinning fake canvas で regression test）。(3) superseded in-flight query が cache-hit/empty-span early return で abort 前に走り stale data が着地 → `++querySeq`+abort を invalidate 先頭へ移動した (Issue #260)。

Issue #436 では、`test-ui-consumer-observation` 11 件（`observes_drawn_meter_and_axis_plans_through_seam` が `new-kinds.json` の drawnBars 3 bar/drawnMarks 3 mark を件数・順序・値 1e-3 許容で照合）・`test-ui-gpui` 72 件・workspace `test` 3088 件失敗 0・MSRV 121 件など全 lane pass だった (Issue #436)。`drawn` の観測値は spec からの純粋計算で実 pixel ではない点、AudioLens 側実 consumer test は pin 更新後の作業として NotRun が残ると記録された (Issue #436)。

Issue #442/#443/#435 では、3 Issue は main 統合済み（PR #444/#445+#446/#440）で、本タスクはコード無変更の検証・文書整合だった (Issue #442)。全 mise lane pass（test 3088 件・consumer-observation 11・parity 11・gpui 72・MSRV 121）。consumer 受入は audio-lens main のソース確認で代替（`gpui_parity.rs` の照合、`view/screen.rs` の `tile.columns()` 使用、`run_screen.rs`/`job_list`/`analysis_screen.rs` の `label()`/`table()` helper）— consumer repo の実 test lane 実行は pin 更新後の作業として残ると記録された (Issue #442)。

## paint 性能の計測と O(n²) 回帰の解消（Issue #461）

headless `TestAppContext` の実 paint 経路（Scene 構築・BoundsTree::insert を含む）で契約上限 payload の repaint を計測し、修正前値として Spectrogram 6.37s / Waveform 45.19s / Meter 13.06s / TimeAxis 906ms（debug build）を取得した (Issue #461)。原因は gpui の `BoundsTree::insert`→`find_max_ordering` の走査構造にあり、4 painter の要素ごとの quad を `window.paint_layer` で囲む修正後は最大約 55ms となった。回帰ゲート budget 500ms（実測の約 10 倍、O(n²) 秒オーダーとの識別優先）で `rvf_view_max_payload_canvas_repaint_budget` を追加した。実機 paint 計測は NotRun（headless で全経路を通すため O(n²) 回帰は検知可能と判断）と記録された (Issue #461)。

残課題スライスでは、`time_axis_element` の label `div`（最大 1,024 要素）を廃止し、canvas prepaint で `WindowTextSystem::shape_text` により shaping した label を tick mark とは別の 1 つの共有 `paint_layer` 内で glyph 描画する方式を採用した (Issue #461)。診断計測では label 空で約 125ms、1,024 labelled で約 0.8〜1.7s、全 cull で約 262µs であり per-line `paint_layer`→`BoundsTree::insert` が支配的と特定した。修正後は約 11ms（budget 500ms）となり、`rvf_view_max_payload_canvas_repaint_budget` に `time axis 1024 labelled ticks` ケースを追加して回帰ゲート化した (Issue #461)。

## plugin editor の実 attach・detach と Native Dock の限界

`GpuiSurfaceAdapter`+consumer PoC で Windows 実 GPUI window 内への実 native editor attach を実機観測した (Issue #388/#389)。3 回連続で同一証拠列を再現した。`GpuiSurfaceAdapter`（`NativeSurfaceSlot` port 実装: `open_window` で `HasWindowHandle` 経由 GPUI top-level raw handle 捕捉、slot 毎に Win32 child container HWND `STATIC` class・`WS_CHILD|WS_CLIPCHILDREN|WS_CLIPSIBLINGS` 生成、geometry/visibility/clip(SetWindowRgn)/z-order/focus/破棄を実表面へ適用）と `ruviform-poc-native-dock` を新設した。`evidence/rf388-389-native-dock/poc-run-win11.log` で container HWND 実生成→probe `open_gui` 実 child editor（`GetParent` で実親子確認）→`set_geometry` rect 追従→hide/show `IsWindowVisible` 追従→`request_focus`→session close で editor 破棄→`release_lease` で container 破棄を 3 回連続 exit 0 で再現した。ただし実 dock layout widget 未実装のため Dock PASS ではないと記録された (Issue #388/#389)。

Issue #388 では、`GpuiDriver::update_view`（mounted `ViewSpec` 差替えを `Op` キューで適用）を追加し tab 切替・resize・focus・close・解放を `check=… result=pass|fail` 証拠行で記録した (Issue #388)。test-spec の Dock PASS 完了条件（clip・入力・re-dock・plugin resize 等）を全ては満たさないため Pass を付与しなかった。`update_view` は拒否された spec の理由を呼び出し元へ返さない（`ViewHost::update` 契約で前の木を残すのみ）既知の制限ありと記録された (Issue #388)。

Vinyl 1.12.0 で dock scenario exit 0・`summary failures=0` だったが Dock PASS は未付与で、clip は slot が window 端まで伸びるため検証不能で NotRun と記録された (Issue #388)。

GPUI 0.2.2 の DirectComposition（`CreateTargetForHwnd(hwnd, topmost=true)`）が window の全 child HWND を覆い plugin editor の画素は画面に合成されないことを Unsupported・実測で確認した (Issue #388)。HWND 機構（parentage/geometry/visibility/clip/hit-test/focus/key/lifetime/re-dock）は全て実測 Pass（`dock-windowing` 33 check 全 pass・`failures=0`）だったが、slice 2 で「Vinyl 背景」と解釈した `38,45,54` は GPUI `control_track`（`0x262d36`）であり plugin 画素ではなかった。`GPUI_DISABLE_DIRECT_COMPOSITION=1` 経路での child 合成確認は GPUI 側 crash（exit 127・artifact なし）で未達だった。Native Dock PASS は画素表示未達のため不成立で、対処は GPUI 側変更（`topmost=false` 等）か adapter 層設計変更が必要で本 slice 範囲外と記録された (Issue #388)。

Issue #388/#389 では、`concluded` flag 追加で born-`Closed` と lifecycle 収束 `Closed` を区別し、full workspace test で second live session 誤通過の回帰を検出・修正した (Issue #388/#389)。独立 review（Reviewer A/B→Validator）で P1 2 件を確認・修正: concluded session の再 open 経路と live session の `end_session` 除去。副次修正として lease seq を registry-global `u32` 化・slot generation tombstone・capability 未 enforcement・sync/async reattach event 不整合・L0 floating open の暗黙 docked 偽装・`NativeSurfaceSlot` trait の `release_lease`/`validate_lease` 欠落・`PluginEditorSessionId` generation `u16`→`u32` 拡幅を実施した。`PluginHandle.generation` は wire 互換のため `u16` のまま残留（別 Issue 候補）と記録された (Issue #388/#389)。

Issue #388 OS-L1 では、対象 SHA `e93c438a` の Full CI run `43428172820` で 19 checkout-sha artifact 全件の実測 SHA 一致を確認し、`ci-summary` verdict=PASS は宣言 lane 全 success の fail-closed 判定だった (Issue #388)。`evidence/rf388-os-l1-ci/report.md` に受入条件↔lane/test↔Run URL↔verdict の対応表を記録した。`native_slot_contract` 13 件・`test-ui-gpui` の `windowed_test_app` bin compile/link・rt-verify strict `rt_` 166 tests・offscreen real_window 3 tests 等を job log で実測値記録した。`ci-summary` verdict=PASS は「宣言 lane が全て success で対象 SHA が一致」の判定であり個別 Case（実 Dock）の Pass を意味しないと明記した。既知 gap として `ruviform-poc-native-dock` の `gpui`/`vst3` feature が CI で compile されない点を注記した (Issue #388)。

## plugin editor session の headless・real lane 検証（Issue #389）

workspace fixture は `clap.ext.gui` を持たないため `open=Unsupported` が正直な実測値だった (Issue #389)。manual lane で load→init→slot→lease→`open_gui` 分類まで検証した。`RVF_POC_CLAP_PATH=target/debug/ruviform_plugin_clap.dll mise run poc-clap-editor` で load→descriptor→init→declare_slot→lease_parent→`open=fail Ops(Unsupported)`→`done Failed{Unsupported}`→clean exit。fixture が `clap.ext.gui` 非実装のため Unsupported が honest 分類で偽装成功しない。size negotiation（`get_size`/`set_size`/`can_resize`/`adjust_size`/`set_scale`）は `open_gui` に含めない（D2 — container 幾何は slot 側所有）と記録された (Issue #389)。

headless fixture は `src/` 差分ゼロで visibility no-op event 抑止と実 `InProcessHost` `process` 経路の DSP/RT 非干渉を 4 case で固定した (Issue #389)。EH-01..04: `visibility_noop_emits_no_event`（同 visibility 要求は event/state を変えない — event 列は実 state 変更と 1:1）、`l0_session_open_close_leaves_dsp_processing_untouched`（`Processing` 中に open→非 lifecycle op 拒否→close、全段 `lifecycle_state==Processing`・`process().status==Continue`）、`l0_session_abnormal_termination_leaves_dsp_processing_untouched`（`notify_terminated(HostLost)`→`Terminated`→`Closed` 収束でも DSP 不変）、`l0_refused_open_leaves_dsp_processing_untouched`（既定 `Unsupported` refused open→`Failed`→adapter 非呼出 close でも DSP 不変）。`editor_session` 49 passed/0 failed だった (Issue #389)。

`real-session-terminate` case で実 `IPlugView` attach 状態の `Deactivate`+`Destroy`→`notify_terminated(HostLost)` 収束を real-window lane で pass した (Issue #389)。`rf631.plugin-dock-real` catalog を 12→13 行に拡充し、test_editor の実 `IPlugView` が real container HWND lease 配下に attach した状態で host loss を実行した。event 列がちょうど `[Attached{Docked}, Terminated{HostLost}]`（2 回目 notify は no-op=exactly-once）、instance drop 由来の child HWND teardown（`IsWindow(child)==0`）、host 存続（`Scan`→`Scanned`+破棄済み handle の `InvalidHandle`）を OS 観測で検証した。verdict 写像は `editor_child_body` に集約し refused open→`Unsupported`・観測 verified→`Pass`・platform 非対応→`Unsupported`・契約不一致→`Fail` を `real-editor-attach` と同一経路で共有した。`real-dcomp-pixel` の既知 fail（#532・exit 1）は設計どおりで緩和しないと記録された (Issue #389)。

`rf631.plugin-dock-real` の 3 row（`real-editor-resize`/`real-editor-redock`/`real-editor-visibility`）を refusal/deferred から実実行+OS-level effect oracle 検証へ flip し全て `pass`/`environment_kind=real-window` となった (Issue #389)。effect oracle: resize = 全 child `GetWindowRect == 800×600`、redock = 旧 child HWND 全 `IsWindow==0`+新 generation title 前進（fixture が window title `rvf-test-editor-attach-{n}` に attach generation を刻印）、visibility = hide で全 child 破棄+show で新 generation 再成立。`EditorOpVerification.effect_confirmed`+`EditorOpIdentity` を追加し `editor_op_body` に `Pass` arm（accepted+lifecycle verified+effect confirmed）を追加した。`ChildWindowFacts` に `width`/`height`/`title` を追加し oschild Windows impl が `GetWindowRect`/`GetWindowTextW` を観測した。headless fail-closed は 22 row 全て `not_run`+`environment_blocked`・violation 0 だった (Issue #389)。

dock-test session-lifecycle で 8 case 実走した (Issue #389)。`mise run test-ui-plugin-dock` で `view-mount=pass`(fake), `artifact=pass`, `host-lifecycle=pass`, `editor-attach=unsupported`, `session-lifecycle=pass`, `host-terminate=pass`, `open-or-degrade=pass`, `dock-pixels=not_run`。test_gain `createView` null→open は honest `Unsupported` であり session 3 case の Pass は session registry 契約（`SessionBusy`・refused-open converge・`Terminated{HostLost}` exactly-once・degrade 非発火）の実走証拠であって実 editor attach・Native Dock・画素の証拠ではないと明記した。`docs/designs/issue-389-plugin-editor-session.md` の stale verification-status（R5 matrix・D8・`GuiParentToken`/`WIRE_VERSION`）を現行コード事実へ reconcile した (Issue #389)。

VST3 では Vinyl 1.12.0（Windows 11・GPUI 0.2.2）で実外部 Editor attach/detach を実機検証した (Issue #389)。`open=ok`、editor HWND parenting/geometry/visibility/close/release 全 Pass。`Vst3Plugin::open_gui` = `IEditController::createView("editor")`→`isPlatformTypeSupported(kPlatformTypeHWND)`→`setFrame(BridgePlugFrame)`→`attached(parent HWND)`。`close_gui`/`close_view`/`Drop` = `removed()`→view/frame release→`disconnect(IConnectionPoint)`→`terminate`。headless fixture（`open_gui` 未 init→`PluginFailed`、fixture `createView=null`→`Unsupported`、close 未 open→`Unsupported`）と canonical 検証全 green だった。keyboard/IME=NotRun、Dock PASS=NotRun（RF387 未接続）、L1/CLAP/AU/他 OS=Unsupported、他社 VST3 広範互換=NotRun と正直に分類した (Issue #389)。

`parent-lease` では `mise` check/lint/test 全 pass・`GuiParentToken` 0 件だった (Issue #389)。fmt-check PASS、plugin-core/plugin-host/plugin-vst3(host)/plugin-clap(host)/poc-native-dock/ui-gpui/plugin-export/plugin-ofx/audio-plugin-bridge の check PASS、6 crate の lint -D warnings PASS、test PASS（plugin-core 126・plugin-host 226+49+187・plugin-export 53）。`grep -r GuiParentToken crates/` は 0 件。実 VST3/CLAP GUI attach・Native Dock pixel・外部 plugin 受入は外部 gate（#388/#532/SV#176）として NotRun を正直に記録した (Issue #389)。

`real-session-lifecycle` では 21 cases・real-dcomp-pixel fail は #532 既知で非起因だった (Issue #389)。`test-ui-plugin-dock-real` interactive run が 21 cases — degraded/recovery/timeout が正確な event 列で pass、visibility/crash/hang は設計どおり unsupported。`real-dcomp-pixel` の fail（`green_pixels=0/1019844`）は baseline/baseline2/dcomp-disabled の 3 過去実走と同一値で #532 の GPUI DComp topmost occlusion 既知問題と確認し、本 slice 起因でないことを切り分けて fail のまま正直に記録した (Issue #389)。

## Issue #631: real-window dock の実走記録

`real-command-receipt` case で gesture→receipt→CommandPhase の実走 link を記録した (Issue #631)。`rf631.plugin-dock-real` catalog に `real-command-receipt` case を追加し、実 Window の gesture→intent→`post_event` receipt/projection→`CommandPhase` 遷移→decode 済み `CommandReceipt` populate の AC1 link + AC4 evidence を実走で記録した。interactive 実走で `real-command-receipt` pass + receipts 2 件、`real-window-close` pass、`real-dcomp-pixel` 既知 Fail のみで exit 1 は設計どおりだった。window D 追加で close case の終端 assert が実走 fail したため「非対象 window 全生存」へ一般化して対処した (Issue #631)。

command-receipt の interactive 実走は 15 pass/1 fail/6 unsupported で `real-command-receipt` = pass（applied `revision:1` + rejected with detail の 2 receipt）、`real-window-close` = pass、`real-dcomp-pixel` = fail（既知 #532、意図的に据え置きで lane exit 1 は設計どおり）だった (Issue #631)。初回実走で close-case が `expected only sentinel window A to remain, got [w1, w4]` で回帰し、`close_body` の終端 assert を「sentinel A のみ」から「全 non-target window が lifecycle-clean で残存」へ一般化して修正（より強い検証）した。`real-dcomp-pixel` の fail は弱めないと記録された (Issue #631)。

headless-command-receipt は `command-receipt` case が pass で applied `revision:1` + rejected（detail `consumer refused the commit`）の 2 receipt を記録し headless lane で初めて `receipts` が非空になった (Issue #631)。catalog 8→9 cells、console projection 30→31 rows。全 mise lane pass（real-dock/lab-gui feature 含む check/lint/test、test-ui-plugin-dock(-console/-real-headless)、scenario-contract、driver-boundary、prek）。helper 複製は既知の文書化済みコストとして ADR に記録した (Issue #631)。

real-row は `REAL_CATALOG` を 5→12 行に拡充（AC1 残件の focus 移譲・実 OS pointer/keyboard・DPI・IME・close と AC3 の resize/redock）した (Issue #631)。interactive Windows desktop 実走で `real-os-input` = Pass（slot bind 後に全注入 Injected）、`real-window-dpi`/`ime`/`close`/`editor-resize`/`editor-redock` = Unsupported、`real-dcomp-pixel` = Fail（既知 #532、Pass に変換しない）で lane exit 1 は意図どおりだった。`--blocked-probe` は 12 case 全て `not_run`+`environment_blocked`+`deterministic-backend` で exit 0。review-protocol の CONFIRMED 指摘（OpLanding::Accepted は verified 必須、pointer aim 非 Injected 時は click 不発火等）を反映した (Issue #631)。

window-surface は `real_window_lifecycle.rs` の 2 test（`window_close_lifecycle`: Veto/Defer/deferred-Approve/default-Approve の全経路と sibling/sentinel 非干渉、`window_dpi_geometry`: live `OsWindowGeometry` 安定性と `Resized` 物理 pixel 一致・破棄後 honest error）が実 Windows desktop で pass した (Issue #631)。`evidence/rf631/real-window-surface.jsonl` に verdict=pass の実測行を記録した。既存 `real_os_input_scenario` が `NotRun — window w1 is not the focused window` で一度 fail したが単独・lane 再実行で pass — 並列 real-window test の OS focus 競合による既知の環境 flake で本変更起因ではない。Win32 invalid-HWND `HRESULT(0x80070578)` 等のログは baseline 既存テストでも出る環境ノイズと記録された (Issue #631)。

test-lab の GUI dogfooding（PostMessage click、interactive Windows session）で catalog 12 row 描画・row select・run-selected（artifact→pass）・run-all headless（8 case の verdict を正直記録）・real row 選択時の委譲（JSONL 非増加・非実行）・Quit clean exit を確認した (Issue #631)。`mise run lint ruviform-plugin-dock-test --features lab-gui`（deps 込み canonical）は path dep `ruviform-audio-test` の既存 `too_many_lines`（`stream.rs:885`）で fail — `git diff HEAD` 無変更の upstream drift で本 slice 無関係・scope 外として PR で報告した。window サイズは初回 dogfooding で results table が actions panel を clip するのを観測し (1100,720)→(1100,860) に修正した (Issue #631)。

evidence-fields は interactive Windows desktop で `test-ui-plugin-dock-real` を実走し `plugin-dock-real-scenario.jsonl` を新 field 込みで再生成（exit 1 = `real-dcomp-pixel` 既知 Fail による設計どおり）した (Issue #631)。全 22 case の `environment.window_identities`・capture row の `capture_sha256`・semantic_tree 全量を確認した。`plugin-dock-real-headless.jsonl` は `--blocked-probe` で再生成（22 row `not_run`+`environment_blocked`、session-scoped field 非記録を維持）、`plugin-dock-scenario`/`plugin-dock-console` も headless `command-receipt` の semantic_tree 全量を含め再生成した。fmt/check/lint/test（default/real-dock/lab-gui）・console/lab/real-headless・scenario-contract・driver-boundary・prek 全て pass だった (Issue #631)。

command-receipt の再実走は 15 pass/1 fail（`real-dcomp-pixel`=#532 既知）/6 unsupported で `real-command-receipt` pass・receipts 2 件（applied revision:1+rejected detail）を実観測した (Issue #631)。`test-ui-plugin-dock-lab` は interactive session 起動・WM_CLOSE clean exit(0) まで確認したが Run all→Quit の dogfooding は `orca computer` が Defender に block・GPUI `Zed::Window` が UIA descendants 0 で programmatic click 不可のため operator 実行の残件として偽 evidence なしに記録した (Issue #631)。

evidence-fields では `test-ui-plugin-dock-lab` が `READY_TIMEOUT`(15s) 超過後も生存し続けたことから interactive Windows desktop が利用可能と判明し（lab window を `CloseMainWindow()` で正常終了）、`test-ui-plugin-dock-real` を実走し 4 window mount・22 case 実 verdict（18 pass/1 fail=`real-dcomp-pixel` 既知/3 unsupported）を記録した (Issue #631)。`plugin-dock-real-scenario.jsonl` に window_identities・capture_sha256・semantic_tree 全量を確認した (Issue #631)。

headless-command-receipt では `plugin-dock-scenario.jsonl` の `command-receipt` 行に `receipts` が非空で記録された (Issue #631)。`{command_id:"view-1:a1:m1:fader-ch1:3",status:"applied",revision:1}` と `{command_id:"view-1:a1:m1:fader-ch1:6",status:"rejected",detail:"consumer refused the commit"}`。Fake driver で AC1 link（minted command_id 相関→delivered event decode）と AC4 evidence populate を実走確認し、`environment_kind=deterministic-backend`・`driver=fake` のみを名乗る honest contract evidence とした (Issue #631)。

## Issue #532: DirectComposition 画素提示の未解決問題

`real-dcomp-pixel` case は screen-DC `BitBlt` を主 oracle、`PrintWindow`/`GetDC`+`BitBlt` を診断 note とした（PrintWindow は `DefWindowProcW` class・Chromium 系で信頼性低く verdict gate にしない）(Issue #532)。実機で baseline と `GPUI_DISABLE_DIRECT_COMPOSITION=1` の双方が exit 1・screen green_fraction=0.0 で Fail だったが、env-on の windowdc 診断は green_fraction=1.0 — child surface は緑を描いているのに画面に出ないため、非 DComp 経路（`CreateSwapChainForHwnd`+`FLIP_SEQUENTIAL`）でも画素が提示されない可能性を示唆し方式 A' の効果は未確定だった。env var が `noredirectionbitmap` exstyle を true→false に変える HWND レベルの実効は確認済みだった (Issue #532)。

occlusion slice の実機 run（baseline・env-on、repo_sha `2e856bca`）は共に exit 1 で `real-dcomp-pixel` verdict fail（screen green 0.0）だったが occlusion note 3 row を記録した (Issue #532)。外部 `Chrome_RenderWidgetHostHWND` が WM z-order で GPUI window より上にある genuine な cover を観測し screen=0.0 は外部 HWND 遮蔽のみで説明可能としたが、compositor 内部遮蔽は `unresolved` のままだった。`GPUI_DISABLE_DIRECT_COMPOSITION=1` が exstyle に実効を持つことを HWND レベルで確認、exit 127 未観測だった (Issue #532)。

presentation-probe の interactive lane は baseline・`GPUI_DISABLE_DIRECT_COMPOSITION=1` の双方で `real-dcomp-present` = pass、`real-dcomp-pixel`/`real-dcomp-foreground` の verdict は不変（前者 fail は既知 occlusion）だった (Issue #532)。inventory は 449 window 中 own-process 0 行で cap/truncation の限界を正直に記録した。残余 ambiguity として topmost-stage の screen capture に settle delay がなく pre-paint frame を排除できない点が key residual として記録され、Issue #532 は open のまま compositor fix 選択は範囲外だった (Issue #532)。

foreground-probe では baseline（DComp on）と `GPUI_DISABLE_DIRECT_COMPOSITION=1` の 2 lane で probe は両方 pass・`real-dcomp-pixel` は両方 fail（authoritative 維持）だった (Issue #532)。hit-test の遮蔽物は run ごとに変わる（`Chrome_RenderWidgetHostHWND`/`Zed::Window`）が、DComp off で `HWND_TOP` のみで hit-test が自 child HWND に解決しても screen green=0 — HWND z-order で説明できない compositor/presentation 内部の第二の境界が存在し、`WindowFromPoint` は DComp visual/plane を見えないため残余機構は未断定のまま Issue #532 open と記録された (Issue #532)。

presentation-probe では baseline/env-on/headless の 3 lane 実走で `real-dcomp-present`=pass（34 steps）・`real-dcomp-pixel`=fail を維持した (Issue #532)。DComp off では child windowdc=1.0・root printwindow≈0.94-0.996 で自 surface に green が存在するのに screen bitblt は依然 0.0。旧 run JSONL との対比で計 4 run 全て「topmost で hit-test が自 child・screen green=0」が環境非依存に再現 — 外部 HWND 遮蔽+uncovered 時の非提示可能性の 2 層と結論づけ、settle 無し即時 capture のため pre-paint frame 説は残ると bounded に記録した (Issue #532)。

## テスト基盤の fixture・conformance lane（Issue #310/#311）

Issue #310 では、legacy 資産の棚卸し discovery doc・spec/design/ADR/plan の doc chain を作成し、Issue #310/#311/#312 系 docs と development-environment.md に legacy banner 注記、ruviform-ui の README と src doc コメントを整理した (Issue #465)。生成 index 再生成・doc audit・merge origin/main 後の mise 正規検証・review-protocol を経て PR（Refs #465）を作成した。物理削除は行わず文書・注記のみの変更だった (Issue #465)。

Issue #310 では、gesture/keyboard-a11y/receipts-states/theme-lifecycle/viz/consumers/deps の各 spec を実装し、`deps.spec.ts` で Ruviform 全 package.json に react/solid/vue/svelte・実 DOM renderer の必須依存がなく `@playwright/test` が browser-test のみであることを検査した (Issue #310)。mise に check/build/test-ui-browser を追加し、ci.yml `ui-verify` に ms-playwright cache→`npx playwright install chromium`（Linux は --with-deps）→`mise run test-ui-browser`→`test-results/` artifact upload（if: always()）の lane を配線した。`test-results/evidence.json` は env/revision/seed/pass-fail を含む。全 spec pass・status done だった (Issue #310)。

Issue #311 では、`faultmatrix.test.ts` で設計 §5 の fault matrix を TS 側に網羅実装（terminal event 存在 assert 付き）した (Issue #311)。`crates/ruviform-ui/tests/ipc_conformance.rs` は fixture 読み込み→`parse_intent`/`Event::to_json`/`DataRegistry::read`/`UiRuntime::handle`/`handle_window` 照合 + host scenario 再生 + job backpressure/panic/cancel。`scheme.rs` の既存 `#[cfg(test)]` を fixture 駆動に拡張（`rvf://data` wire 契約）したが product コードは変更しない。mise に check/test-ui-guitest、ci.yml `ui-verify` に 2 ステップ追加、IR に #294 証拠項目を記録。status done だった (Issue #311)。

T1 では `npm --prefix ui/guitest test`（fixtureload 経由で全 15 fixture parse 済み）と `typecheck`（0 errors）が PASS、Rust 側も `cargo test -p ruviform-ui --test ipc_conformance` + webview scheme fixture lane で同 fixture 群を読み込み済みだった (Issue #311)。環境は Windows 11 x64・Git Bash・Node 26.12.1（mise pin）。fixture は手書き正本のため実装変更時の乖離リスクがあり、契約変更時は fixture 側の `v` バンプ運用が前提（#294 接続時は diff レビュー必須）と記録された (Issue #311)。

T2 では `npm --prefix ui/guitest test` が intents/events/data/binding-messages の全 fixture case で `expect` と一致して PASS、`run typecheck` 0 errors、`mise run check-ui-guitest`/`test-ui-guitest` も lockfile 復元→typecheck→node --test まで一貫して PASS だった（Windows 11・Node 26.12.1）(Issue #311)。

T3 では `npm --prefix ui/guitest test` が 135 tests・0 fail（9 scenario×3 + fault-matrix 10 + recorder + wire + binding）で PASS だった (Issue #311)。同一 seed/clock/入力で 2 回実行の trace が bit 同一であることを scenarios/faultmatrix テストで検証し、genAckStorm/genAttachStorm の seeded fault でも terminal 保存・ghost command 非発生を oracle 照合で確認した (Issue #311)。

T4 では `cargo test -p ruviform-ui --test ipc_conformance` 4 tests、`mise run test-ui-webview` 8 tests、`test-ui-guitest` 135 tests、fmt-check/check/lint（clippy -D warnings）/workspace test が全て PASS だった（Rust 1.99.0・Node 26.12.1）(Issue #311)。`.github/workflows/ci.yml` の `ui-verify` job への配線はローカル検証のみで GitHub Actions 実実行は未確認と正直に記録された (Issue #311)。

## Issue #465: legacy WebView 資産の棚卸し

Issue #465 では `evidence/rf465/inventory.json`（52 entries、分類内訳 ruviform remove/reuse/provisional/exception = 10/16/11/2、AL 1/1/5、SV 2/1/3）を作成し構文・集計検証 pass した (Issue #465)。SV は workflow 内 webview 参照 0 件（mise task のみ残存）だが AL は `webview-verify`/`webview-verify-macos` job で CI 実行中 — consumer 側の「撤去しやすさ」が非対称であることを台帳の retirement_trigger/notes に反映した (Issue #465)。

## Issue #125: ICC LUT fixture と bit-exact oracle

`lut_intent_pair`/`lut_mft1`/`lut_mft2`/`lut_mab`/`lut_a2b0_only` の 5 合成 fixture を生成し、intent 選択差・mft1/mft2/mAB の A2B/B2A 両方向 transform が raw executor oracle に bit-exact 一致、A2B0-only profile が RelCol で `InvalidValue("icc transform")` 拒否・Perceptual 受理を確認した (Issue #125)。fmt-check/check/lint/test/test-video-proc-icc/lint-video-proc-icc 全て exit 0、`.gitattributes` の `*.icc binary` も確認済みだった (Issue #125)。

## 記録の範囲と限界

本記録には、実機検証が NotRun または deferred と明示された項目が複数ある。Issue #243 は複数 slice（window_lifecycle_probe・windows-multiwindow-smoke・window-job-headless）で Issue Open 継続と記録されており、macOS/Linux・動的 DPI・Native Dock・consumer 非回帰は別 Gate とされた (Issue #243)。Issue #311 T4 の `.github/workflows/ci.yml` `ui-verify` job はローカル検証のみで GitHub Actions 実実行は未確認だった (Issue #311)。Issue #431 タスクは T12（commit/push/PR）が pending のままと記録された (Issue #431)。Issue #262 の verdict 決裁宣言・Issue 投稿・ADR status 変更は Coordinator 権限として未実施だった (Issue #262)。Issue #465 の `gh api search/code` は `incomplete_results` で consumer 列挙が checkout 正本依存 — GitHub code search 復旧後の再列挙で差分が出る可能性が残ると記録された (Issue #465)。

Issue #532 の compositor 内部遮蔽判別・方式 A/A'/B/C 採否は全 slice を通じて未解決のままだった（exit-127 原因・topmost=false 効果・実 plugin 画素は実 OS 再現待ち）。compositor/presentation 内部の第二境界（hit-test が自 child HWND に解決しても screen green=0）の残余機構は未断定のまま — settle 付き topmost 再観測・uncovered baseline 環境が残課題として open と記録された (Issue #532)。Issue #631 の AC4 残 field（GPU 情報）は windows-sys feature 追加か GPUI renderer-info API が必要で missing_issue 候補のままだった (Issue #631)。Issue #631 `test-ui-plugin-dock-lab` の Run all→Quit dogfooding は operator 実行の残件（programmatic click 経路なし）だった (Issue #631)。AudioLens/Synvelis consumer 側の実 test lane 実行は各 repo の ruviform pin 更新後の作業であり、本記録群ではソース確認または pinned revision 評価による代替証拠に留まると記録された (Issue #436, Issue #442)。

その他の NotRun・deferred 項目として、Issue #181 の実機検証は実施範囲のみで未検証 OS・異 DPI・モニター切断は `unverified` 明示、Issue #182 の `WindowConfig.width/height` logical vs snapshot physical px の scale_factor 換算は consumer 責務、Issue #461 の実機 paint 計測は NotRun（headless で全経路を通すため O(n²) 回帰は検知可能と判断）、Issue #388 の clip 検証は slot が window 端まで伸びるため検証不能で NotRun、Issue #389 の keyboard/IME=NotRun・Dock PASS=NotRun（RF387 未接続）・L1/CLAP/AU/他 OS=Unsupported・他社 VST3 広範互換=NotRun、Issue #414 の macOS/Linux 実機は NotRun、Issue #423 の WebView2 上の end-to-end と実 OS lane は NotRun、Issue #426 の window pixel scale 1.0・実窓 capture の CI 化・real-os lane は NotRun/blocked、Issue #436 の AudioLens 側実 consumer test は pin 更新後の作業として NotRun、Issue #442 の consumer repo の実 test lane 実行は pin 更新後の作業として残る、Issue #461 の実機 paint 計測は NotRun、Issue #480 の `control_focus`（#479 FU-c）解放は範囲外として deferred、Issue #552 の preflight 不通の実環境再現は自然発生待ちで強制障害注入は行わない、と記録された。

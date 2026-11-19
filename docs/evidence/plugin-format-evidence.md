---
id: "EVI-EeeL01jjqjvnxrnqkhmSiO"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "CLAP・VST3・OFX 規格アダプタの検証記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/28", "https://github.com/scarecrow173/ruviform/issues/29", "https://github.com/scarecrow173/ruviform/issues/30", "https://github.com/scarecrow173/ruviform/issues/31", "https://github.com/scarecrow173/ruviform/issues/32", "https://github.com/scarecrow173/ruviform/issues/34", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/52", "https://github.com/scarecrow173/ruviform/issues/56", "https://github.com/scarecrow173/ruviform/issues/57", "https://github.com/scarecrow173/ruviform/issues/98", "https://github.com/scarecrow173/ruviform/issues/104", "https://github.com/scarecrow173/ruviform/issues/124", "https://github.com/scarecrow173/ruviform/issues/131", "https://github.com/scarecrow173/ruviform/issues/184", "https://github.com/scarecrow173/ruviform/issues/186", "https://github.com/scarecrow173/ruviform/issues/630", "https://github.com/scarecrow173/ruviform/issues/631", "https://github.com/scarecrow173/ruviform/issues/726", "https://github.com/scarecrow173/ruviform/issues/732", "https://github.com/scarecrow173/ruviform/issues/759"]
  informs: []
  explains: []
  supersedes: []
---

# CLAP・VST3・OFX 規格アダプタの検証記録

本書は、CLAP・VST3・OFX の各規格アダプタについて実施された検証と実行の結果を記録する。記録の対象は、後に削除された過去の実装記録であり、本書はそれらの記録に依存せず単独で読めるように構成した。各記述には出典の Issue 番号を付記し、計測値や観測動作として確認された事実と、そこから導かれた解釈を区別して記す。

## 規格アダプタの責務分担と共通契約

plugin-core が規格中立の処理契約を所有し、アダプタは実装・駆動側に位置する一方向性が、spec・plan・task・IR の全層で固定された (Issue #29)。PluginProcessor 相当の prepare/process 形状は plugin-core が所有し、CLAP/VST3 アダプタは契約を実装・駆動する側で契約自体を定義しない。将来の RtContext 等 capability 注入を破壊的変更なく吸収するため、process の context 引数は opaque/予約型とし、現段階で ruviform-rt 依存は付けない決定が記録されている (Issue #29)。

借用 planar view は audio-core の planar モジュールに配置された（fallback は plugin-core）。独立 per-channel planar バッファのゼロコピー借用 view は plugin ABI 境界固有ではなく device/host 境界にも現れる汎用音声形状と判断したためである。S: Sample ジェネリックで f64 経路予約を閉塞せず、BufferView/BufferViewMut trait impl は orphan rule 適合のため plugin-core 側に置き、audio-core→plugin-core の逆依存を回避した (Issue #29)。

in-place 処理は規格ごとに別方式が採られた。エイリアス検出は per-channel ポインタ対比較（部分エイリアス out[i]==in[j]・出力重複も対象）とし、&[f32] と &mut [f32] の同時構築を構造上禁止した。CLAP は clack の ChannelPair::InPlace 分類を写像する専用分岐、VST3 は衝突チャンネルのみ activate/setupProcessing 時に事前確保する per-instance bounce 領域へ逃がす。static/thread_local 共有スクラッチは禁止である (Issue #29)。

binding 方式は方式 ADR（accepted）で確定した。CLAP は clack 0.2.0 系を exact pin（0.x churn 対策）、VST3 は coupler-rs vst3 0.3.0 を採用した。不採用側の退路（clap-sys 直上の自前薄ラッパー / 自前 COM 層）の範囲見積りを併記し、規格単位で方式差異を許容する方針を確認した (Issue #29)。

イベント整列は plugin-core の共有安定ソートに一元化された。CLAP の sort_unstable_by_key（同オフセット順序非保証）と VST3 の sort_by_key（TimSort 系で補助メモリを alloc し得る no-alloc 契約違反リスク）の両方を、plugin-core の sort_events_by_frame_offset（安定・in-place 挿入ソート・alloc なし）へ置き換えた。cargo +1.85 check が両クレートで成功し、方式 ADR の vst3 MSRV 残件を解消した (Issue #29)。

プラグイン分離は「ハンドル+直列化可能コマンド境界」で L0/L1 を分岐する設計-only Issue として記録された (Issue #31)。PluginInstance ハンドルが型で層を隠蔽し、コマンドを直列化可能な形でやり取りする構造制約で、L0 はインプロセス既定、L1 はプロセス分離である。ruviform-plugin-host 未実装のため「新設時の境界として規定」する形で整合確認のみ行い、design-only のため test-spec を skip した (Issue #31)。

第2ターゲットは NLE 編集モデル・OFX 範囲・色管理の3論点を spec/ADR×3/design で確定する design-only チェーンだった (Issue #32)。後続実装 Issue は design「後続 Issue 分解」の4件に委譲された。検証は baseline 分類表との整合（3論点が「決定」へ移行）・plugin-isolation 設計（L0 既定）・video-proc D3（メタデータ搬送規約）との整合・doc-status 監査 findings 0 で実施した (Issue #32)。

中立 param-write 契約は plugin-core、consumer 機構は bridge、規格差分は adapter に触れない構成が ADR Option A として実装された (Issue #186)。plugin-core に ParamWriteMeans（TimedEvent/BlockBoundary/Immediate/PerNote）・ParamWriteRejection（ReadOnly/NotAutomatable/UnsupportedMeans/UnsupportedPrecision/UnknownParam/NonFiniteValue）・check_param_write・opt-in coalesce_param_events_latest を実装した。bridge には TargetId/BindingGeneration/ParamBinding/TargetResolution/ParamTargetTable と DispatchStage/DropReason/ParamValueObservation/ParamDispatchLedger を追加し、cap.rs に param_event_precision/per_note_param/param_reporting を正直に宣言した（per-note/reporting は未証明のため unsupported/Unavailable）。配送経路（TransportPlan→PluginNode→EventList）は既存不変である (Issue #186)。

Dispatched stage は plugin acknowledgment ではなく plan/block partition からの決定論的投影として実装された（honest observability）。plugin→host 報告配線は当該 Issue の範囲外のため、Reported stage は API 形状のみで現行値は Unavailable である。Immediate/PerNote means は中立語彙に存在しないため UnsupportedMeans で reject し、無言の降格は行わない。STEPPED param への timed write は受理し、量子化は consumer の明示ルールに委譲する（silent quantization しない）。汎用 Control Graph クレートは ADR で deferred とされた (Issue #186)。

param-write 契約の検証では、fmt-check/check/lint/test/test-rt/verify-ci が全て PASS した (Issue #186)。param_delivery.rs に AC-3〜7 の E2E/単体9 test（複数時刻×複数対象の順序/offset/value、defined rejection、beat→frame、loop refire、async install、ledger stage）を実装した。rt lane は PluginEndpoint::process を assert_no_alloc 測定窓で検証し、CountPlugin は atomic のみ使用した。bridge 19 test を含む全クレートが green である (Issue #186)。

## CLAP アダプタの実装と検証

CLAP PoC では、clack 0.2.0 で ChannelPair の per-channel スライスを PlanarBufferView に直接ラップし、clack-host ハーネスでポインタ一致を実証した (Issue #29)。アダプタ記述 unsafe は 0（エクスポートマクロ生成部のみ内部 allow）、process 経路 alloc は BlockScratch を activate 時 with_capacity 事前確保で 0、ビルドに clang/cmake/SDK 不要、rust_version 1.85 適合・MIT OR Apache-2.0・依存木 15 パッケージを確認した。cargo test -p ruviform-plugin-clap は Windows x64 で 7 単体 + 3 doctest = 10 合格（clack-host 駆動の実 process 経路を含む）、clippy -D warnings clean、.dll バイト走査で clap_entry シンボルを確認した (Issue #29)。

T3 時点で adapt.rs/plugin.rs に mod tests がなく dev-dep 宣言済みの clack-host も未使用だったため、T6 で adapt::tests 19 件・plugin::tests 14 件・clack-host 駆動 ABI エンドツーエンド 2 件を追加した (Issue #29)。cargo test は plugin-clap 40 単体 + 3 doctest・plugin-vst3 18 単体を含め全パス、clippy 0 findings、cargo +1.85 check --all-targets 両クレート成功で ADR の vst3 MSRV 残件を解消した。公開面監査で CLAP は GAIN_PARAM/GainProcessor/version()/clap_entry のみ、PE エクスポート実測で clap_entry/GetPluginFactory/InitDll/ExitDll を確認した (Issue #29)。

CLAP の state.load 契約違反が test_stateful の検証で発見された。state.load が param 値を変えるなら clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES) が必須だが、当初の実装は値変化後に rescan を要求していなかった。PluginStateImpl が host handle を保持し、load 成功時に rescan を要求する形に修正した（host 不在環境では skip）。validator/host が state load 後の値変化を認識するための必須手順として記録されている (Issue #630)。

CLAP editor は MainThread object に PluginGuiImpl を実装し、&self verb を RefCell<GuiState>+try_borrow_mut/try_borrow で処理する。borrow 失敗は panic ではなく defined rejection に収束する（fixture の reentrancy policy）。set_parent は API-type+create-ordering+double-attach+IsWindow gate を必須化した（host bridge が ParentLease::payload() を HWND として無検証で渡すため、opaque payload が HWND と誤認されるのを防ぐ）。pass_through は fault.rs から pub(crate) 共有で DSP law 重複を排除し、windows-sys は target.'cfg(windows)' dep のみ・clack-extensions gui feature は spec 承認済みである (Issue #630)。

CLAP editor の resize は resize-capable を採用した。fixed-size decline 維持では set_size success 経路を原理的に coverage できないためで、can_resize=true・set_size が created gate+size 記録+win32::resize で SetWindowPos SWP_NOMOVE|SWP_NOZORDER を行う。visibility oracle は hidden parent で IsWindowVisible が祖先連鎖不成立のため、GetWindowLongW(GWL_STYLE)&WS_VISIBLE を child 自身に適用する。destroy で size を DEFAULT_GUI_SIZE に戻し（VST3 の per-createView fresh state と同 semantics）、set_size before create は defined rejection（protocol violation）とした (Issue #630)。

CLAP editor の検証では、非 ignore 層2件（editor no-gui refusal・non-editor 4 class refusal）と Windows #[ignore] 実機層1件を追加し、interactive window station で実走 pass を確認した (Issue #630)。実機層は open_gui→GW_CHILD/GetParent/IsWindow→GetWindowRect 800x600 resize→GWL_STYLE&WS_VISIBLE hide/show→set_gui_parent PluginFailed→close 破棄→double-close Unsupported→audio 継続を実測した。cargo test --locked -p ruviform-plugin-test-clap は 57 passed/1 ignored/0 failed、--ignored 明示実行で set_parent_creates_real_child_hwnd が実 WS_CHILD 生成・GetParent 関係・IsWindow 生存・二重 attach 拒否・DestroyWindow teardown を確認した。check/lint/test-l1-real-plugin（adapter e2e、test-plugin e2e、fault 8+8、stateful 2+2）/test-rt/prek 全 pass、production crate 無変更である (Issue #630)。

clack 0.2.0/clap-sys 0.5.0 の生存期間契約（init 1回制限・activate/deactivate の processor cell・destroy 時自動 deactivate・不正状態 callback 拒否）を確認し、plugin.rs/adapt.rs/host.rs を監査した (Issue #732)。rf#722 の3パターン照合で terminalization/bus-info は欠陥なしと判定したが、受理層で2欠陥を発見し TDD で修正した。plugin.rs の module: b"/"→b""（gain_param_info() 集約 + unit test）、adapt.rs to_process_config の fractional sample rate reject → rate.round() 受理（fixture 同一写像）+ unit test 更新である。run.mjs に --clap-production/--clap-production-dll を追加し fail-closed を維持した (Issue #732)。

lint lane の修復では、mise run lint ruviform-plugin-clap・lint ruviform-plugin-test-clap（先行 slice で記録された破損 lane）・check/test 両 crate・fmt-check・test-l1-real-plugin（host feature の --lib --bins 段で新規 unit test を含む canonical 実行、#[ignore] E2E 段も全 pass）・prek run --all-files が全て exit 0 となった (Issue #759)。host.rs の width/height as u32 に対し cast_possible_truncation と cast_sign_loss が各1件ずつ報告され計4 events・exit 101 で、workspace の clippy::pedantic=warn と lint task の -D warnings の組合せで error 化する構成だった。clamp で実際に安全化しても clippy は値域を追跡せず両 lint を発火し続けるため、局所 #[expect] が不可避と確認された (Issue #759)。飽和境界契約は unit test gui_size_component_saturates_at_pixel_bounds で固定した（800.4→800 / 800.5→801（round）、-3.0/NaN/-inf→0、+inf/1e30/u32::MAX+0.4→u32::MAX）。canonical lane は mise run test-l1-real-plugin の --lib --bins 段が担う (Issue #759)。

## VST3 アダプタの実装と検証

VST3 PoC では、vst3 0.3.0 の生成 trait が全メソッド unsafe fn・レシーバ &self のため、アダプタ側 unsafe はシグネチャ義務 54 + 局所 unsafe ブロック約51（テスト含む）に留まり、全て SAFETY コメント+#[expect] でゲートした (Issue #29)。COM の &self 制約は RefCell（try_borrow_mut で再入を kResultFalse の定義済み拒否）+AtomicU32/AtomicU64 で解決し、panic→FFI 越え unwind（ホスト abort）を回避した。kSample64 は f32 再解釈せず拒否し、VST3 SDK ヘッダはビルド時不要と実測した。cargo test -p ruviform-plugin-vst3 は pinned 1.99.0 で 17 単体テスト全合格（gain/イベント・ブロック検証・bounce/エイリアス・ライフサイクル・状態外拒否・in-place・ゼロコピー直書き）、clippy -D warnings clean である。PE エクスポートディレクトリ解析（nm/dumpbin 非搭載のため Python 代替）で GetPluginFactory/InitDll/ExitDll の3シンボル実在を確認した (Issue #29)。

公開面監査で VST3 は公開項目ゼロであることを確認した。vst3 の rust_version 未宣言で MSRV 1.85 単独ビルドは未実測、macOS/Linux エントリは cfg コードのみでクロスビルド未実測であり、残件として vst3 生成バインディング SDK 出自のライセンス継続調査が記録されている (Issue #29)。

adapter-state-bridge の VST3 側は、BridgeStream（IBStream ComWrapper: RefCell state・部分転送を実数報告・負/overflow seek は kResultFalse・前方 seek の gap は 0 埋め・WIRE_MAX_STATE_BYTES cap）+[u32 LE comp_len][comp][u32 LE ctrl_len][ctrl] framing+spec 順の component/controller 呼出列で構成される (Issue #630)。foreign blob 規則として、8 byte 未満は本 adapter の serialization であり得ないため plugin の setState へそのまま委譲する（厳密 unframe 先行だと L1 e2e の RestoreState{[0xAA,0xBB]} が PluginFailed となり Unsupported oracle を破る）。kNotImplemented→Unsupported・その他→PluginFailed の error mapping とした。fixture の不正直 stub（kResultOk を返す空 no-op）は kNotImplemented に修正し test で pin した。stateless fixture は production bridge 経由で SaveState/RestoreState が wire 越しに Err(Unsupported) を維持することを L1 e2e で確認し、in-process COM double の成功を real-plugin Pass と読み替えない境界を明記した (Issue #630)。

VST3 editor では、isPlatformTypeSupported は HWND には kResultTrue を返す。host open_gui が isPlatformTypeSupported の kResultOk を unsupported と誤読するためである。createView("editor") は ComWrapper::to_com_ptr().into_raw() で owned IPlugView を返す。window class の lpfnWndProc に module-local 関数を使うと cdylib unload 後に code pointer が dangling して host crash するため、DefWindowProcW（user32・unload されない）+ class brush の erase fill に変更した（review B1 P1）。hInstance は GetModuleHandleExW(FROM_ADDRESS) で自 module を取得する（GetModuleHandleW(NULL) は host EXE を返す）。cstring_eq は design の 1024 byte bound を byte 単位 bounded scan で実装し、attached の parent 検証は IsWindow のみ（opaque ParentLease payload を拒否）とした (Issue #630)。

VST3 fixture の検証では、factory 6 class/3 descriptor に対し cargo test 60 unit+10 host_path pass（editor zero-params、pass-through、opaque-lease open_gui rejection=Windows PluginFailed/非 Windows Unsupported）を確認した (Issue #630)。#[ignore]d attached_creates_real_child_hwnd を Windows session で手動実走し、GetWindow(GW_CHILD)/GetParent/IsWindow で実測した。test-l1-real-plugin で adapter 3+3/test-plugin 3+3/fault 8+8 pass（6-class 化後も runner の plugin_id dispatch に影響なし）である (Issue #630)。

VST3 preset interface の帰属は vendored binding で確定した（controller: IUnitInfo/IUnitHandler/IUnitHandler2、component: IProgramListData/IUnitData）(Issue #630)。test_spawn_fail は scan 通過・create 拒否（createInstance 非 dispatch kInvalidArgument）で、decline 証拠は VST3_DECLARED_UNCREATABLE で rawOutput の decline marker 出現回数を記録し occurrences=0 は Failed に倒す fail-closed とした。slice ごとの factory 規模は instrument で 10 class（5 plugin × comp/ctrl、instrument pair は InputSurface::MidiTransport）、stateful で 8 class（gain/fault/editor/stateful × comp/ctrl）と記録されている (Issue #630)。

## OFX アダプタの実装と検証

OFX では、pinned OpenFX SDK（commit c3f1c2dc、ofxInteract.h 等の欠落推移ヘッダと LICENSE も同 pin で byte-for-byte 取得・sha256 manifest 記録）を vendor した (Issue #98)。C bridge が SDK layout/suite table/variadic/OfxHost/typed accessor を所有し、Rust は fixed-arity primitive/pointer のみを扱う。unsafe は FFI モジュールに局在（crate-wide allow なし）、公開 API に外部 crate エラー/SDK 型を出さない。admission は OfxSetHost→GetNumber/GetPlugin→per-plugin setHost/main→Load→Describe/DescribeInContext→Create の順で、admitted index≠native index・SingleInstance を CreateInstance 前に検査・destroy 高々1回 terminal・quarantine で leak-not-UAF とした (Issue #98)。

render は preflight（state+Rgba8+dims・plane_mut 一意）→単一 ClipPrefs helper を activation と各 render で無条件実行（BeginSequenceRender 前）→Begin/Render/End bracket の構成である (Issue #98)。staging は host memory pool 非依存の call-scoped 確保で bottom-left 露出（最終物理行 pointer+負 checked-i32 rowBytes）。Begin/End の ReplyDefault は SDK の optional action 規約で合法成功、Render は strict とした。全成功時のみ staging→dst commit し、失敗時は pixel/meta/padding 不変である。t = at.to_seconds_f64()*num/den（小値ケースのみ保証）とした (Issue #98)。

T1 の core contract 実装では、mise run test-plugin-core exit 0（32 unit + 8 integration + 21 doctest + 2 compile_fail）、mise run lint-plugin-core exit 0 を確認した (Issue #98)。red 記録は新公開 API 不在の E0432 等を正当な red とし、既 green なら人工欠陥を作らない方針とした。doc compile-fail は green 後に real borrow error を示す（unresolved symbol ではない）ことを確認した (Issue #98)。

T2 の native runtime 実装では、mise run test-ofx exit 0（27 native_host + bridge_min + bridge_native 全 pass）、lint-ofx exit 0（clippy -D warnings clean）、test-ofx-msrv exit 0（MSRV 1.89 で lib+tests+C bridge コンパイル・native 27/27 pass）を確認した (Issue #98)。T3 の render 実装では、native_host 36 + bridge_min + bridge_native 全 38 pass、render 9 tests が bracket の frame/time 写像・native entry 前 reject・clip-prefs 再検証・float depth offer 回復・Begin/Render/End 各失敗時の output+padding sentinel 不変・ReplyDefault 合法性・bracket 後 destroy/release 整合を固定した。2×2 orientation oracle・Source SDK corners・負 Source+Output rowBytes assert・PlainParamValue::Double(0.5) gain 画素反映を含む (Issue #98)。

Full Gate では fmt-check/check/lint/test（49 test binaries 全 pass）/test-rt（rt_ filter で 94 tests matched・0 failed、0 件スキップを成功と偽らない）/bench-smoke/prek/doc audit（0 findings）を1回のみ実行し全 log を保存した (Issue #98)。gate 前提の fmt-check green のため、branch 編集由来の rustfmt drift（plugin-ofx 配下7ファイルのみ）を mise run fmt で正規化し crate 外波及ゼロを確認した。mac/Linux/CI matrix は実施せず pass も主張しない (Issue #98)。

独立2レビュアーが Task 毎に P0/P1 級の実害を検出した (Issue #98)。Task 2 では rv_ofx_instance_create 失敗パスの g_mu lock leak（全テストハングの根本原因）・rv_tok_revoke cascade の iterator UAF・rv_po_set 整数 overflow OOB write・rv_mem_free の stale handle UAF 等を、Task 3 では Begin/End ReplyDefault 誤失敗扱い・staging allocator pairing 違反（free vs _aligned_malloc）・configure overflow 未検査等 F1–F12 を検出し全修正した。記録された教訓として、foreign ABI 境界では token revoke 順序・lock 対称性・allocator pairing がレビュー重点とされた (Issue #98)。

window 中 teardown の UAF は、foreign ポインタ寿命と post-window deref を別問題として二分離して解決した (Issue #124)。(1) foreign 側へ渡る OfxHost を静的 g_api へ分離（reclaim 対象外・plugin が window を超えて保持しても fetchSuite は defined degrade）し、(2) RvHost.foreign lease counter で window span のみ保護（無条件 leak にしない）した。host_free は poison-immediate/reclaim-deferred に分離し foreign>0 で reclaim 延期、全 post-window アクセスを !h->alive の defined bail に統一した。dead-host 時は detached partial catalog を rv_ofx_catalog_close で in-place teardown し *catalog=NULL とした（host pool 死滅後の dangling string を caller に escape させない）(Issue #124)。

hostile callback 検証では、window 中 teardown を真の別スレッド競合ではなく同一スレッド再入の hostile callback（fixture observer が実 action 内で rv_ofx_host_free を発火）で固定した。検証対象は「lease 保有中に host_free が走る」コード経路であり同一で、真の MT race は facade の !Send 契約外のため追加 MT ハーネスは作らない。bridge_host_lease.rs に hostile callback テストを追加し、window 中 rv_ofx_host_free を走らせ kOfxStatErrBadHandle・*catalog==NULL・二度目 free no-op・新 session 成功を assert、api.host poison NULL 化・g_api 分離を固定した。FX_PROBE_CLIP_PROPS probe で instance clip property set が descriptor 宣言 kOfxImageEffectPropSupportedComponents=RGBA（Source/Output 両方）を返すことを確認した。mise run test-ofx exit 0（bridge_host_lease 4/4・bridge_min・bridge_native・native_host 37/37）、lint-ofx・test-ofx-msrv（rust 1.89）・fmt-check・check・lint・test・test-rt 全て exit 0、cargo doc 警告 0、audit clean である (Issue #124)。

#124 review が記録した residual（非 catalog window の UAF・nested close deadlock・close 再入・host ABA・g_api.host deputy・seed alloc-failure）を、uniform lease で全 foreign-call window へ一般化した (Issue #131)。(b) !Send 明示依存は contract-violation までの拒否で防御にならず、(c) pool refcount は ownership model の大改修で token sweep+lease で同等の到達不能性が得られるため不採用とした。host handle は H_HOST generation token 化で ABA 解消、poison で session 全 token sweep、g_host_props は dedicated process-lifetime once-bind とした (Issue #131)。

g_host_props（once-bind・sweep exempt）が session pool の string を参照する record を抱えたまま process-lifetime で UAF になる問題に対し、rv_po_clear で poison 時点（host_free の sweep 直後）に record を退役させる方式を採用した (Issue #131)。deferred reclaim（host_reclaim）では open lease 中に次 session が新 record を積み、旧 reclaim がそれを巻き込む cross-session 破壊になるためである。option (a)「g_host_props 宛 write 拒否」は OFX 上 host propset への write は合法行動で defined error にしにくく、内部実装詳細の境界漏洩もあるため不採用とした (Issue #131)。

uniform lease の検証では、window_lease mod の6 hostile window テスト（Create/Changed/ClipPrefs/Render/Destroy/Unload+再入 close）+ release 経由 Destroy + host_propset_records_retire_with_session 回帰を含む bridge_host_lease 12/12 green を確認した (Issue #131)。fixture 実 DLL observer が各 action 内で rv_ofx_host_free（+close では再入 close）を発火する same-thread hostile re-entry で6 window を固定し、file-scope GUARD で両 mod を直列化した。test-ofx/lint-ofx/fmt-check/check/lint/test/test-rt 全て green、doc audit 0 blocking。C bridge は MSVC -WX -W4 で警告ゼロ（非 ASCII コメントを一度入れて C4819→C2220 で落ち ASCII へ修正）である。MSVC/非 Windows host は未検証で、catalog handle の token 化と catalog_identifier/descriptor_string の post-reclaim lifetime は scope 外として Deferred 記録とした (Issue #131)。

## テスト fixture と L1 out-of-process 検証

test-only fixture crate（ruviform-plugin-test-vst3/clap）は suite design どおり各 format で独立実装し、DSP-law commonization は明示的に不採用とした（self-verification loop を断つため変換 law は neutral contract のみに依存）。production host path（Vst3Library/ClapLibrary load→ErasedPlugin）経由で検証し adapter 本体は zero-diff を維持した。L1 runner は各 test crate 内に l1-runner feature + [[bin]] required-features + #![cfg] e2e として置き、fixture_lib_name が compile-time 固定・OutOfProcessSpec に env/argv 拡張がないため per-crate runner が clean boundary である (Issue #630)。

adapter-state-bridge の検証では、Windows OS-L1 で fmt-check/check/lint/test（両クレート）/check-l1-real-plugin/lint-l1-real-plugin/test-l1-real-plugin/test-rt/prek 全て exit 0 を確認した (Issue #630)。adapter e2e real_adapter_l1_full_series 3+3 pass、stateless fixture の SaveState/RestoreState は wire 越しに Err(Unsupported) を維持した。補助証拠として vst3 --features host --lib 84 pass、clap 67 pass を記録した。この時点では in-process COM double の成功は real-ABI oracle ではなく、real stateful round-trip は後続の test_stateful slice まで NotRun と記録された (Issue #630)。

fault 注入は in-band armed fault 方式を採った。fault は ParamValue event で arm し process 内で dispatch する。FAULT_PARAM は plain [0.0,4.0] default 0.0 の STEPPED param で、decode_fault が floor 帯域（none/abort/exit/hang/error）を判定し exit code = round((plain-2)*100) に符号化する。ParamInfo::denormalize が線形で STEPPED が event path で量子化されないことを検証済みである。armed submit は fire-and-forget で drop され得るため arm_fault を4回・5ms cadence で繰り返し送出した。plugin process error は RESULT_FAILED flag ではなく ProcessResult{status:Error} で観測する（flag は synthesized-failure のみ）— oracle を status 観測+Processing+disarm recovery に修正した教訓が記録されている (Issue #630)。

spawn/pre-Hello fault は専用 bin 案から既存 l1-runner bin の --ruviform-test-fault=MODE（exit/abort/hang/stderr/bad-fixture）へ変更した。fixture load 後・run() 前に消費し run() 本体は無変更で、fail-closed・pre-Hello 到達性同等・fixture resolution 単一化のためである（ADR issue-630-test-spawn-fault-runner-argv に記録）。InstanceLimit 到達 probe は host pooling（Bundled 分散/Dedicated 専用子）で到達不能と判明し削除した。pre-Hello 高速死亡は Transport/ExitObserved/Internal の分類 race があるため PRE_HELLO_DEATH 集合 oracle に整理し、mid-handshake 死亡は Scan/CreateInstance が plugin code を実行しないため注入不可とした (Issue #630)。preset-spawn-fault slice では unit 91+66 pass、mise run test-l1-real-plugin で adapter 3+3/real 10+10/fault 16+16/stateful 2+2 全緑、test-rt/prek pass、PR #735（Refs #630）を記録した。canonical=mise.toml task、cargo 直接実行は補助確認で CI lane 正本ではないと明記した (Issue #630)。

state blob は [u32 LE magic RFST][u32 LE version=1][f64 LE gain] の16 byte とした。VST3 は component=plain gain/controller=normalized gain の domain 分離（plain=normalized*2）、CLAP は plain を無 framing で保存する。test_stateful は activation で mirror を reset しない persistent-state law を採った（test_gain の activation reset を継承すると restore→activate で state が消える）。L1 ordering gate は delivered audio とし、process submit は fire-and-forget で state verb が mutation event block を追い越し得るため、3 gain（1.0/0.75/1.75）が constant input に pairwise-distinct output を生むことを利用した。期待 gain の delivered output 観測はそれ以前の全 block 処理済みの証明となり、state verb をその後にのみ送出する (Issue #630)。

test_stateful の検証では、in-process byte-exact round-trip + L1 e2e（delivered-audio gate 付き）全 pass を確認した。VST3 unit 70/host_path 15、CLAP unit 35/host_path 10 pass である。production Vst3Library/ClapLibrary ABI 経由の save→mutate→save→mutate→save→restore→save が byte-exact、malformed（truncated/trailing/bad-magic/NaN）は PluginFailed+instance survival、stateless class は Unsupported/kNotImplemented 維持である (Issue #630)。

test_instrument では、fixture-owned translator で self-verification loop を断った。VST3 events.rs（IEventList→EventKind::Midi・ProcessContext→TransportInfo）と CLAP block.rs（translate_instrument_event・transport_from_clap）は production adapter と独立実装し、変換 law は neutral contract（MidiValue scale・TransportInfo field）のみに依存する。VST3 は InputSurface enum（ParamsOnly/MidiTransport）で instrument のみ inputEvents/processContext を読み stateless class の process path を不変に保つ。CLAP PPQN は fixture law で 960 固定、VST3 は host projectTimeMusic ppq を 960 tick に正規化する。transport click は playing+musical timing で次 PPQN boundary に 1-sample impulse、tick→frame は i128 ceiling division とした (Issue #630)。

L1 wire の 33byte 固定 stride vs 可変長 event の desync は record codec で修正した。event slot は固定 EVENT_WIRE_SIZE=33 stride だが encode_event は可変長 record（MIDI=28 byte）を出力するため、parent が 33-byte stride で staging すると stale tail（debug_assert 発火）・child が packed stream として decode する desync が判明した（MIDI over L1 は未検証だった）。wire contract owner の plugin-core/src/wire.rs に encode_event_record/decode_event_record（zero-pad・nonzero padding reject）を追加し host staging/export decode を移行した。CLAP raw MIDI は d1/d2 の 7-bit 検証を追加（>0x7F を defined skip、VST3 sibling と統一）。Match::All channel は channel 0 に捏造せず defined skip とした。L1 transport oracle は phase 間で異なる click 位置（position_ticks=950→click@250）を使い pass_through last_output replay の false-pass を排除した (Issue #630)。

fresh-context subagent review で3件の CONFIRMED finding を検出・修正した。(1) CLAP translate_raw_midi が >0x7F byte を MidiValue::from_u7 wrap で canonical 値化、(2) Match::All channel が unwrap_or(0) で channel 0 に捏造、(3) L1 transport oracle phase-3 が phase-1 と同一 click@100 で pass_through replay false-pass の3件である。修正後 l1-runner feature 付き clippy -D warnings pass、両 l1 e2e 7/7（instrument 4 含む）、CLAP unit 51+host_path 11+plugin-clap 48+3 pass、fmt/doc lint clean である (Issue #630)。

test_instrument の総合検証では、VST3 は 10 class（5 plugin × comp/ctrl）、CLAP custom factory は 4 plugin を expose し、production Vst3Library/ClapLibrary ABI 経由の in-process note lifecycle（retrigger -1.0→single NoteOff→release silence）と L1 runner 経由の 4 instrument case（lifecycle/release/CC+offset/transport click+seek+loop）が実走・pass した。fmt-check/check/lint/test/check-l1/lint-l1/test-l1/test-rt/prek 全 pass が Done When である (Issue #630)。

VST3 editor fixture では factory 6 class/3 descriptor に対し cargo test 60 unit+10 host_path pass（editor zero-params、pass-through、opaque-lease open_gui rejection=Windows PluginFailed/非 Windows Unsupported）を確認した (Issue #630)。#[ignore]d attached_creates_real_child_hwnd を Windows session で手動実走し GetWindow(GW_CHILD)/GetParent/IsWindow で実測し、test-l1-real-plugin で adapter 3+3/test-plugin 3+3/fault 8+8 pass（6-class 化後も runner の plugin_id dispatch に影響なし）である (Issue #630)。

CLAP editor fixture では cargo test --locked -p ruviform-plugin-test-clap で 57 pass+1 ignored、--ignored 明示実行で set_parent_creates_real_child_hwnd が実 WS_CHILD 生成・GetParent 関係・IsWindow 生存・DestroyWindow teardown を確認した。tests/host_path.rs は 15 passed（opaque lease 拒否・非 editor open_gui→Unsupported 等）、L1 lanes・test-rt・prek run --all-files も全て pass、production crate は一切変更していない。clack-host 0.2.0 の in-crate テストハーネスが実際の clap_entry vtbl 経由でプラグインをインスタンス化し、production ClapLibrary と同一 ABI 経路を通ることを確認した (Issue #630)。

L1 runner の検証では、mise run test-l1-real-plugin exit 0 で adapter e2e 3+3 と新規 test-fixture e2e 3+3 が real child-process spawn で pass した (Issue #630)。test-fixture e2e は handshake・32-block gain-0.5 process・Unsupported state/GUI verbs・kill→Failed+1 notification・missing runner→SpawnFailed を含む。VST3 descriptor oracle は literal vst3::uid(...) から test 時導出+platform byte-order literal pin、CLAP は literal descriptor pin で oracle 独立性を確保した。EXPECTED_GAIN=0.5（normalized 0.25×plain [0,2]）は非恒等で normalized↔plain confusion を検出可能である。default cargo test では e2e target が 0 test に compile されることを確認した（feature gate）(Issue #630)。

test-l1-real-plugin の instrument 系 e2e（l1_real_test_plugin_e2e の note_off_releases_to_silence/transport_clicks）が高負荷時に frame-exact assertion で断続的 fail を観測した。失敗 test は実行ごとに変わり最終実行は両 crate e2e 10/10 全緑で、crates/ は当該 slice で zero-diff のため既存 lane の flaky として正直に記録した (Issue #630)。

## 外部 ABI validator の実行記録

外部 validator として clap-validator 0.4.1（release binary・sha256 を release page 記載値と照合）と VST3 SDK validator（SDK commit 3cdf9ca を CMake+MSVC で build）を ci/plugin-abi-validator/run.mjs（fail-closed・verdict は canonical leg のみ集約）で実走した (Issue #630)。初回で CLAP の param module path/fractional rate/MAX_EVENTS/instrument note/stateful rescan と VST3 の terminate 非終端/setupProcessing state/bus info の違反を検出し、fixture crate 内で TDD 修正した（production adapter zero-diff）。clap-validator 自体が param-fuzz-sample-accurate で panic（exit 0xc0000409）する tool 側 bug を特定し、宣言的除外+informational leg で対処した。最終 verdict Passed・VST3 235 passed/0 failed・CLAP 5 plugin 全 leg 0 failed・contract test 18 pass である (Issue #630)。

初回実走の詳細では、clap-validator 0.4.1 は 5 plugin 全て 0 failed（test_gain 26/17skip/1warn、test_fault 19/17/1〔param 除外〕、test_stateful 31/12/1、test_instrument 30/14/0、test_editor 17/26/1）、vst3-sdk-validator（VST 3.8.1）は 235 passed 0 failed である。test_fault unfiltered informational leg は validator exit 101（tool 側 panic param-fuzz-sample-accurate）で Unsupported 記録とした。warning は全 plugin で denormal 処理性能の情報通知（非失敗）である。VST3 は staging layout（Contents/x86_64-win/・inner=outer 名）整備+契約修正で 235 passed, 0 failed となった (Issue #630)。

意図的 create-decline は宣言的機構で fail-closed を維持した。test_fault param 除外と同型の宣言的機構として、CLAP は per-plugin expectCreateDecline（pattern 一致 declined/不一致 unexpected・crashed==0 && declined>0 && unexpected==0 で Passed）、library-level は CLAP_FIXTURE_LIBRARY_EXCLUDES+clapLegExclusions で target/plugin 両 level 合成、VST3 は VST3_DECLARED_UNCREATABLE で rawOutput の decline marker 出現回数を記録し occurrences=0 は Failed に倒す fail-closed とした。run.json schema を /2 に bump した (Issue #630)。

ABI validator 再実走では、mise run test-plugin-abi-validator で verdict Passed exit 0、canonical 7 leg 全 Passed を確認した (Issue #630)。clap test_spawn_fail は declared expectation met（35 declined/6 passed/1 skipped、validator exit 1）、vst3 は 235 passed/0 failed で test_spawn_fail decline 証拠 47 occurrences、contract test 32 pass である。informational unfiltered fault leg は Unsupported（exit 101 tool panic）。非回帰一式（fmt/check/lint/test 両 crate/L1 lane/test-rt/prek）全 exit 0 である (Issue #630)。

clap-validator 0.4.1 の preset-discovery crawl/load は45秒 timeout+JSON 無し終了する deadlock が観測された。upstream free-audio/clap-validator issue #41 の MetadataReceiver self-deadlock で、0.4.1 pin 以降の master で修正済みだが release >0.4.1 は未発行である。orphan 化した sandbox child が DLL をロックする副作用も確認した (Issue #630)。preset-discovery-reeval では upstream commit 47e2424（PR#41 merge）から build した binary を %LOCALAPPDATA%/ruviform/plugin-abi-validator/pin-47e2424/ に provision し SHA-256 記録で供給した。Cargo dependency/commit 済み binary にはしない決定を ADR に記録し、除外を解除して verdict Passed を再確認した。CLAP_FIXTURE_LIBRARY_EXCLUDES を削除し fixture target を excludes: [] に変更し、CLAP_VALIDATOR=pin-47e2424 VST3_VALIDATOR=vst3sdk-build mise run test-plugin-abi-validator で verdict Passed exit 0、canonical 7 leg 全 Passed、preset-discovery crawl/load が raw output で success として観測され run.json の fixture CLAP leg command に preset-discovery exclude は無い。node --test 32 pass、非回帰一式全 exit 0 である (Issue #630)。

clap-validator の param-fuzz panic の根因は連鎖として特定された (Issue #726)。param-fuzz-basic/param-fuzz-modulation が単独 panic、param-fuzz-sample-accurate は clean crashed 記録と確定した（--include/--exclude filter で絞り込み）。根因連鎖は、fixture exit(0) mode（stepped 2.0）→ sandboxed child success+無出力 → sandbox.rs:98 .expect() panic → cli.rs:55 scoped thread panic → exit 101・JSON 未出力である。upstream checkout master 5bc2ef3 で未修正を確認し、issue list に同件なし・#41/#42 は master 修正済みだが 0.4.1 未収録と整合した。報告ドラフトを記録し、実起票は Coordinator hand-off とした (Issue #726)。

## 記録の範囲と限界

以下は記録上 NotRun・deferred・未検証とされた項目である。

- VST3 は rust_version 未宣言のため MSRV 1.85 単独ビルド未実測、macOS/Linux エントリは cfg コードのみでクロスビルド未実測、生成バインディング SDK 出自のライセンス継続調査が残件である (Issue #29)。
- plugin 基盤の総合検証で mise run bench はベンチ未実装で実質 no-op であり、性能回帰検出は当該検証に含まれなかった (Issue #29)。
- OFX は mac/Linux/CI matrix を実施せず pass も主張しない。MSVC/非 Windows host は未検証である (Issue #98, #131)。
- catalog handle の token 化と catalog_identifier/descriptor_string の post-reclaim lifetime は scope 外として Deferred 記録とした (Issue #131)。
- 実親 HWND+メッセージポンプ経由の open_gui 成功パスは NotRun のまま Issue #631 へ deferred された。Windows パスでの実 WS_CHILD HWND 生成/破棄までは検証済みである (Issue #630)。
- L1 OpenGui/CloseGui は HostError::Unsupported by design、非 Windows runtime 未検証、set_transient/suggest_title は #389 D7 deferred である (Issue #630)。
- adapter-state-bridge は real stateful round-trip の実証が test_stateful fixture slice まで NotRun であり、in-process COM double の成功は real-ABI oracle ではないと明記された (Issue #630)。
- CLAP test_gain は L0 in-process 検証のみで、L1 out-of-process runner・外部 SDK validator・非 Windows lane は後続 slice まで NotRun だった。test_stateful は production host bridge の save_state/restore_state 欠如で blocked であり adapter-state-bridge slice で解消された (Issue #630)。
- L1 evidence は Windows OS-L1 のみで macOS/Linux lane は follow-up とされた (Issue #630)。
- InstanceLimit 到達 probe は host pooling により到達不能で test 削除、mid-handshake 死亡は Scan/CreateInstance が plugin code を実行しないため注入不可である (Issue #630)。
- clap-validator の param-fuzz panic の upstream への実起票は Coordinator hand-off として未実施である (Issue #726)。
- test-l1-real-plugin の instrument 系 e2e の高負荷時断続的 fail は、失敗 test が実行ごとに変わり crates/ zero-diff のため既存 lane の flaky として記録されたが、原因は未分離である (Issue #630)。

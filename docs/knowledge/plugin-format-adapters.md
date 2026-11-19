---
id: "KNOW-qsIqMBUXBhi9tBKUOmG568"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "codebase"
title: "CLAP・VST3・OFX 規格アダプタの設計"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/18", "https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/22", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/28", "https://github.com/scarecrow173/ruviform/issues/29", "https://github.com/scarecrow173/ruviform/issues/30", "https://github.com/scarecrow173/ruviform/issues/31", "https://github.com/scarecrow173/ruviform/issues/32", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/52", "https://github.com/scarecrow173/ruviform/issues/54", "https://github.com/scarecrow173/ruviform/issues/55", "https://github.com/scarecrow173/ruviform/issues/98", "https://github.com/scarecrow173/ruviform/issues/104", "https://github.com/scarecrow173/ruviform/issues/124", "https://github.com/scarecrow173/ruviform/issues/131", "https://github.com/scarecrow173/ruviform/issues/184", "https://github.com/scarecrow173/ruviform/issues/186", "https://github.com/scarecrow173/ruviform/issues/195", "https://github.com/scarecrow173/ruviform/issues/630", "https://github.com/scarecrow173/ruviform/issues/722", "https://github.com/scarecrow173/ruviform/issues/726", "https://github.com/scarecrow173/ruviform/issues/759"]
  informs: []
  explains: []
  supersedes: []
---

# CLAP・VST3・OFX 規格アダプタの設計

本書は、CLAP・VST3・OFX などのプラグイン規格アダプタを設計・実装する際に再利用できる知見をまとめたものである。内容は当時の実装記録（後に削除済み）から抽出したものであり、本書はそれらの文書に依存せず単独で読めることを前提とする。事実は各記述の末尾に `(Issue #番号)` を付けて帰属を示す。記録上で観測・実測として記録された値と、設計判断として記録された決定を区別して記述する。

## 規格中立契約とアダプタの責務分界

規格アダプタの設計は、規格中立の契約を plugin-core が所有し、各アダプタはその契約を実装・駆動する側に徹する一方向構造で確定した。契約自体をアダプタが定義することはない (Issue #29)。将来の RtContext 等の capability 注入を破壊的変更なく吸収するため、process の context 引数は opaque/予約型とし、現段階で ruviform-rt への依存は付けないと決めた (Issue #29)。

イベントの整列は plugin-core の共有実装 `sort_events_by_frame_offset`（安定・in-place・alloc なしの挿入ソート）に一元化した。CLAP 側の `sort_unstable_by_key` は同オフセットの順序を保証せず、VST3 側の `sort_by_key`（TimSort 系）は補助メモリを alloc し得て process 経路の no-alloc 契約に違反し得たため、両方をこの正本へ置き換えた (Issue #29)。

in-place 処理は規格ごとに方式が異なる。エイリアス検出は per-channel のポインタ対比較で行い、部分エイリアス（`out[i]==in[j]`）と出力同士の重複も対象とし、`&[f32]` と `&mut [f32]` の同時構築を構造上禁止する。CLAP は clack の `ChannelPair::InPlace` 分類に対応する専用分岐を持ち、読み側 view を空にして借用の衝突を回避する。VST3 は衝突するチャンネルのみ、activate/setupProcessing 時に確保する per-instance の bounce 領域へ逃がし、非衝突チャンネルはホストバッファへ直接書き込むゼロコピーを維持する。`static`/`thread_local` による共有スクラッチは禁止である (Issue #29)。

借用 planar view（独立した per-channel planar バッファのゼロコピー借用）は、プラグイン ABI 境界固有ではなく device/host 境界にも現れる汎用音声形状と判断し、audio-core 側に配置した。`S: Sample` ジェネリックで f64 経路を予約し、`BufferView`/`BufferViewMut` の trait impl は orphan rule 適合のため plugin-core 側に置き、audio-core→plugin-core の逆依存を回避した (Issue #29)。

## binding ライブラリの選定

CLAP は `clack` 0.2.0 系を `=0.2.0` exact pin で採用し、0.x 系の API churn を抑える。VST3 は coupler-rs の `vst3` 0.3.0 を採用した。規格単位で方式差異を許容する方針であり、不採用側の退路（`clap-sys` 直上の自前薄ラッパー、自前 COM 層）の範囲見積りを ADR に併記した (Issue #29)。

CLAP 側の実測では、clack 0.2.0 で `ChannelPair` の per-channel スライスを `PlanarBufferView` に直接ラップし、ポインタ一致によるゼロコピーを clack-host ハーネスで実証した。アダプタ記述の unsafe は 0、process 経路の alloc は activate 時の `with_capacity` 事前確保で 0、clang/cmake/SDK 不要、rust_version 1.85 適合・MIT OR Apache-2.0・依存木 15 パッケージを確認した (Issue #29)。

VST3 側では、生成 trait が全メソッド unsafe fn・レシーバ `&self` という制約に対し、アダプタ側の unsafe はシグネチャ義務分と局所 unsafe ブロックに留め、全て SAFETY コメントと `#[expect]` でゲートした。COM の `&self` 制約は `RefCell`（`try_borrow_mut` 失敗を `kResultFalse` の定義済み拒否に収束）と `AtomicU32`/`AtomicU64` で解決し、panic が FFI 越えに unwind してホストを abort させる経路を回避した。`kSample64` は f32 への再解釈をせず拒否し、VST3 SDK ヘッダはビルド時に不要と実測した (Issue #29)。

## VST3 lifecycle・COM・エディタの契約

VST3 adapter の `terminate` は非終端化とし、`Destroy` 検証後に `Created` 状態へリセットして再初期化を許す。`setupProcessing` の受理集合は `Initialized | Activated` に拡大した。fixture 側の正本と SDK 契約に照合して修正したものである (Issue #722)。

VST3 preset interface の帰属は vendored binding の調査で確定した。controller 側が `IUnitInfo`/`IUnitHandler`/`IUnitHandler2`、component 側が `IProgramListData`/`IUnitData` を持つ (Issue #630)。

エディタ実装では、`isPlatformTypeSupported` は HWND に対し `kResultTrue` を返す。`kResultOk` を返すと host が unsupported と誤読するためである (Issue #630)。DLL 登録の window class の `lpfnWndProc` に module-local 関数を使うと、cdylib unload 後に code pointer が dangling して host crash する。`DefWindowProcW`（user32 内・unload されない）と class brush の erase fill に変更し、module 所有 code を class に残さない設計にした。`hInstance` は `GetModuleHandleExW(FROM_ADDRESS)` で自 module を取得する（`GetModuleHandleW(NULL)` は host EXE を返す）(Issue #630)。`createView("editor")` は `ComWrapper::to_com_ptr().into_raw()` で owned `IPlugView` を返し、他の view 名は null を返す (Issue #630)。

## CLAP state・editor・preset の契約

CLAP の `state.load` で param 値が変わる場合、`clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)` の要求が契約上必須である。host handle を `PluginStateImpl` に保持させ、load 成功時に rescan を要求する形にした (Issue #630)。

CLAP エディタ verb は `RefCell<GuiState>` + `try_borrow`/`try_borrow_mut` で `&self` verb の再入を defined rejection に収束させる。`set_parent` は API-type・create-ordering・double-attach・`IsWindow` gate を必須化し、host bridge が `ParentLease::payload()` を HWND として無検証で渡すため、opaque payload が HWND と誤認されるのを防ぐ (Issue #630)。

CLAP エディタは resize-capable を採用した。fixed-size decline を維持すると `set_size` の success 経路を実バイナリで原理的に coverage できないためである。`can_resize=true`、`set_size` が created gate・size 記録・`win32::resize`（`SetWindowPos` の `SWP_NOMOVE|SWP_NOZORDER`）を行い、`destroy` で既定サイズに戻す。visibility oracle は hidden parent では祖先連鎖が不成立のため `IsWindowVisible` ではなく `GetWindowLongW(GWL_STYLE)&WS_VISIBLE` を使う (Issue #630)。

CLAP の preset-discovery は factory-level extension であり、`clack-extensions` の preset-discovery feature が必要。preset load は既存の `clap.ext.state` blob を再利用するため state-context feature は不採用とした (Issue #630)。

## OFX の C/Rust 分担と admission

OFX では、pinned OpenFX SDK（commit `c3f1c2dc`、欠落推移ヘッダと LICENSE も同 pin で byte-for-byte 取得・sha256 manifest 記録）を vendor し、C 側が SDK layout・suite table・variadic・`OfxHost`・typed accessor を所有する。Rust は fixed-arity の primitive/pointer のみを扱い、unsafe は FFI モジュールに局在させ、公開 API に外部 crate エラーや SDK 型を出さない (Issue #98)。

admission の順序は `OfxSetHost`→GetNumber/GetPlugin→per-plugin setHost/main→Load→Describe/DescribeInContext→Create である。admitted index は native index と一致せず、SingleInstance は CreateInstance 前に検査し、destroy は高々1回の terminal、quarantine で leak-not-UAF を実現する (Issue #98)。

## OFX render の staging・transactional commit

OFX render は adapter 所有の call-scoped 16B aligned staging を host memory pool 非依存で確保し、bottom-left 露出は最終物理行の data pointer + 負の checked-i32 `rowBytes` という SDK 標準形で実現する。単一の ClipPrefs validation helper を activation と各 render で無条件に実行し（local preflight 後・`BeginSequenceRender` 前）、unsupported/failed prefs は Begin/Render/出力変化なしに reject する。Begin→Render→End の bracket を組み、Begin/End の `ReplyDefault` は SDK の optional action 規約で合法成功、Render は mandatory のため strict とする。staging→dst の commit は bracket 全成功時のみで、失敗時は pixel/meta/padding を不変に保つ transactional 設計である。`t = at.to_seconds_f64()*num/den`（小値ケースのみ保証）(Issue #98)。

render 実装の lead review 12 指摘から得た再利用可能な教訓がある。staging 解放は `free` 直呼びではなく allocator pairing を保つ `stage_free`（Windows `_aligned_malloc` 対応）を使う。bracket 中の destroy/release/close は `render_active` guard で quarantine し、live attempt 残存 descriptor は `catalog_close` で deliberate leak とする（UAF 防止）。configure 時に `h<=INT32_MAX`・`stride*h<=PTRDIFF_MAX` 等の overflow bound を検査する。staging は `memset 0` で src padding を決定化し dst の未定義値露出を防ぐ。image props は descriptor を clone せず fresh seed で全必須 prop を供給する (Issue #98)。

## OFX foreign-call window の寿命管理（lease/token）

foreign ポインタの寿命と post-window deref は別問題として扱い、静的 `g_api` + lease counter の二分離で解決した。foreign 側へ渡る `OfxHost` を静的 `g_api` へ分離して reclaim 対象外とし（plugin が window を超えて保持しても `fetchSuite` は defined degrade）、`RvHost.foreign` の lease counter で window span のみ保護する。`host_free` は poison-immediate/reclaim-deferred に分離し、`foreign>0` で reclaim を延期、全 post-window アクセスを `!h->alive` の defined bail に統一する。dead-host 時は detached partial catalog を `rv_ofx_catalog_close` で in-place teardown し `*catalog=NULL` として、dangling string を caller に escape させない (Issue #124)。

この lease モデルは Issue #131 で uniform lease として全 foreign-call window（instance Create/Changed/ClipPrefs/render bracket/Destroy/release-drop、catalog close の Unload 群）へ一般化した。host handle を `H_HOST` generation token 化して ABA を解消し、`rv_tok_sweep(keep)` が poison 時に `g_host_props->tok` 以外の全 live token を revoke する。`g_api.host` は一度だけ束縛する read-only 値として専用 process-lifetime property set `g_host_props` に移し、confused-deputy と off-lock write を解消した。`catalog_close` に `closing` 再入ガードを入れ、dead 経路 close を lock 非保持へ移動して nested-close self-deadlock を解消し、`seed()` は snapshot 確保後にのみ `seeded`/`ddim` を設定する (Issue #131)。

不採用案とその理由も記録されている。`!Send` 文書化は contract-violation までの拒否で防御にならず、fixture observer は same-thread で実際に到達するため不採用。pool refcount は ownership model の大掛かりな変更であり、token sweep + lease で同等の到達不能性が得られるため YAGNI として不採用とした (Issue #131)。process-lifetime record（`g_host_props`）は poison 時点で退役させる。deferred reclaim では open lease 中に次 session が新 record を積み、旧 reclaim がそれを巻き込む cross-session 破壊になるためである (Issue #131)。

## テスト fixture の独立実装と self-verification loop の回避

テスト fixture は format 毎に独立実装し、DSP law の共通化は明示的に不採用とした。production host path（load→ErasedPlugin）経由で検証し、adapter 本体は zero-diff を維持する。共有するのは SDK-equivalent binding 層と neutral contract のみで、変換 law は neutral contract のみに依存させて self-verification loop を断つ (Issue #630)。

fixture の param は normalized↔plain の mapping を意図的に非恒等（plain = normalized*2、plain domain [0.0,2.0]）にし、host が domain を混同した場合に誤った gain として顕在化するようにした。L1 runner でも `EXPECTED_GAIN=0.5`（normalized 0.25×plain 2.0）で恒等 1.0 を避けて検証した (Issue #630)。

in-band armed fault は `ParamValue` event で arm し process 内で dispatch する方式を採り、exit code は plain param にエンコードする（plain=2.0+code/100.0、`ParamInfo::denormalize` が線形で STEPPED を量子化しないことを検証済み）。armed submit は fire-and-forget で drop され得るため 4 回・5ms cadence で繰り返し送出する。plugin process error は `RESULT_FAILED` flag ではなく `ProcessResult{status:Error}` で観測する（flag は synthesized-failure 専用）(Issue #630)。

state blob は `[u32 LE magic RFST][u32 LE version=1][f64 LE gain]` の 16 byte codec とし、VST3 は component=plain gain/controller=normalized gain の domain 分離、CLAP は plain を無 framing で保存する。activation は mirror を reset しない persistent-state law を採る（test_gain の activation reset を継承すると restore→activate で state が消える）。L1 ordering gate は delivered audio とし、process submit は fire-and-forget で state verb が mutation event block を追い越し得るため、3 つの pairwise-distinct gain の delivered output 観測を「それ以前の全 block 処理済み」の証明として state verb をその後にのみ送出する (Issue #630)。

L1 wire の event slot は固定 `EVENT_WIRE_SIZE=33` stride だが `encode_event` は可変長 record（MIDI=28 byte）を出力するため、33-byte stride で staging すると stale tail・packed stream decode の desync が起きる。wire contract owner に `encode_event_record`/`decode_event_record`（zero-pad・nonzero padding reject）を追加して修正した。CLAP raw MIDI は d1/d2 の 7-bit 検証を追加し、`Match::All` channel は channel 0 に捏造せず defined skip とした (Issue #630)。

VST3/CLAP の state bridge は、VST3 が `IBStream` ComWrapper + 4-call sequence + `[u32 LE comp_len][comp][u32 LE ctrl_len][ctrl]` framing で中立 `Vec<u8>` に橋渡し、CLAP が `PluginState` save/load を `PluginMainThreadHandle` 経由で生 byte 中継する。foreign blob 規則として、8 byte 未満は本 adapter の serialization であり得ないため plugin の `setState` へそのまま委譲し、8 byte 以上の corrupt frame は `PluginFailed` とする。`kNotImplemented`→`Unsupported`、その他→`PluginFailed` の error mapping を採る (Issue #630)。

fixture の不正直 stub（成功を装う空 no-op）を `kNotImplemented` に修正し test で pin した。「成功偽装しない」誠実な rejection が契約である (Issue #630)。

## 外部 ABI validator の運用

外部 validator は clap-validator 0.4.1 の pinned release binary（sha256 照合）と VST3 SDK validator（SDK commit `3cdf9ca` を CMake+MSVC で build）を provisioning し、`run.mjs`（fail-closed・verdict は canonical leg のみ集約）で実走した。tool 解決は flag>env>`--validator-dir` 規約 layout で推測しない。初回実走で fixture 側の複数契約違反を検出し、全て test crate 内で TDD 修正した（production adapter は zero-diff）(Issue #630)。

clap-validator 0.4.1 には preset-discovery crawl/load が 45 秒 timeout+JSON 無しで終了する `MetadataReceiver` self-deadlock（upstream issue #41）があり、fix は master のみで release >0.4.1 は未発行だった。upstream commit `47e2424` から build した binary を provision し SHA-256 を記録して供給する決定を ADR に記録し、Cargo dependency/commit 済み binary にはしない。orphan 化した sandbox child が DLL をロックする副作用も確認された (Issue #630)。

pipe stdio では orphan grandchild が pipe を保持し続け、完了済み JSON を捨てて leg を Failed/ETIMEDOUT と誤記録する問題があった。file-backed stdio（`spawnCapture`）で解決した (Issue #726)。

validator が decline を不可視にする問題には宣言的機構で対処した。CLAP は per-plugin `expectCreateDecline{pattern,reason}`、library-level は `CLAP_FIXTURE_LIBRARY_EXCLUDES`+`clapLegExclusions`、VST3 は `VST3_DECLARED_UNCREATABLE` で rawOutput 中の decline marker 出現回数を記録し、Passed leg で 0 件なら Failed に倒す fail-closed とした。run.json schema を /2 に bump した (Issue #630)。

## param-write の中立契約と honest observability

param-write 契約は plugin-core に `ParamWriteMeans`（TimedEvent/BlockBoundary/Immediate/PerNote）・`ParamWriteRejection`（ReadOnly/NotAutomatable/UnsupportedMeans/UnsupportedPrecision/UnknownParam/NonFiniteValue）・`check_param_write`・opt-in `coalesce_param_events_latest` を実装し、bridge に `TargetId`/`BindingGeneration`/`ParamBinding`/`ParamTargetTable`/`ParamDispatchLedger` を追加した。配送経路は既存の `TransportPlan→PluginNode→EventList` を正本として再利用し、汎用 Control Graph クレートは ADR 判断で deferred とした (Issue #186)。

中立語彙に存在しない `Immediate`/`PerNote` means は `UnsupportedMeans` で定義済み reject とし、無言の降格を避けた。`STEPPED` param への timed write は受理し、量子化は consumer の明示ルールに委譲する（silent quantization しない）。`Dispatched` stage は plugin acknowledgment ではなく plan/block partition からの決定論的投影として実装し、`Reported` stage は plugin→host report 配線が無いため現行 `Unavailable` とした。per-note modulation/reporting は未証明のため unsupported/Unavailable を正直に宣言した (Issue #186)。

## 計測・検証の運用規約

no-alloc 契約の検証は `rt_` 命名テストと構造レビュー（`static`/`thread_local` grep）に依存し、実 alloc 計測レーンは別 Issue の範囲である。`alloc_counter`（`assert_no_alloc` と同一機構）の thread-local 窓は設置スレッドのみ観測するため、backend 所有スレッド上の alloc は呼出側窓では機構的に検出できない。窓を計測対象スレッド自身に設置する所有権設計（callback 内で窓を開く・process-wide 計数アロケータ併用）が必要である (Issue #104)。

タイムスタンプは `Instant::now()` が 29.2 ns で `timestamped_record` 39.4 ns の 74% を占めるため、全件スタンプは高頻度経路で支配コストになる。「呼出側提供 epoch + 観測点ごとの必要箇所のみスタンプ」が推奨で、実機経路は `StreamInstant::as_nanos`（コールバック提供時刻で取得コスト自体なし）が既存実装である (Issue #104)。

計装コストは必ず同一 base workload での差分として測る。非 equal-work 比較（disabled 列のみが wrap+black_box のワークを持つ構成）とゼロ値 fold（値0記録を fold する `fetch_add(0)` 構成）は計装コスト評価を無効化し、初回計測値は superseded として撤回された (Issue #104)。

criterion 計測では run→集約→次 run の順序を厳守する（`estimates.json` は run ごとに上書きされる）。BATCH=512 op/iteration・warm-up 300ms/measurement 1s/sample 20 の削減予算で、run 間 |Δ%| は p50=0.4%/p90=16.4%/max=38.5% の分散があるため、判定には run 内比較（同一 run のバリアント比率）と 3 run 平均を併用して絶対値差を相殺する (Issue #104)。

高負荷環境ではベンチをカーネル単位に分割実行する。`mise run bench`（単一プロセス全量）は並行エージェント ~350 プロセス稼働下で Windows コミット上限逼迫により ~82–85% 地点で abort した。カーネル単位の分割実行でプロセス寿命を限定してコミット滞留を抑え、中断時の再開性も確保した (Issue #19)。

Full Gate（fmt-check/check/lint/test/test-rt/bench-smoke/prek/doc audit）は plan 最終段で1回のみ実施し全 log を保存する。`test-rt` は実一致件数で証明し、0 件スキップを成功と偽らない (Issue #98)。canonical 検証は `mise run` task が正本で、cargo 直接実行は crate 単位の補助確認である (Issue #630)。

## SIMD・RT 経路の設計知見

SIMD 採否では「スカラー参照常備+サイズ応答ルーティング」の二層構造を採用した。単一 dispatched 経路は小サイズで回帰し（fearless_dispatched は elementwise @64/256 で 0.58–0.84x 劣後、per-call dispatch コストが軽量カーネルで顕在）、multiversion は dot ≈1.0x・fir 0.69–0.73x 回帰・個別 clone 強制 API 不在で検証網羅性に構造差があり除外した (Issue #19)。

SIMD 等価性契約の確定値は、`dot` が ulp ≤256（実測 max 46）、`fir`/`biquad` が ulp ≤8（実測 max 0）、elementwise/再帰一致系 5 カーネルは bit 完全一致である。FMA 版 fir は ~3994 ulp の打ち消しドリフトを実測したため非融合 multiply-add を採用した (Issue #19)。

`fearless_simd` 1.1.0 は MSRV 1.89、criterion 0.8.2 は MSRV 1.86 で workspace MSRV 1.85 を超過するため、dev-dep 封じ込めで回避した。本番 `[dependencies]` 昇格は MSRV 引上げ Issue 前提である。denormal 崖（fir 最大 ~31.8x）は性能のみで等価性は維持され、FTZ/DAZ は Rust 上で形式的 UB のためカーネル側では対策せずエンジン層へ移管した (Issue #19)。

## 未確定・保留事項

- `vst3` 生成バインディングの SDK 出自のライセンス継続調査が未完了である (Issue #29)。
- macOS `bundleEntry`/`bundleExit`・Linux `ModuleEntry`/`ModuleExit` は cfg ゲート済みコードとして存在するが、Windows ホストでのクロスビルドは未実測で CI 3OS matrix 待ちである (Issue #22)。
- WASAPI は出力 xrun を報告しないため「発生時の報告挙動」は未観測であり、error event 0 は「xrun 未発生」と「未報告」を区別しない。bounded chunked write 経路は実機で未発火でユニットテストのみの実証である (Issue #23)。
- SIMD PoC の `fearless_dispatched` 小サイズ劣後は per-call dispatch コストかノイズか未分離で、低負荷環境での再計測が残課題である (Issue #19)。
- OFX の `catalog_identifier`/`descriptor_string` の raw `char*` 戻り値は post-reclaim で dangling になり得るが、facade が open 時複製済みで contract 内では発生しない。catalog handle の token 化は caller contract 維持で対象外とした (Issue #131)。
- plugin 分離の L1（プロセス分離）は ruviform-plugin-host 未実装のため「新設時の境界として規定」する形で整合確認のみ行い、実装は後続 Issue に委譲された (Issue #31)。
- `ParamDispatchLedger` の `Reported` stage は plugin→host report 配線が無いため API 形状のみで実値は `Unavailable` であり、配線時に `record_reported` の接続が必要な follow-up として残る (Issue #186)。
- Issue #195 の ownership PoC（Plan-owned vs Stable Slot 比較）は計画段階の記録であり、実施結果の詳細は本書の記録範囲外である (Issue #195)。

---
id: "SPEC-034NV8AWA7qua5MfPc7q6i"
type: "spec"
status: "approved"
title: "Issue #118 video/NONRT プラグインの L1 経路"
created: "2026-10-29"
updated: "2026-10-29"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-118-video-l1.md"
    - "../plans/implement-issue-118-video-l1.md"
  depends-on:
    - "issue-119-l1-wire-version-policy.md"
    - "issue-117-bundle-child-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "plugin-l1-out-of-process.md"
    - "issue-119-l1-wire-version-policy.md"
    - "issue-117-bundle-child-process.md"
    - "issue-114-real-adapter-l1-e2e.md"
    - "engine-plugin-host-integration.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/engine-plugin-host-integration.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../adr/ofx-video-plugin-scope.md"
    - "../adr/plugin-isolation-strategy.md"
    - "../discovery/issue-118-video-l1.md"
    - "../impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md"
    - "../designs/issue-118-video-l1.md"
    - "../adr/issue-118-video-l1-host-surface.md"
    - "../adr/issue-118-video-l1-create-path-segment.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-118-video-l1.md"
    - "plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "issue-119-l1-wire-version-policy.md"
    - "issue-117-bundle-child-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "engine-plugin-host-integration.md"
    - "../designs/engine-plugin-host-integration.md"
  derived-by:
    - "../designs/issue-118-video-l1.md"
    - "../adr/issue-118-video-l1-host-surface.md"
    - "../adr/issue-118-video-l1-create-path-segment.md"
    - "../plans/implement-issue-118-video-l1.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-118-wire-v3-codec-tests.md"
    - "../test-specs/issue-118-video-failed-parity-tests.md"
    - "../test-specs/issue-118-video-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/98"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/123"
    - "https://github.com/scarecrow173/ruviform/issues/131"
  defers: []
  deferred-by: []
---

# Issue #118 video/NONRT プラグインの L1 経路

## What Should Be Built

Issue #118 — audio `process` ブロック経路とは別の **NONRT video データ面**を L1 wire/shm に確立する。`VideoPluginInstance`（plugin-core `video.rs` — 全操作 NONRT）を持つプラグインが L1 子プロセス内で spawn・lifecycle・render できるようにする:

1. **video wire 面** — `VideoPluginInstance` の lifecycle（`initialize(VideoProcessConfig)` → `activate` → `start_processing` → `render` → `stop_processing` → `deactivate` → `destroy`）と plain param チャネル（`parameters`/`parameter`/`parameter_value`/`set_parameter`）を運ぶ wire 動詞・codec。`SaveState`/`RestoreState`/GUI 動詞は video trait に存在しないため対象外
2. **フレーム交換データ面** — `VideoFrame`（format・dims・planes・PTS/duration・`ColorInfo`）を host↔子間で受け渡す shm 輸送。フレーム payload は pipe に載せず、descriptor（wire メッセージ）と payload（shm）を分離する（サイズ観測: 1080p Rgba8 ≈ 7.9MiB・4K RgbaF32 ≈ 126MiB ≫ audio `SLOT_BYTES` 545KiB・`WIRE_MAX_PAYLOAD` 16MiB — discovery 軸 1）
3. **runner 側 video 表面** — `ErasedPlugin` 相当の video erased surface・factory・instance registry・dispatch（`run(factories)`/audio registry は video を載せられない — `VideoPluginInstance` は別 trait）
4. **`Failed` + 通知 + scope 契約の parity** — #117 確定の 子スコープ terminal + 帰属可能 per-instance 収束 + inert 規則 + `InstanceFailed` fan-out を video 経路にも適用。自動再起動なし
5. **wire バージョン** — 新動詞・新 shm layout・video bounds は `WIRE_VERSION` bump 内に閉じ込める（#119 単一最新方針 — 交渉機構なし）

呼出し側の `plugin_id` route・spawn・handle・通知モデルは #117 の形状を維持する。統合面の正確な形は R-F5 で確定済み — 別立て video 表面 `PluginHost::video() -> VideoPluginHost<'_>`（`VideoHostCommand::Instantiate{plugin_id, config}` が video route を引く。design `docs/designs/issue-118-video-l1.md`・ADR `docs/adr/issue-118-video-l1-host-surface.md` 参照）。

## Why Now

- `docs/designs/plugin-l1-out-of-process.md` L313 deferred 行（video plugin は L1 不可）・`docs/designs/engine-plugin-host-integration.md` L285-289（決定 `k`: L1 video wire 非確立・trigger 付き延期）の re-engagement
- #114 で OFX は video wire 不在を理由に範囲外化・Issue #118 追跡がユーザー決定済み（`docs/discovery/issue-114-real-adapter-l1-e2e.md` L181/L198）
- task-4 IR が video 経路・`WIRE_MAX_*` 引上げ・可変 slot を明示 deferred（`docs/impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md` Non-Goals）
- post-#117 の wire v2（per-instance envelope）+ per-instance shm + Bundled pool が確立し、video plane が乗る基盤が揃った
- video plugin が L1/RT 経路を使えないままでは、NLE 第二ターゲットの OFX プラグインをアウトプロセス隔離できない

## Users and Value

- ホストアプリケーション/エンジン実装者 — video plugin（OFX 等）を L1 アウトプロセスで動かし、プラグイン crash からホストを隔離できる
- 未検証・外部 video プラグインを扱う利用側 — プラグインのクラッシュが `Failed` + 通知 + `Destroy` で閉じる既存運用モデルに video が乗る
- video/audio 混在 route を組む利用側 — video 子プロセスの失敗が定義済みの粒度（子スコープ/インスタンススコープ）で収束し、未定義挙動に落ちない

## Scope

### In Scope

- wire codec — video lifecycle 動詞（`VideoInitialize{VideoProcessConfig}` 相当を含む）・frame descriptor・plain param get/set/info 動詞・video 固有 bound（`WIRE_MAX_VIDEO_*` 相当: dims/plane/frame bytes）・`ColorInfo`/`PlainParamValue`/`FrameRate` 等の codec 追加。`MediaTime`/`MediaDuration` codec は既存を再利用
- フレーム shm 輸送 — per-instance 専用 video segment（descriptor + payload 分離）を前提とする要求・invariant・bounds。確定形（slot 形状・attach タイミング・doorbell/Command 畳み）は R-F1 で design 委譲
- runner 側 video registry — `ErasedVideoPlugin` 相当の erased surface・`CreateInstance` 系の video 分岐・instance 単位 dispatch・panic→`Err(PluginFailed)` 正規化
- `Failed` 粒度・通知・inert 規則の video 経路 parity（mid-render 死亡・`CreateVideoInstance`/`DestroyInstance` 送信中死亡・帰属可能違反を含む全分岐）
- per-instance FIFO・同期 request/response render（`&mut self` 鏡写し）・render 失敗時は出力不 commit（L0 OFX 契約の写像 — `instance.rs` render の commit-on-success）
- isolation — video route の isolation 既定値と Bundled 許容可否の決定（R-F4 で design 委譲・HoL トレードオフ明記）
- 検証 — codec round-trip/bounds 単体・mock video fixture の E2E（scan→create→lifecycle→render→destroy）・`Failed` parity・既存 audio E2E の無退行
- `WIRE_VERSION` bump + shm header `wire_version` 検証の踏襲

### Out of Scope

- GPU/texture interop・OFX GPU suites — 上流 ADR（`ofx-video-plugin-scope` D2-a・`ofx-host-video-contract` 初期 profile = CPU Filter/Rgba8）で後続 Issue 確定済み
- 真の zero-copy（shm 領域の直接 `VideoFrame` view/backing 抽象拡張）— `SharedPlane`/`Arc<[u8]>` 制約上別拡張が要る。v1 は copy-in/copy-out
- RT 契約 — video は全操作 NONRT（`rt_` テスト・`assert_no_alloc`・pass-through 語彙は適用外）。endpoint/`prepare_endpoint` は audio のみ
- OFX 実バイナリの L1 E2E — 構造的には可能になるが、runner 組込み・規格 fixture の E2E は後続（#114 と同じ disposition 判断 — mock video で wire/意味論を先に確定）
- L1 GUI・video GUI — `OpenGui`/`CloseGui` は video trait に存在せず L1 GUI 自体も deferred（#115）
- engine/利用側統合 — `PluginHost` 経由の video 到達面は本 spec で wire/host 意味論まで（engine 側配線は別 Issue）
- supervisor/#116 util 層 — video インスタンスを supervise するかは supervisor 側の拡張論題（core 自律再起動なしの契約は不変）
- 複数 wire バージョン共存・交渉 — #119 で非採用確定のまま
- audio `process` 経路・`SLOT_BYTES` レイアウト・`BlockIn`/`BlockDone` 意味論の変更 — video は別系統として追加し audio を無変更に保つ

## Requirements

### Functional / Behavioral

- **R-B1**: wire は `VideoPluginInstance` の全 lifecycle 遷移を表現できる — `initialize(VideoProcessConfig{format,width,height,frame_rate})` は config 持ち動詞（audio `Activate{ProcessConfig}` とは別形）・`render` は `{seq 相関, at: MediaTime}` を運び `Reply` が成功/失敗を返す。違法遷移は `Err(HostError::Lifecycle(_))`・`Failed` インスタンスへの `Destroy` 以外のコマンドは `Err(HostError::Failed)`（audio と同一語彙）
- **R-B2**: frame 交換は descriptor + shm payload 分離 — wire メッセージが `{format,w,h,pts,duration,color,plane 記述, seq 相関}` を運び、plane bytes は per-instance video segment 経由。受信側は `Arc` 跨ぎ不可のため copy-in/copy-out
- **R-B3**: `render` は同期 request/response・per-instance FIFO — out-slot は `Ok` reply 到達時のみ有効・`Err`/timeout/死亡時は出力を読まない（L0 OFX commit-on-success の写像）
- **R-B4**: 全 decode/attach は bounded — `WIRE_MAX_VIDEO_*`（dims/plane count/frame bytes 相当）で hostile dims・巨大要求・stride/len 不整合を defined 拒否し panic・無制限 alloc・OOB しない
- **R-B5**: `Failed` 粒度は #117 確定形を video にも適用 — 子スコープ terminal + 帰属可能 per-instance + inert + 通知 fan-out。mid-render 死亡は render waiter `Err(Ipc)` + 影響インスタンス `Failed` + `InstanceFailed`
- **R-B6**: audio 経路の意味論無変更 — 既存 wire codec・audio slot・doorbell・`Failed` 分岐・Bundled pool は video 追加で退行しない
- **R-B7**: video registry/dispatch は runner で `ErasedPlugin` audio registry と同居可能 — plugin hook panic は `Err(PluginFailed)` に正規化され instance のみ失敗。未登録 instance 宛の video 動詞は flagged `Err(InvalidHandle)` 相当の defined 拒否
- **R-B8**: plain param チャネル — `parameters()` 一覧・`parameter(id)` の `PlainParamInfo`（owned 化 — String choices は `WIRE_MAX_*` 制約内）・`parameter_value`/`set_parameter` が wire 越しに動く

### Decision points（design/ADR に委譲 — spec が確定しない内部構造・公開既定値）

確定状況（2026-10-29 時点 — design `docs/designs/issue-118-video-l1.md` 決定サマリを正本とする）:

- **R-F1**: フレーム輸送の確定形 — **✅ 確定（人間確認済み）**: per-instance 専用 video segment + descriptor-over-Command/Reply（軸 1 (a)+(d)）。`VideoRender{at, input:VideoFrameDesc}` → `Reply::Rendered{output:VideoFrameDesc}`、plane bytes は per-instance segment の in/out 領域。新 doorbell kind は作らない。attach タイミングは `CreateVideoInstance{plugin_id, shm_key, config}` で create 時に解決（ADR `docs/adr/issue-118-video-l1-create-path-segment.md`）
- **R-F2**: render timeout — **design で確定案**: video 専用 `render_timeout`（`VideoOutOfProcessSpec` 字段・既定 60 s）を `VideoRender` reply wait に適用。`command_timeout`（既定 10 s）は handshake/lifecycle/param 動詞に適用。timeout→`Err(Ipc)`+子スコープ `Failed` 収束（帰属不能 wedge の #117 規則）
- **R-F3**: pool depth — **design で確定案**: depth=1 固定（in/out 各 1 領域・`VideoProcessConfig` から一意に算出された canonical packing — `stride=row_bytes` tightly packed）。slot ring/depth 字段は v1 wire に持たせない
- **R-F4**: video route isolation 既定 — **✅ 確定（人間確認済み）**: video-capable instance は **Dedicated のみ**。`VideoOutOfProcessSpec` に `isolation` 字段を持たせず `Bundled`+video を構造的に表現不能にする（runtime 拒否でなく「書けない」pin — AC5/R-V5 は「非対応の形で pin」側で満たす。ADR `docs/adr/issue-118-video-l1-host-surface.md`）
- **R-F5**: host 統合面 — **✅ 確定（人間確認済み + design で形を確定）**: **別立て video 表面** — audio `HostCommand`/`Instantiate` には video 動詞を足さない。形は `PluginHost::video(&mut self) -> VideoPluginHost<'_>` 借用 façade + `VideoHostCommand`/`VideoHostReply`（`#[non_exhaustive]`）。`plugin_id` は単一名前空間・route kind タグ（cross-kind `Instantiate` → `Err(UnknownPluginId)`・登録衝突 → `Err(DuplicatePluginId)`・cross-kind 動詞 → `Err(Unsupported)`。ADR `docs/adr/issue-118-video-l1-host-surface.md`）
- **R-F6**: `Scan`/capability 広告 — **design で確定案**: wire に kind 記述を足さない（`Hello.capabilities` は v3 でも reserved-zero 維持）。video 子の `Scan` は video factory の descriptor を返し、kind 混同は host 側 `spec.descriptor` field-match で検出。route kind は host 側のみの属性

### Verification

- **R-V1**: video wire codec round-trip + 拒否テスト（新 tag・`WIRE_MAX_VIDEO_*` 超過 dims/plane/bytes・不正 `instance`・reserved・`VideoProcessConfig`/`ColorInfo`/`PlainParamValue` codec）
- **R-V2**: video E2E — mock video fixture runner で scan→create→initialize(config)→activate→start→render→stop→deactivate→destroy + param get/set。frame 内容の正当性（出力ピクセル一致）
- **R-V3**: `Failed` parity — 子死亡/pipe 破壊/mid-render 死亡/帰属可能違反/inert/`Shutdown` 後 frame の全分岐が #117 確定形で振る舞う
- **R-V4**: audio 無退行 — 既存 L1 E2E・codec・`rt_` テストが video 追加後もグリーン
- **R-V5**: isolation 挙動 — R-F4 確定値どおり video route の isolation が機能（Dedicated 限定なら Bundled video は defined 拒否 or 非対応の形で pin）
- **R-V6**: bounds — hostile config/dims/plane descriptor が decode/attach 両層で defined 拒否・無制限 alloc なし

## Acceptance Criteria

- [ ] **AC1（video wire codec）**: `WIRE_VERSION` bump 後の codec で、video lifecycle 動詞・frame descriptor・param 動詞・`VideoProcessConfig`/`ColorInfo`/`PlainParamValue` が round-trip し、不正 `instance`・上限超過・不正 tag・truncated payload が defined 拒否（panic・無制限 alloc なし）。`Hello.version` 不一致は handshake 失敗。audio codec は無変更
- [ ] **AC2（video lifecycle parity）**: L1 video インスタンスが scan→create→`initialize(VideoProcessConfig)`→`activate`→`start_processing`→`render`→`stop_processing`→`deactivate`→`destroy` を L0 と同一意味論で実行する — 違法遷移 `Err(Lifecycle)`・`Failed` への `Destroy` 以外 `Err(Failed)`・`Destroy` は `Ok` で slot 解放。render 失敗時に出力 frame は commit されない
- [ ] **AC3（フレーム輸送）**: input `VideoFrame`（format・dims・planes・PTS・duration・`ColorInfo`）が video segment 経由で子に届き、出力 `VideoFrame` が `Reply` 到達時に host で有効。`Rgba8` 以外の format も wire 表現可能（adapter 受理は negotiate 次第）。per-instance FIFO・`(seq, instance)` 相関・跨 instance 誤配なし
- [ ] **AC4（`Failed` parity）**: (a) 子死亡/帰属不能違反 → 影響範囲の全 live インスタンス `Failed` + 全 pending waiters `Err(Ipc)` + `InstanceFailed` fan-out（同一 `ChildExit`）; (b) 帰属可能違反 → 対象 instance のみ `Failed`; (c) inert — post-removal/`Destroying`/`Failed` 宛の遅延 frame/done は静かに drop; (d) `Shutdown` 後 frame・EOF は non-event; (e) mid-render 死亡 → render waiter `Err(Ipc)`・mid-`CreateVideoInstance` 死亡 → 通知なし `Err(Ipc)`・mid-`DestroyInstance` 死亡 → `Failed`+通知+`Destroy` は `Ok`
- [ ] **AC5（isolation）**: R-F4 で確定した video route isolation が観測可能 — Dedicated 限定なら Bundled video 要求は defined 拒否/非対応で pin、opt-in 許可なら Bundled video 子が動作し HoL 挙動が文書化されている
- [ ] **AC6（param チャネル）**: `parameters`/`parameter`/`parameter_value`/`set_parameter` が wire 越しに動作 — `PlainParamInfo`（String choices 含む）が bounded に往復し、未知 `ParamId` の答えが定義どおり（`parameter(id)` → `ParamInfo(None)`・`parameter_value`/`set_parameter` → `Err(InvalidValue)` — runner 側 `id ∈ parameters()` 先行検査による defined 拒否）
- [ ] **AC7（runner registry）**: runner が video registry で instance dispatch し、video hook panic が `Err(PluginFailed)` に正規化され instance のみ失敗。`CreateInstance` 系で video entry を mint し、上限・重複・未登録宛は audio と同型の defined 拒否で runner が死なない
- [ ] **AC8（audio 無退行）**: 既存の L1 audio E2E（proxy/runner/bundle）・codec テスト・`rt_` 命名テストが全て無変更でグリーン
- [ ] **AC9（CI/監査）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0

## Deferred Design Concerns

- GPU/texture interop・OFX GPU suites | 上流 ADR で後続 Issue 確定・初版は CPU Filter/Rgba8 | GPU suite 需要・media-io shared-buffer 要件 | GPU プラグインは L1 で使えない
- 真の zero-copy shm→frame backing | `Arc<[u8]>` 制約で表現不能・video-core 拡張が別 Issue 級 | copy コストが支配的になる負荷 | 全フレームが copy-in/copy-out コストを払う
- async render / frame pool depth>1 / pipeline 化 | v1 は同期 request/response で十分・depth=1 | 長 render・連続フレーム処理の throughput 需要 | render latency が制御面を占有する
- Bundled video の HoL 局所化（reader/channel 分離・子側 render スレッド） | R-F4 で Dedicated 限定に倒せば回避可能・構造コスト大 | Bundled video が実運用で要求される段階 | Bundled 下で遅い render が兄弟 audio doorbell を塞ぐ
- video 動詞の `PluginHost` 統合面（R-F5 の公開 API 形） | wire/host 意味論を先に確定するため内部形は design 委譲 | 公開 API 確定が必要になる段階 | video plugin が `PluginHost` 経由で instantiate できない
- engine 側 video port 接続 | 別 Issue（第二ターゲット統合） | video engine 統合の要求 | L1 video が engine から到達不能
- video GUI / `OpenGui` 系 | video trait に動詞が存在しない・L1 GUI は #115 で deferred | L1 GUI 需要 | GUI 持ち video plugin は L1 で GUI を開けない

## Sources and Relations

- Issue [#118](https://github.com/scarecrow173/ruviform/issues/118)（要求元）。upstream: [#76](https://github.com/scarecrow173/ruviform/issues/76)・[#113](https://github.com/scarecrow173/ruviform/issues/113)・[#114](https://github.com/scarecrow173/ruviform/issues/114)・[#117](https://github.com/scarecrow173/ruviform/issues/117)・[#119](https://github.com/scarecrow173/ruviform/issues/119)
- 下流 discovery: [issue-118-video-l1.md](../discovery/issue-118-video-l1.md) — 全 code 観測・軸 1-8 の比較・確定済み前提の列挙
- 一次文書: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（L313 deferred 行）・[engine-plugin-host-integration.md](../designs/engine-plugin-host-integration.md)（決定 `k` L285-289）・[issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md)・[issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)・[ofx-video-plugin-scope.md](../adr/ofx-video-plugin-scope.md)・[ofx-host-video-contract-and-c-abi.md](../adr/ofx-host-video-contract-and-c-abi.md)・[issue-119-l1-wire-version-policy.md](issue-119-l1-wire-version-policy.md)・[issue-76 task-4 IR](../impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md)（Non-Goals — video 経路/上限引上げ/可変 slot の明示 deferred）
- ローカル一次証拠: `crates/ruviform-plugin-core/src/video.rs`・`wire.rs`・`boundary.rs`・`plain_param.rs`・`crates/ruviform-video-core/src/{frame,pixel,plane,color}.rs`・`crates/ruviform-plugin-ofx/src/{instance,host}.rs`・`crates/ruviform-plugin-host/src/l1/{mod,shm}.rs`・`crates/ruviform-plugin-export/src/lib.rs`

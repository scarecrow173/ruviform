---
id: "DISC-034NV8A2eMJbUhGFvQgL3W"
type: "discovery"
status: "active"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/plugin-l1-out-of-process.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../specs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
    - "../designs/engine-plugin-host-integration.md"
    - "../specs/engine-plugin-host-integration.md"
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../adr/ofx-video-plugin-scope.md"
    - "issue-114-real-adapter-l1-e2e.md"
    - "explore-video-foundation-api-scope.md"
    - "../specs/implement-video-foundation-api.md"
    - "../adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "../impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md"
    - "../designs/issue-118-video-l1.md"
    - "../adr/issue-118-video-l1-host-surface.md"
    - "../adr/issue-118-video-l1-create-path-segment.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/plugin-l1-out-of-process.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/engine-plugin-host-integration.md"
    - "../specs/engine-plugin-host-integration.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../specs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../adr/ofx-video-plugin-scope.md"
    - "issue-114-real-adapter-l1-e2e.md"
  derived-by:
    - "../specs/issue-118-video-l1.md"
    - "../designs/issue-118-video-l1.md"
    - "../adr/issue-118-video-l1-host-surface.md"
    - "../adr/issue-118-video-l1-create-path-segment.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/98"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
    - "https://github.com/scarecrow173/ruviform/issues/124"
    - "https://github.com/scarecrow173/ruviform/issues/125"
    - "https://github.com/scarecrow173/ruviform/issues/130"
    - "https://github.com/scarecrow173/ruviform/issues/131"
  defers: []
  deferred-by: []
---

# Issue #118 video/NONRT プラグインの L1 経路

## Exploration Goal

Issue [#118](https://github.com/scarecrow173/ruviform/issues/118) は **audio `process` ブロック経路とは別の NONRT データ面（video port・フレーム交換）の wire/shm 設計・実装**を要求する。`VideoPluginInstance` は L0（in-process）では OFX host アダプタ経由で既に動作するが、L1（アウトプロセス）経路は存在しない — wire v2 の動詞は audio lifecycle + audio doorbell（`BlockIn`/`BlockDone`）のみで、shm スロットは planar `f32` audio ブロック固定レイアウトである。

トリガ（upstream の deferred 行 — 本 Issue がその re-engagement）:

- `docs/designs/plugin-l1-out-of-process.md` L313 —「video/NONRT プラグインの L1 経路 | 本 Issue は audio `process` 経路のみ | NONRT video port の分離要求 | video plugin は L1 不可」
- `docs/designs/engine-plugin-host-integration.md` L285-289（決定 `k`: L1 video wire 非確立）・L326 deferred 行
- `docs/specs/engine-plugin-host-integration.md` L107・L145（同 deferred 表）
- `docs/impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md` Non-Goals —「video/NONRT plugin の L1 経路・`WIRE_MAX_*` 上限引上げ・可変 slot・shm 内共有制御状態（head/tail・ロック — design 棄却どおり非採用）」
- `docs/discovery/issue-114-real-adapter-l1-e2e.md` L181/L198 — OFX は video wire 非確立を理由に範囲外化・Issue #118 追跡がユーザー決定で確定

本探索が渡す成果物: (a) 現行 NONRT video 契約面と L0 OFX 実装の正確な形状、(b) post-#117 wire v2 + shm substrate の audio 専有部分と再利用可能部分の分離、(c) フレームサイジングと輸送モデルの選択肢比較、(d) NONRT 意味論（crash/`Failed`・所有・順序・flow control・API 面・isolation 相互作用）の要求列挙、(e) design/spec フェーズへ渡す tentative lean と人間判断が必要な OPEN 論点。最終の wire/shm アーキテクチャ確定権限は design/ADR フェーズにある。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — design 論点ではない）

- **`VideoPluginInstance` は audio `ErasedPlugin` 表面に載らない別 trait** — `crates/ruviform-plugin-core/src/video.rs` L94-141。`type Error: std::error::Error` 関連型・`initialize(&VideoProcessConfig)` → `activate` → `start_processing` → `render(&VideoFrame, &mut VideoFrame, MediaTime)` → `stop_processing` → `deactivate` → `destroy` の lifecycle。`SaveState`/`RestoreState`/`OpenGui`/`CloseGui` は trait に**存在しない**。`docs/discovery/issue-114-real-adapter-l1-e2e.md` L82・L101 が「`ErasedPlugin`/`run(factories)`/shm audio 面に載らない」を confirmed 済み — L1 に載せるには **video 専用の erased 表面・factory・runner registry が新規に要る**。
- **video は全操作 NONRT・RT 契約なし** — `video.rs` L1-5・`docs/designs/engine-plugin-host-integration.md` L287（`prepare_endpoint`/`PluginEndpoint` は audio のみ対象）。`rt_` テスト・`assert_no_alloc`・pass-through（RT フォールバック語彙）は video 経路に**適用されない**。失敗時は同期呼出しが `Err` を返すだけ — audio の「無音フォールバック」相当の概念は存在しない。
- **`Failed` + 通知 + 自動再起動なし契約は #116/#117 で確定・video にも持ち越す** — 子スコープ terminal 収束 + 帰属可能 per-instance 収束 + inert 規則 + `InstanceFailed` fan-out（`docs/designs/issue-117-bundle-child-process.md`・ADR `issue-117-bundle-child-failed-granularity`・`l1/mod.rs` L583-769）。supervisor は #116 で host util 層に確定 — video 経路も core 自律再起動なし。
- **wire は単一最新バージョン・交渉機構なし** — `wire.rs` L43-54・`docs/specs/issue-119-l1-wire-version-policy.md`。video 動詞/codec 追加は **WIRE_VERSION bump（2→3）を伴う互換性変更** — 既存動詞の意味は変えられない。
- **`PluginHost` は `!Send`/`!Sync`・非 RT 制御スレッド専有** — composite.rs L304-308（`Shmem` raw pointer + `Box<dyn ErasedPlugin>` auto-traits）。video proxy も同じ制約に乗る（L1 shm を触る限り `!Send`）。
- **OFX の初期 profile は CPU Filter・`Rgba8` のみ・GPU suites は後続** — `docs/adr/ofx-video-plugin-scope.md`（D2-a: host 側 Image Effect API・GPU suites/追加 capability は需要発生時の後続 Issue）・`docs/adr/ofx-host-video-contract-and-c-abi.md`（初期 profile は CPU Filter・`Rgba8`）。

### コード観測事実（confirmed — 現行構造の正確な形）

#### Video 契約面（`ruviform-plugin-core` + `ruviform-video-core`）

- `VideoProcessConfig`（video.rs L22-73）: `{format: PixelFormat, width: u32, height: u32, frame_rate: FrameRate}` — **dims/format/frame-rate は `initialize` 時に固定**され processing 中は不変。検証は非零 dims のみ（L37-41）— pixel format・stride・layout 制限は adapter 責務と core doc が明記（L19-20）。
- `render` シグネチャ（L127-132）: `input: &VideoFrame`（共有可）・`output: &mut VideoFrame`・`at: MediaTime` — **同期 request/response・1 フレーム単位・`&mut self` で排他**（per-instance 逐次性は trait の借用規則に内在）。
- `VideoPluginInstance` の param 面は **plain param チャネル**（audio の normalized `ParamInfo`/イベント経路と別系統 — `plain_param.rs` L1-6）: `parameters() -> &[ParamId]`・`parameter(id) -> Option<PlainParamInfo>`・`parameter_value`/`set_parameter`（`PlainParamValue::{Double,Integer,Boolean,Choice}` — plain_param.rs L20-29）。**audio wire に param get/set 動詞は存在しない**（audio は process 内イベント + `plugin_mut` 直接アクセスで済む）— video では全操作が wire 越えになるため **param get/set/info の wire verb が新規に要る**。
- `VideoFrame`（video-core `frame.rs` L1-16 冒頭 doc）: `format`・`width/height`・PTS（`MediaTime`）・optional `MediaDuration`・`ColorInfo`・planes。**plane backing は `Arc<[u8]>`** — `Clone` は shallow・`plane_mut` は `Arc::get_mut` 成功時のみ（shared clone は read-only）・最後の Arc drop で解放は NONRT 側。**`SharedPlane`/`from_shared_planes` が唯一の外部バッファ zero-copy 供給経路**（`plane.rs` L301-337）で `Arc<[u8]>` 所有が必須 — shm 領域を `Arc` で包めないため、**プロセス境界を跨ぐ真の zero-copy は現行型では表現不能**（受信側は shm→owned frame へのコピーか、新しい backing 抽象が要る）。
- 平面幾何（`plane.rs` L8-12・L16-28）: `stride ≥ row_bytes`・per-plane stride・`required_len(stride, height, row_bytes) = stride*(height-1)+row_bytes`。`PixelFormat` は `#[non_exhaustive]`（`pixel.rs`）で `Yuv420p/Yuv422p/Yuv444p`（3-plane 8bit）+ `Rgba8/RgbaF16/RgbaF32`（1-plane packed）の 6 variant — **wire 上の frame 表現は可変 plane 数・per-plane stride・subsampled dims を扱えなければならない**。
- `ColorInfo`（`color.rs`）: `ColorPrimaries`/`TransferCharacteristics`/`MatrixCoefficients`/`ColorRange` 4 enum + optional `HdrMetadata` — `VideoFrame`/`VideoFormat` に乗るため **輸送対象に含めるか否かは仕様論点**（wire codec は現存しない）。
- `FrameRate` は `FrameRate(TimeBase)`（media-core `time.rs` L160）。`MediaTime`/`MediaDuration`/`MediaRange` の wire codec は `wire.rs` L1456-1499 に既存（`TransportInfo` 用）— video `at`/`pts`/`duration`/`frame_rate` も同じ helper 族に載せられる。

#### L0 OFX adapter（`ruviform-plugin-ofx` — L1 が鏡写しにする挙動）

- 取得経路: `OfxHost::load_library` → `OfxLibrary::create_instance(admitted_index)` → `OfxInstance: VideoPluginInstance`（`host.rs` L147/L190・`instance.rs` L104）。in-process E2E は既存（`tests/native_host.rs`・build.rs が実 `.ofx` fixture を生成）。
- `render`（`instance.rs` L397-449）: `Rgba8` のみ受理（L406-409）・in/out dims は config dims と完全一致必須（L411-417）・plane 0 のみ・`at` は finite `f64` 秒へ変換（L419-424）。C bridge は per-render に **16-byte aligned staging を alloc/free** し入力行を pad 付きでコピー、`BeginSequenceRender → Render → EndSequenceRender` ブラケット後に **成功時のみ出力を commit**。frame range は `[time,time]` 単点・render window は全フレーム・scale 1.0。
- 失敗語彙: `transit(LifecycleEvent::Process)` 前置で out-of-state は `IllegalState`（L401-405）・native status は `map_status` — **render 失敗時に出力ピクセルは commit されない**（partial-output なし）。
- 所有・同期性: `&mut self` で逐次・同期完了 — プラグイン側が render 中に別スレッドで output を掴む形はない。host/session lease が render 窓を跨いでインスタンス寿命を保護。

#### Post-#117 wire v2 + shm substrate（再利用可能部分と audio 専有部分）

- フレーミング `[u32 len][u8 kind][payload]`・6 メッセージ（`wire.rs` L25-26・L285+）: `Hello`(version)・`Command`・`Reply`・`BlockIn{seq,instance}`・`BlockDone{seq,instance,ProcessResult}`・`Shutdown`。**video 動詞・video payload は存在しない**。
- 再利用可能な基盤語彙: `(seq, instance)` envelope・scope モデル（`Hello`/`Shutdown`/`Scan` child-scope・他 instance-scope・`WIRE_INSTANCE_NONE=u32::MAX`・`WIRE_MAX_INSTANCES=64` — L103-145）・`WIRE_MAX_*` decode bounds・`HostError` codec・`MediaTime` codec 族。
- audio 専有部分: `Command` 語彙（L358+）は audio lifecycle+`ProcessConfig` 型の `Activate` payload・shm `SLOT_BYTES=558,208` 固定レイアウト（L173-258 — seq/frames/channels/events/transport 96B/result 8B/`f32` planar 16ch×32KiB）・`BlockIn`/`BlockDone` doorbell。
- host shm（`l1/shm.rs` L28-42・L292-309）: **1 インスタンス = 1 named segment**（Bundled でも per-instance — #117 確定不変）、host が create/own/unlink・子が `shm_key` 経由で attach・`Shmem` は `!Send` で driver スレッド上で attach（`l1/mod.rs` L543-552 `AddInstance`）。in/out slot index は `seq % depth`。
- driver ループ（`l1/mod.rs` L1186-1301）: `DriverCmd` channel が全 wire write を直列化 → `drain_submits` が各 instance の submit seq を shm in-slot 書込 + `BlockIn` doorbell 化 → 子の単一 serve loop が `BlockIn` を順に処理し out-slot 書込 + `BlockDone`。**Bundled（既定）では 1 reader + 1 driver per child** — 遅い処理・巨大 payload は全兄弟の HoL ブロック源（#117 design が `RestoreState` 級 multi-MB payload の HoL を既に文書化）。
- `CreateInstance` payload は `{plugin_id, shm_key, in_depth, out_depth}` と **audio segment 前提**（video segment を同時に attach する語彙がない）。`run(factories)`（export `lib.rs` L136）は `Box<dyn PluginFactory>` = `ErasedPlugin` 生成面のみ — video registry/dispatch パスは不在。

#### フレームサイジング（輸送モデル選択の定量入力）

- audio slot: `SLOT_BYTES = 558,208`（~545 KiB・固定）。
- video frame（tightly packed・stride=row_bytes の下限）:
  - 1920×1080 `Rgba8` ≈ **7.9 MiB** / `Yuv420p` ≈ 3.0 MiB / `RgbaF32` ≈ 31.6 MiB
  - 3840×2160 `Rgba8` ≈ **31.6 MiB** / `Yuv420p` ≈ 11.9 MiB / `RgbaF32` ≈ 126.6 MiB
- `WIRE_MAX_PAYLOAD = 16 MiB`（wire.rs L117）— 1080p Rgba8 は pipe に乗るが 4K は乗らない。仮に上限を上げても **pipe 上の multi-MB payload は兄弟の doorbell/command 全てを直列の後ろに並ばせる**（Bundled 単一 reader/driver）。→ **フレーム payload を pipe に載せる輸送は構造的に不適**と観測（量と multiplexing の両面）。
- segment 上限の実用目安: named shm は巨大でも作れるが、per-instance × depth で倍増する — `depth` と max-format/dims の**規格化された上限**（例: `WIRE_MAX_VIDEO_DIMS`・`WIRE_MAX_VIDEO_FRAME_BYTES`）が decode/attach 検証に要る。

### 主要な構造的不整合（この Issue が解く対象）

1. `ErasedPlugin`/`run(factories)`/registry は audio 専用 — video インスタンスを子内で保有・dispatch する **video registry + erased surface が存在しない**。
2. `CreateInstance` が audio shm segment のみを attach する前提 — video の「config 確定後に segment サイズが決まる」鶏卵問題がある（`VideoProcessConfig` は `initialize` で届くが segment は `CreateInstance` で attach される）。
3. `BlockIn`/`BlockDone` は audio block 意味論（`ProcessResult`・transport・events・planar f32）— frame 交換の doorbell 語彙・slot フォーマットがない。
4. video 固有の `initialize(config)`（config 持ち）・`render`・plain param get/set が wire `Command` 語彙に不在。
5. Bundled 既定の単一 serve loop に同期 render（ms~sec 級）を載せると audio 兄弟の doorbell を長時間塞ぐ — **isolation 粒度と data-plane の multiplexing 再評価が要る**。

## Alternatives and Comparison

### 軸 1: フレーム輸送モデル（payload の置き場所）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) **per-instance 専用 video shm segment**（config 確定後にサイズ算出・in/out リング or descriptor+payload 領域・doorbell は pipe）— #117 の per-instance segment 不変式を video にも拡張 | audio と同じ所有/監査/invariant を踏襲（host create/own/unlink・子 attach・`!Send` attach on driver thread）。payload が pipe を汚染しない。slot リング化すれば depth>1 の pipeline 化も自然に表現可能 | 新しい shm layout 族（header 検証・slot フォーマット）が要る。config 未確定で attach できない → attach タイミング語彙（`CreateInstance` 時固定サイズ宣言 or `initialize` 時の第 2 attach）が必要 | **yes**（最有力） |
| (b) `Command`/`Reply` payload にフレーム bytes を直載せ（pipe 輸送） | 新 shm なし・最小 wire 差分 | 16 MiB bound で 4K 表現不能・multi-MB が兄弟全員を HoL ブロック・受信側 Vec alloc 往復。構造的に不適（上記定量観測） | no |
| (c) 既存 audio slot に chunked 分割 | segment 追加なし | slot は audio 意味論に占有・最悪の HoL・SLOT_BYTES 固定前提を破壊 | no |
| (d) descriptor over pipe + payload in shm の hybrid — doorbell を `BlockIn` 類似の新メッセージではなく **`Command`/`Reply` が frame descriptor（seq・dims・format・time・slot index）を運び payload は segment 経由** | (a) と直交しない設計 — doorbell を Command 語語彙に畳むと request/reply 相関が既存 seq に統一できる（新 doorbell kind を増やさない）。`Reply` 到来時点で out-slot 準備済みという単純契約 | descriptor が Command payload 化する分 codec 差分は同等。audio の doorbell/queue パターンと造形がずれる（doorbell は RT 由来の非同期 submit 向き — video は同期 request/response なので Command 畳みの方が自然） | **yes**（(a) とセット — 「descriptor in Command + payload in dedicated segment」が実質の推奨形） |
| (e) child-scope の共有 video segment（1 child = 1 segment・複数 instance で分割） | segment 数を減らせる | #117 の per-instance 所有・scope 帰属・`Failed` 粒度（instance 単位の shm 撤去）と衝突 — 1 instance の `Failed` 収束で segment を unlink すると兄弟を殺す | no |

推奨方向: **(a)+(d)** — per-instance 専用 video segment（host-owned・`shm_key` attach）に frame payload を置き、`Command`/`Reply` が descriptor を運ぶ request/response 形。doorbell を新 kind にするか Command に畳むかは design 決定（descriptor が request 語彙の一部なら新 doorbell kind は不要）。**OPEN — design 確定対象**（attach タイミング: `CreateInstance` payload 拡張で第 2 key を渡すか、`initialize` reply 経路で segment を後付けするか — 後者は「`initialize` 応答後に再度 attach 命令が要る」複雑化）。

### 軸 2: render 契約の形（同期 request/response vs async queue）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) **同期 request/response** — `RenderVideo{seq, at, in-slot 記述}` → `Reply`（成功=出力準備済み）1 対 1。呼出し側は既存 waiter/timeout 経路に乗る | L0 `render` の `&mut self` 同期契約をそのまま鏡写し — per-instance FIFO が自然成立・probe 側の acquire/release 語彙最小 | render が `command_timeout`（既定 10s）を超える重い filter で timeout→`Failed` 化し得る — video 用に長め or 可設定な timeout が要る | **yes**（v1 基本形） |
| (b) async submit queue — submit（frame pool slot 確保）→ 後続 `VideoDone` doorbell で完了通知・呼出し側はノンブロッキング | pipeline 化・長い render でも制御面を塞がない | L0 契約にない非同期待機 surface（完了 read API・in-flight 管理）を host 側に新設 — v1 では過剰、depth>1 ring で将来拡張余地を残せる | maybe（depth>1 ring の設計余地は残す） |
| (c) `process` 系 audio doorbell への fold | 語彙最小 | audio 意味論（ProcessResult・pass-through）を video に引きずる — 全く別契約 | no |

推奨方向: **(a)**。timeout は video 用に分離（`render_timeout` 相当 — 既定値・設定経路は design/spec 論点）。**OPEN — 確定要**（timeout 値の所有: SpawnSpec? 固定?）。

### 軸 3: wire 動詞・registry 形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 既存 `Command` 語彙に video 動詞を追加（`VideoInitialize{VideoProcessConfig}`・`VideoRender{...}`・`GetParam`/`SetParam`/`GetParamInfo` 等）+ `CreateInstance` を instance-kind タグ付き or `CreateVideoInstance` 新動詞で分岐 | 単一 registry/envelope/scope モデルを共有・`Failed` 粒度・seq 相関がそのまま適用 | audio と video の entry が同一 registry に同居 → 動詞×kind の dispatch 表が要る。`ErasedVideoPlugin` という第二の erased 表面が要る | **yes** |
| (b) `Command` とは別の新メッセージ kind 群（`VideoCommand`/`VideoReply`） | audio wire を無変更に保てる | envelope/demux/failure 機構を二重化 — `Failed`・scope・waiter を二系統で保守する構造問題 | no |
| (c) 別プロセス・別 pipe の video 専用子（専用 runner プロトコル） | isolation 完全 | `PluginHost` route・spawn/pool・通知を全面二重化 — #117 の統一 handle/pool モデルを割る | no |

推奨方向: **(a)** — `CreateVideoInstance`（または `CreateInstance{kind}`）+ video registry entry を runner に持たせ、`initialize`（config 持ち・audio `Activate` と別動詞）・`render`・`param get/set/info`・`SaveState` 非対応（trait に存在しない）を video 動詞集合として定義。`ErasedPlugin` 相当の video erased surface（`ErasedVideoPlugin` — `HostError` 返しの dyn 安全ラッパ）が plugin-core に要る点は不可避。**OPEN — design 確定対象**（verb 名・kind タグ方式・`Scan` reply への video capability 記述）。

### 軸 4: `Failed`/crash 意味論（#117 契約の video への写像）

| 観点 | 確定写像（confirmed — #117 のまま持ち越せる） |
| --- | --- |
| 子死亡/pipe 破壊/プロトコル違反（帰属不能） | 子スコープ terminal → 全 live インスタンス `Failed` + pending video waiters `Err(Ipc)` + `InstanceFailed` fan-out |
| mid-render 死亡 | render waiter は `Err(Ipc)` wake・out-slot 内容は未定義（reader が読まない）|
| mid-`CreateVideoInstance` 死亡 | 通知なし・`Err(Ipc)` で instantiate 失敗（handle 未 mint — #117 AC12 と同型）|
| mid-`DestroyInstance` 死亡 | `Failed` 収束 + 通知 + `Destroy` は `Ok`（slot 解放 — 同型）|
| 帰属可能違反（in-flight seq 不一致の video doorbell/done 等） | 対象 instance のみ `Failed`・兄弟生存 |
| inert 規則 | post-removal/`Destroying`/`Failed` 宛の遅延 frame は静かに drop |
| wedge（render が返らない）| command timeout → `Failed` 収束 + kill — video timeout 値を別管理にするかは軸 2 の OPEN |

**OPENS**: (i) video doorbell/done の帰属判定を audio `block_in_flight` 相当の別 map で持つか、(ii) render が timeout した時点で shm in/out slot を「汚染済み」として instance を `Failed` にするか再試行可能にするか（audio は doorbell timeout → Failed 収束 — video も同型が自然だが NONRT なので「遅い render」への猶予幅の置き所は design）。

### 軸 5: flow control・順序・バッファ所有

| 観点 | 選択肢 | Lean |
| --- | --- | --- |
| 順序 | per-instance FIFO のみ保証（`&mut self` と serve loop 逐次で自然成立）。跨 instance の順序は保証しない（#117 と同じ） | **confirmed** |
| pool depth | (i) depth=1 固定（render は同期・次 frame は reply 後にしか積めない）/ (ii) depth≥2 ring（pipeline 余地 — 軸 2(b) 将来拡張） | (i) で十分 — **depth は slot ring 構造に合わせ design で固定値として定義**（depth=1 なら in-slot/out-slot 各 1 領域でよい） |
| 呼出し側の満杯時挙動 | 同期 render なので「満杯」は pool slot 未解放時のみ — block して待つか `Err` か | 同期表面なら **block**（NONRT なので待機許容 — audio の try-脚とは根本的に違う点）or timeout 付き wait |
| バッファ所有 | host-owned segment に in-frame を書き、子が out-slot に書き、`Reply` で done — **slot 上の有効期間は request→reply の窓のみ**。host 側 `VideoFrame` は copy-in/copy-out（`Arc` 跨ぎ不可） | **confirmed**（`Arc` 制約上 zero-copy 跨ぎは不可 — 軸 7 参照） |
| plane 幾何 | wire 上の frame = `{format,w,h,color,pts,duration, plane_count, [ {stride,offset,len}×N ], data…}`。stride は explicit 記述子（pad 保持）か canonical tightly-packed（stride=row_bytes・受信側で必要に応じ再 pad）か | **explicit stride descriptor 推奨**（adapter が 16B-align 等を自分で要求可能・round-trip で stride を保持 — ただし canonical packed でも要件は満たせる。design で確定） |

### 軸 6: Bundled isolation・HoL 相互作用

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) video も既存 serve loop/reader に載せる（Bundled のまま） | 機構最小 — envelope/demux/convergence をそのまま使う | 遅い render・巨大 descriptor/payload が兄弟 audio インスタンスの doorbell を HoL ブロック — `#117` の既知トレードオフを悪化 | maybe — **専用の運用ガードとセットなら**（下記 (c)） |
| (b) video route は Dedicated 既定/強制 | video child が他 instance と serve loop を共有しない — HoL 問題を構造的に消す | `SpawnSpec.isolation` 既定 Bundled を video だけ変えるのは公開 API 既定値の分岐 — spec/design で「video route の isolation 既定」を別定義する必要 | **yes 推奨**（少なくとも初版は video route = Dedicated のみ許可、Bundled video は明示 opt-in か非対応、という形が安全） |
| (c) reader/driver の video/audio 分離（別 channel・reader partition） | Bundled でも HoL を局所化 | L1 構造の大改造 — 単一 pipe の multiplexing・waiter 複数化・driver の両 queue 処理 — design 範囲が大きく増える | no（v1） — ただし deferred 表記録 |
| (d) 子側 render ワーカスレッド（serve loop 外で render 実行） | serve loop を塞がない | runner の single-threaded 前提破壊・`ErasedVideoPlugin` に `Send` 要求が波及 — `#117` design の liveness 議論と直交するが別 Issue 級 | no（v1） |

推奨方向: **(b)（Dedicated 既定/強制）を主案・(a) を opt-in 許容として spec/design に提示 — 人間判断を仰ぐ論点**（公開既定値・Bundled video 許可の是非）。

### 軸 7: GPU/texture・zero-copy・format/色域スコープ

| 論点 | 観測 | Lean |
| --- | --- | --- |
| GPU/texture interop | ADR `ofx-video-plugin-scope` D2-a + `ofx-host-video-contract` で GPU suites は**後続 Issue に確定延期済み**。`SharedPlane`/`Arc` 制約上、shm 跨ぎ zero-copy は現行型で表現不能 | **v1 範囲外 — deferred 表へ**（trigger: GPU suite 需要・media-io decoder 側の shared-buffer 要件） |
| 真の zero-copy（shm→`VideoFrame` 直接 view） | `from_shared_planes` は `Arc<[u8]>` 必須 — shm 領域は Arc 化不可。真 zero-copy には video-core の backing 抽象拡張（borrowed-plane コンストラクタ or unsafe 局所化）が要る | **v1 は copy-in/copy-out**（NONRT・1-2 コピーは許容域）。backing 抽象の拡張は deferred |
| format 集合 | wire は全 `PixelFormat` variant（6種・`#[non_exhaustive]`）を表現可能にする（tag + plane 記述）— **adapter が何を受理するかは negotiate 次第**。OFX L0 は `Rgba8` のみ | **wire は format-agnostic・初期 E2E は Rgba8 のみ pin** |
| stride/alignment | per-plane explicit stride descriptor で丸め問題を adapter 側に残す（軸 5） | 推奨同上 |
| `ColorInfo` | `VideoFrame` の構成要素 — wire codec 要（4 enum + optional HdrMetadata）| **輸送対象に含める**（frame 意味を失わせない） — codec 新規必要・design で tag 定義 |
| dims 上限 | `u32` dims は攻撃面（`u32::MAX` dims で巨大 segment 要求）→ `WIRE_MAX_VIDEO_*` で decode/attach 時検証 | **bounded 必須** — 具体的 cap 値（16K? 512MiB?）は spec/design で確定 |

### 軸 8: param チャネル

- `parameters()`（id 一覧）・`parameter(id)`（`PlainParamInfo` — name/unit/String choices・f64 domain）・`parameter_value`/`set_parameter` — 全て wire verb 化が要る（audio wire に不在のため新規 codec）。
- `PlainParamInfo` は `String` choices を含む → wire 表現は owned 化（`WIRE_MAX_STRING_BYTES`/elements 制約を適用）。
- `set_parameter` の lifecycle 意味は adapter 責務（core doc L88-90）— wire は素通し。
- **推奨**: `GetParams`/`GetParamInfo`/`GetParamValue`/`SetParamValue` を instance-scope verb として追加 — 詳細 shape は design。

## Tentative Conclusions and Hypotheses

- **confirmed**: video L1 は **「audio wire v2 基盤語彙（envelope/scope/seq/Failed/notify/pool）を共有しつつ、data-plane・verb・registry・erased surface は video 専用の新系統」**という二層構成が構造整合的 — audio wire を「拡張」ではなく「同梱する新面」として扱う。
- **confirmed**: frame payload は pipe に載せられない（size + multiplexing）— **per-instance 専用 shm segment への descriptor+payload 分離が唯一の構造整合解**（軸 1）。audio `BlockIn` 型 doorbell か `Command` descriptor 畳みかは design。
- **confirmed**: `CreateInstance` 時点では frame サイズが未確定（config は `initialize` で届く）→ **video segment の attach は `initialize` 前後の何らかの語彙で行う必要がある**（`CreateInstance` 拡張で上限宣言→固定 segment・または `initialize` 後の attach 命令 — **OPEN・design 確定対象**）。
- **hypothesis**: request/response 同期 render + depth=1 slot 対が v1 の最简形 — `command_timeout` 分離（video timeout）と「timeout=Failed」の写像は audio と同型で足りる。
- **hypothesis**: video route は **Dedicated のみ許可**（または Bundled opt-in で HoL 文書化）が初期の安全側 — Bundled+video で HoL を受け入れるかは **人間判断を仰ぐ**。
- **confirmed**: GPU/zero-copy/texture interop は初期範囲外（上流 ADR 確定済み）— v1 は copy-in/copy-out。将来の backing 抽象（shm direct-view）は video-core 側の別 Issue 級拡張。
- **confirmed**: `WIRE_VERSION` bump（2→3）が必要 — 新動詞・新 shm layout・`WIRE_MAX_VIDEO_*` bound は全て bump 内に閉じ込める（#119 単一最新方針）。

## Open Questions

1. **輸送モデルの最終形** — per-instance 専用 segment + descriptor-over-Command の推奨形でよいか、それとも audio doorbell 型（`VideoFrameIn`/`VideoFrameDone` 新 kind）を取るか → **design/ADR 確定**（本書では両案を spec の R-F 行として提示）。
2. **Bundled video 許可の是非** — video route を Dedicated に限定するか、Bundled でも HoL 許容で許可するか → **人間判断を仰ぐ**（公開既定値に関わる）。
3. **segment attach タイミング** — `CreateInstance` 時に最大サイズ宣言（静的 cap）か、`initialize`（config 確定）後の第 2 attach か → design 確定対象。
4. **render timeout** — audio `command_timeout`（10s）とは別の video 専用 timeout（値・設定経路・SpawnSpec 拡張の有無）→ spec/design 確定対象。
5. **pool depth>1 / async 完了の将来拡張余地を残すか**（slot ring 形状に depth 字段を入れるか）→ design。
6. **`Scan`/capability で video factory をどう広告するか** — `Reply::Scanned` descriptor に kind/能力記述を足すか → design。
7. **GPU/zero-copy/backing 抽象** — deferred で確定か、初版から抽象を敷くか → **人間判断を仰ぐ**（上流 ADR は deferred 確定なので「deferred 維持」が既定結論）。
8. **video param の wire 語彙粒度** — `parameters()` 一覧 + `parameter()` info + value get/set を個別 verb にするか、`GetParamSet` のような集約 reply にするか → design。
9. **`PluginHost` 統合面** — `Instantiate{plugin_id}` が video route を引けるよう `HostCommand`/`HostReply` に video 動詞を足すか、video 専用 host 表面（`VideoHost` 相当）を別立てするか → **人間判断を仰ぐ**（公開 API 形状）。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-118-video-l1.md` — video L1 経路の振る舞い・invariant・AC を定義（本書の下流として作成済み・design 決定点は R-F 行で提示）
- [ ] adr-doc needed: video frame transport（per-instance segment + descriptor/doorbell 形）・video isolation 既定値（Dedicated/Bundled）— design フェーズで確定・公開契約に関わるため ADR 候補
- [ ] design-doc needed: wire v3 codec・video shm layout・runner registry・driver/reader 統合点・failed 帰属表 — Issue #118 の設計確定文書
- [ ] test-spec needed: wire codec round-trip/bounds・video E2E（mock video fixture runner）・`Failed` 粒度パリティ

## Sources and Relations

- Issue [#118](https://github.com/scarecrow173/ruviform/issues/118)（本探索対象）・upstream: [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 本体・CLOSED）・[#113](https://github.com/scarecrow173/ruviform/issues/113)（engine 統合・video 境界明文化・CLOSED）・[#114](https://github.com/scarecrow173/ruviform/issues/114)（実アダプタ L1 E2E — OFX disposition で本 Issue を trigger 化・CLOSED）・[#117](https://github.com/scarecrow173/ruviform/issues/117)（bundle 子・wire v2・CLOSED）・[#119](https://github.com/scarecrow173/ruviform/issues/119)（wire バージョン方針・CLOSED）
- 一次文書: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（L313 deferred 行 — 本 Issue の trigger）・[plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)・[issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md)・[issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md)・[issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)・[engine-plugin-host-integration.md](../designs/engine-plugin-host-integration.md)（決定 `k` L285-289）・[ofx-host-video-contract-and-c-abi.md](../adr/ofx-host-video-contract-and-c-abi.md)・[ofx-video-plugin-scope.md](../adr/ofx-video-plugin-scope.md)（D2-a — GPU 後続確定）・[issue-114-real-adapter-l1-e2e.md](issue-114-real-adapter-l1-e2e.md)（L81-82/L101/L157-163/L181 — OFX は video wire 不在で L1 不可・#118 追跡確定）・[issue-76-l1-shm-data-plane-task-4-implementation.md](../impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md)（Non-Goals — video 経路・上限引上げ・可変 slot が本 Issue の前提作業として明示的に deferred 済み）
- ローカル一次証拠: `crates/ruviform-plugin-core/src/video.rs`（`VideoPluginInstance` L94-141・`VideoProcessConfig` L22-73）・`crates/ruviform-plugin-core/src/wire.rs`（L103-145 envelope/bounds・L173-258 shm 定数・L285+ メッセージ・L358+ Command）・`crates/ruviform-plugin-host/src/l1/shm.rs`（L28-42 header・L292-309 slot index）・`crates/ruviform-plugin-host/src/l1/mod.rs`（L495-558 `L1Child`/`DriverCmd`・L1186-1301 driver loop・L2244-2403 `execute`/`request`）・`crates/ruviform-plugin-export/src/lib.rs`（`run(factories)` L136・serve L182-・registry L315-・`process_block` L425+）・`crates/ruviform-video-core/src/frame.rs`/`pixel.rs`/`plane.rs`/`color.rs`・`crates/ruviform-plugin-ofx/src/instance.rs`（L397-449 render・staging/bracket）・`crates/ruviform-plugin-ofx/src/host.rs`（L147/L190 取得経路）・`crates/ruviform-plugin-core/src/boundary.rs`（`HostError` — `EndpointUnavailable` は wire 非対応の local-only 語彙 L155-171）

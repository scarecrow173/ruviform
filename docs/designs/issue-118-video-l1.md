---
id: "DESIGN-034NVCweXy0wECErcpEcv8"
type: "design"
status: "draft"
title: "video/NONRT プラグインの L1 経路設計（Issue #118）"
created: "2026-10-29"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/98"
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
  implements:
    - "docs/specs/issue-118-video-l1.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/issue-116-supervisor.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
    - "docs/adr/ofx-video-plugin-scope.md"
    - "docs/adr/plugin-isolation-strategy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-118-video-l1.md"
    - "docs/discovery/issue-118-video-l1.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
    - "docs/adr/ofx-video-plugin-scope.md"
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
    - "https://github.com/scarecrow173/ruviform/issues/131"
  defers: []
  deferred-by:
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/engine-plugin-host-integration.md"
---

# video/NONRT プラグインの L1 経路設計（Issue #118）

## Context

Issue #118 — audio `process` ブロック経路とは別の **NONRT video データ面**を L1 wire/shm に確立する。`VideoPluginInstance`（`crates/ruviform-plugin-core/src/video.rs` L94-141 — 全操作 NONRT の別 trait）を持つプラグインが L1 子プロセス内で spawn・lifecycle・render できるようにする。wire v2 の動詞は audio lifecycle + audio doorbell（`BlockIn`/`BlockDone`）のみで、shm slot は planar `f32` audio 固定レイアウト（`wire.rs` `shm` mod L172-263）であり、video を載せる語彙・data-plane・registry・host 表面の全てが不在である。

本書は spec `docs/specs/issue-118-video-l1.md` の R-F 決定点と discovery `docs/discovery/issue-118-video-l1.md` の Open Questions を確定する。上流で人間判断済みの **confirmed 前提**（再決定しない）:

1. **輸送モデル（discovery Q1 / spec R-F1）**: per-instance 専用 video shm segment + descriptor-over-`Command`/`Reply`。frame payload を pipe に載せる形は構造的に不適（4K `RgbaF32` ≈ 126.6 MiB ≫ `WIRE_MAX_PAYLOAD` 16 MiB・multi-MB payload は単一 reader/driver 配下の兄弟 doorbell/command を HoL ブロック — discovery 軸 1 (b)/(c) 却下の定量根拠）。
2. **isolation（discovery Q2 / spec R-F4）**: video-capable instance は **Dedicated のみ**。`Bundled` + video は defined な形で拒否（遅い render が兄弟 audio doorbell を塞ぐ HoL リスクの排除）。
3. **host API（spec R-F5）**: **別立て video host 表面**。audio `HostCommand`/`Instantiate` には video 動詞を足さない。
4. **GPU/zero-copy**: deferred。`VideoFrame` の plane backing は `Arc<[u8]>` で shm 領域を包めないため v1 は **copy-in/copy-out**（`frame.rs` L232-251 `from_shared_planes` 制約）。
5. **`WIRE_VERSION` 2→3**: #119 単一最新方針 — 新動詞・新 shm layout・`WIRE_MAX_VIDEO_*` は bump 内に閉じ込める。negotiation なし。
6. **`Failed` parity**: #117 確定契約（子スコープ terminal + 帰属可能 per-instance 収束 + inert 規則 + `InstanceFailed` fan-out + mid-verb `Err(Ipc)` 写像）を video 経路に verbatim 適用（ADR `docs/adr/issue-117-bundle-child-failed-granularity.md`）。

確定の入力となる現行実装の事実:

- **wire**: `Message` 6 kind・`Command` tag 0-11（`Scan`=0 … `DestroyInstance`=11）・`Reply` tag 0-3（`Ok`/`Scanned`/`State`/`Opened`）・envelope `{seq:u64, instance:u32}`・`WIRE_INSTANCE_NONE=u32::MAX`・`WIRE_MAX_INSTANCES=64`・`WIRE_MAX_PAYLOAD=16MiB`・全 decode bounded（`wire.rs` L103-145・L358-441）。
- **host**: `L1Child` が子 + driver/reader スレッド + `Shared`/`Inner`（waiters・`instances` map・terminal・shutdown）を所有し、`L1Instance` facade が `request`/`execute`/mirror/notify/RT leg/audio segment を持つ（`l1/mod.rs`）。`Shmem` は `!Send` — 子側 attach は runner 内、host 側は owner `Segment`（control スレッド所有可・`PluginHost` は `!Send`/`!Sync`）か `View` attach（I/O スレッド）（`l1/shm.rs` L253-273）。`create_instance` は **host が segment を先に create** し `shm_key` を `CreateInstance` payload で子へ渡す（`l1/mod.rs` L1746-1875）。
- **runner**: `run(factories)` + `Runner{factories, instances}` + `InstanceEntry{instance, lifecycle, segment, …}` が audio 専用。`CreateInstance` で plugin id 解決 → attach → `create()` → insert（`export/lib.rs` L300-445）。
- **composite**: `PluginHost` が `routes: HashMap<String, Route>`・unified slot/generation table・`try_recv_notification` fan-out・`prepare_endpoint` を持ち、`HostCommand` は audio 形のみ（`composite.rs`・`command.rs`）。
- **video 契約**: `VideoProcessConfig{format,width,height,frame_rate}`（非零 dims のみ検証 — `video.rs` L22-73）・`render(&mut self, input:&VideoFrame, output:&mut VideoFrame, at:MediaTime)` 同期・plain param チャネル（`PlainParamValue`/`PlainParamInfo` — wire codec 不在）・`VideoFrame` = format+dims+pts+opt duration+`ColorInfo`+planes（per-plane `{data,stride,width,height,sample_bytes}`）。L0 OFX は `Rgba8` のみ・in/out dims は config dims 一致・render 失敗時は出力不 commit（`instance.rs` L397-449）。
- **supervisor**: `RestartRecipe` は audio 形（`ProcessConfig`/`start_processing`/endpoint_bounds — `supervisor.rs` L148-191）。video recipe は別論題。

## 決定サマリ（spec R-F / discovery OPEN → 本書の結論）

| # | 論点 | 決定 |
| --- | --- | --- |
| 1 | フレーム輸送（R-F1・Q1 — **confirmed**） | **per-instance 専用 video segment + descriptor-over-`Command`/`Reply`** — `VideoRender{at, input:VideoFrameDesc}` → `Reply::Rendered{output:VideoFrameDesc}`、plane bytes は segment の in/out 領域。新 doorbell kind は作らない（request/reply 相関は既存 seq で十分 — 同期 render なので doorbell の非同期利点がない） |
| 2 | isolation（R-F4・Q2 — **confirmed**） | **Dedicated 固定・構造的 pin** — `VideoOutOfProcessSpec` に `isolation` field を**持たせない**（Bundled video を公開面上で表現不能にする = 最強の defined 拒否）。pool 機構は video route に存在しない。詳細は ADR `issue-118-video-l1-host-surface` |
| 3 | host 表面（R-F5・Q9 — **confirmed 方向 + 本書で確定**） | **`PluginHost::video(&mut self) -> VideoPluginHost<'_>` 借用 façade** + `VideoHostCommand`/`VideoHostReply`（両 `#[non_exhaustive]`）。audio `HostCommand`/`execute` 無変更。slot/generation/notify 機構は共有（handle は全 surface で一意）。詳細は ADR `issue-118-video-l1-host-surface` |
| 4 | plugin_id 名前空間 | **単一 namespace・route kind タグ** — `Route::OutOfProcessVideo` を追加し `plugin_id` の audio/video 重複登録は `Err(DuplicatePluginId)`（双方向）。audio `Scan`/`Instantiate` は video route を見せず（cross-kind `Instantiate` → `Err(UnknownPluginId)`）、逆も同型。1 id = 1 route = 1 kind の不変式を維持 |
| 5 | create-path / segment sizing（Q3 — **本書+ADR 確定**） | **`CreateVideoInstance{plugin_id, shm_key, config}`** — `VideoProcessConfig` を create 動詞に載せ、host が config から segment をサイズ算出→create→key を同送。`VideoInitialize` は payload なし（runner が stash した config を `initialize(&config)` hook に再生）。遅延 attach（initialize 後の第 2 attach 動詞）と固定最大 segment は却下。詳細は ADR `issue-118-video-l1-create-path-segment` |
| 6 | render 形（Q1 副次） | **同期 request/response・per-instance FIFO・in-flight 1 本** — `&mut self` 鏡写し。reply 到達時のみ out-slot を読む（commit-on-success・L0 OFX 写像）。timeout 時の slot 汚染は instance `Failed` 収束で無効化（読まれない） |
| 7 | render timeout（R-F2・Q4） | **`VideoOutOfProcessSpec.render_timeout`（既定 60 s）** — `command_timeout`（既定 10 s）は handshake/lifecycle/param 動詞に、`render_timeout` は `VideoRender` reply wait にのみ適用。timeout→`Err(Ipc)`+子 `Failed` 収束は `command_timeout` と同型（wedge は帰属不能として扱う #117 規則）。waiter に per-command deadline を載せる実装（`request` の timeout 引数化） |
| 8 | segment 深さ（R-F3・Q5） | **depth=1 固定**（in 領域 1・out 領域 1）— 同期 render では次 frame は reply 後にしか書けない。slot ring/depth 字段は持たせない（v1 の語彙に存在しない概念を入れない）。将来の depth>1 は wire 面ではなく v4 の論点 |
| 9 | wire 動詞集合（Q8） | tag 12-22 の **video 動詞** + `Scan`/`DestroyInstance` の**共有動詞 reuse**（下表）。audio 動詞（`Initialize`/`Activate`/`SaveState`/`RestoreState`/GUI）を video entry に送ると `Err(Unsupported)`、逆も同型（runner kind とは別の動詞系統を受けた場合の defined 拒否） |
| 10 | `Failed` parity | **#117 契約 verbatim** — `Inner.instances`/`InstPhase`/waiter/notify/`converge_child` 機構は kind 非依存で再利用。帰属可能違反は「live instance 宛・well-formed で instance 一意帰属の frame」のみ（下の確定契約表） |
| 11 | `Scan`/capability（R-F6・Q6） | **kind 記述を wire に足さない** — `Hello.capabilities` は v3 でも reserved-zero 維持。video 子の `Scan` は video factory の descriptor を返し、host 側は `spec.descriptor` field-match（既存機構）で kind 混同を検出（audio runner に video spec を spawn すると descriptor 不一致 → spawn 失敗 `Err(Ipc)`）。route kind は host 側のみの属性 |
| 12 | runner 形 | **別 runner entry `run_video(factories)`** + video registry — `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video` を plugin-core に新設。`run`/audio registry 無変更。子は kind 単一（Dedicated で 1 子 1 instance — 混在 registry は構造上起きない） |
| 13 | test fixture | **新規 `l1-test-video-worker`**（audio fixture 無変更で R-B6 を構造的に保証）。fault flag は audio fixture と同族 + video 固有（下記 Verification） |
| 14 | supervisor | **v1 範囲外・明記 deferred** — `RestartRecipe` は audio 形。video recipe 変種は supervisor 側拡張論題（spec Out of Scope どおり・core 自律再起動なし契約は不変） |

## Architecture

### wire v3 プロトコル（`ruviform-plugin-core::wire`）

`WIRE_VERSION: u16 = 2 → 3`。framing（`[u32 len][u8 kind][payload]`）・6 `Message` kind・envelope `{seq,instance}`・scope 規則（`WIRE_INSTANCE_NONE` child-scope・`< WIRE_MAX_INSTANCES` instance-scope）・`HostError`/`LifecycleError`/`PrepareError` codec・`MediaTime`/`MediaDuration` helper 族は**無変更**。audio `shm` mod 定数・`BlockIn`/`BlockDone` doorbell も無変更（video は doorbell を使わない — `Command`/`Reply` のみ）。

#### 新規 `Command` 動詞（tag 12-22 — v2 の続き）

| tag | `Command` variant | payload | → `Reply` | scope |
| --- | --- | --- | --- | --- |
| 12 | `CreateVideoInstance` | `{plugin_id: String, shm_key: String, config: VideoProcessConfig}` | `Ok` | instance（envelope = mint する handle） |
| 13 | `VideoInitialize` | — | `Ok` | instance |
| 14 | `VideoActivate` | — | `Ok` | instance |
| 15 | `VideoStartProcessing` | — | `Ok` | instance |
| 16 | `VideoRender` | `{at: MediaTime, input: VideoFrameDesc}` | `Rendered{output: VideoFrameDesc}` | instance |
| 17 | `VideoStopProcessing` | — | `Ok` | instance |
| 18 | `VideoDeactivate` | — | `Ok` | instance |
| 19 | `VideoGetParams` | — | `Params{ids: Vec<u32>}` | instance |
| 20 | `VideoGetParamInfo` | `{id: u32}` | `ParamInfo{info: Option<PlainParamInfoOwned>}` | instance |
| 21 | `VideoGetParamValue` | `{id: u32}` | `ParamValue{value: PlainParamValue}` | instance |
| 22 | `VideoSetParamValue` | `{id: u32, value: PlainParamValue}` | `Ok` | instance |

#### 共有動詞（tag 変更不要 — video runner も同じ codec で解釈）

| tag | `Command` | video での意味 |
| --- | --- | --- |
| 0 | `Scan` | video factory の descriptor 列 → `Scanned`（子は kind 単一 — audio/video factory を 1 子が混載する形は v1 では存在しない） |
| 11 | `DestroyInstance` | video entry の normalize + registry 解放 → `Ok`（audio と同一 contract — `Failed`/`Destroying` entry 宛も許可） |

audio 動詞（tag 1-10）と新 video 動詞は**同じ `Command` enum の variant**として追加（#119 方針の additive bump — 既存 tag・意味は不変）。kind 不整合の defined 拒否: audio runner が tag 12-22 を、video runner が tag 1-10 を受けた場合、**entry dispatch の catch-all で `Err(HostError::Unsupported)`**（entry が登録済みなら — 未登録 handle は先に `Err(InvalidHandle)`、registry 動詞 `CreateVideoInstance`/`CreateInstance` の cross-kind も同じ `Unsupported` 拒否で子は死なない）。

#### 新規 `Reply` variant（tag 4-7 — v2 の続き）

| tag | `Reply` variant | payload |
| --- | --- | --- |
| 4 | `Rendered` | `{output: VideoFrameDesc}` — `VideoRender` 成功の出力 descriptor（plane bytes は out 領域に準備済み） |
| 5 | `Params` | `{ids: Vec<u32>}` — `parameters()` の `ParamId` 列（`WIRE_MAX_ELEMENTS` 上限） |
| 6 | `ParamInfo` | `{info: Option<PlainParamInfoOwned>}` — `parameter(id)` の `Some`/`None` 写像（未知 `ParamId` は error でなく `None` — L0 `Option` 契約の正直な写像） |
| 7 | `ParamValue` | `{value: PlainParamValue}` — `parameter_value(id)` 成功値（失敗は `Err(PluginFailed)` — plugin error の正規化先） |

#### wire 型 codec（新規 — 全て bounded）

- **`VideoProcessConfig`**（13 B 固定）: `u8 format` + `u32 width` + `u32 height` + `FrameRate{u32 numerator, u32 denominator}`。decode 検証: `width/height ∈ [1, WIRE_VIDEO_MAX_DIM]`・`format` tag ∈ 0-5・`frame_rate` の num/den 非零（`FrameRate::new` の不変式）。違反は `Malformed`/`UnsupportedVariant` 系の defined 拒否
- **`PixelFormat` tag**: 宣言順 `0=Yuv420p, 1=Yuv422p, 2=Yuv444p, 3=Rgba8, 4=RgbaF16, 5=RgbaF32`（`#[non_exhaustive]` — 未知 tag は `UnsupportedVariant`/`Malformed` defined 拒否、audio enum codec と同じ規則）
- **`VideoFrameDesc`**（可変長・≦ 127 B）: `u8 format` + `u32 width` + `u32 height` + `u8 plane_count` + `plane_count × {u32 offset, u32 len, u32 stride}` + `MediaTime pts` + `u8 has_duration` +（present なら）`MediaDuration duration` + `ColorInfo`。decode 検証: dims/format 上記・`plane_count ∈ [1, WIRE_VIDEO_MAX_PLANES]` かつ `format.plane_count()` と一致・各 `stride ≤ WIRE_VIDEO_MAX_STRIDE`・canonical 配置（下記「canonical packing」）・pts/duration の TimeBase 非零。field order/premult/clip-range は `VideoFrame`/`VideoProcessConfig` が model しないため v1 wire にも載せない（追加は v4 論点）
- **`ColorInfo`**: `u8 primaries` + `u8 transfer` + `u8 matrix` + `u8 range`（各 enum は宣言順 tag・`#[non_exhaustive]` 未知 tag は defined 拒否）+ `u8 has_hdr` +（present なら）`u8 has_mastering_display` + `MasteringDisplayInfo`（`3×{u16 x,u16 y}` + `{u16,u16}` white point + `u32 max_lum` + `u32 min_lum` = 24 B）+ `u8 has_content_light_level` + `{u16 max_cll, u16 max_fall}`（4 B）
- **`PlainParamValue`**: `u8 kind tag`（宣言順 `0=Double:f64, 1=Integer:i32, 2=Boolean:u8, 3=Choice:u32`）+ payload。`Double` は **finite 必須** — decode で NaN/Inf を `Malformed` 拒否（境界検証の原則: 非有限値を wire に流さない — `plain_param.rs` L8-10「finiteness は boundary 責務」に一致）
- **`PlainParamInfoOwned`**（新規 public 型 — `plain_param.rs` に追加・`PlainParamInfo<'_>` の owned 対）: `u32 id` + `String name` + `String unit` + `f64 min_value` + `f64 max_value` + `f64 display_min` + `f64 display_max` + `PlainParamValue default_value` + `u32 flags`（`ParamFlags::bits`）+ `Vec<String> choices`（`WIRE_MAX_ELEMENTS` 件以下・各 `WIRE_MAX_STRING_BYTES` 以下）。全 f64 は finite 必須・`min ≤ max`・`Choice` 時 `choices` 非空で index in-range、他 kind では `choices` 空 — `PlainParamInfo` doc 契約（`plain_param.rs` L36-43）を decode 検証として写す

#### video 固有 bound（`WIRE_MAX_VIDEO_*` — decode/attach 両層で適用）

| 定数 | 値 | 根拠 |
| --- | --- | --- |
| `WIRE_VIDEO_MAX_DIM` | `8_192` | width/height の次元上限。8K UHD（7680×4320）を含み、u32 攻撃面を遮断（dims は segment サイズ計算の乗数） |
| `WIRE_VIDEO_MAX_PLANES` | `4` | 現行 `PixelFormat` の最大 3 plane + headroom。desc 配列長の上限 |
| `WIRE_VIDEO_MAX_STRIDE` | `262_144`（256 KiB） | 8K `RgbaF32` row_bytes = 131,072 B の 2 倍の padding 許容。canonical packing では stride = row_bytes で運ぶため実効上は row_bytes 上限と同等 |
| `WIRE_VIDEO_MAX_FRAME_BYTES` | `268_435_456`（256 MiB） | 1 frame の plane bytes 合計上限。4K `RgbaF32` ≈ 126.6 MiB・8K `Rgba8` ≈ 31.6 MiB・8K `RgbaF16` ≈ 253.1 MiB を収める（8K `RgbaF32` ≈ 506 MiB のみ超過 — v1 上限として正直に宣言） |
| `WIRE_VIDEO_MAX_SEGMENT_BYTES` | `28 + 2 × WIRE_VIDEO_MAX_FRAME_BYTES`（≈ 512 MiB） | `vshm::segment_len` の絶対上限 — segment create 前の early 拒否と attach 時の declared_len 検証に使う |

byte math（canonical packing — stride = row_bytes、plane は region 先頭から連続）:

```text
plane_row_bytes(p)  = plane_width(p)  × bytes_per_sample(p)
plane_height(p)     = height  (p=0) | ceil(height/sub_h)  (chroma)
plane_width(p)      = width   (p=0) | ceil(width/sub_w)   (chroma)
plane_len(p)        = plane_row_bytes(p) × plane_height(p)      // stride==row_bytes → required_len == stride*h
frame_bytes         = Σ_{p<plane_count} plane_len(p)
segment_len         = vshm::HEADER_LEN(28) + in_bytes + out_bytes
                    where in_bytes = out_bytes = frame_bytes(config)
例: 3840×2160 Rgba8   → 33,177,600 B/slot、segment ≈ 63.3 MiB
    3840×2160 Yuv420p → 12,441,600 B/slot、segment ≈ 23.7 MiB
    1920×1080 Rgba8   →  8,294,400 B/slot、segment ≈ 15.8 MiB
```

`CreateVideoInstance` 前の host 側 early 検証: `frame_bytes(config) ≤ WIRE_VIDEO_MAX_FRAME_BYTES`・`segment_len ≤ WIRE_VIDEO_MAX_SEGMENT_BYTES` — 違反は **`Err(HostError::Prepare(PrepareError::UnsupportedConfig))`**（`check_uniform_bounds` と同語彙 — wire 非送信の defined 拒否）。子側 attach 時は header 宣言値（下記 `vshm` header）と config からの再計算を照合。

### video shm segment レイアウト（`wire::vshm` — `shm` の兄弟 module として新設）

audio `shm` を触らず **別 module `wire::vshm`** に video layout を定義する（audio 定数・slot 契約は無変更 — R-B6）。`ruviform-plugin-host::l1::vshm`（create/own/unlink）と `ruviform-plugin-export::vshm`（attach/validate）がこの正本を共有し、SAFETY 論証は audio `l1/shm.rs` と同形（`#[expect(unsafe_code)]` + SAFETY コメントの 2 箇所のみ）。

```text
segment（per instance・host が create/own/unlink・子が shm_key で attach）
┌────────────────────────────────────────────────────────────┐
│ header（28 B・全 LE u32）                                    │
│  +0  magic          = vshm::MAGIC  ("RVLV")                  │
│  +4  wire_version   = 3                                      │
│  +8  in_off         = HEADER_LEN (28)                        │
│  +12  in_bytes      = frame_bytes(config)                    │
│  +16  out_off       = 28 + in_bytes                          │
│  +20  out_bytes     = frame_bytes(config)                    │
│  +24  reserved      = 0                                      │
├────────────────────────────────────────────────────────────┤
│ input frame region（in_bytes）— host が render 前に書込      │
│  plane 0 .. plane N-1 を offset 0 から連続配置               │
│  plane p: stride = row_bytes(p)（canonical packing）         │
│           len = stride × plane_height(p)                     │
│           offset(p) = Σ len(0..p)                            │
├────────────────────────────────────────────────────────────┤
│ output frame region（out_bytes）— 子が render 中に書込       │
│  同じ canonical packing。reply `Rendered{output}` が          │
│  実際に使った descriptor を運ぶ（v1 は config dims/format と  │
│  stride=row_bytes を host が検証）                            │
└────────────────────────────────────────────────────────────┘
```

- **canonical packing（確定）**: stride は desc で explicit に運ぶが **v1 の実値は常に `stride = row_bytes` の tightly packed** — (a) slot 占有量が config から一意に決まり per-frame stride 変動の検証面が消える、(b) host 側は caller の pad 付き frame を row copy で repack するだけ、(c) 子側 adapter（OFX は既に自前で 16B-aligned staging する）が必要なら自身で再 pad。desc が stride 字段を持つため、将来 pad-preserving 化しても wire 互換を壊さない。非 canonical desc（`offset(p) != Σlen`・`stride != row_bytes`・`offset+len > region_bytes`）は **decode 後の意味検証で `Err(HostError::Ipc)`**（host 契約違反 — duplicate handle と同クラスの defined 拒否・子は死なない）。dims/format の config 不一致は **`Err(HostError::Prepare(PrepareError::UnsupportedConfig))`**（audio `Activate` bound 拒否の写像）
- **所有・有効窓（確定）**: host は render 不在時のみ in 領域を書き、子は `VideoRender` dispatch 中のみ in 領域を読み out 領域を書く、host は `Rendered`/`Err` reply 受信後にのみ out 領域を読む — **request→reply の窓だけが slot 有効期間**。doorbell がないため acquire/release は pipe frame の read/write + `fence`（runner serve loop と同じ `Ordering` 規則）で成立。`Shmem` `!Send` は façade 所有（`PluginHost` が `!Send` なので control スレッド上の owner `Segment` で直接 RW — driver thread 中継は要らない）
- **`VideoFrameDesc` の信頼性**: desc は descriptor-over-Command/Reply 確定どおり pipe を経由し、plane bytes は触れない。host は `Rendered.output` が config dims/format・canonical・`out_bytes` 内に収まることを検証 — **well-formed で instance 帰属確立の違反なので帰属可能 per-instance `Failed`**

### host 公開面（`ruviform-plugin-host` — 新 module `video.rs`）

```rust
#[non_exhaustive]
pub enum VideoHostCommand {
    Scan,
    Instantiate { plugin_id: String, config: VideoProcessConfig },
    Destroy { handle: PluginHandle },
    Initialize { handle },
    Activate { handle },
    StartProcessing { handle },
    Render { handle, input: VideoFrame, at: MediaTime },
    StopProcessing { handle },
    Deactivate { handle },
    GetParams { handle },
    GetParamInfo { handle, id: ParamId },
    GetParamValue { handle, id: ParamId },
    SetParamValue { handle, id: ParamId, value: PlainParamValue },
}

#[non_exhaustive]
pub enum VideoHostReply {
    Ok,
    Scanned(Vec<PluginDescriptor>),
    Instantiated { handle: PluginHandle },
    Rendered { frame: VideoFrame },
    ParamIds(Vec<ParamId>),
    ParamInfo(Option<PlainParamInfoOwned>),
    ParamValue(PlainParamValue),
}

pub struct VideoOutOfProcessSpec { /* descriptor, program, args, command_timeout(10s), render_timeout(60s) — isolation 字段なし */ }

pub struct VideoPluginHost<'a> { /* &'a mut PluginHost */ }
impl PluginHost { pub fn video(&mut self) -> VideoPluginHost<'_>; }
impl VideoPluginHost<'_> {
    pub fn register_out_of_process(&mut self, spec: VideoOutOfProcessSpec) -> Result<(), HostError>;
    pub fn execute(&mut self, cmd: VideoHostCommand) -> Result<VideoHostReply, HostError>;
    // descriptor/lifecycle_state/kill_child/try_recv_notification は PluginHost 本体へ delegate
}
```

- **借用 façade を選んだ理由**（ADR `issue-118-video-l1-host-surface`）: `PluginHandle{slot,generation}` は `PluginHost` の単一 slot table で mint されるため、独立した `VideoPluginHost`（別 table）を持つと **2 系統の handle が値として alias し得る**（`{slot,generation}` が両 table で一致しても意味を失わない安全弁が存在しない）。借用 view は単一 table を保ったまま video 動詞だけを型で分離する — audio `execute` を汚さず、通知・mint・`Failed`・`prepare_endpoint` の全機構を共有できる最小差分
- **route**: `Route::OutOfProcessVideo { spec: VideoSpawnSpec }` を追加（`pool` 字段なし — Dedicated のみなので pool は要らない）。audio `Scan`/`Instantiate`/`register_out_of_process` は `OutOfProcessVideo` route を「存在しない route」として扱い（`Instantiate` → `Err(UnknownPluginId)`・重複登録 → `Err(DuplicatePluginId)` は map key 衝突で自然成立）。video `Scan` は video route のみ列挙
- **cross-kind 動詞**: audio `HostCommand` が video slot を指す場合 → dispatch の slot 分岐で **`Err(HostError::Unsupported)`**（「その instance は要求された操作を実装しない」既存語彙）。逆（`VideoHostCommand` が audio slot）も同型。`prepare_endpoint` が video slot を指す → **`Err(HostError::EndpointUnavailable)`**（wire を通らない local-only 語彙が既に存在する）
- **`Instantiate{plugin_id, config}`**: `VideoProcessConfig` を取る — audio の `Instantiate`+`Activate{config}` 分離と形が違うが、video では config が segment sizing の前提でありかつ `initialize(&config)` がその受け手なので **caller が instantiate 時に format/dims を決める**のが正直な契約（OFX/NLE の実使用形と一致）。format 変更は v1 では destroy+再 instantiate（format 再交渉は deferred 表へ）
- **`Render{handle, input, at}`**: façade が `input.format()/width()/height()` を cached config と照合（不一致 → **`Err(Prepare(UnsupportedConfig))`** — wire 非送信）→ mirror が `Processing` であることを確認（不一致 → `Err(Lifecycle)` — L0 `transit(Process)` 前置の写像）→ plane rows を in 領域に canonical repack → `VideoRender{at, input:desc}` を `render_timeout` で request → `Rendered{output:desc}` を検証（config dims/format・canonical・`out_bytes` 内 — 違反は帰属可能 `Failed`）→ out 領域から `VideoFrame::new_owned` + row copy + pts/duration/color を desc から復元して返す。`Err` reply/timeout/子死亡時は out 領域を読まない（commit-on-success）
- **mirror**: `rt_state` の lock-free mirror を audio facade と共有機構で維持。post/err_committed の video 写像: `VideoInitialize`/`VideoDeactivate` → Initialized・`VideoActivate`/`VideoStopProcessing` → Activated（`StopProcessing` は Err でも commit 済み — `err_committed_state` 同型）・`VideoStartProcessing` → Processing・`VideoRender`/param 動詞 → 変化なし。`Err(Lifecycle)` は常に pre-commit で mirror 不変
- **runner 側 `Failed` 語彙との対称**: `Failed` entry 宛の video 動詞は host mirror で `Err(Failed)`（`Destroy` 以外 — audio と同一）

### host 内部（`l1/` — video facade + 共有機構の再利用）

- **`VideoL1Instance`**（`l1/video.rs` 新規 — `L1Instance` と parallel な façade）: `{shared, child, instance, notify_rx, rt_state, segment: vshm::Segment, config, render_timeout}` — **RT leg・staging・arm 系・audio segment は一切持たない**（`drain_endpoint_channels` 等の audio data-plane 機構は無関係）。`request`/`destroy`/`kill`/`lifecycle_state`/`try_recv_notification` の収束・通知経路は `L1Instance` と同一機構を共有
- **`Inner.instances`**: `InstShared` の audio data-plane 字段（`shm: SegmentInfo`・`staging`・`result_tx`）を kind 別に分離 — `enum InstKind { Audio { shm, staging, result_tx }, Video { config: VideoProcessConfig } }` を `InstShared` に持たせる（`phase`/`mirror`/`rt_state`/`notify_tx`/`notify` は共通字段のまま）。envelope `instance` 名空間は child local で変わらず、`WIRE_MAX_INSTANCES` cap も同一（video child は Dedicated で 1 子 1 entry）
- **`create_video_instance(spec, config)`**（`L1Child` に追加 — `create_instance` と同型の流れ）: `vshm::Segment::create(slot, config)` → `InstShared{kind:Video{config}, phase:Creating}` insert → `CreateVideoInstance{plugin_id, shm_key, config}` request（`command_timeout`）→ `Ok` で `Live` flip + façade mint。失敗時の rollback（`remove_instance` + `Err(Ipc)`/`InstanceLimit` 透過）は verbatim reuse。**`DriverCmd::AddInstance`/`RemoveInstance` は送らない**（driver 側に video の per-instance 状態は存在しない — submit/result チャネルも作らない）
- **`request` の timeout**: `request(instance, cmd)` が固定 `self.command_timeout` を使う現状を `request_timed(instance, cmd, timeout)` に一般化（既存呼出しは `command_timeout` を渡す薄い wrapper として互換）。`VideoRender` のみ `render_timeout` を渡す。timeout→waiter 除去→`converge_child`→`Err(Ipc)` の写像は不変
- **`vshm::Segment`**（`l1/vshm.rs` 新規 — `l1/shm.rs` と同型）: `unique_key` 共用・`create(slot, config)` が `frame_bytes` を算出して `header + 2×frame_bytes` の named segment を作り header を書込。façade からの `in_region_mut()`/`out_region()` byte slice accessor（`bytes`/`bytes_mut` に SAFETY コメント + `#[expect(unsafe_code)]` — 書込単位は request→reply 窓で直列化される論証）

### runner 側（`ruviform-plugin-export`）

- **`ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video`**（`ruviform-plugin-core` 新規 `video_plugin.rs` — `video.rs` と隣接）: `VideoPluginInstance` の dyn-safe mirror。`type Error` 関連型を持たず **全 hook は `Result<_, HostError>`** を返す（`erase_video<P: VideoPluginInstance>` の adapter が plugin `Error` を `HostError::PluginFailed` に正規化 — L0 で adapter error が host surface に出る写像と同型）。`state() -> LifecycleState`・`descriptor() -> PluginDescriptorView<'_>`・`parameters() -> &[ParamId]`・`parameter(id) -> Option<PlainParamInfo<'_>>`・`parameter_value`/`set_parameter`/`initialize`/`activate`/`start_processing`/`render`/`stop_processing`/`deactivate`/`destroy` の全 hook。`VideoPluginFactory { descriptor() -> PluginDescriptor; create() -> Box<dyn ErasedVideoPlugin> }` — audio `PluginFactory` 対の最小形（`create_sendable` は video に不要 — RT endpoint 経路がない）
- **`run_video(factories: Vec<Box<dyn VideoPluginFactory>>) -> ExitCode`**（`ruviform-plugin-export` 新規 — `run` と parallel な別 entry）: framing/`Hello`/exit semantics/argv 拒否（引数なし・`--ruviform-*` → exit 2）/`Shutdown`+EOF normalize は `serve` の骨格を共有し、dispatch registry のみ video 専用（`VideoRunner{factories, instances: HashMap<u32, VideoEntry>}`）。`run(factories)` 側は無変更 — 1 entry point に両 kind を載せる統合形は却下（Dedicated で混在 child は起きず、registry dispatch を kind 別表に増やす利益がない）
- **`VideoEntry`**: `{instance: Box<dyn ErasedVideoPlugin>, lifecycle: Lifecycle, config: VideoProcessConfig, segment: vshm::Segment}` — `CreateVideoInstance` で stash した config は `VideoInitialize` 時に `initialize(&config)` へ再生。`render` dispatch は `transition(Process)` 前置 → `vshm` desc 検証 → in 領域から `VideoFrame` を復元（`from_shared_planes` 同等の per-plane 所有 copy — `Arc<[u8]>` plane を `new_owned`+row copy で構築）→ hook 呼出し → `Ok` なら out 領域に出力を書き `Rendered{output:desc}`、hook `Err`/panic → 出力不 commit + `Err(PluginFailed)`（catch_unwind 正規化は audio と同じ）
- **registry 動詞**: `CreateVideoInstance` は audio `CreateInstance` と同一手順（id 解決 → 上限 → 重複 → attach → create → insert）。`DestroyInstance` は audio entry と video entry を同じ `DestroyInstance` で処理できるが registry が kind 単一なので video runner のそれは video entry にのみ効く。audio 動詞宛 → `Err(Unsupported)`。`Scan` は video descriptor 列

### `Failed` 粒度・通知の確定契約（#117 verbatim の video 写像）

| 観測 | video での収束（#117 と同一） |
| --- | --- |
| 子死亡・kill・stdout EOF・stdin write 失敗・decode 不能・未知 kind・scope/`instance` 範囲外・live instance への waiter-less `Reply`・`Reply.instance` 不一致 | **子スコープ terminal** — 全 live instance `Failed` + 全 pending waiter `Err(Ipc)` + 各 instance に `InstanceFailed{handle, exit}` |
| mid-`VideoRender` 死亡 | render waiter `Err(Ipc)` wake・out 領域は読まれない・影響 instance `Failed` + `InstanceFailed` |
| mid-`CreateVideoInstance` 死亡 | 通知なし・`Err(Ipc)`・handle 未 mint（#117 AC12 同型） |
| mid-`DestroyInstance` 死亡 | `Failed` + 通知 + `Destroy` は `Ok`（slot 解放） |
| 帰属可能違反 — well-formed で live instance 一意帰属（`Rendered` desc の semantic 検証失敗・video reply の `instance`/seq 相関違反等） | **対象 instance のみ `Failed`**・兄弟（Dedicated では存在しないが機構は同一）生存 |
| inert — registered-but-not-live（`Failed` 未 `Destroy`・`Destroying`）宛 frame・post-removal 宛 `Reply`/`Rendered`・post-`Shutdown` frame | **静かに drop** — 違反でも収束でもない |
| wedge — `render_timeout`/`command_timeout` 超過 | **帰属不能 → 子スコープ `Failed`**（timeout は instance を名指しできない #117 規則） |

### lifecycle・render の状態機械（確定）

```text
public surface                    wire                          child (VideoEntry)
─────────────                     ────                          ──────────────────
Instantiate{id,config}
  [frame_bytes ≤ cap 検証]   →   CreateVideoInstance{id,key,config}
                                                            attach vshm・create()・
                                                            config stash・entry mint
                                                          ← Ok ──── mirror: Created
Initialize{handle}           →   VideoInitialize            initialize(&stash config)
                                                          ← Ok ──── Initialized
                                                            （Err → InitFailed → Created 維持）
Activate{handle}             →   VideoActivate              activate()
                                                          ← Ok ──── Activated
                                                            （Err → ActivateFailed → Initialized 維持）
StartProcessing{handle}      →   VideoStartProcessing       start_processing()
                                                          ← Ok ──── Processing
                                                            （Err → rollback → Activated 維持）
Render{handle,input,at}      →   VideoRender{at,desc}       in 領域→frame 復元・
  [input vs config 検証]                                   render(input,out,at)
  [mirror == Processing]                                   out 領域へ書込
                                                          ← Rendered{out_desc} ── mirror 不変(Processing)
                                                            （Err → PluginFailed・出力不 commit）
StopProcessing{handle}       →   VideoStopProcessing        stop_processing()
                                                          ← Ok ──── Activated（Err でも commit 済）
Deactivate{handle}           →   VideoDeactivate            deactivate()
                                                          ← Ok ──── Initialized
Destroy{handle}              →   DestroyInstance            normalize + registry 解放
  [mirror: Created/Initialized/Failed のみ]               ← Ok ──── Destroyed
param verbs                  →   VideoGet*/VideoSet*        hook dispatch（lifecycle gate なし —
                                                          adapter 責務の set_parameter 意味は素通し）
```

## Non-Functional Constraints / invariants（維持を要求 — video で変えない）

- **RT 契約不適用**: video 経路は全操作 NONRT — `rt_` テスト・`assert_no_alloc`・pass-through 語彙・`prepare_endpoint`/RT leg は対象外（audio 経路の RT 不変式は無変更で維持）
- **bounded 必須**: 新 decode/attach は全て `WIRE_MAX_VIDEO_*`・`WIRE_MAX_STRING_BYTES`/`WIRE_MAX_ELEMENTS` で bounded — hostile dims・stride/len 不整合・非有限値は defined 拒否（panic・無制限 alloc・OOB なし）
- **公開 enum `#[non_exhaustive]`**: `VideoHostCommand`/`VideoHostReply`/将来の video 系 enum は全て enum-level `#[non_exhaustive]`
- **`unsafe` 局所化**: `vshm` の mapped slice accessor のみ `#[expect(unsafe_code)]` + SAFETY コメント（audio `shm.rs` と同じ 2 箇所パターン）
- **`PluginHost` `!Send`/`!Sync`・単一最新 wire version・`Hello.capabilities` reserved-zero・自動再起動なし** — 全て不変
- **audio 無退行（R-B6）**: `run`/`ErasedPlugin`/audio `shm`/doorbell/`HostCommand`/`OutOfProcessSpec`/`OutOfProcessIsolation` は無変更 — 共有機構（`Inner`/`waiters`/`converge`/`notify`/`mint`）への video 追加は kind 字段の additive な拡張に留める

## Risks and Trade-offs

- **`VideoProcessConfig` を create 時点に固定する trade** — caller は instantiate 時に format/dims を決める必要がある（audio の `Activate` 時 config と位相が違う）。format 再交渉は v1 で不可（destroy+再 instantiate）。選んだ理由: lazy attach は「segment 無しの minted instance」窓と第 2 attach 動詞を要し、fixed-max は最大 512 MiB/instance を常時確保する — どちらも config を知ってからサイズを決める要求を満たさない。詳細は ADR `issue-118-video-l1-create-path-segment`
- **copy-in/copy-out コスト** — 4K `Rgba8` で片道 ≈ 31.6 MiB、往復 ≈ 63 MiB + 子側復元/書出で更に同等。NONRT 経路なので許容域だが、fps×frames で帯域支配になる — zero-copy backing 抽象は deferred 表で追跡
- **render_timeout=60 s の恣意的既定** — 重い filter で本当に 60 s を超える render は `Failed` 化する。route field で tune 可能にしているが「十分な既定値」の実証は fixture/E2E では得られない（review で人間確認が要る残件）
- **`VideoPluginHost` 借用 façade** — audio `execute` を型レベルで無変更に保つ代わりに、`video()` の `&mut` 借用が audio `execute` の呼出しと compile-time で直列化される（両方必要な処理は `drop(video_view)` 跨ぎが要る — 単一 control スレッド前提なので実害なし、ただし API 使用感は review 観点）
- **`Err(Unsupported)` による cross-kind 拒否** — 「video 動詞を audio instance に撃つ」誤用が `Unsupported` に畳まれるため、caller は kind 間違いに気付きにくい可能性（`UnknownPluginId` とは別クラスとして意図的に分離 — doc で明記）
- **HDR/色域 payload の wire 化** — `ColorInfo` 全量を desc に載せる（≦ 34 B）は小さいが、HDR semantics を wire 層が tag 化して固定する = video-core enum 追加時に wire 更新が要る（`#[non_exhaustive]` + 未知 tag 拒否で保守可能・#119 bump 規則の適用範囲）
- **Dedicated のみ = spawn/proc コスト** — video instance 毎に子プロセス + 最大 512 MiB 級 segment。多数小 filter の運用では Bundled のコスト利点を捨てている（人間確定済みの安全側 trade — Bundled video の HoL 局所化は deferred 表）
- **`l1-test-video-worker` 新規** — audio fixture と flag/dispatch を一部 mirror するため二重保守面が増えるが、audio fixture 無変更という回帰保証がその対価
- **`PlainParamInfoOwned` の新規 public 型** — wire 化に伴う owned mirror の追加は避けられないが、`PlainParamInfo<'a>` との二正面になる（`From<PlainParamInfo<'_>>`/`as_view` で変換面を一箇所に集約する実装指示）

## Deferred Design Concerns

- GPU/texture interop・OFX GPU suites | 上流 ADR 確定・初版 CPU Filter/Rgba8 | GPU suite 需要・media-io shared-buffer 要件 | GPU プラグインは L1 で使えない
- 真の zero-copy（shm→`VideoFrame` backing/view） | `Arc<[u8]>` 制約で表現不能・video-core backing 抽象は別 Issue 級 | copy コストが支配的になる負荷 | 全フレームが copy-in/copy-out を払う
- async render・depth>1 ring・pipeline 化 | 同期 request/response が v1 契約 | 連続フレーム throughput 需要 | render latency が control 面を占有
- Bundled video の HoL 局所化（reader/channel 分離・子側 render スレッド） | Dedicated 限定で v1 回避 | Bundled video が実運用で要求される段階 | Bundled 下で遅い render が兄弟 doorbell を塞ぐ
- video 用 supervisor/`RestartRecipe` 変種（`VideoProcessConfig` 再生・segment 再確立） | #116 util 層は audio 形 | video インスタンスの自動復旧要求 | crash した video instance は手動復旧のみ
- engine/利用側 video port 接続 | 別 Issue（第二ターゲット統合） | engine 統合要求 | L1 video が engine から到達不能
- video GUI / `OpenGui` 系・format 再交渉（config 変更の再 instantiate なし経路） | trait に動詞なし・v1 は固定 config | 需要発生時 | GUI 持ち/可変 format plugin は L1 で限定的

## Verification Notes

- [ ] wire codec round-trip + 拒否: 新 tag 12-22・reply 4-7・`VideoProcessConfig`/`VideoFrameDesc`/`ColorInfo`/`PlainParamValue`/`PlainParamInfoOwned` codec・`WIRE_MAX_VIDEO_*` 超過（dims/plane/frame bytes/stride）・不正 `instance`・reserved 非零・truncated・未知 tag・`Hello.version` 不一致
- [ ] `vshm` layout 単体: header 検証・`frame_bytes`/`segment_len` byte math・canonical packing 配置・attach 拒否分岐
- [ ] host 面: `video()` façade の route kind 分離（audio/video `Scan` filter・cross-kind `Instantiate`→`UnknownPluginId`・登録衝突→`DuplicatePluginId`・cross-kind 動詞→`Unsupported`・`prepare_endpoint`→`EndpointUnavailable`）・`Instantiate` bound 拒否
- [ ] video E2E（`l1-test-video-worker`）: scan→create→initialize→activate→start→render（frame 内容一致・Rgba8 pin）→stop→deactivate→destroy・param get/set/info
- [ ] `Failed` parity: 子死亡/pipe 破壊/mid-render/mid-create/mid-destroy/帰属可能違反/inert/`Shutdown` 後 frame — 全分岐 #117 写像・通知 once・sibling 影響なし
- [ ] timeout: `VideoRender` が `render_timeout` に従う・他動詞は `command_timeout`・timeout→`Err(Ipc)`+`Failed`
- [ ] panic 正規化: video hook panic → `Err(PluginFailed)` で子生存
- [ ] audio 無退行: 既存 L1 E2E・codec・`rt_` テスト無変更グリーン
- [ ] `doc_status.js audit` blocking 0

## Sources and Relations

- Issue [#118](https://github.com/scarecrow173/ruviform/issues/118)（要求元）・upstream [#76](https://github.com/scarecrow173/ruviform/issues/76)・[#98](https://github.com/scarecrow173/ruviform/issues/98)・[#114](https://github.com/scarecrow173/ruviform/issues/114)・[#117](https://github.com/scarecrow173/ruviform/issues/117)・[#119](https://github.com/scarecrow173/ruviform/issues/119)
- spec `docs/specs/issue-118-video-l1.md`・discovery `docs/discovery/issue-118-video-l1.md`（OPEN 論点の元表）・ADR `docs/adr/issue-118-video-l1-host-surface.md`（決定 2-4）・ADR `docs/adr/issue-118-video-l1-create-path-segment.md`（決定 5・8）
- 一次文書: `docs/designs/issue-117-bundle-child-process.md`・`docs/adr/issue-117-bundle-child-failed-granularity.md`（`Failed` 契約の正本）・`docs/designs/plugin-l1-out-of-process.md`（L313 deferred — 本書が re-engage）・`docs/designs/engine-plugin-host-integration.md`（決定 k L285-289 — 同）・`docs/specs/issue-119-l1-wire-version-policy.md`・`docs/adr/ofx-host-video-contract-and-c-abi.md`・`docs/adr/ofx-video-plugin-scope.md`・`docs/designs/issue-116-supervisor.md`・`docs/designs/issue-114-real-adapter-l1-e2e.md`
- ローカル一次証拠: `crates/ruviform-plugin-core/src/{video,plain_param,wire,boundary,plugin}.rs`・`crates/ruviform-video-core/src/{frame,pixel,plane,color}.rs`・`crates/ruviform-plugin-host/src/{composite,command,lib}.rs`・`crates/ruviform-plugin-host/src/l1/{mod,shm,data}.rs`・`crates/ruviform-plugin-export/src/{lib,shm}.rs`・`crates/ruviform-plugin-ofx/src/{instance,host}.rs`・`crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`

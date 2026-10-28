---
id: "TSPEC-034NXpKKMs4FptRr5mjttQ"
type: "test-spec"
status: "approved"
title: "L1 wire v3 video codec tests（Issue #118 — video 動詞・Reply・video 型 codec・WIRE_MAX_VIDEO_*・InvalidValue）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plain_param.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
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
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/issue-118-video-failed-parity-tests.md"
    - "docs/test-specs/issue-118-video-e2e-tests.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 wire v3 video codec tests（Issue #118 — video 動詞・Reply・video 型 codec・WIRE_MAX_VIDEO_*・InvalidValue）

## Purpose

wire v3（`WIRE_VERSION` 2→3 一括 bump・#119「単一最新版のみ」方針に従う wholesale 改訂）で **additive に追加される**契約の oracle。video 動詞 tag 12-22・`Reply` tag 4-7・video 型 codec（`VideoProcessConfig`/`PixelFormat`/`VideoFrameDesc`/`ColorInfo`/`PlainParamValue`/`PlainParamInfoOwned`）・`WIRE_MAX_VIDEO_*` bound・`HostError::InvalidValue` wire tag 10・presence/tag strictness・`wire::vshm` header 定盤と byte math を pin する。framing・envelope・scope 規則・audio 動詞・audio `shm` mod 等の**引き継ぎ**契約は `plugin-l1-wire-codec-tests.md`（v3 修正版の baseline oracle）が引き続き所有する — 本 spec は #118 が追加する面のみを対象とする。wire v3 が supersede された後は本 spec は引き継ぎ oracle へ統合され得る。

## Feature

wire v3 codec は video lifecycle 動詞・frame descriptor・plain param 動詞を `[u64 seq][u32 instance][rest]` envelope で符号化し、宣言 tag/dims/plane/byte 量を `Malformed`/`UnsupportedVariant` で bounded に拒否する。`HostError::InvalidValue` は encode/decode の両方向で tag 10 と round-trip する。`wire::vshm` は video segment の header 定盤（`MAGIC`/`HEADER_LEN`/field offset）と `frame_bytes`/`segment_len` byte math を提供する。

## Rules

- `WIRE_VERSION == 3` が codec 定数として存在する。`Command` tag 0-11・`Reply` tag 0-3・`BlockIn`/`BlockDone`・envelope・scope 判定表・audio `shm` mod 定数は v2 から無変更（additive bump — R-B6）
- `Command` に 11 個の video 動詞が追加される: tag 12 `CreateVideoInstance{plugin_id, shm_key, config: VideoProcessConfig}`・13 `VideoInitialize`・14 `VideoActivate`・15 `VideoStartProcessing`・16 `VideoRender{at: MediaTime, input: VideoFrameDesc}`・17 `VideoStopProcessing`・18 `VideoDeactivate`・19 `VideoGetParams`・20 `VideoGetParamInfo{id}`・21 `VideoGetParamValue{id}`・22 `VideoSetParamValue{id, value}`。全て instance-scope（non-`WIRE_INSTANCE_NONE` 必須 — `Scan` のみ child-scope の既存規則は不変）
- `Reply` に 4 個の video variant が追加される: tag 4 `Rendered{output: VideoFrameDesc}`・5 `Params{ids: Vec<u32>}`（`WIRE_MAX_ELEMENTS` 上限）・6 `ParamInfo{info: Option<PlainParamInfoOwned>}`・7 `ParamValue{value: PlainParamValue}`
- `VideoProcessConfig` codec（13 B）: `u8 format` + `u32 width` + `u32 height` + `FrameRate{num,den}`。decode は `width/height ∈ [1, WIRE_VIDEO_MAX_DIM]`・`format` tag ∈ 0-5（宣言順 `Yuv420p,Yuv422p,Yuv444p,Rgba8,RgbaF16,RgbaF32`・未知 tag は defined 拒否）・`frame_rate` num/den 非零を codec-bounds として検証する
- `VideoFrameDesc` codec（可変長・≦127 B）: dims/format 同上・`plane_count ∈ [1, WIRE_VIDEO_MAX_PLANES]` かつ `format.plane_count()` と一致・各 `stride ≤ WIRE_VIDEO_MAX_STRIDE`・pts/duration の TimeBase 非零を codec-bounds として検証する。**canonical 配置（`stride==row_bytes`・`offset(p)==Σlen`・`offset+len ≤ region_bytes`）と config 一致は codec 層の責務ではない**（region 容量は config 由来で codec が知らない — post-decode semantic 検証。本 spec は codec 層のみ対象）
- `PlainParamValue` codec: kind tag ∈ 0-3（宣言順 `Double,Integer,Boolean,Choice`）・`Double` は **finite 必須**（NaN/Inf は decode で `Malformed` 拒否 — `plain_param.rs`「finiteness は boundary 責務」の写像）・`Boolean` の値 byte は strict `{0,1}`
- `PlainParamInfoOwned` codec: `id` + `name`/`unit`（各 `WIRE_MAX_STRING_BYTES`）+ `min_value`/`max_value`/`default_value` は finite 必須・`min ≤ max`・`default_value ∈ [min,max]`（kind 別値の f64 比較）・`Choice` 時 `choices` 非空で default index in-range・他 kind は `choices` 空・`choices` 件数 ≤ `WIRE_MAX_ELEMENTS`。**`display_*` は strict-finite を要求しない**（UI hint — 非有限を受理するのは decode の意図的 leniency）
- `ColorInfo` codec: 4 enum（宣言順 tag・未知 tag defined 拒否）+ `has_hdr` presence tag strict `{0,1}` + optional `MasteringDisplayInfo`（24 B）/content light level（4 B）— 全 presence tag は `{0,1}` 以外 `Malformed`
- `HostError::InvalidValue` は encode/decode の両方向で tag 10 と round-trip する（wildcard arm が `UnsupportedVariant`/`Malformed` へ潰さない — 「定義済み拒否が子死亡に化ける」退行防止。`EndpointUnavailable` は引き続き local-only で encode は `UnsupportedVariant` 拒否）
- decode は untrusted 入力の検証境界 — `WIRE_MAX_VIDEO_*` 超過の dims/plane count/stride・不正 `instance`・不正 tag・truncated payload・reserved 非零を panic・無制限 alloc なしで拒否する
- `Hello.capabilities` は v3 でも **reserved-zero**（非零 decode 拒否を維持 — kind 広告を capabilities bit に載せる形は採用されない）。`Hello.version != 3` は handshake 失敗
- `wire::vshm` 定盤: `MAGIC = "RVLV"`・`HEADER_LEN = 28`・header field（`in_off == 28`・`out_off == 28 + in_bytes`・`reserved == 0`）・`frame_bytes(config)`/`segment_len` byte math（canonical packing — `stride=row_bytes`・`offset(p)=Σlen`）・`WIRE_VIDEO_MAX_SEGMENT_BYTES = 28 + 2×WIRE_VIDEO_MAX_FRAME_BYTES`
- 公開 enum（`Command`/`Reply`/`HostError`/`PlainParamValue`）は `#[non_exhaustive]` を維持する

## Examples

- 全 11 video 動詞 + 4 video Reply の round-trip: `CreateVideoInstance{plugin_id, shm_key, config}`（tag 12）・`VideoRender{at, input:desc}`（tag 16）・`Rendered{output:desc}`（tag 4）・`VideoSetParamValue{id, value}`（tag 22）等が encode→decode で同一値を返す
- scope 違反: video 動詞に `instance=NONE` → `Malformed`・`instance ≥ 64` → `Malformed`
- `VideoProcessConfig` 拒否行列: `width=0`/`width=WIRE_VIDEO_MAX_DIM+1`・`format tag=6`・`frame_rate.den=0` → defined 拒否
- `VideoFrameDesc` 拒否行列: `plane_count=0`/`plane_count=5`・`plane_count` と `format.plane_count()` 不一致（`Rgba8` で `plane_count=3`）・`stride=WIRE_VIDEO_MAX_STRIDE+1` → defined 拒否
- presence strictness: `has_duration=2`・`has_hdr=0xFF`・`Boolean` 値 byte `>1` → `Malformed`
- `PlainParamValue::Double` の非有限値（NaN/±Inf）→ `Malformed`・`PlainParamInfoOwned` の非有限 `min`/`max`/`default`・`min>max`・`default ∉ [min,max]`・`Choice` で `choices` 空 or index out-of-range → defined 拒否・非有限 `display_*` → **受理**
- `InvalidValue` 両方向 pin: `Reply{seq, instance, Err(InvalidValue)}` の encode→decode が `InvalidValue` を返す（tag 10 — wildcard arm に落ちない）
- `vshm` byte math const test: `frame_bytes(3840×2160 Rgba8) == 33_177_600`・`frame_bytes(3840×2160 Yuv420p) == 12_441_600`・`frame_bytes(1920×1080 Rgba8) == 8_294_400`・`segment_len = 28 + 2×frame_bytes`・`segment_len(8K RgbaF16)` ≈ 506 MiB ≤ `WIRE_VIDEO_MAX_SEGMENT_BYTES`
- `Hello{version: 2, capabilities: 0}` は `version != 3` で handshake 失敗・`Hello{version: 3, capabilities: 1}` は capabilities 非零で拒否（v3 でも維持）

## Guarantees

- codec は wire v3 video 面の authoritative 境界 — 動詞 tag・Reply tag・video 型 codec・`WIRE_MAX_VIDEO_*`・presence strictness の正本を本 spec が pin する
- `InvalidValue` は encode/decode の両方向で明示 round-trip され、wildcard arm による黙認退行を防ぐ
- codec 層は bounds のみを検証する — canonical 配置・region-fit・config 一致の post-decode semantic 検証が codec 層に紛れ込まない（責務分離は `issue-118-video-failed-parity-tests` が帰属分岐を pin）
- decode は adversarial dims/plane/tag/truncated/presence 値を panic・無制限 alloc・OOB なしで拒否する
- audio codec（動詞 0-11・Reply 0-3・`shm` mod）は無変更 — v3 bump は additive のみ

## Non-goals

- post-decode semantic 検証（canonical packing・`offset+len ≤ region_bytes`・config dims/format 一致）の受け手分岐 — `issue-118-video-failed-parity-tests`（host 側帰属可能 `Failed`）と runner dispatch 検証（`issue-118-video-e2e-tests`/runner E2E）が対象
- `vshm` attach/host create の検証 — `issue-118-video-failed-parity-tests` と runner/host 側検証が対象（本 spec は wire 定盤の定数・byte math のみ）
- host/runner の demux・registry・`Failed` 意味論・facade・E2E — `issue-118-video-failed-parity-tests`・`issue-118-video-e2e-tests` が対象
- encode obligations（(a) façade pre-validate・(b)(c)(d) erase/adapter 境界 normalize）— encode 側は trusted で bound 検査しない方針どおり codec 検証対象外。`issue-118-video-e2e-tests` が defined `Err` 経路を pin
- `InstanceLimit`/`InvalidHandle`/`UnknownPluginId` 等の v2 引き継ぎ語彙 — `plugin-l1-wire-codec-tests.md` が所有

## Risk

- video 動詞/Reply の tag・payload 順序の誤り → 全 video frame が parse ずれで壊れる
- `VideoFrameDesc`/`VideoProcessConfig` の bound 漏れ → hostile dims が巨大 alloc・segment サイズ計算の乗数として攻撃面化
- `Double` finiteness・presence strictness の抜け → 非有限/非 {0,1} 値が wire を通り契約違反が未定義化（decode 拒否→子死経路）
- `InvalidValue` の wildcard arm 見落とし → `id ∈ parameters()` 拒否等の defined 拒否が `Malformed`/`UnsupportedVariant` 化けて子が誤収束
- `vshm` byte math の誤り → in/out 領域サイズ不整合で frame が region を溢れる or segment が必要量を下回る
- canonical packing と codec bounds の混同 → codec が region-fit を検証しようとして config 無しで検証不能になり検証責務が宙吊りになる

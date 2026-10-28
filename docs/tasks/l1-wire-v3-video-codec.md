---
id: "TASK-034NXuhy2LmLb9qDBo7gj6"
type: "task"
status: "todo"
title: "L1 wire v3 codec + vshm 定盤 + InvalidValue（video 動詞 tag 12-22・Reply 4-7・video 型 codec・WIRE_MAX_VIDEO_*）"
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
  implements:
    - "docs/plans/implement-issue-118-video-l1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-118-video-l1.md"
  blocks:
    - "docs/tasks/l1-video-runner-and-erased-surface.md"
    - "docs/tasks/l1-video-host-surface.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-wire-v2-codec-cutover.md"
    - "docs/tasks/l1-wire-version-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-118-wire-v3-codec-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  defers: []
  deferred-by: []
---

# L1 wire v3 codec + vshm 定盤 + InvalidValue（video 動詞 tag 12-22・Reply 4-7・video 型 codec・WIRE_MAX_VIDEO_*）

## Work

wire v3 の **additive** codec — #117 T1（atomic cutover）と違い既存 tag・envelope・audio `shm` mod は無変更で、`WIRE_VERSION` 定数の bump は `Hello.version`/shm header `wire_version` が両側とも定数参照するため既存 suite を壊さない。新規動詞・Reply・video 型 codec・bound・`wire::vshm` 定盤・`HostError::InvalidValue`・`PlainParamInfoOwned` を plugin-core 単独で完結させる。

1. `crates/ruviform-plugin-core/src/wire.rs` — `WIRE_VERSION` 2→3。`Command` に tag 12-22 を追加（`#[non_exhaustive]` 維持）: `CreateVideoInstance{plugin_id: String, shm_key: String, config: VideoProcessConfig}`(12)・`VideoInitialize`(13)・`VideoActivate`(14)・`VideoStartProcessing`(15)・`VideoRender{at: MediaTime, input: VideoFrameDesc}`(16)・`VideoStopProcessing`(17)・`VideoDeactivate`(18)・`VideoGetParams`(19)・`VideoGetParamInfo{id: u32}`(20)・`VideoGetParamValue{id: u32}`(21)・`VideoSetParamValue{id: u32, value: PlainParamValue}`(22)。`Reply` に tag 4-7: `Rendered{output: VideoFrameDesc}`(4)・`Params{ids: Vec<u32>}`(5)・`ParamInfo{info: Option<PlainParamInfoOwned>}`(6)・`ParamValue{value: PlainParamValue}`(7)。scope 判定表を拡張 — video 動詞は全て instance-scope（`Scan` だけが `WIRE_INSTANCE_NONE` 必須の既存規則は不変）。`plugin_id`/`shm_key` は `WIRE_MAX_STRING_BYTES`・`ids` は `WIRE_MAX_ELEMENTS` の既存 bound を再利用
2. `wire.rs` — video 型 codec（全て bounded）: `VideoProcessConfig`（13 B: `u8 format` + `u32 w` + `u32 h` + `FrameRate{num,den}` — dims `[1,WIRE_VIDEO_MAX_DIM]`・format tag 0-5・num/den 非零）・`PixelFormat` tag codec（宣言順 `0=Yuv420p,1=Yuv422p,2=Yuv444p,3=Rgba8,4=RgbaF16,5=RgbaF32`・未知 tag defined 拒否）・`VideoFrameDesc`（可変長 ≦127 B — dims/format 同上・`plane_count ∈ [1,WIRE_VIDEO_MAX_PLANES]` かつ `format.plane_count()` 一致・per-plane `{u32 offset,u32 len,u32 stride}` で `stride ≤ WIRE_VIDEO_MAX_STRIDE`・`MediaTime pts`・`has_duration` presence + `MediaDuration`・`ColorInfo`）・`ColorInfo`（4 enum 宣言順 tag + `has_hdr` + optional `MasteringDisplayInfo` 24 B + content light level 4 B）・`PlainParamValue`（kind tag 0-3 宣言順・`Double` finite 必須・`Boolean` 値 byte strict `{0,1}`）・`PlainParamInfoOwned`（`id`+`name`/`unit`（`WIRE_MAX_STRING_BYTES`）+`min/max/display_min/display_max`+`default_value`+`flags`+`choices`（`WIRE_MAX_ELEMENTS` 件・各 `WIRE_MAX_STRING_BYTES`）— decode は `min/max/default` finite・`min≤max`・`default ∈ [min,max]`・`Choice` 時 `choices` 非空+index in-range/他 kind 空・**`display_*` は strict-finite 不要求**の lenient 受理）。**codec 層は bounds のみ** — canonical 配置（`stride==row_bytes`・`offset(p)==Σlen`・`offset+len ≤ region_bytes`）と config 一致は post-decode semantic（region 容量を codec が知らない — T2/T3 の責務。design「wire 型 codec」先頭の責務分離を doc に明記）。presence/`Option` tag は全て strict `{0,1}`（`decode_transport` の `{0,1}` house style L1582-1590 と同じ）
3. `wire.rs` — `WIRE_VIDEO_MAX_*` 定数: `WIRE_VIDEO_MAX_DIM=8_192`・`WIRE_VIDEO_MAX_PLANES=4`・`WIRE_VIDEO_MAX_STRIDE=262_144`・`WIRE_VIDEO_MAX_FRAME_BYTES=268_435_456`・`WIRE_VIDEO_MAX_SEGMENT_BYTES=28+2*WIRE_VIDEO_MAX_FRAME_BYTES`（design の値・根拠表どおり）。`encode_host_error`/`decode_host_error` に `InvalidValue` → tag 10 を additive 追加（`EndpointUnavailable` encode 拒否は維持 — tag は 0-9 + 10 連続）
4. `wire.rs` — `pub mod vshm` を `shm` mod の兄弟として新設: `MAGIC = b"RVLV"`・`HEADER_LEN = 28`・field offset（`in_off=+8`/`in_bytes=+12`/`out_off=+16`/`out_bytes=+20`/`reserved=+24`）・`frame_bytes(config: &VideoProcessConfig) -> u64`・`segment_len(frame_bytes) -> u64` の byte math（canonical packing: `stride=row_bytes`・`offset(p)=Σlen`・plane dims は `PixelFormat::{plane_count,bytes_per_sample,chroma_subsampling}` + ceil-div — `pixel.rs` が正本）。audio `shm` mod は無変更
5. `wire.rs` — `Hello.capabilities` の doc を「v3 でも reserved-zero」へ再 pin（「`WIRE_VERSION` 2 では reserved」の現行 wording を v3 契約として維持 — 非零 decode 拒否は既存どおり）。codec 全体の `WIRE_VERSION`/wire 契約記述を v3 へ更新
6. `crates/ruviform-plugin-core/src/boundary.rs` — `HostError::InvalidValue` variant を追加（`#[non_exhaustive]` 維持）・`Display` impl・doc に「caller 供給値が定義 domain 外 — `VideoSetParamValue` 非有限 `Double`・未知 `ParamId` の `parameter_value`/`set_parameter`・`Render{input}` config dims/format 不一致」の意味を記述
7. `crates/ruviform-plugin-core/src/plain_param.rs` — `PlainParamInfoOwned` 新規 public 型（`PlainParamInfo<'_>` の owned 対: `id`/`name`/`unit`/`min_value`/`max_value`/`display_min_value`/`display_max_value`/`default_value`/`flags`/`choices` — 全 owned）。`From<&PlainParamInfo<'_>>`/`From<PlainParamInfo<'_>>` と `as_view() -> PlainParamInfo<'_>` の変換面を一箇所に集約（design Risks の二正面 note どおり）
8. `crates/ruviform-plugin-core/src/lib.rs` — `PlainParamInfoOwned`・`VideoFrameDesc`（codec 型 — wire::vshm 定数とともに public surface 化）の re-export を確定。`wire` mod は既に `pub mod` — codec 対象の video 型（`VideoProcessConfig`/`PixelFormat`/`ColorInfo`/`PlainParamValue`/`MediaTime`/`MediaDuration`/`FrameRate`）は既存 re-export で到達可能か確認し、欠落分のみ追加
9. codec unit test（`wire.rs` mod tests または `#[cfg(test)]` ブロック — 既存 codec テストの配置に合わせる）: 全 11 video 動詞 + 4 Reply の round-trip・scope 違反行列（video 動詞に `instance=NONE`/`≥64` → `Malformed`）・`VideoProcessConfig`/`VideoFrameDesc`/`ColorInfo`/`PlainParamValue`/`PlainParamInfoOwned` 拒否行列（test-spec `issue-118-wire-v3-codec-tests` Examples 行と 1:1 — dims/format/plane_count/stride/finiteness/presence strictness/`min>max`/`default ∉ [min,max]`/`display_*` lenient 受理）・`InvalidValue` 両方向 pin・`Hello{version:2}` 拒否・`Hello{version:3, capabilities:非零}` 拒否・bounded decode（adversarial payload で panic/無制限 alloc なし）・`vshm` byte math const test（`frame_bytes(3840×2160 Rgba8)=33_177_600`・`Yuv420p=12_441_600`・`1920×1080 Rgba8=8_294_400`・`segment_len=28+2×frame_bytes`・上限 const の相互関係）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T1 — plan の wire v3 基盤節が範囲根拠）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md) L120-207（wire v3 プロトコル — 動詞表・Reply 表・wire 型 codec・encode obligations・`WIRE_MAX_VIDEO_*`）・L209-241（`vshm` segment レイアウト — header・canonical packing・byte math）
- [../adr/issue-118-video-l1-create-path-segment.md](../adr/issue-118-video-l1-create-path-segment.md)（`CreateVideoInstance` payload 形・`vshm` header 字段・canonical packing・`VideoInitialize` payload 無し）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md) AC1・R-B4・R-V1
- [../specs/issue-119-l1-wire-version-policy.md](../specs/issue-119-l1-wire-version-policy.md)（wholesale bump・単一最新版方針 — additive 拡張のルール）
- [issue-118-wire-v3-codec-tests.md](../test-specs/issue-118-wire-v3-codec-tests.md)（本 task の oracle — Rules/Examples と実装行の 1:1 対応）

## Dependencies

- 先行タスク: なし — plan T1 は全後続の基盤
- 前提文書: `docs/designs/issue-118-video-l1.md` approved・両 ADR accepted・`docs/specs/issue-119-l1-wire-version-policy.md` approved
- `docs/test-specs/issue-118-wire-v3-codec-tests.md` approved（oracle — 先に着地済み）

## Affected Files

- `crates/ruviform-plugin-core/src/wire.rs` — `WIRE_VERSION`=3・`Command` tag 12-22・`Reply` tag 4-7・video 型 codec・`WIRE_MAX_VIDEO_*`・`pub mod vshm`・`InvalidValue` codec・doc 更新・unit test
- `crates/ruviform-plugin-core/src/boundary.rs` — `HostError::InvalidValue` variant + `Display` + doc
- `crates/ruviform-plugin-core/src/plain_param.rs` — `PlainParamInfoOwned` + `From`/`as_view` 変換
- `crates/ruviform-plugin-core/src/lib.rs` — re-export（`PlainParamInfoOwned` 等）

## Done When

- [ ] `WIRE_VERSION == 3`・`WIRE_VIDEO_MAX_*` 5 定数・`wire::vshm`（`MAGIC`/`HEADER_LEN`/`frame_bytes`/`segment_len`）が codec に存在する
- [ ] `Command` tag 12-22・`Reply` tag 4-7 が encode/decode され、video 動詞の instance-scope 判定（`NONE`/`≥64` → `Malformed`）が効く。audio 動詞 0-11・Reply 0-3・`shm` mod は無変更
- [ ] `VideoProcessConfig`/`PixelFormat`/`VideoFrameDesc`/`ColorInfo`/`PlainParamValue`/`PlainParamInfoOwned` codec が round-trip し、test-spec の拒否行列（bounds・finiteness・presence strictness・`min>max`・`default` out-of-range・`display_*` lenient 受理）が全て defined 拒否（panic・無制限 alloc なし）
- [ ] `HostError::InvalidValue` が boundary + wire tag 10 の両方向で round-trip し wildcard arm で `Malformed`/`UnsupportedVariant` へ潰れない
- [ ] `Hello.capabilities` 非零拒否が v3 で維持され、`Hello{version:2}` は handshake 失敗語彙を保持する
- [ ] `PlainParamInfoOwned` が `From`/`as_view` の一箇所変換を持ち、`#[non_exhaustive]` 公開 enum 規則を維持する
- [ ] codec 層に canonical/region-fit/config 一致の semantic 検証が入っていない（bounds のみ — 責務分離をコードレビューで確認）

## Verification

- [ ] `cargo test -p ruviform-plugin-core` — wire codec unit test（round-trip・scope/拒否行列・`InvalidValue`・`vshm` byte math）が pass
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass（既存 audio suite が `WIRE_VERSION` 3 のまま green — audio 無退行の T1 時点証拠）
- [ ] `grep -n "WIRE_VERSION" crates/ruviform-plugin-core/src/wire.rs` で `= 3` が定数定義に存在し `= 2` が残らない（doc 内の `WIRE_VERSION` `2` 言及は v3 wording へ更新）
- [ ] `cargo doc -p ruviform-plugin-core --no-deps` が warning 0

---
id: "DESIGN-K3VAKLOqVu5SxMRUaZCAso"
type: "design"
status: "approved"
title: "Issue #301 ruviform-io-compression — Zstd Frame 借用 Stream 設計 (IO-IMP-07)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/301"
    - "https://github.com/scarecrow173/ruviform/issues/285"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-compression/src/zstd.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/zstd_format.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-compression/README.md"
      - type: "file"
        path: "crates/ruviform-io-compression/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/api_surface.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "ci/impact/map.mjs"
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-301-io-compression-zstd.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-333-impact-based-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-301-io-compression-zstd.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #301 ruviform-io-compression — Zstd Frame 借用 Stream 設計 (IO-IMP-07)

## Context

Issue #300 で確立した借用 stream 契約（`Encoder`/`Decoder`・明示
`finish`・VerifiedEnd・累積 budget・backend 隠蔽）に、RFC 8878 の
Zstd Frame を載せる。核心判断は 3 点:

1. **Frame 境界は Ruviform 所有** — magic/FHD/Window_Descriptor/
   Dictionary_ID/Frame_Content_Size と skippable frame の境界走査を
   クレート側で parse し、`ZSTD_decompressStream` の auto-skip /
   auto-連結処理には委譲しない。これにより frame 数・byte 精密消費・
   checksum flag 観測・window 上限の確定的強制が可能になる。
2. **低水準 backend** — `zstd-safe` の `CCtx`/`DCtx` streaming
   primitive（`compress_stream2`/`decompress_stream`、in/out buffer
   pos 追跡）を flate2 `Compress`/`Decompress` と同じ役割で使う。
3. **保証できない capability は Unsupported 固定** — dictionary・
   magic-less frame 等は見せかけ receipt を作らず明示拒否。

## Scope

### In Scope

- `src/zstd.rs`（feature `zstd`）: `Encoder`/`Decoder`/`EncodeOptions`/
  `DecodeOptions`/`SkippableFramePolicy` + private FHD parser・
  boundary scan・backend error 分類。
- `src/stream.rs`: private `trait OutPump`（feed/flush_sync/finish）導入、
  `EncoderCore<W, P: OutPump>` 一般化（gzip/deflate の既存
  `DeflateOut` がそのまま実装）。zstd 用 `ZstdOut`/`ZstdIn` pump と
  共有 helper（`take_exact`・`checked_emit`・`EncState`・
  `invalid_state_io`・`to_usize`）の feature ゲート整理。
- `src/lib.rs`/`receipt.rs`/`error.rs` の cfg ゲートに `zstd` を追加。
- `Cargo.toml` feature `zstd = ["dep:zstd-safe"]`・optional dep・
  dev-dep `zstd`（高水準 crate — wire interop 相互検証のみ）。
- `mise.toml` 3 task の feature matrix 拡張・`ci/impact/map.mjs`
  `LANES["compression-verify"].features += "zstd"`・`ci.yml` コメント。
- crate README・`crates/README.md`・crate-level doc。

### Out of Scope

新規クレート・所有型・dict・zstdmt・legacy/magic-less/seekable
format・skippable 書込・gzip/deflate への変更。

## Backend 選定と隠蔽

| 候補 | 採否 | 理由 |
| --- | --- | --- |
| `zstd-safe` 8.2.0 低水準 `CCtx`/`DCtx`（`default-features=false`） | **採用** | `compress_stream2`/`decompress_stream` が `InBuffer.pos`/`OutBuffer.pos` で byte 精密計測を返し、`Ok(0)` が frame 完全終了を示す。`try_create` は OOM 非 panic。`DParameter::WindowLogMax`・`CParameter::{CompressionLevel,ChecksumFlag}` が非 experimental で利用可。BSD-3-Clause・rust-version 1.64（<= MSRV 1.89）。Drop は CCtx/DCtx の解放のみで書込副作用なし。 |
| `zstd`（高水準 crate 0.14.x）`stream::Encoder`/`Decoder` | 不採用（runtime）。dev-dep として相互検証にのみ使用 | `stream::read::Decoder` が BufRead を内部 buffer へ先読みし frame 境界の論理未消費保証が壊れる。`stream::write::Encoder` は Drop で `ZSTD_endStream` 書込を試み「Drop writes nothing」と矛盾。finish が self 消費型で借用契約に合わない。 |
| `ruzstd` 0.9.x（pure Rust） | 不採用 | `StreamingDecoder`/`FrameDecoder` が `Read` から内部 buffer へ先読みし、消費 byte が byte 精密に観測できない（prefetch 境界を壊す → IO-03「観測できない backend は推測 receipt を返さない」に抵触）。encoder も「speed/ratio/configurability が未成熟」と upstream 自身が明記。 |
| `zstd` C lib 直接 FFI | 不採用 | unsafe 方針（`#![forbid(unsafe_code)]`）と重複実装。 |

- build 依存: `zstd-safe` → `zstd-sys` が vendored C source を `cc` で
  コンパイル（cmake 不要・bindgen 不使用）。CI `compression-verify`
  （ubuntu-latest: gcc 有り）と `verify` windows lane（MSVC）で問題ない
  — ただし `zstd` feature は非 default のため `verify`（default
  features のみ）ではコンパイルされず、C toolchain 必須は
  `compression-verify`（ubuntu）とローカル matrix のみ。
- `zstd-safe` の型・`ErrorCode`・`zstd_sys` は一切公開 API に露出
  しない。エラーは `get_error_name(code)` を `source()` の
  type-erased diagnostic（`BackendError`）に載せ、kind は正規化分類。

## Frame header parse（RFC 8878 §3.1.1.1.1.2 — Ruviform 所有）

```
magic(4B LE) | FHD(1B) | Window_Desc(0|1B) | Dict_ID(0|1|2|4B) | FCS(0|1|2|4|8B)
FHD: [7:6]=fcs_flag [5]=single_segment [4]=unused [3]=reserved [2]=checksum [1:0]=did_flag
fcs_len: flag0 -> sss?1:0 ; 1 -> 2(+256) ; 2 -> 4 ; 3 -> 8
header_len <= 18 (FRAMEHEADERSIZE_MAX)
```

- `take_exact` で magic → FHD → 残 field を byte 精密に読み、stack
  buffer（<=18B）で parse。受理した header はその buffer から
  `InBuffer` として backend へ feed（reader 消費計上は take_exact 側 —
  二重計上しない）。
- 確定的拒否（backend 未到達）: reserved bit=1 → `InvalidFormat`、
  DID≠0 → `Unsupported`（dict 未対応）、window 超過 → `LimitExceeded`。
- window_size: `sss` → `max(FCS, 1KiB)`; 非 sss →
  `(1<<(10+exp)) + ((1<<(10+exp))/8)*mantissa`。
  `window_size > 1u128 << max_window_log` で `LimitExceeded`（mantissa
  含む厳密 byte 比較、u128 で overflow 回避）。
- `ZSTD_d_windowLogMax` にも `clamp(max_window_log, 10, 41)` を設定し
  C 側の内部確保を上限付きにする（defense-in-depth。authoritative
  判定は header parse）。
- checksum flag → frame 受理時に `checked/unchecked` を確定し
  `IntegritySummary` へ計上。checksum 自体は frame 終端で backend が
  検証（不一致 → backend error → `ChecksumMismatch` 分類）。

## Skippable Frame

magic `0x184D2A50..=0x184D2A5F`（LE 先頭 byte `0x50..=0x5F`）。
- `SkippableFramePolicy::Accept`（既定）: 4B magic + 4B LE size を
  読み、`size` byte を `take_exact` 相当で skip。data frame ではない
  ため `units_decoded`/`max_units`/`IntegritySummary` に計上しない
  （byte は `compressed_bytes_consumed`/`max_compressed_bytes` に計上）。
- `Reject`: `Unsupported` で拒否。
- backend の auto-skip に委譲しない（policy 強制と観測のため）。

## Decoder 状態機械

```
Start --EOF--> TruncatedInput | --非magic--> InvalidFormat
      --zstd--> header parse --(受理)--> Body
      --skippable--> policy(skip/reject) -> Start再評価相当
Body  --produced--> emit | --starved--> TruncatedInput
      --frame end(ret==0)--> Between(WholeInput)|Done(SingleUnit)
Between --EOF--> Done | --zstd--> units cap 確認 -> header -> Body
        --skippable--> policy -> Between | --その他--> TrailingData
Done = VerifiedEnd （finish() -> Ok(DecodeReceipt)・冪等）
*    --失敗--> Failed （read/finish -> InvalidState）
```

- `units` は data frame の header 受理時に +1（gzip member と同じ
  計上点）。`max_units` 確認は header 読取前。
- `ZstdIn::pump`: `fill_buf` → `decompress_stream` → `in.pos` 分だけ
  `consume`（byte 精密・frame 終端以後は論理未消費）。`starved` =
  fill_buf 空 ∧ produced==0 ∧ !end。
- DCtx は frame 間で session を reset しない — `decompress_stream`
  は `Ok(0)` 後に次 frame header を受理する設計のため、次 frame の
  header byte をそのまま feed する（skippable/inter-frame byte は
  Ruviform が先に走査するため backend には data frame byte のみ渡る）。
- DCtx 生成は lazy（最初の pump 時 `try_create` + `WindowLogMax` 設定）。
  `Decoder::new` は gzip/deflate と同じく infallible を維持。
- dict 非宣言だが raw-content dict で圧縮された frame は形式仕様上
  区別不能 — backend が `Dictionary mismatch`/corruption error を
  返し `Unsupported`/`CorruptData` に分類（見せかけ成功はしない）。

## Encoder

- `ZstdOut`（`OutPump` 実装）: `CCtx::try_create` +
  `set_parameter(CompressionLevel(level))`（範囲外 `InvalidInput`）+
  `ChecksumFlag(bool)`。`feed` = `compress_stream2(Continue)`、
  `flush_sync` = `compress_stream2(Flush)` until ret==0、
  `finish` = `compress_stream2(End)` until ret==0（epilogue+checksum）。
- `Encoder<'w, W>` = `EncoderCore<'w, W, ZstdOut>`（gzip/deflate と同一
  状態機械・budget・`finish(&[])` — frame epilogue は pump 側が出す）。
- `EncodeOptions { level: i32 = 0（backend 既定≈3）, checksum: bool =
  false（zstd lib 既定）, limits }`。level 範囲は
  `min_c_level()..=max_c_level()` で `Encoder::new` 時に検証
  （`CompressionLevel` param 設定失敗も `InvalidInput`）。
- `ContentSizeFlag`/`set_pledged_src_size` は呼ばない — stream 中の
  総量不明のため FCS は emit されない（deterministic header）。

## Error 分類（backend `ErrorCode` → `get_error_name`）

| backend error（name 含有） | kind |
| --- | --- |
| `checksum`（"Restored data doesn't match checksum"） | `ChecksumMismatch` |
| `too much memory`（"Frame requires too much memory for decoding"） | `LimitExceeded` |
| `Dictionary`/`dictionary` | `Unsupported` |
| `memory`/`Allocation`（"Allocation error : not enough memory"） | `Io` |
| `parameter`（`set_parameter` 失敗） | `InvalidInput` |
| その他（"Data corruption detected" 等） | `CorruptData` |

- name は backend 安定文字列だが `source()` にも保存し、分類のずれは
  Negative test で pin（変更時は test で検出）。
- `try_create` `None`（CCtx/DCtx 割当失敗）→ `Io`（`from_kind` —
  到達不能に近いが panic 回避を優先）。

## Module 構成

| ファイル | 変更 |
| --- | --- |
| `src/lib.rs` | doc table + `#[cfg(feature="zstd")] pub mod zstd;` + stream cfg 拡張 |
| `src/stream.rs` | `OutPump` trait・`EncoderCore<W,P>` 一般化・flate2 部を `any(gzip,deflate)` cfg・`take_exact` を `any(gzip,zstd)`・`ZstdOut`/`ZstdIn` pump（`#[cfg(feature="zstd")]`） |
| `src/zstd.rs` | 新規。options・policy enum・Encoder/Decoder・FHD parser・boundary scan・error 分類 |
| `src/receipt.rs` | `new` の cfg に `zstd` 追加 |
| `src/error.rs` | `CompressionError::io` の cfg に `zstd` 追加 |
| `Cargo.toml` | `zstd = ["dep:zstd-safe"]`・dep・dev-dep `zstd` |
| `mise.toml` | 3 task の matrix に `zstd`・`gzip,deflate,zstd` を追加 |
| `ci/impact/map.mjs` | `features: ["gzip","deflate","zstd"]` |
| `ci.yml` | lane コメント更新（job 本体は mise task 経由で変更不要） |

## Risks and Trade-offs

- **backend error name 文字列分類**: zstd のエラー名は安定文字列だが
  upstream 変更リスクあり → `source()` 保存 + Negative test で pin。
  分類不能は `CorruptData` 側に倒す（誤って成功させない方向）。
- **header 二重 parse**: Ruviform parse と backend parse は同一 byte を
  対象（drift なし）。backend が受理済 header を再拒否する余地は
  残る（frameParameter_unsupported 等 → `CorruptData` 相当）。
- **`no_std`**: `zstd-safe` は no_std だが本 crate は std 前提
  （`std::io`）。`default-features=false` で `std` feature は不要
  （`[u8]` の `WriteBuf` は無条件実装）。
- **skippable skip の読取**: `size` byte を chunk 読捨て
  （allocation なし・compressed budget 計上）。巨大 size は
  `max_compressed_bytes` で既に制限。
- **zstd feature 無しでの dead code**: stream.rs の共有 helper は全て
  `#[cfg]` で最細粒度に gate し、各 feature 組合せの `-D warnings`
  で検証（lane の matrix が担保）。

## Verification Notes

- [ ] test-spec `docs/test-specs/issue-301-io-compression-zstd.md` の
      AC1-AC6 が `tests/zstd_format.rs` + contract test 拡張に落ちる。
- [ ] `mise run fmt`/`check`/`lint`/`test` + feature matrix
      （default・zstd・gzip,deflate,zstd）全緑 + `compression-verify`。
- [ ] `cargo doc` で `zstd_safe`/`zstd_sys` 型が公開 API に現れない。

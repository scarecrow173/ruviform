---
id: "TSPEC-9sy40kbL7d1xqwhmY2sDLw"
type: "test-spec"
status: "approved"
title: "Issue #301 ruviform-io-compression Zstd Frame 試験仕様"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/301"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-compression/tests/zstd_format.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/api_surface.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-301-io-compression-zstd.md"
    - "docs/plans/issue-301-io-compression-zstd.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-300-io-compression-gzip-deflate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-301-io-compression-zstd.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-301-io-compression-zstd.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #301 ruviform-io-compression Zstd Frame 試験仕様

## Scope

`zstd` feature（`--cfg feature="zstd"`）の format 固有試験
`tests/zstd_format.rs` と、共有契約 test の zstd 拡張
（`encoder_contract.rs`/`decoder_contract.rs`/`api_surface.rs`/
`common/mod.rs`）。frame byte は dev-dep `zstd` crate で生成
（`zstd::bulk::compress`・`zstd::stream::Encoder` で checksum 有無・
level を制御）、header 破損・skippable frame 合成のみ最小限の手組。

## Cases

### AC1 — Single/Concatenated/Skippable

| case | 手順 | 期待 |
| --- | --- | --- |
| single round-trip | `zstd` crate で圧縮 → `Decoder` WholeInput | byte 一致・`units_decoded=1`・`compressed_bytes_consumed` = 全 byte |
| concatenated | 2 frame 連結 → WholeInput | byte 一致・`units=2` |
| skippable accept | `0x184D2A50 + size + payload` を data frame の前/間/後に配置・`Accept` | data のみ復号・`units` は data frame 数のみ・`integrity` 0 |
| skippable reject | 同上・`Reject` | `Unsupported` |
| skippable のみ | data frame なし | WholeInput: skippable のみで終端 → `units=0`・receipt OK（`decoded=0`） |
| SingleUnit | frame + 非 frame 連結 | `units=1`・`compressed_bytes_consumed`=先頭 frame のみ・残りは `read_to_end` で byte 一致 |

### AC2 — Checksum 混在

| case | 手順 | 期待 |
| --- | --- | --- |
| checked frame | `checksum=true` で圧縮 | `integrity.checked=1`/`unchecked=0` |
| unchecked frame | `checksum=false`（既定） | `checked=0`/`unchecked=1` |
| mixed concatenated | checked+unchecked 連結 | `checked=1`/`unchecked=1` |
| checksum corrupt | checked frame の末尾 checksum byte 反転 | `ChecksumMismatch` |
| content corrupt | body byte 反転 | `CorruptData` or `ChecksumMismatch`（backend 検出点） |

### AC3 — Window/破損/切断/末尾

| case | 手順 | 期待 |
| --- | --- | --- |
| window 超過 | `Window_Descriptor` を大 exp で手組、`max_window_log` 既定 | `LimitExceeded` |
| window 許容 | 小 exp + 明示 `max_window_log` | pass |
| `max_window_log` 範囲外 | `DecodeOptions{max_window_log: >41}` | `Encoder`/`Decoder::new` で `InvalidInput` |
| wrong magic | 先頭 byte を非 magic | `InvalidFormat` |
| reserved FHD bit | FHD bit3=1 | `InvalidFormat` |
| DID≠0 | did_flag=1..3 で DID byte 合成 | `Unsupported` |
| truncated header | magic のみ / FHD 途中 | `TruncatedInput` |
| truncated body | frame 中間切断 | `TruncatedInput` |
| truncated checksum | checksum 数 byte 不足 | `TruncatedInput` |
| trailing data | frame + 非 frame byte | `TrailingData` |

### AC4 — Budget 厳密

| case | 手順 | 期待 |
| --- | --- | --- |
| decoded ちょうど | `max_decoded_bytes=payload_len` | pass |
| decoded +1 超過 | `max_decoded_bytes=payload_len-1` | `LimitExceeded` |
| compressed ちょうど | `max_compressed_bytes=frame_len` | pass |
| compressed +1 超過 | `frame_len-1` | `LimitExceeded` |
| units ちょうど | `max_units=frame_count` | pass |
| units 超過 | `max_units=frame_count-1` | `LimitExceeded`（2 個目 frame header で拒否） |
| skippable は units 対象外 | `max_units=1` + skippable+data | `units=1` pass |
| many-small-frame | 64 小 frame 連結 | 累積 decoded/units 厳密 |
| encode input/output cap | `EncodeLimits{max_input_bytes,max_output_bytes}` | `LimitExceeded` |
| AtOutputLimit 相当 | decoded cap 到達後も残出力 | `finish` → `NotExhausted` |

### AC5 — 境界/観測

| case | 手順 | 期待 |
| --- | --- | --- |
| 論理消費 | `BufReader::new(&[frame1+frame2])` SingleUnit | `compressed_bytes_consumed`=frame1・残り caller 側で読める |
| prefetch 非混同 | 同上で `BufReader` capacity > 入力 | receipt は論理消費・物理 prefetch と一致しない |
| `finish` 未読 | read せず `finish` | `NotExhausted`・出力非破棄・再 read→finish 成功 |
| `read(&mut [])` | 空 buf | `Ok(0)` でも finish は検証独立 |
| finish 冪等 | 2 回目 | 保存 receipt |
| Failed latch | corrupt 後の read/finish | `InvalidState` |
| Drop | Writer 借用 drop | 一切書込なし |
| write 部分 | partial write → continue | byte 一致 |
| `zstd` crate interop | `zstd::stream::decode_all` で復号 | byte 一致 |

### AC6 — feature/公開表面

| case | 期待 |
| --- | --- |
| `zstd` 単独 feature compile | pass |
| `gzip,deflate,zstd` compile+test | pass |
| `Send` bound | `zstd::{Encoder,Decoder}` + option/policy が Send |
| backend 型非公開 | `cargo doc` で `zstd_safe`/`zstd_sys` 不在（手動確認） |
| MSRV/unsafe | `cargo +1.89 check` 相当（lane）・`unsafe_code` lint |

## Non-goals

- Preset dict frame の decode 成功（`Unsupported` 拒否のみ pin）。
- `zstdmt`・legacy/magic-less/seekable format。
- backend byte 列完全一致（round-trip byte 一致のみ）。

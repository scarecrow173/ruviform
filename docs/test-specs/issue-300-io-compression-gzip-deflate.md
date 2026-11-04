---
id: "TSPEC-c6xrG6sZjlJbmoJEzBYzJG"
type: "test-spec"
status: "approved"
title: "Issue #300 ruviform-io-compression Gzip・Raw Deflate テスト仕様"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# Issue #300 ruviform-io-compression Gzip・Raw Deflate テスト仕様

## Purpose

`ruviform-io-compression` の Gzip/Raw Deflate 借用 Stream が、IO-03 v0.1/
v0.2 で合意した「明示 `finish` 完了・VerifiedEnd・Backend 隠蔽・累積
Budget・member 境界」を厳密に守ることを pin する。Backend（flate2）の
高水準 API に由来する先読み・暗黙 finish といった未検証挙動は本 spec
ではなく設計書が排除済みであり、ここでは Ruviform 公開契約の厳密性を
検証する。未検証・範囲外の保証は正直に記録する。

## Feature

Gzip（RFC 1952）と Raw Deflate（RFC 1951）の借用
`Encoder<'w, W: Write + ?Sized>` / `Decoder<'r, R: BufRead + ?Sized>`
による byte-stream 変換を、`std::io` 境界と in-memory fixture で検証する。

## Rules

- R1（Encoder 契約）: `finish()` は `Finishing` 経路を 1 度だけ通り
  `Finished` で冪等（2 回目は同一 receipt）。成功後の write は
  `io::Error` + `InvalidState`。`Failed` では write/finish 両方が
  `InvalidState` で panic しない。
- R2（Drop/flush/借用）: `Drop` は一切書込しない（backend の implicit
  trailer 書込なし）。`flush()` は deflate sync marker + 下流
  `W::flush` まで伝搬。`write` は入力全体受理か Err。`Encoder` の
  scope が終われば `&mut W` の借用は呼出側へ戻る。
- R3（Decoder 完了）: `finish()` は `VerifiedEnd` のみ成功。未読出力
  が残る（unit 未終端・buffer 未 drain）状態では `NotExhausted`。
  非空 `&mut [u8]` の `read` が `Ok(0)` を返した時点でのみ VerifiedEnd
  になり、`read(&mut [])` の `Ok(0)` は完了証拠にしない。
- R4（gzip 形式）: 単一 member/連結 member を WholeInput で全 unit
  復号。header の FEXTRA/FNAME/FCOMMENT/FHCRC を parse し、trailer
  の CRC32/ISIZE 不一致は `ChecksumMismatch`、unit 未完の EOF は
  `TruncatedInput`、末尾 trailing byte は `TrailingData`。
- R5（raw deflate）: checksum なし・単一 deflate stream。`WholeInput`
  でも `SingleUnit` でも 1 unit のみ消費し、残 byte は呼出側に残る
  （trailer 無し）。WholeInput で残 byte があれば `TrailingData`。
- R6（scope/境界）: `SingleUnit` は先頭 unit 終端で停止し、以後の byte
  を caller の BufRead 内部に残す。`compressed_bytes_consumed` は本
  クレートが `consume`/`read_exact` した論理消費量。BufRead の物理
  prefetch（caller 所有）は receipt に含まず混同しない。
- R7（Budget）: `max_decoded_bytes` ちょうど一致 + unit 終端は正常。
  +1 byte で `LimitExceeded`（超過 byte は Consumer に返さない）。
  `max_compressed_bytes`/`max_units` も同様に厳密。`EncodeLimits`
  の入力/出力 cap は「上限を超える物理書込を行わない」。
- R8（feature/層）: `default=[]` で compile、`gzip`/`deflate` 各
  feature が独立・併用で build 可能。公開 API に flate2 型なし、
  ruviform-* 依存ゼロ、全 API nonRT、MSRV 1.89、unsafe なし。
- R9（合成）: TAR 様 byte 列（512B header+payload+終端 block）を
  `gzip::Encoder<&mut Vec<u8>>` で書き、decoder で byte 一致。
  `finish` の順序が合成時に維持される。

## Examples

- ENC-01: `gzip::Encoder::new(&mut vec, opts)` → `write_all(payload)` →
  `finish()` → receipt `{input_bytes, output_bytes, units_encoded=1}`。
  `finish()` 2 回目は同一 receipt。`write(b"x")` で `InvalidState`。
- ENC-02: writer が 2 回目 `write` で `io::Error` を返す fake で
  `Failed` → その後 `finish()`/`write()` が `InvalidState`。
- ENC-03: `flush()` 呼出しで sync flush marker（`00 00 ff ff` 系）
  が emit され、以後の write/finish が継続動作する。
- ENC-04: `drop(enc)` で `vec` の byte 数が変わらない（header 以外
  何も書かれない/ trailer も書かれない）。
- DEC-01: WholeInput で単一 `.gz` を read_to_end → `Ok(0)` 後の
  `finish()` が `Ok(receipt)`（`checked_units=1`）。
- DEC-02: `member1 ++ member2 ++ trailing` の WholeInput で
  `TrailingData`、SingleUnit で member1 のみ復号し残 byte が
  `reader.fill_buf()` で残存。
- DEC-03: trailer の CRC32 を 1 byte 反転 -> `ChecksumMismatch`、
  ISIZE を +1 -> `ChecksumMismatch`、trailer を 1 byte 欠落 ->
  `TruncatedInput`。
- DEC-04: `max_decoded = payload.len()` で正常、`+1` で `LimitExceeded`、
  `payload.len()-1` では上限到達後の probe が LimitExceeded。
- DEC-05: `max_units=1` で member 連結は `LimitExceeded`、
  `max_compressed = compressed.len()-1` で `LimitExceeded`。
- BUF-01: `CountingReader`（raw read byte 計測）+ `BufReader` で
  decoder の `compressed_bytes_consumed` が論理消費と一致し、
  `CountingReader` 物理 prefetch は consumed を含んで大きい
  （両者を別の値として観測）。
- INT-01: TAR 様 entry byte を encode->decode で byte 一致、
  decode 前の `finish` 成功が publish 順序を担保（契約記述どおり）。

## Non-Goals

- Backend 内部 heap/CPU 時間の Hard Limit、backend version 間の
  byte 列完全一致、ネットワーク FS/電源断の保証、Zstd、zlib wrap、
  所有型 API、Archive 側 codec 統合、実 FS 動作（io-file が別責務）。
- BufReader サイズや CPU 時間の性能 assertion（fault/fuzz の強制は
  本 spec 範囲外。破損/上限等の deterministic fault は範囲内）。

## Sources

- `docs/specs/issue-300-io-compression-gzip-deflate.md`（AC1-AC6）
- `docs/designs/issue-300-io-compression-gzip-deflate.md`（framing・
  状態機械・scope/limits/error mapping）
- Issue #300 本文の Acceptance 列挙（finish 冪等・partial write・
  Gzip member・corrupt trailer・truncated・trailing・raw deflate・
  BufRead 保持・Verified EOF・Budget 上限ちょうど/超過・default=[]/
  feature・Backend 非公開・MSRV・TAR 合成）。

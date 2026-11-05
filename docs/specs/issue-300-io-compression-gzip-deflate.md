---
id: "SPEC-iRf6rX6sS5vJpc9b5uOxEk"
type: "spec"
status: "approved"
title: "Issue #300 ruviform-io-compression — Gzip・Raw Deflate Stream 変換・Finish・Checksum (IO-IMP-06)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/300"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-compression/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-compression/README.md"
      - type: "file"
        path: "crates/ruviform-io-compression/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/options.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/gzip.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/deflate.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/format_cases.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/composition.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "ci/impact/map.mjs"
      - type: "file"
        path: "ci/impact/tests/classify.test.mjs"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/designs/issue-333-impact-based-ci.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #300 ruviform-io-compression — Gzip・Raw Deflate Stream 変換・Finish・Checksum (IO-IMP-06)

## What Should Be Built

独立クレート `ruviform-io-compression` を scaffold し、IO-03（Issue #285
v0.1/v0.2 合意設計）のうち **Gzip（RFC 1952）と Raw Deflate（RFC 1951）**
の Stream 変換基盤を実装する。`std::io::{Read, Write, BufRead}` のみへ接続
し、Archive・File I/O・Serde・他 ruviform-* クレートへ一切依存しない。

提供する公開 API:

- `gzip` / `deflate` module の Format 別 `Encoder<'w, W: Write + ?Sized>` /
  `Decoder<'r, R: BufRead + ?Sized>` — 底流 Writer/Reader を**借用**する
  初期契約（所有型は必要性確認後に別設計）。
- Format 別 `EncodeOptions` / `DecodeOptions`。共有型
  `EncodeLimits` / `DecodeLimits` / `DecodeScope`。
- `EncodeReceipt` / `DecodeReceipt` / `IntegritySummary` と
  `CompressionError` / `CompressionErrorKind`（`#[non_exhaustive]`）。
- 明示 `finish()` のみが完了証拠を構成する状態機械:
  Encoder `Writing -> Finishing -> Finished | Failed`、
  Decoder `Reading / AtOutputLimit / VerifiedEnd / Failed`。
- Gzip の CRC32/ISIZE 検証・single/concatenated Member・Trailing 拒否と、
  Checksum なし単一 Stream の Raw Deflate の明確な区別。
- Input/Output Byte Budget・Member/Stream 数の累積制限。CPU 時間や
  Backend 内部メモリを Hard Limit とは宣言しない。

Backend は `flate2`（pure-Rust `rust_backend` / miniz_oxide 系統）の
低水準 `Compress` / `Decompress` / `Crc` のみを内部利用し、その型・
エラー・version を公開 API に露出しない。Gzip header/trailer の framing
は Ruviform 側で所有し、Member 境界・byte 計測・Drop 副作用を
Backend の挙動から切り離す。

## Why Now

IO チェーン（IO-01 File 実 OS backend 完了後）で Archive/Structured 系の
次の必須ピースであり、Consumer（AudioLens #139・Synvelis #82）が
TAR.GZ・StagedFile への圧縮合成を必要とする。IO-IMP-07（#301）が Zstd を
担当するため、公開型・Feature 名・共有 vocab（`DecodeScope`・
`IntegritySummary`・`CompressionErrorKind`）を Zstd 追加と矛盾しない
形で先に確立する必要がある。

## Users and Value

- Ruviform 上の Archive（IO-04）・Structured export・StagedFile 合成:
  `tar writer -> gzip::Encoder -> StagedFile` という `std::io` 合成で
  「圧縮 finish 成功 -> seal -> publish」の順序を強制できる。
- 部分読み取り・連結 member・破損/切断入力に対し、Consumer が
  「検証済み終了」を `DecodeReceipt` として受け取れる。
- 後続 Issue #301（Zstd）・#306（Cross-Consumer E2E）が依存する
  独立クレートの土台。

## Scope

### In Scope

- `crates/ruviform-io-compression` 新設。`default = []`、features
  `gzip` / `deflate`（`dep:flate2` で共有）。層位置は
  `ruviform-io-file` と同じ汎用 I/O 基盤層・ruviform-* 依存ゼロ。
- `flate2 = { version = "1.1.10", default-features = false,
  features = ["rust_backend"] }` の internal backend 採用
  （公開 API への露出なし・pure Rust・MSRV 1.67 <= 1.89・
  MIT OR Apache-2.0）。
- Encoder: `impl Write`・`flush()` = sync flush + 下流伝搬・
  `finish()` = deflate StreamEnd +（gzip のみ）trailer 書込・
  成功後の `finish()` 冪等・Failed/Finished での `write()` 拒否
  （`io::Error` + `InvalidState` cause）・Drop は一切書込しない。
- Decoder: `impl Read`（展開 byte 逐次）・`finish()` は `VerifiedEnd`
  のみ成功・未読出力がある限り `NotExhausted`・空 `&mut []` の
  `read` 0 を EOF 証拠にしない・`Read::read` の返す論理 EOF と
  `finish()` の検証結果は独立。
- `DecodeScope::{WholeInput, SingleUnit}`: WholeInput は連結 member/
  deflate stream 全体 + 入力 EOF まで検証（末尾の非 member byte を
  `TrailingData` で拒否）。SingleUnit は先頭 unit 終端で停止し、
  後続 byte を呼出側 BufRead に残す。
- `DecodeLimits { max_compressed_bytes, max_decoded_bytes, max_units }`
  （連結 Stream 全体で累積）。上限ちょうどは正常許可、+1 byte で
  `LimitExceeded`・超過 byte は Consumer に返さない。
- `EncodeLimits { max_input_bytes, max_output_bytes }`（累積）。
- Gzip framing は自前実装: header（FEXTRA/FNAME/FCOMMENT/FHCRC 対応・
  member あたり 64KiB 上限）+ deflate body + CRC32/ISIZE trailer 検証。
- `CompressionErrorKind`（v0.1 提案 + 拡張）: `Io / InvalidInput /
  InvalidFormat / TruncatedInput / CorruptData / ChecksumMismatch /
  TrailingData / LimitExceeded / Unsupported / Cancelled / InvalidState /
  NotExhausted`。`impl Read/Write` では `io::Error` に包んでも
  `source()` で `CompressionError` に到達可能。
- 検証: Golden/Negative 試験・feature 別 `cargo check`・
  TAR 様バイト列 + `Vec<u8>` + BufReader 合成試験・
  feature 専用 CI lane（`compression-verify`）を icc-verify 型で追加。
- `Cargo.toml` members・`crates/README.md`・`mise.toml`・
  `ci/impact/*`・`.github/workflows/ci.yml` の規約どおりの更新。

### Out of Scope

- Zstd（IO-IMP-07 / Issue #301 が所有。`zstd` feature・Frame/Skippable/
  Window・`max_window_log` 等は本 Issue では追加しない）。
- zlib wrapper（RFC 1950）。Raw Deflate とは別形式であり必要時に明示追加。
- Writer 所有型 Encoder、Reader 所有型 Decoder、`async`、
  `no_std` 対応、独自圧縮アルゴリズム。
- Archive（ZIP/TAR）側の Codec/Entry capability — IO-04 の責務。
  本 crate は TAR 実装を持たず `std::io` 合成のみ検証する。
- Consumer 側の移行・AudioLens Job/Dataset・Synvelis Project/Revision
  意味論（Consumer の責務）。
- CPU 時間・Backend 内部 heap・出力 Sink 側メモリの Hard Limit 宣言。
- OS/FS・ネットワーク FS の保証（`std::io` 境界以上を主張しない）。

## Requirements

- 借用契約: `Encoder<'w, W: Write + ?Sized>` / `Decoder<'r, R: BufRead +
  ?Sized>` は `&'w mut W` / `&'r mut R` を取り、失敗時を含め Writer/
  Reader の所有権は呼出側に留まる（scope 終了で借用が戻る）。
- 公開 enum は全て `#[non_exhaustive]`（ADR 準拠）。Backend 由来の
  `flate2` 型・エラー・`Compression` 等が公開 API に現れないこと。
- `finish()` 成功のみが Format 終了の証拠。Drop で backend の
  trailer 書込等の副作用を起こさず、「Drop では何も保証しない」を
  公開契約として明記。
- Decoder の `finish()` は Backend の同名操作へ無条件委譲せず、
  未読出力を破棄しない（`NotExhausted` で拒否）。
- Budget は累積・`u64` 単位で追跡し、上限到達を即座に EOF と誤認しない
  （有界 probe で超過の有無を判定）。
- 同一入力・同一設定の round-trip byte 一致を保証。Backend version 間の
  圧縮 byte 列完全一致は契約に含めない。
- Gzip header に filename/mtime/OS 依存値を初期既定で持たせない
  （再現性・機密性。固定 10 byte header: `1f 8b 08 00 00 00 00 00
  <xfl> ff`）。
- MSRV 1.89・edition 2024・`workspace.lints`・`-D warnings`・
  `publish = false`・unsafe 不使用（`unsafe_code` warn 継承で deny）。
- `mise run fmt` / `check` / `lint` / `test` 全緑 + feature matrix
  （`default`、`gzip` のみ、`deflate` のみ、`--all-features`）で
  check が通ること。

## Acceptance Criteria

- [ ] AC1（Encoder 契約）: `finish()` が冪等（2 回目は保存 receipt を
      返し backend を再終了しない）、`write` の partial write 処理、
      `flush()` の sync flush + 下流伝搬、Drop が物理書込を行わない
      こと、Failed 後の `write()`/`finish()` が `InvalidState` で拒否
      され panic しないこと、借用 Writer が scope 後に回復されることを
      試験で pin する。
- [ ] AC2（Gzip 形式境界）: 単一 member・連結 member（WholeInput で
      全 unit 復号）、corrupt trailer（CRC/ISIZE 不一致 ->
      `ChecksumMismatch`）、truncated input（-> `TruncatedInput`）、
      末尾 trailing byte（-> `TrailingData` 相当の拒否）、
      header 付加フィールド（FNAME/FCOMMENT/FEXTRA/FHCRC）を Golden/
      Negative で検証。
- [ ] AC3（Decoder 検証終了）: `finish()` が未読出力を捨てず
      `NotExhausted` を返すこと、非空 buffer の `read` が `Ok(0)` を
      返した時点のみ VerifiedEnd、`compressed_bytes_consumed` が論理
      消費量（SingleUnit で後続 byte 未消費）を正確に計測し caller の
      BufRead prefetch（物理取得）と混同しないことを検証。
- [ ] AC4（Budget）: `max_decoded_bytes` ちょうど一致は正常終了、
      +1 byte で `LimitExceeded`、`max_compressed_bytes`・`max_units`
      （member 累積）も同様に厳密。`EncodeLimits` の input/output cap。
      CPU/Backend メモリを Hard Limit と文書でも宣言しない。
- [ ] AC5（feature/DAG/非 RT）: `default=[]` で crate が compile し、
      `gzip`/`deflate` 各 feature が独立・併用で build 可能。Backend 型
      非公開・MSRV 1.89・unsafe なし・全 API nonRT・ruviform-* 依存
      ゼロを `cargo metadata` 的にも確認。
- [ ] AC6（TAR 合成）: TAR 様 byte 列（512B header + payload + 終端
      block）を `gzip::Encoder<&mut Vec<u8>>` 経由で書き、
      `gzip::Decoder<&mut BufReader>` で復号して byte 一致すること、
      `finish` 順序の契約が合成時に維持されることを試験。

## Deferred Design Concerns

- Zstd `zstd::EncodeOptions`/`DecodeOptions`（`max_window_log` 等）、
  Skippable Frame・mixed Checksum の `IntegritySummary` 拡張 |
  IO-IMP-07（#301）実装時 | 本 Issue の共有型を拡張可能な形に留める。
- Writer 所有型 `finish(self) -> Result<W, _>` | `&mut` 借用で不足する
  実需が確認された時点 | 現段階では YAGNI。
- zlib（RFC 1950）wrap 形式 | ZIP 内 codec 等の需要が確定した時点 |
  Raw Deflate と混同しないため本 Issue では明示的に対象外。
- Header 64KiB 上限・probe buffer 大の調整 | 実害報告が出たら |
  初期値は防衛的定数として文書化。

## Sources and Relations

- 正本: Issue #300 本文（作業範囲・Acceptance・完了条件）。
- 設計基準: Issue #285（IO-03）v0.1/v0.2 合意追補、Issue #282（統合
  設計 #1-#11）、Issue #288（IO-06 Cross-Consumer 受入）。
- 形式仕様: RFC 1951（DEFLATE）、RFC 1952（GZIP）。
- 実装基盤: `docs/specs/issue-295-io-file-staged-publish-contract.md`・
  `docs/designs/issue-296-io-file-os-backend.md`（層・文書駆動先例）。
- Backend: `flate2` 1.1.10（`rust_backend` = miniz_oxide + crc32fast。
  採用理由・license は `crates/ruviform-io-compression/Cargo.toml`
  コメントに記録）。

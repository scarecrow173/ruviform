---
id: "DESIGN-eGBdnzbpDjWwMwTEldA9gI"
type: "design"
status: "approved"
title: "Issue #300 ruviform-io-compression — Gzip・Raw Deflate 借用 Stream 設計 (IO-IMP-06)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/300"
    - "https://github.com/scarecrow173/ruviform/issues/285"
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
  depends-on:
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
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
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #300 ruviform-io-compression — Gzip・Raw Deflate 借用 Stream 設計 (IO-IMP-06)

## Context

Issue #285（IO-03）v0.1/v0.2 で合意済みの設計方針を、Gzip・Raw Deflate
の 2 Format に限定して実装可能な Rust 型・状態機械・framing 契約へ固定
する。#300 は独立 `ruviform-io-compression` クレートの scaffold Owner で
あり、#301（IO-IMP-07 Zstd）が共有語彙を拡張する前提を置く。

設計上の核心判断は 3 点:

1. **framing は Ruviform 所有** — Gzip header/trailer の parse/生成を
   backend（flate2）に委譲せず、deflate 本体だけを backend の低水準
   `Compress`/`Decompress` に任せる。これにより byte 単位の消費計測・
   member 境界保持・Drop 副作用ゼロを backend 依存なしに保証する。
2. **借用型のみ** — `Encoder<'w, W: Write + ?Sized>` /
   `Decoder<'r, R: BufRead + ?Sized>` を初期公開形とし、所有型は
   実需確認まで作らない（IO-03 v0.2 A 準拠）。
3. **`finish()` のみが検証済み完了** — Drop・`Ok(0)` の空 buffer
   read・backend の implicit finish を完了証拠としない。

## Scope

### In Scope

- 新規クレート `crates/ruviform-io-compression`（`default=[]`、
  features `gzip`/`deflate`、internal dep `flate2` rust_backend のみ、
  ruviform-* 依存ゼロ・全 API nonRT）。
- 共有型: `CompressionError(Kind)`・`EncodeLimits`/`DecodeLimits`・
  `DecodeScope`・`EncodeReceipt`/`DecodeReceipt`/`IntegritySummary`。
- `gzip`/`deflate` 各 module の `Encoder`/`Decoder`/`EncodeOptions`/
  `DecodeOptions`。
- feature 専用 CI lane `compression-verify`（icc-verify 型・ubuntu のみ）
  と classifier ルール、mise task 3 本、crate/索引・impact 設計書の更新。

### Out of Scope

- Zstd・zlib wrapper・所有型 API・async・no_std・Archive 側 codec 統合・
  Consumer 移行・物理 FS 保証（#295/#296 が別責務）。

## Backend 選定と隠蔽

| 候補 | 採否 | 理由 |
| --- | --- | --- |
| `flate2` 低水準 `Compress`/`Decompress`/`Crc`（`rust_backend`） | **採用** | pure Rust（miniz_oxide + crc32fast）、`total_in`/`total_out` で byte 精密計測可、MSRV 1.67 <= 1.89、MIT OR Apache-2.0、メンテ活発（1.1.10, 2026-08）。 |
| flate2 高水準 `GzEncoder`/`GzDecoder`/`MultiGzDecoder` | 不採用 | BufRead から先読みして member 境界の未消費 byte 保証が壊れる、`finish` が `self` 消費または Drop で trailer 書込を試みる（IO-03 A の「Drop 副作用を保証と切り離す」と矛盾）。 |
| `libz-sys`/C backend | 不採用 | C toolchain・unsafe 方針・MSRV/配布の複雑化。 |
| 独自 DEFLATE 実装 | 不採用 | IO-03「独自圧縮アルゴリズムを作らない」。 |

- `flate2` の公開型は一切露出しない: `flate2::Compression` は内部値に
  留め、`DecompressError`/`CompressError` は `source()` の type-erased
  diagnostic としてのみ保持。
- `default-features = false, features = ["rust_backend"]` で
  `runtime_detection`/`zlib-rs`/C backend を依存木に入れない。

## 公開 API 外形

```text
ruviform-io-compression
├── CompressionError { kind: CompressionErrorKind, source: Option<Box<dyn Error+Send+Sync>> }
├── CompressionErrorKind #[non_exhaustive]
├── DecodeScope #[non_exhaustive]      { WholeInput, SingleUnit }
├── EncodeLimits / DecodeLimits        (u64 3 軸 + builder)
├── EncodeReceipt { input_bytes, output_bytes, units_encoded }
├── DecodeReceipt { compressed_bytes_consumed, decoded_bytes_delivered,
│                    units_decoded, integrity }
├── IntegritySummary { checked_units, unchecked_units }
├── gzip   (feature gzip):    Encoder<'w,W>, Decoder<'r,R>,
│                             EncodeOptions, DecodeOptions
└── deflate(feature deflate): Encoder<'w,W>, Decoder<'r,R>,
                              EncodeOptions, DecodeOptions
```

- `Encoder::new(writer: &'w mut W, options) -> Result<Self, CompressionError>`
  （gzip はここで固定 header を即時書込する — 下流 I/O 失敗を new で
  報告するため `Result`）。
- `Decoder::new(reader: &'r mut R, options) -> Self`（読取は read() 側で
  lazy に開始。`Result` を取らない — 入力の妥当性は format layer が
  進行中に判定する）。
- `finish(&mut self) -> Result<EncodeReceipt | DecodeReceipt,
  CompressionError>`。
- Format の `EncodeOptions` は現時点で `{ level: u32 }`（0..=9、既定 6）。
  `level` の semantic は DEFLATE level であり、Format 横断の統一型には
  しない（Zstd は別 options を追加）。

## Encoder 状態機械

```text
new() ──header(gzip)──> Writing
Writing --write--> Writing        (input 全体を compress -> write_all)
Writing --flush--> Writing        (Sync flush + writer.flush())
Writing --finish-> Finishing      (Finish flush -> StreamEnd -> trailer(gzip))
Finishing  --ok---> Finished      (receipt 保存)
Finishing  --err--> Failed
Finished   --finish-> Finished    (保存 receipt を再返却・冪等)
*          --失敗--> Failed        (backend/IO どの失敗でも)
Failed     --write/finish-> Err(InvalidState)
Finished   --write-> Err(InvalidState)
Finished   --flush-> Ok(())        (no-op・Stacked writer 互換)
```

- `write(&mut self, buf)` は入力全体を消費して `Ok(buf.len())` を返すか
  `Err` かの二者（`Ok(n<len)` を返さない。`Write` の partial write
  契約に反しない — 全体受理をアトミックに扱う）。
- `flush()` の semantic は「deflate stream の pending bit を sync marker
  で byte 境界へ押し出し、下流 `W::flush()` まで伝搬」= `FlushCompress::
  Sync` + `write_all` + `self.writer.flush()`。
- `finish()` は `Compress::compress(&[], out, Finish)` を `StreamEnd`
  まで回し、gzip は `crc32 LE + isize LE` の 8 byte trailer を追記、
  最後に `writer.flush()` して receipt を返す。**失敗した場合は
  `Failed` に遷移し再試行しない。**
- 成功後の `finish()` は保存した receipt の clone を返す（冪等）。
- **Drop は何も書かない**: `flate2::Compress` の Drop 自体が副作用を
  持たないため、暗黙 trailer 書込・暗黙 flush は物理的に発生しない。
  これを「Drop は完了証拠でない」ではなく「Drop は書込を行わない」と
  実装保証として明記する（backend 差し替えでも維持すべき不変条件）。

## Decoder 状態機械

```text
Reading --(decode unit 処理中)------> Reading
Reading --delivered == max_decoded--> AtOutputLimit
Reading --unit 終端 + trailer/checksum 検証済み
          + scope 条件充足----------> VerifiedEnd
Reading --任意の失敗----------------> Failed
AtOutputLimit --probe で追加出力----> Failed (LimitExceeded)
AtOutputLimit --unit 終端 + 検証---> VerifiedEnd
VerifiedEnd --finish()-------------> Ok(DecodeReceipt)
Reading/AtOutputLimit --finish()---> Err(NotExhausted)  (state 不変)
Failed     --任意 read/finish------> Err(InvalidState)
```

- `impl Read`: 非空 `out` で展開 byte を逐次返す。`Ok(0)` は
  `VerifiedEnd` 到達時のみ（`read(&mut [])` は状態を変えず `Ok(0)` を
  返すだけで EOF 証拠にしない）。
- `finish()` は**状態照会のみ**: `VerifiedEnd -> Ok(receipt)`（冪等）、
  `Failed -> Err(InvalidState)`、その他 -> `Err(NotExhausted)`。
  読取を進めない（未読出力破棄も入力消費もしない）。
- `Read::read` の途中で unit 終端に到達した後も、trailer 検証と scope
  条件の確認は次の `read(0)` / `finish` 判定まで継続する。
- 出力の probe（AtOutputLimit 時）: 消費者 buffer の代わりに内部の
  1 byte scratch へ decompress し、produced >= 1 なら `LimitExceeded`
  に遷移（probe byte は Consumer に返さない）。produced == 0 で
  StreamEnd なら unit 終端の検証経路へ進む。

## Format 別 framing

### gzip（RFC 1952）

- **Header**（decode 側・member 毎に parse）: `1f 8b` magic・method=08・
  FLG（FTEXT/FHCRC/FEXTRA/FNAME/FCOMMENT、reserved bit=0 必須）・
  MTIME・XFL・OS。FEXTRA は LE16 長、FNAME/FCOMMENT は NUL 終端、
  FHCRC は header 累計 CRC32 の下位 16bit を検証。member あたりの
  header 総長に 64KiB 上限（`InvalidFormat`）。
- **Body**: `Decompress::new(false)` の raw deflate。`total_in` delta
  を BufRead `consume` に写像（byte 精密）。
- **Trailer**: 8 byte `crc32 LE + isize LE`。`crc32` は当該 member の
  展開 byte から計算し、`isize` は展開長 mod 2^32 と比較。不一致は
  `ChecksumMismatch`。
- **Encode header**（固定・再現性）: `1f 8b 08 00 00 00 00 00 <xfl>
  ff`（FLG=0、MTIME=0、OS=255=unknown、filename なし）。
  XFL: level>=9 -> 2、level<=1 -> 4、それ以外 0。

### deflate（Raw・RFC 1951）

- framing なし・checksum なし・単一 stream。`Decompress::new(false)` が
  StreamEnd を検出した時点で unit 終了。
- `WholeInput` scope は stream 終端後の入力 EOF を要求（残 byte は
  `TrailingData`）。`SingleUnit` も同じ 1 unit だが終端以降の byte を
  呼出側に残す（trailer が無いので boundary は StreamEnd byte まで
  consume した位置）。

## DecodeScope と入力境界

- `WholeInput`（既定）: 連結 member/frame 全 unit を復号し、最終 unit
  終端後に `fill_buf` が空 == 入力 EOF であることを要求。途中で
  非 member byte が現れたら `InvalidFormat`（先頭/mid）、末尾に残ったら
  `TrailingData`。gzip 空入力 -> `TruncatedInput`。
- `SingleUnit`: 先頭 unit（gzip member / deflate stream）の終端まで
  復号・検証。終端以降は caller の BufRead 内部に残る（`consume`
  しない）。`VerifiedEnd` はこの unit 単位で成立。
- `compressed_bytes_consumed` = `R::consume` + `read_exact` で本
  クレートが論理消費した byte 数。呼出側 BufRead が物理 source から
  prefetch した分は含まず混同しない（BufReader の残 buffer は caller
  から再 readable）。BufRead の物理 prefetch は本クレートから観測不能
  なので receipt に `source_bytes_fetched` は持たせず、BufRead を
  直接渡す層が所有する責務として文書化する。

## Limits

- `DecodeLimits { max_compressed_bytes, max_decoded_bytes, max_units }`
  — 連結 stream 全体に累積適用。各 `consume`/`read_exact`/unit 完了で
  即時比較し、超過は `LimitExceeded` -> `Failed`。
- `max_decoded_bytes` に**ちょうど一致**して unit が終端した場合は許可。
  上限到達で即 EOF を返さず、AtOutputLimit で scratch probe を経て
  「追加出力の有無」を判定する（SEC-04）。
- `EncodeLimits { max_input_bytes, max_output_bytes }` — `write` 入力と
  `W` への累積書込（header/trailer/flush 生成 byte を含む）を cap。
  出力 cap は「書込済み + 当該 chunk 生成分 > 上限」なら当該分を
  書かずに `LimitExceeded`（budget を超える物理書込を行わない）。
- CPU 時間・Backend 内部 heap（deflate window ~32KiB）・出力 Sink 側
  メモリは Hard Limit として宣言しない。deflate 系の backend 内部確保
  は形式上限定されるが、これを「メモリ上限機構」とは呼ばない。

## Error 正規化

| CompressionErrorKind | 意味 | `io::ErrorKind` 対応 |
| --- | --- | --- |
| `Io` | 下流 Read/Write/BufRead 失敗 | 元の kind を維持 |
| `InvalidInput` | options/引数不正（level 範囲外等） | `InvalidInput` |
| `InvalidFormat` | magic/method/reserved/header 異常 | `InvalidData` |
| `TruncatedInput` | unit 未完のまま入力 EOF | `UnexpectedEof` |
| `CorruptData` | deflate body 破損 | `InvalidData` |
| `ChecksumMismatch` | CRC32/ISIZE 不一致 | `InvalidData` |
| `TrailingData` | WholeInput の末尾余剰 byte | `InvalidData` |
| `LimitExceeded` | 累積 budget 超過 | `QuotaExceeded` |
| `Unsupported` | 未提供 format/capability | `Unsupported` |
| `Cancelled` | 上位 job 中断（予約・現在未使用） | `Interrupted` |
| `InvalidState` | Finished/Failed での操作 | `InvalidInput` |
| `NotExhausted` | `finish()` 時に未読出力/未検証残 | `Other` |

- `impl Write`/`impl Read` では `io::Error::new(kind,
  CompressionError)` として返し、`source()` で `CompressionError`
  -> さらに backend/IO 原因へ辿れる鎖を維持する。`finish()`/`new()`
  は `CompressionError` を直接返す。
- Gzip trailer の `isize` mismatch と `crc32` mismatch は同じ
  `ChecksumMismatch` だが message で区別する。

## Module 構成

| ファイル | 責務 |
| --- | --- |
| `src/lib.rs` | crate-level doc（RT/feature/Drop 契約・例）、共有型 re-export、feature 別 module cfg |
| `src/error.rs` | `CompressionError`/`CompressionErrorKind`・`io::Error` 変換 |
| `src/options.rs` | `DecodeScope`・`EncodeLimits`・`DecodeLimits`・format 共通 options 基底 |
| `src/receipt.rs` | `EncodeReceipt`/`DecodeReceipt`/`IntegritySummary` |
| `src/stream.rs` | private 共有機構: deflate pump（fill_buf->decompress->consume）、byte 計測、encoder の compress loop、state enum |
| `src/gzip.rs` | feature `gzip`: header/trailer codec・`Encoder`/`Decoder`・format options |
| `src/deflate.rs` | feature `deflate`: 単一 stream の `Encoder`/`Decoder`・format options |
| `tests/encoder_contract.rs` | AC1: finish 冪等/partial write/flush/Drop/Failed->InvalidState/借用回復 |
| `tests/decoder_contract.rs` | AC3: finish 未読拒否・Verified EOF・0-len read・consumed vs prefetch |
| `tests/format_cases.rs` | AC2: gzip golden/negative・連結 member・trailer・trailing・raw deflate 範囲 |
| `tests/composition.rs` | AC6: TAR 様 byte + Vec/BufReader 合成・budget・header parse |

## Feature / CI lane 設計

- `default = []`・`gzip = ["dep:flate2"]`・`deflate = ["dep:flate2"]`。
  `flate2` は optional dep として両 format が共有する（zstd 追加時は
  `dep:zstd` + 別 module で本共有語彙を再利用）。
- CI lane `compression-verify`（`ci/impact/map.mjs` + `ci.yml` +
  `ci-gate` needs 追加 + `classify.test.mjs` 期待追加）:
  `test-io-compression`（`cargo test -p ruviform-io-compression
  --all-features`）・`lint-io-compression`（同 clippy）・
  `check-io-compression`（feature matrix: `default`/`gzip`/`deflate`/
  `--all-features` 個別 check）を icc-verify と同構造で定義。
  `LANES["compression-verify"].features = ["gzip","deflate"]`。
- `Cargo.toml`/`mise.toml`/`ci/**` 変更自体が `shared-config`/
  `ci-config` のため本 PR は Full CI で lane が実走する。

## Non-Functional Constraints

- unsafe 不使用（`unsafe_code` warn -> `-D warnings` deny）。
- MSRV 1.89・edition 2024・`workspace.lints`・`publish=false`。
- 依存は `flate2 1.1.10`（`rust_backend`）のみで ruviform-* 依存ゼロ。
- 全 API nonRT — `impl Read`/`Write` は caller thread で blocking I/O
  と backend alloc を行い得る。
- 公開 enum は `#[non_exhaustive]`。`#![allow(clippy::exhaustive_enums)]`
  は置かない。
- round-trip byte 一致は同一入力・同一設定内で保証し、backend version
  間の byte 列完全一致は主張しない。

## Risks and Trade-offs

- **BufRead feed 粒度**: `fill_buf` が巨大（例: 8MB）でも
  decompress が消費した `total_in` delta 分しか `consume` しないため、
  member 境界の先読み問題は生じない。trailer/header は `read_exact`
  系で byte 精密に引く — 物理 source は BufRead が勝手に読むが論理
  消費は正確。
- **flate2 `Decompress` の StreamEnd 境界精度**: raw deflate は
  StreamEnd を最終 deflate block 位置で返す（`total_in` は byte 境界
  まで）。trailing 判定は golden test で pin。
- **`finish()` の NotExhausted 設計**: 最終 `read(0)` を呼ばずに
  `finish()` すると `NotExhausted`。`Read` 契約として「Ok(0) を一度
  観測するまで検証未完了」を文書化し、誤用を防ぐ（設計上の
  意図的 strictness）。
- **`flate2` version pin**: `1.1.10` は 2026-08-28 publish で
  minimum_release_age 相当を満たす。`Cargo.lock` は cargo 再解決で
  更新し手編集しない。
- **Encoder sync flush の互換性**: `FlushCompress::Sync` の empty
  stored block が decoder 側で正常 skip されるかは round-trip test で
  pin（flush 経由の中間状態を decode 可能なこと）。
- **probe 判定の boundedness**: AtOutputLimit probe は最大で入力残を
  1 byte 単位まで舐めるだけで bounded（無限ループしない — stream が
  生きていれば input/produced が進むか StreamEnd/Truncated になる）。

## Deferred Design Concerns

- `zstd` module: `zstd::{Encoder,Decoder,EncodeOptions,DecodeOptions}`
  + `max_window_log`/`skippable` 系固有 option + `dep:zstd` —
  IO-IMP-07 が共有 `DecodeScope`/`IntegritySummary`/`CompressionError`
  Kind をそのまま使う前提で追加（`TrailingData`/`NotExhausted` を
  Zstd にも適用可）。
- 所有型 `finish(self)->Result<W,_>`・`Read` 所有型・`Read+Seek` 対応・
  preset dictionary・zlib wrap: 実需確認後の別設計。
- Header 64KiB cap・encode buffer 大の可設定化: 計測後に必要になったら。

## Verification Notes

- [ ] AC1-AC6 の各試験が test-spec `docs/test-specs/
      issue-300-io-compression-gzip-deflate.md` に落ちること。
- [ ] `mise run fmt`/`check`/`lint`/`test` + feature matrix + 新 CI
      lane `compression-verify` が PR で緑になること（Full CI）。
- [ ] Backend 型が `cargo doc`/public API に現れないこと（grep + doc
      で確認）。

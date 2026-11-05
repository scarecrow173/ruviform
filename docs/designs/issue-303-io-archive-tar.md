---
id: "DESIGN-HR0Ud1VXoIpQCfUcosM6ib"
type: "design"
status: "approved"
title: "Issue #303 ruviform-io-archive — TAR Streaming Reader/Pending・PAX/GNU・Writer 設計"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar/header.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar/read.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/tar/write.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/tar_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/tar_writer.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/tar.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-archive/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/entry.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/mod.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-303-io-archive-tar.md"
    - "docs/specs/issue-302-io-archive-zip.md"
    - "docs/designs/issue-302-io-archive-zip.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-303-io-archive-tar.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-303-io-archive-tar.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #303 ruviform-io-archive — TAR Streaming Reader/Pending・PAX/GNU・Writer 設計

## Context

Issue #303（IO-IMP-09）は #302 で scaffold 済みの
`ruviform-io-archive` に `tar` module を追加する。IO-04（Issue #286）
v0.2 で合意済みの「TAR は `Read` Streaming・`Ready/EntryOpen/Pending/
Failed` 状態機械・途中 Drop は `discard_pending`/`discard_remaining`
で明示消費・payload checksum 非存在の明示・TAR/圧縮 2 層 EOF 分離・
Sparse/Special 復元は初期 Unsupported」をコンパイル可能な契約にする。

中核判断は **TAR backend を外部 crate（`tar` 等）にせず内部 Parser
で構成する**こと:

- Issue Acceptance が「既存 Backend Iterator が暗黙スキップ・先読み
  する場合も契約を満たす Adapter を使用する」を要求。`tar` crate の
  `Entries` iterator は次 header へ進む際に残 payload を**暗黙
  skip** し、Drop の Pending 化・budget 内 discard・「読み切った」と
  「破損不存在」の区別を外側から強制できない。内部 Parser なら
  全 byte の消費位置と状態遷移を監査可能にできる。
- TAR の構造は固定 512 byte block + octal/base-256 field + PAX record
  で単純であり、圧縮 codec を必要としない（新規依存ゼロ）。
- #302 と同一方針（内部 Parser + 成熟 codec のみ backend 化）を
  継承し、Archive 系の構造解析をこのクレートの責務として一貫させる。

## Scope

### In Scope

- `tar` module: `src/tar.rs`（module root・re-export）・
  `src/tar/header.rs`（非公開 header/field/PAX record 解析・生成）・
  `src/tar/read.rs`（`TarArchiveReader`/`TarEntryReader`/状態機械）・
  `src/tar/write.rs`（`TarArchiveWriter`/`TarEntryWriter`/
  `TarEntryOptions`）。
- 共通型の整合的拡張: `ArchiveEntryInfo` に `link_name:
  Option<ArchiveName>`、`EntryReadReceipt` に `disposition:
  EntryDisposition`（`ReadToEnd`/`Discarded`）。
- 否定 fixture を含む `tests/tar_reader.rs`・`tests/tar_writer.rs`・
  `tests/common/tar.rs`（byte 級 TAR builder）。
- crate doc/README・`crates/README.md` 台帳更新。

### Out of Scope

- Secure Extraction（#304）・sparse/special の受理・multi-volume・
  Writer の link/GNU 拡張出力・indexed access・compression codec
  実装・新規依存。

## Components and Boundaries

### TAR 構造と解析（`tar/header.rs`・非公開）

- Entry = 512 byte header + `ceil(size/512)` data block（payload +
  zero padding）。終端 = zero block 開始（全 512 byte が 0）。
- Header field: name(0..100) mode(100..108) uid gid size(124..136)
  mtime(136..148) chksum(148..156) typeflag(156) linkname(157..257)
  magic(257..263) version(263..265) uname gname dev prefix(345..500)。
- checksum 検証: chksum field を space(0x20) 扱いした header 全
  byte の unsigned 和と、chksum field の octal 値を比較。不一致 →
  `ChecksumMismatch`（宣言 vs 実測の checksum 不一致であり、構造的に
  不可能な値とは区別する）。
- 数値 field: octal（digit 0-7・space/NUL padding 許容）または GNU
  base-256（先頭 byte MSB set の big-endian 2 の補数）。それ以外 →
  `Malformed`。
- magic 受理: `"ustar\0"+"00"`（POSIX/USTAR）・`"ustar "+" \0"` 系
  （GNU）。それ以外 → `Malformed`（magic 非対応 = USTAR/GNU/PAX 系
  以外は本 Issue の形式外）。
- typeflag dispatch:
  - `'0'`/`'\0'`/`'7'` → `EntryKind::File`、`'5'` → `Directory`、
    `'1'` → `HardLink`、`'2'` → `SymbolicLink`。
  - `'x'`/`'X'`（PAX per-entry）、`'g'`（PAX global）、`'L'`/`'K'`
    （GNU longname/longlink）は内部拡張として消費し Entry として
    露出しない。
  - `'S'`（GNU sparse）・`'3'`/`'4'`/`'6'`（char/block/fifo
    special）・その他全 typeflag（`'V'`/`'M'`/`'N'`/`'D'`/未知）→
    `Unsupported`。
- PAX record 文法: `"<len> <key>=<value>\n"`、len は自身を含む 10
  進。record 列が payload を丁度消費しない・len 不正・`=`/改行欠落
  → `Malformed`。
- GNU `L`/`K` payload は末尾 NUL を剥がして実効 name/link target に
  する（raw bytes 保持）。

### 実効値の解決順位

- 実効 name = PAX `x` の `path` > GNU `L` > USTAR `prefix/name`
  （prefix 非空時のみ `/` 連結）> header name。
- 実効 link target = PAX `x` の `linkpath` > GNU `K` > header
  linkname（`'1'`/`'2'` のみ意味を持つ）。
- 実効 size = PAX `x` の `size`（10 進）> header size。
- PAX `g`（global）record は文法検証と budget 計上のみ行い、
  `path`/`linkpath`/`size` 等の実効値へは**適用しない**（global が
  全 Entry を改名し得る挙動を安全側に遮断。実装間差の大きい領域で
  本 Issue は読み取り検証を優先）。これは設計上の明示的な非対称で
  あり rustdoc に記す。
- 拡張状態（pending PAX attrs / GNU longname）は直後の実 Entry に
  1 回だけ適用し消費する。実 Entry の前に現れず残った拡張は
  EOF/終端で自然に破棄される。

### Reader 状態機械（`tar/read.rs`）

```
Ready --next_entry(header ok)--> EntryOpen(TarEntryReader 借用中)
EntryOpen --entry.finish()--> Ready        （ReadToEnd 証明）
EntryOpen --entry.discard_remaining()--> Ready（Discarded 証明）
EntryOpen --Drop(未 finish)--> Pending
Pending --discard_pending(ok)--> Ready     （budget 内 drain）
Pending --next_entry/finish/discard失敗--> Failed または InvalidState
Ready|EndSeen --archive.finish()--> Finished(inner + ArchiveReadReceipt)
* --io/構造エラー--> Failed
```

- `TarEntryReader<'a, R>` は `&'a mut TarArchiveReader<R>` を保持し
  同時に 1 つ（借用規則で強制）。
- `next_entry` が Err を返した場合は Reader を `Failed` にする
  （逐次形式では header/拡張の parse 失敗・Limit 拒否・Unsupported
  のいずれも、その後の byte 位置を用いた継続を主張しない）。
  ただし `Pending`/`Failed` 中の `next_entry`・`finish` 等の**契約
  違反**呼出しは `InvalidState` であり Reader 状態を変えない
  （誤用と archive 破損を分離）。
- `TarEntryReader::read` — `min(残宣言量, hard_cap)` で inner から
  直接読む（Take 等の中間層なし）。`produced == declared` で EOF。
  `hard_cap = min(宣言 size, 残り total budget)`;宣言残が残るのに
  `hard_cap` に達したら `LimitExceeded`（超過 byte を下流に出さ
  ない途中遮断）。
- `TarEntryReader::finish(self)` — `produced == declared`（呼出し側
  が全量消費）の場合のみ成功。未消費での呼出しは `InvalidState` で
  self を消費し Pending 化する。成功時は padding（512 境界までの
  zero byte）を消費し**全 zero を検証**（非 zero → `Malformed`、ただ
  し位置は正しく Ready 継続可能）。Receipt は
  `EntryChecksum::NotPresent` + `EntryDisposition::ReadToEnd`。
- `TarEntryReader::discard_remaining(self)` — 残 payload を scratch
  buffer で budget 内 drain（total budget 計上）→ padding 検証 →
  `EntryDisposition::Discarded` の Receipt。drain 中の io 失敗 →
  `Failed`。
- `TarArchiveReader::discard_pending(&mut self)` — `Pending` でのみ
  有効（他状態 → `InvalidState`）。drop 時点で記録した「残 payload
  + padding」を budget 内 drain → padding 検証 → `Ready`。
- 補助 API: `entry_count()`（surfaced Entry 数）・
  `has_pending_entry()`（`Pending` 中かの判定）・`into_inner()`
  （検証せず inner を返す）を公開する。
- `TarArchiveReader::finish(self)` — `Ready`/`EndSeen` でのみ有効
  （`Pending` → `InvalidState`、IO-04 v0.2 の「Pending 中の全体
  finish 拒否」どおり）。未消費 Entry が残っていれば header 解析 +
  budget を適用しながら **discard として drain し構造検証だけは行う**
  （unverified に計上）。zero block に達したら `EndSeen`、以後の全
  byte が zero であることを EOF まで検証（非 zero → `Malformed`
  の「終端後後続 byte」）。`(inner, ArchiveReadReceipt)` を返す。
  - `entries_total` = surfaced Entry 数、`entries_verified` =
    `ReadToEnd` 数、`entries_unverified` = Discarded + 途中 Drop +
    finish drain 分、`payload_bytes` = 実測 uncompressed byte
    （read + discard の消費量）。
- 終端規則: 最初の全 zero block を終端開始とする。lone zero block +
  EOF は受理（GNU 実装と整合）。2 block 目以降や record padding は
  「全 zero 末尾」として同一の検証で扱う。

### 共通型の拡張（最小・整合）

- `ArchiveEntryInfo` に `link_name: Option<ArchiveName>` を追加
  （`pub(crate) new` の第 6 引数）。TAR `'1'`/`'2'` の実効 link
  target を保持し ZIP は `None`。`link_name()` accessor を追加。
- `EntryReadReceipt` に `disposition: EntryDisposition` を追加。
  `EntryDisposition`（`#[non_exhaustive]`）: `ReadToEnd`（呼出し側
  が全 payload を消費）/`Discarded`（Reader が明示 discard で構造
  検証のみ実施）。ZIP は常に `ReadToEnd` を返す。`receipt.rs` に
  enum を置き、`EntryReadReceipt::new` の引数を拡張する
  （`pub(crate)` ctor のため外部影響なし）。

### Writer 状態機械（`tar/write.rs`）

```
Ready --start_entry(name, declared_size, opts)--> EntryOpen
EntryOpen --entry.finish()(書込==宣言)--> Ready   （padding 出力）
EntryOpen --Drop(未 finish)/finish 失敗--> Poisoned
Ready --finish()--> Finished（2 zero block + inner 返却）
```

- `start_entry(name: &str, declared_size: u64, options:
  &TarEntryOptions)` — name 検査（非空・NUL なし・UTF-8）。name ≤
  100 byte → USTAR。分割可能（`/` 境界で prefix ≤155・suffix ≤
  100）→ USTAR prefix。それでも収まらない → **PAX `x` Entry を
  先行出力**（`path=<full>`）し header name は UTF-8 境界で切詰めた
  ≤100 byte を書く。declared_size が octal field 上限
  （`0o77777777777`）を超える場合も PAX `x`（`size=`）を先行し、
  header size 欄は octal 最大値を書く。
- `TarEntryWriter: Write` — 実書込量を計数し `declared_size` で cap
  する。cap 以降の write は `io::Error`（`Write` 契約の範囲で
  partial write → 超過分拒否）にし、宣言長を超えた payload が
  archive に混入しない。
- 「宣言長だけ `Read` から取り込む」合成の境界: caller は
  `source.take(remaining)` で cap して `io::copy` する。source が
  宣言より長い場合の余分 byte は source 側に残り Entry に混入しない
  （先読みしない）。source の EOF までを検証する操作は別概念であり、
  source 側の余分 byte 検査は caller の契約として rustdoc に明記する
  （Issue Acceptance「余分 Reader Byte 検査の境界」）。
- `TarEntryWriter::finish(self)` — 実書込 == 宣言を要求。不足時は
  `SizeMismatch` Err **かつ archive poison**（header は宣言値で確定
  済みのため archive を valid に戻せない — ZIP の未 finish poison
  と同一原則）。一致時は padding zero を出力して Ready 復帰。
- `TarArchiveWriter::finish(self)` — 2 block（1024 byte）の zero を
  書いて `W` を返す唯一の完了証拠。blocking factor（20 block）への
  padding は行わない（仕様最小 — Reader 側は全 zero 末尾を受理）。
- 出力形式: 常に USTAR header（`ustar\0`+`00`）+ 必要時 PAX `x`。
  GNU `L`/`K`・base-256 は**書かない**（読み側は受理する非対称 —
  POSIX 相互運用で最大互換の PAX を優先）。`add_directory` は
  typeflag `'5'`・size 0。link/special/sparse の Writer 出力は
  本 Issue 対象外。
- `TarEntryOptions`（`#[non_exhaustive]` + pub field）:
  `mode: u32`（file 0o644 / dir 0o755 default）・`mtime: u64`
  （default 0 — 決定的出力）。uid/gid/uname/gname は 0/空固定
  （拡張は実需待ち）。

### Budget 強制点

- `max_entries`: surfaced Entry（実 Entry のみ。`x`/`g`/`L`/`K` は
  Entry 数に含めず metadata byte に計上）を `next_entry`/finish
  drain 中に逐次カウントし超過で `LimitExceeded`。
- `max_metadata_bytes`: header block（512/個）+ PAX/GNU 拡張
  payload byte を累積。拡張 payload の宣言 size は Vec 確保**前**に
  残 budget と照合（宣言先行確保しない）。
- `max_path_bytes`: 実効 name・実効 link target の byte 長を
  `ArchiveName` 確保前に強制。
- `max_entry_uncompressed_bytes`: 実効宣言 size を Entry open 時に
  事前拒否。
- `max_total_uncompressed_bytes`: read/discard/drain の消費を累積し
  `hard_cap` で途中遮断。

### 脅威境界（契約として明記・実装は #304）

- Path 安全性: 実効 name/link target は raw bytes で受け渡すのみ。
  `..`/絶対 path/drive/UNC/symlink 展開先等の拒否は #304 の
  `SafeRelativePath`/`ExtractPolicy` の責務。
- 「読み切った」≠「無破損」: TAR payload に内容 checksum は無く、
  `ReadToEnd`/`Discarded` は終端整合と構造検証のみを意味する。
  header checksum は header 自体の破損検出であって payload の
  真正性ではない。
- 宣言 size 偽装: 宣言は stream 位置決めの根拠。size を偽ると次
  header 位置がずれ、`ChecksumMismatch`/`Malformed`/`Unsupported`
  として後続解析で表面化する設計（逐次形式の限界を rustdoc に明記）。
- `tar.gz`/`tar.zst`: TAR 層 `finish` は decode 済み stream 内の
  TAR 終端のみを証明し、compression stream（member/trailer/CRC/
  圧縮 EOF）の検証は IO-03 decoder の `finish` 契約に分離する。

## Data and Control Flow

- Read: caller → `TarArchiveReader::new(R, limits)` →
  `next_entry()`（header+拡張 parse、budget 統合）→
  `TarEntryReader.read` → `finish` か `discard_remaining`；途中
  Drop なら `discard_pending` → … → `finish()` で残り drain +
  終端検証 → `(R, ArchiveReadReceipt)`。
- Write: caller → `TarArchiveWriter::new(W)` → `start_entry` →
  `TarEntryWriter.write*`（cap 管理）→ `entry.finish`（size 確認 +
  padding）→ 繰返し → `writer.finish`（zero block 出力）→ `W`。

## Non-Functional Constraints

- non-RT のみ。全 API がブロッキング I/O + alloc を許容。
- `unsafe` 不使用・MSRV 1.89・edition 2024・`default = []`。
- 新規依存なし。既存 `crc32fast`/`flate2` は ZIP 内部依存のまま
  （TAR 経路では不使用）。公開 API に backend 型を出さない。
- 公開 enum は全て `#[non_exhaustive]`（`EntryDisposition` 含む）。
- 拡張 payload（PAX record・GNU longname）は宣言 size を budget で
  上界した上で Vec に読む（entry payload の stream read と異なり
  metadata は一括 parse が必要 — 上界は budget が担保）。
- `#[non_exhaustive]` な `TarEntryOptions` は pub field + Default
  で、`Default` を緩めない既定値。

## Risks and Trade-offs

- 途中 Drop → Pending → `discard_pending` の API は ZIP の「再 open
  可能」より厳しいが、one-way stream では seek 復帰が不能なため
  本質的に正しい。Pending 中の操作拒否を `InvalidState`（非破壊）と
  構造失敗 `Failed`（破壊）に分けたのは誤用耐性のため。
- magic 厳格化（`ustar` 系のみ受理）は V7 等の旧形式を `Malformed`
  で拒否する。issue の対象形式（USTAR/GNU/PAX）に合致し、受理する
  形式の範囲を検証可能に限定する意図的な strictness。
- header checksum を `ChecksumMismatch` に分類（`Malformed` でなく）
  したのは「宣言値 vs 実測値の不一致」という kind 定義に忠実で、
  #304 が「構造的に不可能」か「破損検出」かを区別できるため。
- `finish()` が残 Entry を auto-discard する設計は「未読 Entry の
  存在を許して archive を閉じる」挙動だが、各 Entry の構造
  （header/padding/budget）は drain 中に全て検証されるため「未検証
  のまま完了扱い」にはならない。unverified 件数は Receipt で報告
  される。
- PAX `g` の実効値不適用は POSIX 厳密解釈からの逸脱だが、安全側
  （全 Entry 改名の拒否）かつ実害の無い領域であり rustdoc に明記。

## Deferred Design Concerns

- 自動 Pending（Unsupported Entry の透過 skip） | #304 要件確定後 |
  Unsupported で Failed とする現行は保守的。
- PAX `g` 実効値の厳密適用 | 実装間差・実需確定後 | 現行は構造検証
  のみ。
- Writer の GNU 拡張/base-256 出力・link 出力・record padding |
  Consumer 互換要件確定後 | USTAR+PAX の最小出力で意図的に非対称。
- `ArchiveReadLimits` への TAR 固有追加（max_pax_records 等） |
  実需確定後 | 現行の 5 budget で metadata 爆弾は抑止可能。

## Verification Notes

- [ ] 負 fixture 行列: bad checksum・短 header・非 octal field・
      非 zero padding・終端欠落・終端後 garbage・PAX/GNU 構造不正・
      未対応 typeflag が期待 kind に分類されること。
- [ ] Pending 契約: 途中 Drop → `next_entry`/`finish` 拒否 →
      `discard_pending` 復帰・drain 失敗時 `Failed`。
- [ ] 実効値: USTAR prefix・GNU L/K・PAX path/linkpath/size の解決
      順位と raw bytes 保持。
- [ ] Writer: cap・不足 finish の poison・padding・長名 PAX 出力・
      round-trip。
- [ ] `tar.gz` 2 層: TAR finish と gzip decoder finish が独立（片方
      の成功が他方を保証しない）。
- [ ] ZIP 側既存テスト（`verify_all` receipt・`EntryReadReceipt`
      拡張の互換）の非退行。

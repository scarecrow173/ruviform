---
id: "DESIGN-TUrjxcJLe7rvYr6up8mhf3"
type: "design"
status: "approved"
title: "Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/specs/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-301-io-compression-zstd.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
    - "docs/designs/issue-303-io-archive-tar.md"
    - "docs/designs/issue-304-io-archive-extract.md"
    - "docs/designs/issue-409-scoped-enumeration.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
    - "https://github.com/scarecrow173/ruviform/issues/285"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/300"
    - "https://github.com/scarecrow173/ruviform/issues/301"
    - "https://github.com/scarecrow173/ruviform/issues/302"
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/25"
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計

## Context

Issue #285 は、独立クレート `ruviform-io-compression` の Stream
Encoder/Decoder・Backend 実装隠蔽・resource limits・
finish/Drop/error/OS/RT 契約を実装前に確定する IO-03 の設計
Issue である（親 #282）。Issue 本文の「合意済み設計 v0.1・
v0.2（2026-11-04）」と「最終整合性レビュー — 実装 Issue への
引継ぎ」（IO-IMP-06 = #300・IO-IMP-07 = #301）が要求の正本
であり、本書はそれを docs チェーンの正本設計書として文書化
する。

**本書が固定するもの / 固定しないもの**

- 固定する: byte-stream 層としての Serde 非依存・backend
  中立契約、Format/Version 契約の骨格（3 形式の区別・
  Format 別 options・`DecodeScope`・連結単位・checksum
  区別・再現性の範囲）、Finish/状態機械/所有権の横断契約、
  Resource Budget の累積適用規則、Error 契約の語彙、
  Cargo feature/Backend 選定の確定値、Consumer 利用面、
  証拠項目（ENC/DEC/SEC/INT/ARC/COMP/BUF）と他 IO gate
  への対応索引。
- 固定しない: per-format の framing 内部・buffer 戦略・
  test 規則の実装詳細（#300/#301 の設計・test-spec が正
  本）、実走テスト結果、Consumer repo の実測、backend の
  将来更新。

**実装の現状（2026-11-09 時点の snapshot）**

- `crates/ruviform-io-compression` は main に存在し、
  #300（gzip・raw deflate・PR #356）・#301（zstd・
  PR #359）で merge 済み。`gzip`/`deflate`/`zstd` feature
  matrix が `Cargo.toml` に定義され、`mise run
  test-io-compression`（feature matrix: default・`gzip`・
  `deflate`・`zstd`・`gzip,deflate,zstd`）と CI lane
  `compression-verify`（`.github/workflows/ci.yml`）が検証
  入口である。
- 合成 E2E クレート `ruviform-io-e2e`（#306）が TAR→
  Gzip/Zstd→`&mut StagedFile`→seal→readback→publish の
  実 OS 境界（`tests/archive_compression_staged.rs`）と、
  破損 fixture・層別 mutant（`tests/negative_fuzz.rs`）を
  検証する。`ruviform-io-archive` は `ruviform-io-compression`
  を **dev-dependency のみ**で参照し（tar.gz 合成試験用）、
  production 依存はない。
- IO-06 cross-consumer gate 設計（#288）が IO-03 を
  matrix 行 C-N1/C-N2/C-X1〜C-X4 と合成 E2E 行 E-N1/
  E-X1/E-X2 として受入対象に含めている。
- Consumer 側: AudioLens #139・Synvelis #82 は CLOSED、
  Synvelis #84 は OPEN・#25 は CLOSED（Issue 状態は変動
  し得る snapshot として記録）。

**契約試験棚卸し（2026-11-10・本 slice）**

- `docs/specs|designs|test-specs|plans|tasks/
  issue-285-io-03-compression-contract-audit.md` の slice で、
  `tests/` を Issue 受入条件の ENC/DEC/SEC/BUF 契約へ写像して
  棚卸しし、failure-path（writer write 失敗・finish 時底流
  flush 失敗・失敗後 writer 回復・decoder source I/O 失敗）・
  Drop 無書込・flush≠finish・budget ちょうど/+1・高圧縮率
  payload・連結単位の独立検証・`NotExhausted` 非破壊性の
  不足を `tests/encoder_contract.rs`・`tests/decoder_contract.rs`
  に追加した。`tests/common/mod.rs` に `FailFlush`・
  `RecoveringWriter`・`FailingReader` fixture を新設し、
  `FailAfter` は受理 byte を `inner` に保持するよう強化した。
- 照合の結果 `src/` の設計不整合は確認されず、公開 API・
  状態機械・error 語彙・budget 意味論は本書の記述どおりで
  あった（実装変更なし）。`mise run fmt-check`/
  `check-io-compression`/`lint-io-compression`/
  `test-io-compression`（5 feature 組合せ）/`test-io-e2e`/
  `test-rt`/`test-io-msrv`（rust@1.89.0）が pass。

文書チェーン: [spec](../specs/issue-285-io-03-compression-serde-boundary.md)・
[plan](../plans/issue-285-io-03-compression-serde-boundary.md)・
[test-spec](../test-specs/issue-285-io-03-compression-serde-boundary.md)・
[task](../tasks/issue-285-io-03-compression-serde-boundary.md)。

## Scope

### In Scope

- Serde 非依存・driver/backend 中立契約の正本記述。
- Format/Version 契約（3 形式・Format 別 options・
  `DecodeScope`・連結単位・checksum 区別・再現性の範囲）。
- Finish/状態機械/所有権契約（借用型・`finish()` 冪等・
  Drop 無書込・`NotExhausted`/`InvalidState`・底流
  Writer 保全）。
- Decoder の検証済み終了（VerifiedEnd）と入力境界
  （WholeInput/SingleUnit・prefetch 非混同）。
- Resource Budget（`EncodeLimits`/`DecodeLimits`/
  `max_window_log`・累積適用・境界包含・非保証の明記）。
- `CompressionError` 系の Error 契約と `io::Error` 包摂時
  の `source()` 復元。
- Cargo feature・Backend 選定の確定値（採用版・MSRV・
  ライセンス・feature 構成・不採用候補）。
- AudioLens/Synvelis の利用面と責務分界。
- 証拠項目（ENC/DEC/SEC/INT/ARC/COMP/BUF・Design
  Acceptance Gate）から既存 test-spec・#288 matrix・実装
  Issue/PR への対応索引。
- 設計合意と実装/検証の分離の明記。

### Out of Scope

- per-format 詳細設計の再定義（#300/#301 が正本）。
- コード変更・テスト追加・CI 変更。
- Consumer repo の変更・実測（#219/#294 consumer-verify、
  AL #139、SV #82/#84/#25 が Owner）。
- Zlib wrapper・preset dictionary・所有型 API・async/
  no_std・skippable frame 書込・Archive 内 codec 統合
  （ZIP entry codec は IO-04 側の Capability）。
- 新規 CI Workflow・証拠スキーマ・Dashboard。

## Components and Boundaries — 責務分担

| 領域 | Owner | 本書との関係 |
| --- | --- | --- |
| IO-03 横断契約（Serde 非依存・Format/Version 骨格・Finish/状態機械・Budget・Error 語彙・Feature/DAG・Consumer 面・証拠索引） | **Issue #285（本書）** | 正本 |
| Gzip（RFC 1952）・Raw Deflate（RFC 1951）の framing・状態機械・test | **#300**（`docs/designs/issue-300-…`・`docs/test-specs/issue-300-…`） | 詳細正本。本書は横断契約のみ所有 |
| Zstd（RFC 8878）Frame 境界・Window/Skippable・Integrity の詳細 | **#301**（`docs/designs/issue-301-…`・`docs/test-specs/issue-301-…`） | 同上 |
| File 公開契約（StagedFile type-state・PublishOptions・Receipt/Failure・PlatformFileOps） | **#295**（`docs/designs/issue-295-…`） | 合成先の `seal()`/`publish()` 契約 |
| 実 OS backend（renameat2/renamex_np/MoveFileExW 等）・OS/FS capability | **#296**（`docs/designs/issue-296-…`） | publish の OS 保証範囲の正本 |
| TAR Streaming・ZIP indexed・Secure Extraction | **#303/#302/#304**（`docs/designs/issue-30{2,3,4}-…`） | TAR→Gzip 合成の相手側・tar.gz 展開の下位層 |
| 旧 `StagedOutput`/旧 `PublishOptions` 撤去・WAV→StagedFile 移行 | **#305** | IO-03 以前の Media I/O 境界。本書は触れない |
| 合成 E2E（層横断境界試験） | **#306**（`docs/designs/issue-306-…`・`ruviform-io-e2e`） | TAR→Gzip/Zstd→StagedFile 境界の実 OS 証拠 |
| Rooted capability（`RootedDir`/`open_scoped_root`/bounded 列挙） | **#409**（`docs/designs/issue-409-…`） | path 規則の下位機構。本書は scoped I/O を所有しない |
| Cross-consumer/OS-FS/Security/性能/互換性 Gate | **#288**（`docs/designs/issue-288-…`） | 本書の証拠項目が同 gate の IO-03 matrix 行の根拠 |
| Cross-Repo CI 運用 | **#219/#294** | Consumer tuple 検証の実行機構 |
| Consumer 固有の Schema/Project/Dataset 意味論・Golden・復旧 | **AL #139、SV #82/#84/#25** | Consumer 側の受入 Owner |

原則: 本書は **横断契約の定義** を所有し、**per-format 詳細**
は #300/#301、**File/OS 保証**は #295/#296、**Archive 層**は
#302/#303/#304、**実 OS 境界証拠**は #306、**採否判定**は
#288、**実行機構**は #219/#294 に委ねる。責務の重複定義を
しない。

## Serde 境界と Driver/Backend 中立契約

Issue #285「合意済み構造」および v0.1-1/v0.2 を正本とし
て固定する。Compression は **byte-stream 層**であり、
document/Serde 層（IO-02 `ruviform-io-structured`）とは
別の層に位置する。

### 許可される公開外部型

- `std::io::{Read, Write, BufRead}` — 公開 API の境界
  trait。`Encoder` は `impl Write`、`Decoder` は `impl
  Read` を実装し、入力/出力 stream の所有権は借用により
  Consumer に残る。
- `std::path::Path`・`std::error::Error` 等の標準型。
- Ruviform 独自型: `CompressionError`/`CompressionErrorKind`/
  `DecodeScope`/`EncodeLimits`/`DecodeLimits`/
  `EncodeReceipt`/`DecodeReceipt`/`IntegritySummary` と、
  Format 別 module の `Encoder`/`Decoder`/`EncodeOptions`/
  `DecodeOptions`（`zstd::SkippableFramePolicy` を含む）。

### 非公開とするもの

- **serde への依存はゼロ**: `serde` の trait・型・derive
  を公開 API にも内部実装にも要求しない（`Cargo.toml` に
  serde 依存なし — `src/` 全体で serde を参照しない）。
  圧縮は document 意味論を持たず、Serde 境界は「Serde を
  通さない byte 変換」として下位に位置する。
- Backend の公開型・Error・feature 名・version:
  `flate2::Compress`/`Decompress`/`Crc`/`Compression`/
  `FlushCompress`、`flate2::DecompressError`/`CompressError`、
  `zstd_safe::CCtx`/`DCtx`/`InBuffer`/`OutBuffer`/
  `ErrorCode`、`zstd_sys` の型は公開 API に一切露出しない
  （`Cargo.toml` の dependency record と `src/lib.rs` の
  module doc が正本）。
- Backend 診断は `CompressionError::source()`
  （`Box<dyn Error + Send + Sync>`）経由の `dyn Error`
  としてのみ到達可能。`impl Read`/`impl Write` 面では
  `io::Error::new(kind, CompressionError)` に包まれるが、
  `source()` 鎖で `CompressionError` → backend/IO 原因へ
  辿れる（`error.rs` が正本）。`CompressionError` の
  `Display` は kind に加えて `source` の `Display` を連結
  する — 正規化 kind が安定語彙であり、backend 詳細は
  source 側の責務として扱う。
- 自作の codec registry・format 自動判定・「とりあえず
  gzip として試す」フォールバックは行わない。Format は
  Consumer が module を明示選択する。

### Driver 中立性

公開 API は特定 backend の「driver」（圧縮/解凍の具象実装）
に依存しない。framing（gzip header/trailer・zstd frame
header/boundary/skippable 走査）は **Ruviform 側が所有**し、
backend には deflate body / frame body のみを委譲する
（#300/#301 設計の核心判断）。backend 差し替えは
`Cargo.toml` の optional 依存と module/`stream.rs` 内部に
閉じ、公開シグネチャ・Error 語彙・Options/Receipt 型は
Ruviform 型のみで構成される。

### 検証根拠

backend 型非露出は実装側で pin されている:
`tests/api_surface.rs`（公開型の `Send`/`Sync`・kind 保持・
feature 既定値）、per-format test-spec の feature/audit
規則（`issue-300` R8、`issue-301` AC6 の「backend 型非公開
— `cargo doc` で `zstd_safe`/`zstd_sys` 不在」）、crate
README の backend-hiding 記述。#288 matrix の C-X 系が同
契約を gate 項目として参照する。

## Format/Version 契約

### 形式の区別（wire format が契約の正本）

| Format | module/feature | wire format | checksum | 単位 |
| --- | --- | --- | --- | --- |
| Raw Deflate | `deflate`/`deflate` | RFC 1951 bare DEFLATE bitstream（zlib/gzip container ではない） | なし（`unchecked_units` に計上） | 単一 stream。連結は暗黙処理しない |
| Gzip | `gzip`/`gzip` | RFC 1952 member（header + deflate body + CRC32/ISIZE trailer） | CRC32+ISIZE を member 毎に検証（`checked_units`） | member。連結 member を `WholeInput` で累積 |
| Zstd | `zstd`/`zstd` | RFC 8878 frame（frame header + blocks + optional checksum） | frame checksum flag に従い検証（checked/unchecked 混在を `IntegritySummary` で分離） | frame。連結 frame を `WholeInput` で累積 |

- Zlib wrapper（RFC 1950）は **別形式として明示的に対象
  外**（Issue #285 v0.1-1）。Raw Deflate 入力に zlib
  wrapper が被さった場合は `InvalidFormat`/`CorruptData`
  系で拒否される（`tests/deflate_format.rs` の
  `zlib_wrapped_input_is_rejected`/`gzip_wrapped_input_is_rejected`）。
- Archive の ZIP 内部 codec/entry は IO-04（#302）の
  Capability に属し、本 crate とは統合しない
  （`ruviform-io-archive` は ZIP method 8 を内部の
  `flate2` で処理し、`ruviform-io-compression` への
  production 依存を持たない）。

### API 族（実装済みの確定形）

| Format | Encoder | Decoder | Options |
| --- | --- | --- | --- |
| gzip | `gzip::Encoder<'w, W: Write + ?Sized>` — `new(&'w mut W, EncodeOptions) -> Result<Self, CompressionError>`（header 即時書込のため `Result`）、`impl Write`、`finish(&mut self) -> Result<EncodeReceipt, CompressionError>` | `gzip::Decoder<'r, R: BufRead + ?Sized>` — `new(&'r mut R, DecodeOptions) -> Self`（infallible・読取は lazy）、`impl Read`、`finish(&mut self) -> Result<DecodeReceipt, CompressionError>` | `EncodeOptions{level: u32(0..=9, 既定6), limits}`、`DecodeOptions{scope, limits}` |
| deflate | `deflate::Encoder<'w, W>` 同上 | `deflate::Decoder<'r, R>` 同上 | 同上（level は DEFLATE semantics） |
| zstd | `zstd::Encoder<'w, W>` 同上（`EncoderCore<W, ZstdOut>`） | `zstd::Decoder<'r, R>` 同上 | `EncodeOptions{level: i32(既定0=backend 既定), checksum: bool(既定 false), limits}`、`DecodeOptions{scope, limits, max_window_log(既定27, clamp 10..=41), skippable_frames(既定 Accept)}` |

- 借用型のみが初期公開形（IO-03 v0.2-A）。所有型
  `finish(self)->Result<W,_>` は実需確認まで作らない
  （#300/#301 の Deferred Design Concerns と同一条件）。
- `Encoder::new` は `&'w mut W` を借用し、scope 終了で
  底流 Writer の所有権は Consumer に戻る。失敗時も同様
  （借用なので所有権喪失は起きない — `tests/
  encoder_contract.rs` の `writer_borrow_recovered_after_scope`）。

### Level semantics（形式別・統一しない）

Issue #285 v0.1-5「Format別Level Optionsを設け、Gzip/
Deflate/Zstdを無理に単一数値Levelへ統一しない」に従い、
確定値は以下のとおり（実装済み）:

- gzip/deflate: `level: u32`（DEFLATE semantics、`0` =
  stored・`9` = maximum・既定 `6`）。範囲外は
  `Encoder::new` で `InvalidInput`。
- zstd: `level: i32`（zstd semantics、`0` = backend 既定・
  負値 = faster）。範囲は backend の
  `min_c_level()..=max_c_level()` で `Encoder::new` 時に検証。
- `checksum`（zstd encode）は frame content checksum の
  有無を制御し、decode 側で checked/unchecked unit として
  計上される。

### 再現性・Version 契約

- 同一入力・同一設定の **round-trip 内容一致** が主な保証。
  backend version 間で圧縮後 byte 列が完全一致することは
  追加の明示契約なしに保証しない（Issue #285 v0.1-5・
  crate README）。
- gzip の encode header は固定・再現可能:
  `1f 8b 08 00` + `mtime=0` + `xfl`（level 由来）+
  `os=0xff` — filename/path/mtime 等の機密・非決定要素を
  持たない（`src/gzip.rs` module doc・
  `tests/gzip_format.rs::emitted_header_is_fixed_and_reproducible`）。
- zstd encoder は `ContentSizeFlag`/`set_pledged_src_size`
  を呼ばず FCS を emit しない（deterministic header —
  `docs/designs/issue-301-…`）。
- Content Identity / Artifact Hash は、圧縮前・圧縮後の
  どちらの byte を対象にするか **Consumer が決める**
  （Issue #285 v0.1-5）。Compression 層は File Sync・
  Atomic Publish を実行しない。
- Format version は wire format の RFC で固定され、
  「schema/version/migration」は document 層（IO-02）・
  Consumer 側の責務 — Compression はそれらを所有しない
  （#219 consumer-compat-contract の影響分類と整合:
  typed I/O 追加は Consumer の schema 互換を変更しない）。

### DecodeScope と連結単位

- `DecodeScope::WholeInput`（既定）: 連結 member/frame
  全 unit を復号し、最終 unit 終端後の入力 EOF を要求。
  末尾の非 format byte は `TrailingData`、unit 未完の EOF
  は `TruncatedInput`。独立 `.gz`/`.zst` blob の推奨 mode。
- `DecodeScope::SingleUnit`: 先頭 unit の終端で停止し、
  終端以降の byte を caller の `BufRead` 内部に残す
  （埋め込み stream 用途）。`finish()` はその unit の
  end/trailer/checksum を検証するが入力 EOF を要求しない。
- gzip: `WholeInput` で連結 member を累積 decode、
  `SingleUnit` で先頭 member のみ。member header の
  FEXTRA/FNAME/FCOMMENT/FHCRC を parse（reserved bit・
  method≠8・header 総長 >64KiB は `InvalidFormat`）。
- deflate: 単一 stream のみ。連結の暗黙処理なし —
  `WholeInput` で stream 後の byte は `TrailingData`。
- zstd: `WholeInput` で連結 frame を累積 decode。skippable
  frame（`0x184D2A50..=0x184D2A5F`）は
  `SkippableFramePolicy::Accept`（既定・unit 非計上）/
  `Reject`（`Unsupported`）。Dictionary_ID≠0 の frame は
  `Unsupported`（dict 未対応）。magic-less/legacy/
  seekable format は対象外。

### Integrity の区別

- `DecodeReceipt.integrity()` は `IntegritySummary
  {checked_units, unchecked_units}` — checksum を持つ
  unit と持たない unit を混在 stream で正確に分離する
  （Issue #285 v0.2-D「`ChecksumVerified` 一値に丸めない」）。
- gzip member は CRC32+ISIZE を毎回検証（checked）、
  raw deflate は checksum を持たず（unchecked）、zstd は
  frame checksum flag に従う。
- CRC32/zstd checksum は **非暗号学的な破損検出**であり、
  出所の真正性・悪意ある改変への保護を保証しない
  （Issue #285 v0.2-D・#288 否定系判定表の「CRC 等は整合
  性検出であり真正性保証ではない」行と同一原則）。

## Finish/状態機械/所有権契約

Issue #285 v0.2-A/B を正本として固定する（実装済みの確定
形 — `src/stream.rs` の `EncoderCore`/`EncState` と各
module doc が正本）。

### Encoder

```text
new() ──header(gzip)──> Writing
Writing --write--> Writing        (入力全体受理か Err)
Writing --flush--> Writing        (deflate/zstd sync flush + 下流 flush — 完了ではない)
Writing --finish-> Finishing      (format 終端処理: deflate final block / gzip trailer / zstd epilogue+checksum)
Finishing  --ok---> Finished      (EncodeReceipt 保存)
Finishing  --err--> Failed
Finished   --finish-> Finished    (保存 receipt を再返却・冪等)
Finished   --write-> Err(InvalidState)   (Panic にしない)
Failed     --write/finish-> Err(InvalidState)
```

- `finish()` のみが完了証拠。**Drop は一切書込まない**
  （flate2 `Compress`/`Decompress`・zstd-safe `CCtx`/`DCtx`
  の Drop は解放のみで trailer/frame 終端を書かない —
  実装保証として pin、`tests/encoder_contract.rs` の
  `drop_writes_no_trailer`/`drop_writes_no_final_block`/
  `drop_writes_no_frame_epilogue`）。Drop による暗黙の
  圧縮完成を成功条件にしない（Issue #285 v0.1-4）。
- `flush()` は pending byte の下流伝搬のみで、format
  終了・trailer/frame 終端とは別物。
- 失敗した Encoder は terminal `Failed` — 初期版は圧縮
  再開を提供しない。`Failed`/`Finished` での追加 write
  は panic せず `io::Error` + `InvalidState` を返す
  （ENC-05）。
- Encoder 失敗時、底流 Writer の所有権は借用のため
  Consumer 側に残るが、**失敗後に底流へ残る部分出力を
  正式 Publish してはならない**（ENC-03 — publish 判断は
  Consumer/IO-01 側の責務として合成順序で保証）。

### Decoder — VerifiedEnd

Decoder の内部状態は format 毎の private `Phase` と
`failed: bool` latch で構成される。gzip は
`Start/Header/Body/Trailer/Between/Done`、zstd は
`Start/Header/Body/Between/Done`、raw deflate は
`Body/Boundary/Done`。doc 上の **VerifiedEnd** は
`Phase::Done`（unit 終端・trailer/checksum 検証・scope
の入力境界条件の全てが確認済み）を指し、**Failed** は
`failed` latch を指す（`src/{gzip,deflate,zstd}.rs` が
正本）。

```text
Body/Between --(unit 処理中)--> 同相
Body --delivered == max_decoded--> 1-byte scratch probe
        （追加出力があれば LimitExceeded → Failed、
          probe byte は Consumer に返さない）
unit 終端 + trailer/checksum 検証済み + scope 条件充足
        --> Phase::Done（= VerifiedEnd）
任意の失敗 --> failed latch（Failed）
Phase::Done --finish()--> Ok(DecodeReceipt)（冪等）
Done 以外 --finish()--> Err(NotExhausted)（state 不変）
Failed --任意 read/finish--> Err(InvalidState)
```

- `impl Read` は展開 byte を逐次返し、`Ok(0)` は
  `Phase::Done`（VerifiedEnd）でのみ返る。`read(&mut [])`
  の `Ok(0)`・非空 buffer での途中 `Ok(0)` は完了証拠に
  しない（DEC-05 — `read(&mut [])` は state を変えない）。
- decoded budget 到達時の「追加出力の有無」判定は独立
  state ではなく `Body` 相の inline probe — 残量 0 の
  とき 1 byte scratch へ pump し、`produced > 0` なら
  `LimitExceeded`（その byte は Consumer に返さない）。
  これは SEC-04「上限と完全一致する正常データ/1 Byte
  超過の厳密な区別」の実装形。
- `Decoder::finish()` は **状態照会のみ** — `Phase::Done`
  → `Ok(receipt)`、`failed` → `Err(InvalidState)`、他 →
  `Err(NotExhausted)`。読取を進めず、未読出力を破棄して
  成功させない（DEC-04）。
- 未読の展開出力を持つ Consumer は、検証済み完了まで
  一時領域/非公開 Sink へ書く（Issue #285 v0.2-B）。
  実 OS 境界では `SealedFile::open_readback` の検証と
  `Decoder::finish()` の VerifiedEnd を publish 前に
  確認する（#306 合成 E2E が実演）。

## Resource Budget 契約

Issue #285 v0.1-4/v0.2-D を正本として固定する（実装済み
の確定値 — `src/options.rs`・`src/stream.rs`）。

### Hard limit（この層で実強制される上限）

- `EncodeLimits`（budget accessor: `max_input_bytes` /
  `max_output_bytes`）: `write` が受理する未圧縮 byte と
  底流 writer への累積書込 byte（format framing を含む）
  を cap。上限を超える物理書込は行わない（`LimitExceeded`
  → `Failed`）。既定は `u64::MAX`（無制限 — 「無制限
  Default」は byte budget のみを意味し、他の資源を保証
  しない）。
- `DecodeLimits`（budget accessor: `max_compressed_bytes` /
  `max_decoded_bytes` / `max_units`）: 連結 stream
  **全体に累積適用**。`max_compressed_bytes` は `BufRead`
  からの論理消費量、`max_decoded_bytes` は Consumer へ
  返した展開 byte、`max_units` は decode した unit 数
  （gzip member / deflate stream / zstd data frame。
  skippable frame は units に計上しないが byte には計上）。
- `max_decoded_bytes` に**ちょうど一致**して unit が終端
  した場合は許可。上限到達で即座に正常 EOF を返さず、
  `Body` 相の inline probe（1 byte scratch）で「追加
  出力の有無」を判定する — 超過なら `LimitExceeded`
  （probe byte は Consumer に返さない）。これは SEC-04
  「上限と完全一致する正常データ/1 Byte 超過の厳密な
  区別」の実装形。
- zstd `max_window_log`（既定 27 = 参照 decoder の
  `ZSTD_WINDOWLOG_LIMIT_DEFAULT`・128 MiB、`10..=41` に
  clamp）: frame header parse で `Window_Size >
  1 << max_window_log` を backend 到達前に
  `LimitExceeded` で拒否し、backend の
  `ZSTD_d_windowLogMax` にも同値を設定して二重に強制
  （SEC-05 — Window/Frame 数の強制可能性）。

### Advisory（厳密保証を主張しない）

- CPU 時間、backend 内部 heap（deflate window・zstd DCtx
  の frame 依存確保）、consumer/出力 sink 側メモリは
  **hard limit として宣言しない**（`src/options.rs`・
  crate README が正本）。`max_decoded_bytes` は「Consumer
  へ返す byte 数」の上限であり、backend が内部に確保する
  memory の上限ではない。
- `max_window_log` は frame 宣言 window の強制であり、
  backend 内部の全確保を一般化して bound する機構ではない
  （defense-in-depth の位置づけ）。
- byte budget は「入力/出力サイズの上限」であり解析・
  圧縮コストの保証ではない。

### Cancellation

- `CompressionErrorKind::Cancelled` は予約済み variant で
  現在の cancellation surface は存在しない（`error.rs`）。
  Cancellation は既存の上位 Job 制御と連携可能にしつつ、
  **Compression 専用 Scheduler を作らない**（Issue #285
  v0.1-4）。中断後は `finish()` 成功扱いせず、IO-01 の
  `StagedFile::abort` で temp を回収する — この配線は
  Consumer/上位 Job の責務であり、本層は状態機械上
  `Failed`/未完了のまま公開面へ出さないことを保証する。

## Error 契約

`CompressionErrorKind`（`#[non_exhaustive]`・12 variant）
を正規化分類の正本とする（`src/error.rs` が正本）:

| Kind | 意味 | `io::ErrorKind` 対応 |
| --- | --- | --- |
| `Io` | 下流 Read/Write/BufRead 失敗 | `Other`（元の `io::Error` は `source()` 経由で保持） |
| `InvalidInput` | options/引数不正（level 範囲外等） | `InvalidInput` |
| `InvalidFormat` | magic/method/reserved/header 異常 | `InvalidData` |
| `TruncatedInput` | unit 未完のまま入力 EOF | `UnexpectedEof` |
| `CorruptData` | deflate body/zstd block 破損 | `InvalidData` |
| `ChecksumMismatch` | CRC32/ISIZE/zstd checksum 不一致 | `InvalidData` |
| `TrailingData` | WholeInput の末尾余剰 byte | `InvalidData` |
| `LimitExceeded` | 累積 budget/window 超過 | `QuotaExceeded` |
| `Unsupported` | 未提供 format/capability（dict frame・skippable Reject 等） | `Unsupported` |
| `Cancelled` | 上位 job 中断（予約・現在未使用） | `Interrupted` |
| `InvalidState` | Finished/Failed での操作 | `InvalidInput` |
| `NotExhausted` | `finish()` 時に未読出力/未検証残 | `Other` |

- `finish()`/`new()` は `CompressionError` を直接返し、
  `impl Read`/`impl Write` では `io::Error` に包まれる —
  いずれも `source()` 鎖で kind と backend/IO 原因へ
  辿れる（classification は失われない）。
- backend error の kind 分類は zstd で `get_error_name`
  文字列を介する（`docs/designs/issue-301-…` の分類表）
  — 分類不能は `CorruptData` 側に倒し、誤って成功させ
  ない方向を pin している。
- 「Format 書込み失敗/検証失敗」と「File Publish 失敗」
  を混同しない — Compression 層は publish を所有せず、
  `StagedFile::seal().publish()` の `PublishFailure`
  （effect/phase/recovery）情報は IO-01/Consumer 側で
  保持される（#295 契約・#306 E2E R3 系と整合）。

## Cargo Feature・Backend 選定（確定値）

### Feature matrix

`default = []`。format feature は独立に組合せ可能
（`gzip`/`deflate` は `flate2` を共有、`zstd` は
`zstd-safe`）。`ruviform-*` 依存はゼロ。

| feature | module | backend（private・optional） |
| --- | --- | --- |
| `gzip` | `gzip` | `flate2 = "1.1.10"`（`default-features=false` + `rust_backend`） |
| `deflate` | `deflate` | 同上（共有 dep） |
| `zstd` | `zstd` | `zstd-safe = "8.2.0"`（`default-features=false`） |

- backend は `Cargo.toml` の dependency record が正本
  （`flate2`/`zstd-safe` とも `version = "x.y.z"` 指定 +
  `Cargo.lock` 追跡。公開日・MSRV・ライセンスは同
  record の comment が正本）。
- dev-dependencies: `flate2`（高水準 adapter — 参照
  stream 生成/消費の wire interop 検証のみ）、`zstd`
  `0.14.0`（同 — frame 生成・独立 decode oracle）。
  いずれも test 専用で公開 API・runtime 依存に入らない。

### Backend 選定表（Issue #285 Design Acceptance Gate 対応）

| Format | 採用 | 解決版 | 公開日 | MSRV | License | 選定根拠 |
| --- | --- | --- | --- | --- | --- | --- |
| Gzip/Deflate | `flate2` 低水準 `Compress`/`Decompress`/`Crc`（`rust_backend` = miniz_oxide + crc32fast、pure Rust） | 1.1.10 | 2026-08-28 | 1.67 | MIT OR Apache-2.0 | `total_in`/`total_out` で byte 精密計測可・C toolchain 不要・メンテ活発（#300 設計） |
| Zstd | `zstd-safe` 低水準 `CCtx`/`DCtx`（`compress_stream2`/`decompress_stream` over `InBuffer`/`OutBuffer`） | 8.2.0 | 2026-10-16 | 1.64 | BSD-3-Clause | byte 精密計測・`Ok(0)`=frame 完全終了・`DParameter::WindowLogMax` 強制可・`try_create` は OOM 非 panic（#301 設計） |

不採用候補（#300/#301 設計の比較表が正本）:

- flate2 高水準 `GzEncoder`/`GzDecoder`/`MultiGzDecoder` —
  BufRead 先読みで member 境界の未消費保証が壊れ、
  `finish` が self 消費または Drop で trailer 書込を試みる
  （「Drop writes nothing」と矛盾）。
- `zstd` 高水準 crate（runtime 用途）— `stream::read` が
  BufRead を内部 buffer へ先読み、`stream::write` が Drop
  で `ZSTD_endStream` を書き、finish が self 消費型。
- `ruzstd`（pure Rust）— `StreamingDecoder` が先読みし
  byte 精密消費を観測できず、encoder は upstream 自身が
  未成熟と明記。
- `libz-sys`/C backend・`zstd` C lib 直接 FFI — C
  toolchain・unsafe 方針（`#![forbid(unsafe_code)]`）・
  MSRV/配布の複雑化。
- 独自 DEFLATE/圧縮アルゴリズム — Issue #285「既存 Rust
  Compressor/Decompressor を採用して独自圧縮アルゴリズム
  を作らない」。

全 backend の MSRV は workspace MSRV 1.89 以下。`zstd` は
`zstd-sys` が vendored C source を `cc` でコンパイルする
（cmake/bindgen 不要）— `zstd` feature 有効時のみ C
toolchain が必要で、`compression-verify` lane（ubuntu）
とローカル matrix が対象。security advisory の継続監視は
#288 gate の Cargo feature/OS/MSRV/license/security
audit 項目に接続する。

### 依存 DAG・RT 制約

- `ruviform-io-compression` は `ruviform-*` への依存を
  持たない（葉側）。`ruviform-io-archive` →
  `ruviform-io-compression` は **dev-dependency のみ**
  （tar.gz 合成試験）で production DAG に edge はない。
  `ruviform-io-e2e` が `gzip,zstd` feature で consumer
  側合成を検証する。
- Archive/Compression/File との合成は `std::io` の
  Write-in-Write/Read-in-Read のみで行い、Compression が
  Archive/IO-01 へ依存する方向は禁止（Issue #285
  v0.2-E・v0.1-3「Archiveとは独立」）。
- media-io / domain / engine / DSP / RT core への依存は
  持たない。Parser/Compression/Archive・ブロッキング
  I/O を RT コアへ混入しない（DAG 正本:
  `docs/designs/ruviform-architecture-design-baseline.md`・
  各 crate README）。
- 全 API 非 RT — `Read`/`Write` はブロック・alloc・
  backend 計算量を行い得る。RT オーディオコールバック
  からは絶対に呼ばない（crate-level doc が正本）。
  `Encoder`/`Decoder` は借用 stream が `Send` なら `Send`
  （`tests/api_surface.rs` で pin）。
- 公開 enum（`CompressionErrorKind`/`DecodeScope`/
  `SkippableFramePolicy`）は `#[non_exhaustive]`。

## Consumer 利用面（AudioLens / Synvelis）

Issue #285「関連」・v0.1-6・COMP-01/COMP-03 を正本とし
て固定する。Consumer の schema/version/migration/
canonical signed bytes/manifest semantics は各 Consumer
が所有し、Ruviform はそれらを移動しない。

### AudioLens（AL-IO-01 #139 — CLOSED）

- 利用面: Dataset 圧縮・archive 要件（#288 Owner 接続表
  の「Dataset 圧縮（AL）」行）。既存の dataset/manifest
  schema を Consumer 側に保持したまま、payload の
  gzip/zstd 圧縮を `Encoder`/`Decoder` の借用 stream で
  行う。`std::io` 合成のみで完結し、Ruviform 側に schema
  を要求しない。
- 検証観点: Round-trip/性能/保存結果の回帰を AL 側で検証
  し #288 へ証拠を接続（COMP-01/COMP-03）。Artifact
  Hash/Canonical Bytes の対象を圧縮前 byte とするか圧縮
  後 byte とするかは AL の設計判断（Issue #285 v0.1-5）。
- 破損検出は gzip CRC32/ISIZE・zstd checksum（存在時）
  による整合性確認であり、真正性・改竄耐性は AL 側の
  別手段（署名等）の責務。

### Synvelis（SYN-IO-01 #82 — CLOSED・SYN-IO-02 #84 — OPEN・#25 — CLOSED）

- 利用面: Backup 圧縮（#288 Owner 接続表の「Backup 圧縮
  （SV #84）」行）— tar.gz / tar.zst のような
  `TarArchiveWriter → Compression Encoder → &mut
  StagedFile` の `std::io` 合成、または独立 Raw Zstd
  保存。Project Snapshot/Revision/HEAD の意味論は SV が
  所有し、Ruviform は byte 変換のみを提供する。
- 検証観点: SV #84（DirectoryBundleStore・Snapshot/HEAD
  保存の StagedFile 移行・OPEN）が実装段階で本契約を
  消費する。Ruviform 単一 File Publish は Project 全体の
  Transaction や HEAD の CAS とみなさない（#288 設計の
  Consumer 要件と同一原則）。
- WAV PCM/Riff finalize/再 Open（#25 CLOSED の既存 E2E）
  は Media I/O（#305）側の責務であり IO-03 の対象外。

### Consumer 独立判定

#219/#288 の契約どおり、同一候補 Ruviform Revision でも
AudioLens と Synvelis は独立して判定する。IO-03 の採用は
consumer-verify.yml（#294）の候補 tuple lane で実測され、
`impacted`/`no impact`/`unknown` の影響分類と
`pass`/`fail`/`blocked`/`not run`/`unsupported` の Gate
結果で記録する。本書は要件の定義のみで、実施は各
Consumer repo が Owner。

## File/Archive 境界と合成契約

Issue #285 v0.1-3/v0.2-E を正本として固定する。

- **依存方向**: Compression は `ruviform-io-file`・
  `ruviform-io-archive` に依存しない。合成は全て
  `std::io` 経由の Write-in-Write / Read-in-Read で行い、
  Consumer（または `ruviform-io-e2e` 等の試験層）が層を
  束ねる。`ruviform-io-archive` → `ruviform-io-compression`
  は dev-dependency のみで production edge はない。
- **Encode 側の必須順序**（TAR.GZ 等）:
  `Archive Writer.finish()` → `Compression Encoder.finish()`
  → `StagedFile::seal()` → `StagedFile::publish()`。
  1 段でも失敗した場合は正式公開しない。実装形は
  `Encoder<&mut StagedFile>` の借用で、Encoder scope 終了
  後に `seal()` する（`crates/ruviform-io-e2e/tests/
  archive_compression_staged.rs` が実演）。
- **Decode 側**: 展開出力は検証済み完了まで一時領域/
  非公開 Sink へ書く。`Decoder::finish()` の VerifiedEnd・
  `SealedFile::open_readback` の独立検証・上位層
  （TAR entry 等）の終端検証は **別々の証明**であり、
  TAR Reader が終端へ到達しても Compression Decoder の
  完了とは扱わない（ARC-01 — 実装形は readback 検証 +
  `decoder.finish()` の VerifiedEnd を publish 前に確認）。
  余剰展開データは IO-04 側の明示的 Policy で扱い、
  Compression 層は黙って読み捨てない。
- **検証補助**: `drain_and_verify` 相当の補助を設ける
  場合でも、残余データを無条件に無視して正常完了を返す
  ことはしない（Issue #285 v0.2-E）。現行 API は
  `DecodeScope`/`DecodeLimits`/`finish()` の組合せでこの
  契約を表現する。
- **scoped/rooted path**: `StagedFile::create` の宛先
  path 規則は IO-01/#409 が所有し、Compression は path を
  一切扱わない（`std::io` stream のみ）。

## 証拠項目対応（Evidence Mapping）

Issue #285 の ENC/DEC/SEC/INT/ARC/COMP/BUF 証拠項目と
Design Acceptance Gate を、既存の正本（per-format
test-spec・合成 E2E・#288 matrix・実装 Issue/PR）へ対応
付ける索引。本書は参照のみを持ち、証拠内容を複写しない。

### 証拠項目 → 正本対応表

| 証拠項目（Issue #285） | 内容 | 正本（test-spec / gate / 実装） |
| --- | --- | --- |
| ENC-01 | Format 別 `Encoder<W:Write>`・借用・状態遷移がコンパイル可能 | `issue-300` R1/R8、`issue-301` AC6、`tests/encoder_contract.rs`（gzip/deflate/zstd 各 feature） |
| ENC-02 | `flush`/`finish` の意味・Trailer/Frame 完了・Drop 未完了・追加 Write の InvalidState | `issue-300` R1/R2、`issue-301` AC5、#288 matrix C-X4、`tests/encoder_contract.rs`（`flush_*`/`drop_writes_*`/`write_after_finish_rejected_*`/`flush_mid_stream_then_finish_still_decodes`/`flush_after_finish_is_ok`/`finish_fails_when_underlying_flush_fails`） |
| ENC-03 | Encoder 失敗時の底流 Writer 所有権保全・部分出力の禁止 Publish | `issue-300` R2、`writer_borrow_recovered_after_scope`、`failed_encoder_releases_usable_writer`/`failed_encoder_drop_writes_no_*`、#306 `archive_compression_staged.rs`（未 finish → abort → destination 不在） |
| ENC-04 | 借用 Writer の Ownership と失敗後の底流 Writer 回復可能性 | `issue-300` R2・`writer_borrow_recovered_after_scope`・`io_failure_latches_failed_and_rejects_reuse`・`failed_encoder_releases_usable_writer` |
| ENC-05 | finish 成功の冪等性・Failed からの追加 Write 拒否・Panic 防止 | `issue-300` R1、`issue-301` AC5（finish 冪等・Failed latch）、#288 matrix C-X4、`finish_is_idempotent_after_verified_end`（decoder 側） |
| ENC-06 | Backend Drop 副作用の調査・未完成 Stream の誤 Publish 防止 | `issue-300` R2（Drop は書込まない実装保証）、`issue-301` AC5（Drop）、#300/#301 設計「Backend 選定と隠蔽」 |
| DEC-01 | 展開終了/Trailer/Checksum/EOF の検証 Receipt・Checksum 非存在との区別 | `issue-300` R3/R4/R5、`issue-301` AC2、`IntegritySummary` checked/unchecked（`tests/*_format.rs`・`decoder_contract.rs`・`finish_is_idempotent_after_verified_end`） |
| DEC-02 | Gzip Member/Zstd Frame 単独・連結の処理・入力境界・Trailing 拒否 | `issue-300` R4/R5/R6、`issue-301` AC1/AC3、#288 matrix C-X1/C-X3、`tests/gzip_format.rs`/`zstd_format.rs`（連結・trailing・SingleUnit）・`chained_decoder_*`/`sequential_encoders_*` |
| DEC-03 | 実際に読んだ圧縮 Byte/返した展開 Byte/Frame 数の正確な計測 | `issue-300` R6（consumed vs prefetch）、`issue-301` AC5、#288 matrix C-X3、`consumed_counts_logical_bytes_not_prefetch`/`compressed_consumption_ignores_caller_prefetch` |
| SEC-01 | 展開 Byte 上限到達を正常 EOF と誤認しない・高圧縮率/過大 Window/連結 Frame で予算強制 | `issue-300` R7、`issue-301` AC3/AC4、#288 matrix C-X2、`decoded_budget_*`/`compressed_budget_*`/`unit_budget_*`/`oversized_window_rejected_as_limit_exceeded`/`decoded_budget_enforced_on_high_ratio_payload` |
| SEC-02 | Corrupt/Truncated/Bad Checksum/Input Overrun/Cancel/Backend Panic 境界の否定テスト・Fuzz | `issue-300` R4/R5/R7、`issue-301` AC2/AC3/AC4、#306 `negative_fuzz.rs`（corrupted gzip/composite mutant）、#288 matrix C-X1/C-X2、`read_error_latches_failed`（source I/O 失敗の `Io` 正規化と latch） |
| SEC-04 | 出力上限と完全一致する正常データ/1 Byte 超過の厳密な区別 | `issue-300` R7（ちょうど許容・+1 `LimitExceeded`・probe）、`issue-301` AC4、#288 matrix C-X2、`output_budget_exact_boundary_succeeds`/`decoded_budget_allows_exact_and_rejects_one_over`/`decoded_budget_enforced_on_high_ratio_payload` |
| SEC-05 | Window/Frame 数/Backend 内部 Memory の強制可能性 | `issue-301` AC3（`max_window_log`・units）/AC4、#300/#301 設計「Limits」、本書「Resource Budget」節の非保証明記 |
| INT-01 | TAR→Gzip→StagedFile の `finish` 順序と Publish 失敗の保全・独立 Raw Zstd 保存・IO-04 Archive への接続 | `issue-300` R9（TAR 様合成）、#306 `archive_compression_staged.rs`（tar_gzip/tar_zstd staged round-trip・layered completion proofs）、#288 matrix E-N1 |
| INT-02 | 複数 Frame で Checksum 有無が混在する Receipt 検証 | `issue-301` AC2（mixed concatenated → checked=1/unchecked=1）、`IntegritySummary` |
| ARC-01 | TAR 終端と Gzip 終端を別々に検証 | #306 `fault_boundary.rs`（`verify_composite`: gzip verified EOF wrapping TAR read — `archive.finish()` と `decoder.finish()` の両方を要求）・`archive_compression_staged.rs`（layered completion proofs）・`docs/designs/issue-303-…`（TAR EOF 検証）・`docs/designs/issue-304-…`（tar.gz 展開） |
| COMP-01 | AudioLens #139・Synvelis #82 の Round-trip/性能/保存結果を各 Consumer で検証し #288 へ証拠を接続 | #288 Owner 接続表 IO-03 行・Consumer 非回帰判定表・#219/#294 consumer-verify・AL #139・SV #82/#84/#25（いずれも実走は Consumer repo の段階） |
| COMP-03 | 両 Consumer に精度・性能の回帰を生まない・横断受入は #288 | 同上（COMP-01 と同一の gate 経路） |
| BUF-01 | BufRead の残余 Byte 境界が保たれる | `issue-300` R6、`issue-301` AC5、#288 matrix C-X3、`single_unit_*`/`SingleUnit` 系試験・`chained_decoder_*`（次単位の独立検証） |
| BUF-02 | compressed bytes consumed と underlying prefetch を区別 | `issue-300` R6、`issue-301` AC5、#288 matrix C-X3、`consumed_counts_logical_bytes_not_prefetch`/`compressed_consumption_ignores_caller_prefetch` |

### Design Acceptance Gate → 対応

| Gate 項目（Issue #285） | 対応する正本 |
| --- | --- |
| Gzip/Deflate/Zstd 別の Reader/Writer API・所有権・Finish 状態遷移 | 実装済み API（`src/{gzip,deflate,zstd}.rs`・`src/stream.rs`）+ per-format 設計（#300/#301）+ 本書「Finish/状態機械/所有権契約」節 |
| Backend 型が Public API へ露出しない設計・候補実装比較 | 本書「Serde 境界」「Backend 選定表」+ #300/#301 設計「Backend 選定と隠蔽」+ `tests/api_surface.rs`・#301 AC6 |
| 完全入力/途中 EOF/破損/Checksum/過大展開/Cancel/Writer Error の契約と試験 | 本書「Error 契約」「Resource Budget」節 + `issue-300` R1–R7・`issue-301` AC1–AC5 + #306 `negative_fuzz.rs`/`fault_boundary.rs` |
| TAR.GZ・Raw Zstd・StagedFile 保存の Consumer コードで合成可能 | #306 `archive_compression_staged.rs`（tar.gz/tar.zst staged round-trip）+ 本書「File/Archive 境界と合成契約」節 + crate README の合成例 |
| 性能・バッファ・feature・MSRV・ライセンス/セキュリティ評価と RT 境界 | 本書「Backend 選定表」「依存 DAG・RT 制約」節 + `Cargo.toml` dependency record + `mise run check/lint/test-io-compression`（feature matrix）+ CI lane `compression-verify` |
| 独立実装 Issue 化可能な仕様・ADR | #300/#301 の docs チェーン（spec/design/plan/test-spec/task）+ 本書の対応索引（実装 Issue は既に分解・merge 済み） |

### #288 gate との接続

#288（IO-06）の受入 matrix は IO-03 を次の行で参照する
（`docs/designs/issue-288-…` が正本）:

- C-N1（Gzip/Raw Deflate round-trip・明示 finish）→
  `issue-300` R1–R3。
- C-N2（Zstd single/concatenated/skippable・frame 単位
  receipt）→ `issue-301` AC1。
- C-X1（Truncated/Checksum 不一致/Trailing）→
  `issue-300` R4–R5、`issue-301` AC2/AC3。
- C-X2（上限ちょうど/+1・window/frame/budget）→
  `issue-300` R7、`issue-301` AC3/AC4。
- C-X3（Window/Frame 数・先読み境界・prefetch 非混同）→
  `issue-300` R6、`issue-301` AC5。
- C-X4（未 finish/Drop のみを完成証拠にしない）→
  `issue-300` R1–R2、`issue-301` AC5、本書「Finish/状態
  機械」節。
- E-N1/E-X1/E-X2（合成 E2E: TAR→Gzip/Zstd→StagedFile・
  完成証明なし publish 阻止・実 OS fault class）→
  `issue-306` test-spec・`mise run test-io-e2e`・本書
  「File/Archive 境界」節。

IO-03 単体の実走証拠は `mise run test-io-compression`
（feature matrix: default・`gzip`・`deflate`・`zstd`・
`gzip,deflate,zstd`）と `cargo test --workspace`（doc test
含む）、CI lane `compression-verify` が入口。Consumer
tuple の採否判定は #288 gate と consumer-verify（#219/
#294）が Owner。

## Design Done / Implementation Done の分離

- **本 slice（docs）**: 横断契約・Format/Version 骨格・
  Finish/状態機械・Budget・Error 語彙・Feature/Backend
  確定値・Consumer 面・証拠索引の **設計正本の文書化**。
  実装は #300/#301 で merge 済みだが、本書の存在は
  「設計が文書化された」ことのみを意味し、Consumer 実測・
  OS 別実機・性能比較の完了を意味しない。
- **残る検証（実装/受入段階）**: Consumer 側の Golden・
  byte 互換・性能の独立検証（AL/SV Issue・consumer-verify
  tuple）、macOS/Linux 実機での `zstd` feature（vendored
  C build）経路、`Cancelled` の上位 Job 連携、CI lane の
  継続監視。未実施は `not run`/`NotRun`/`Unsupported` と
  記録する。
- Issue #285 の受入条件のうち「API 署名/所有権が実際に
  利用可能」「Backend 型非露出」「Limits/security の実
  強制」「Error/Publish 連携」「feature/OS/MSRV/license/
  security audit と nonRT DAG」は #300/#301 の実装と
  test-spec で既に検証済みの範囲を指し、本書はその正本
  への索引を提供する。「両 Consumer 回帰」は
  cross-consumer gate（#288・#219/#294）の実走段階の
  要件であり、本 slice の対象外。

## Risks and Trade-offs

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | 横断契約と per-format 正本の記述が drift する | med | med | 本書は骨格と索引のみ所有し、詳細は #300/#301 へ参照。test-spec の audit 規則（backend 非露出・budget・error 分類）が pin |
| 2 | 「byte 上限があるから heap/CPU も bounded」と誤読される | med | high | Advisory 節で CPU/backend 内部 heap/consumer heap を非保証と明記。`options.rs`・crate README と同一方針 |
| 3 | Consumer が schema/version を Ruviform に移そうとする・checksum を真正性と誤認する | low | med | Consumer 利用面節で schema/version/canonical bytes の Consumer 所有と checksum=整合性検出（真正性でない）を明記。crate README・#288 否定系判定表と一致 |
| 4 | Encoder 失敗後の部分出力が誤って Publish される | low | high | 借用型で底流所有権は Consumer に残り、publish は `finish()` 成功 + seal/readback 検証の後に限定する合成契約を「File/Archive 境界」節と証拠索引（ENC-03→#306）で pin |
| 5 | backend 更新で公開境界・budget 保証・kind 分類が壊れる | med | med | `Cargo.lock` 追跡 + release-age ポリシー。upgrade は PR で明示的に行い per-format test-spec の audit 規則で再検証。zstd backend error name 分類は Negative test で pin（#301 設計） |
| 6 | docs のみの変更が「Consumer 受入完了」と誤解される | low | med | Design/Implementation Done 分離節で明示。Consumer 実測は #288/#219/#294 の段階 |
| 7 | `SingleUnit` の論理消費と物理 prefetch の混同 | low | med | `compressed_bytes_consumed` は論理消費のみを計上し `source_bytes_fetched` を持たない契約を BUF-02・DEC-03 の証拠索引で pin |

## Deferred Design Concerns

- Consumer 側の実測・移行（AL Dataset 圧縮 Golden・byte
  互換、SV Backup tar.gz/zip の旧経路互換） | cross-repo
  gate（#219/#294）と各 Consumer Issue が Owner | consumer
  候補 tuple の実走時 | 設計どおりの互換性が実測で
  裏付けられないまま採用されるリスク。
- macOS/Linux 実機での `zstd` feature（vendored C build・
  `cc` toolchain）経路の実測 | 本 slice の作業環境は
  Windows のみ | CI 既存 OS lane または実機環境が利用
  可能になった時点 | OS/C toolchain 差異が zstd feature
  で未検証のまま残る。
- 所有型 API・zlib wrapper・preset dictionary・skippable
  frame 書込・async/no_std | 実需が確認できた時点 |
  #300/#301 設計の Deferred Design Concerns と同一の再開
  条件 | 公開面の拡張なしに Consumer が必要とする合成が
  実現できないリスク。
- `Cancelled` kind の上位 Job 連携・中断後の StagedFile
  abort 配線 | 上位 Job/Consumer の cancellation 実装時 |
  中断後に未完了 stream が成功扱いされるリスク。
- `drain_and_verify` 相当の検証補助 | tar.gz 等で残余
  展開データの扱いが必要になった時点 | 残余を無条件に
  無視する補助は Issue #285 v0.2-E で禁止済み — 実需と
  policy が確定してから | 未読残余の黙殺リスク。

## Verification Notes

- [ ] Serde 境界節が「serde 依存ゼロ・許可型は std::io +
      Ruviform 独自型・backend 型/Error/feature/version
      非公開・source() 経由のみ・自作 codec/registry なし」
      を全て含み、`tests/api_surface.rs`・`issue-300` R8・
      `issue-301` AC6 への参照を持つこと。
- [ ] Format/Version 契約節が 3 形式（RFC 1951/1952/8878）
      の区別・Zlib wrapper 対象外・Format 別 level
      semantics・`DecodeScope`・連結単位・checksum
      checked/unchecked・再現性の範囲（同一設定 round-trip
      + gzip header 固定・backend version 間 byte 一致は
      非保証）を含むこと。
- [ ] Finish/状態機械節が Encoder/Decoder の状態遷移・
      `finish()` 冪等・Drop 無書込・`NotExhausted`/
      `InvalidState`・底流 Writer 保全を含み、
      `src/stream.rs`・`tests/encoder_contract.rs`・
      `tests/decoder_contract.rs` の実装と一致すること。
- [ ] Resource Budget 節が `EncodeLimits`/`DecodeLimits`/
      `max_window_log` の累積適用・境界包含・probe・
      CPU/backend heap/consumer heap の非保証を含むこと。
- [ ] Error 契約節が `CompressionErrorKind` 12 variant と
      `io::ErrorKind` 対応・`source()` 復元を含み、
      `src/error.rs` と一致すること。
- [ ] Backend 選定表の版・MSRV・ライセンスが
      `crates/ruviform-io-compression/Cargo.toml` の
      dependency record と #300/#301 設計の記述に一致する
      こと（flate2 1.1.10 rust_backend・zstd-safe 8.2.0・
      dev-dep zstd 0.14.0）。
- [ ] 証拠項目対応表の ENC/DEC/SEC/INT/ARC/COMP/BUF 各行
      が実在する test-spec rule/節・#288 matrix 行・
      Issue/PR 番号へ解決できること。
- [ ] Consumer 利用面節が AL #139・SV #82/#84/#25 の Issue
      番号と責務分界（schema/version は Consumer 所有・
      checksum は整合性検出・Ruviform 単一 Publish ≠
      Project Transaction/HEAD CAS）を含むこと。
- [ ] 「設計合意と実装/検証の分離」節が存在し、未実施項目
      を `not run`/`NotRun`/`Unsupported` と記録する方針を
      明記していること。

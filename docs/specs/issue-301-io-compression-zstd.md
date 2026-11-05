---
id: "SPEC-GvsblrwUriPJe6B9kCJ118"
type: "spec"
status: "approved"
title: "Issue #301 ruviform-io-compression — Zstd Frame 境界・Window/Skippable・Verification Receipt (IO-IMP-07)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/301"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/288"
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
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
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

# Issue #301 ruviform-io-compression — Zstd Frame 境界・Window/Skippable・Verification Receipt (IO-IMP-07)

## What Should Be Built

既存 `ruviform-io-compression` クレートに **`zstd` feature / `zstd` module**
を追加し、Zstandard Frame（RFC 8878）の借用 Stream Encoder/Decoder を
IO-03 v0.1/v0.2 合意設計と Issue #300（IO-IMP-06）で確立した公開契約に
乗せる。新規クレートは作らない。

提供する公開 API（gzip/deflate と同一形状）:

- `zstd::Encoder<'w, W: Write + ?Sized>` / `zstd::Decoder<'r, R: BufRead +
  ?Sized>` — 借用 `std::io` 型。明示 `finish()` のみが完了証拠（冪等）。
  `Drop` は一切書込しない。
- `zstd::EncodeOptions` / `zstd::DecodeOptions` — Zstd 固有の format
  option（level・content checksum・`max_window_log`・Skippable Frame
  Policy）。共有語彙 `EncodeLimits` / `DecodeLimits` / `DecodeScope` /
  `EncodeReceipt` / `DecodeReceipt` / `IntegritySummary` /
  `CompressionError(Kind)` をそのまま再利用する。
- Decoder は Single/Concatenated Frame・Skippable Frame・
  checksum あり/なし混在を扱い、`units_decoded`（正確な Frame 数）・
  `compressed_bytes_consumed`（論理消費・prefetch 非混同）・
  `integrity.checked_units / unchecked_units`（checksum 混在を丸めない）
  を検証済み receipt として正確に観測する。Backend で保証できない
  capability は Unsupported 固定とし、見せかけの receipt を生成しない。

## Why Now

IO-IMP-06（Issue #300 / PR #356）で `ruviform-io-compression` scaffold・
共有語彙・`compression-verify` lane が main に揃った。Issue #301 は
IO-03 設計の初期対象 format 3 種のうち最後の Zstd を追加し、
Consumer（AudioLens #139・Synvelis #82）の `.zst` 保存/復号需要と
IO-06 横断受入（Issue #288）へ接続する。

## Users and Value

- `.zst` 単独保存・連結 frame の検証済み復号: `DecodeReceipt` で
  「Frame 終端 + checksum + scope 境界が全て検証済み」を受け取れる。
- Skippable Frame（metadata block）を含む wire stream の厳密な扱い:
  accept/skip と reject を policy で選択できる。
- Window 宣言・Frame 数・byte budget の累積制限で、過大 Window・
  大量小 Frame・圧縮爆弾の系統的拒否が Consumer 側に実装を要さない。

## Scope

### In Scope

- `crates/ruviform-io-compression` への `zstd` feature 追加
  （`zstd = ["dep:zstd-safe"]`）。`default = []` 維持。
- Backend は `zstd-safe` 低水準 `CCtx` / `DCtx` のみ内部利用
  （`default-features = false`。C ライブラリは vendored source を `cc`
  でビルド — cmake 不要。採否判断・license・依存木・MSRV・CI lane
  影響は `crates/ruviform-io-compression/Cargo.toml` コメントと design
  doc に記録）。`zstd` 高水準 crate・`ruzstd` 純 Rust 実装との比較も
  design doc に記録する。
- `zstd::EncodeOptions { level: i32, checksum: bool, limits }` /
  `zstd::DecodeOptions { scope, limits, max_window_log,
  skippable_frames }`（`max_window_log` 既定 = zstd の decoder 既定
  上限 `WINDOWLOG_LIMIT_DEFAULT`（27）に準拠した値）。
- `SkippableFramePolicy`（`#[non_exhaustive]` enum: `Accept` 既定 /
  `Reject`）— accept は skippable frame を 8 byte header + 宣言 size
  分だけ skip し data frame として数えない。reject は `Unsupported`
  で拒否。
- Frame header（magic・FHD・Window_Descriptor・Dictionary_ID・
  Frame_Content_Size）は Ruviform 側で byte 精密に parse:
  reserved bit / DID≠0（dictionary 未対応）/ window size 超過は
  backend 到達前に `InvalidFormat` / `Unsupported` / `LimitExceeded`
  で確定的に拒否。checksum flag は `IntegritySummary` の
  checked/unchecked 分類に使用。
- 連結 Frame（WholeInput で全 data frame 復号・末尾厳密）・
  SingleUnit（先頭 frame 終端で停止・後続 byte を caller BufRead に
  残す）・skippable frame の境界走査を所有。backend
  `ZSTD_decompressStream` の auto-skip には委譲しない。
- Encoder `finish()` = `ZSTD_e_end` epilogue（+ 付与時 checksum）書込
  のみを完了証拠とし冪等。`flush()` = block flush（`ZSTD_e_flush`）+
  下流伝搬で完了ではない。
- `mise.toml` 3 task・`ci/impact/map.mjs` features・`ci.yml` コメントの
  feature matrix（default・zstd 単独・gzip+deflate+zstd 全有効）更新と、
  crate README・`crates/README.md`・crate-level doc の zstd 記述更新。
- `Cargo.lock` は cargo 再解決で更新（手編集しない）。

### Out of Scope

- 新規クレート・所有型 Encoder/Decoder・async・no_std。
- Preset dictionary / raw content dictionary / `zstdmt` multithreading /
  legacy frame format / magic-less frame / seekable format — 全て
  未対応 capability として `Unsupported` または format 拒否で pin。
- Skippable frame の書込 API（decode 側の policy のみ）。
- gzip/deflate 既存 feature・公開契約・他 crate の変更。
- AudioLens Job/Dataset・Synvelis Project/Revision の意味論
  （Consumer 側責務）、Consumer 実移行。
- CPU 時間・backend 内部 heap・出力 sink 側メモリの Hard Limit 宣言
  （window 宣言上限は byte budget とは別物として format option で
  強制する）。

## Requirements

- 公開 enum は全て `#[non_exhaustive]`（`SkippableFramePolicy` 含む）。
  Backend 由来の `zstd_safe` / `zstd_sys` の型・エラー・version が
  公開 API に一切現れないこと（`cargo doc` で確認）。
- `ZSTD_decompressStream` の `in.pos` を BufRead `consume` に写像し
  byte 精密な論理消費を維持する（`compressed_bytes_consumed`）。
  backend 高水準 adapter への先読みに委譲しない。
- Decoder の `finish()` は `VerifiedEnd` のみ `Ok`、途中は
  `NotExhausted`、Failed は `InvalidState`。未読出力を破棄して
  成功させない。`read(&mut [])` は EOF 証拠にしない。
- `max_decoded_bytes` ちょうど一致は正常終了、+1 byte は
  `LimitExceeded`（有界 probe・超過 byte は Consumer に返さない）。
  `max_compressed_bytes` / `max_units`（data frame 累積）も同様に厳密。
- `max_window_log` は frame header parse で確定的に強制し
  （`window_size > 1 << max_window_log` で `LimitExceeded`）、backend
  側にも `ZSTD_d_windowLogMax` を設定して内部確保を上限付きにする。
- Error 正規化: backend `ErrorCode` は `zstd_safe::get_error_name` で
  kind へ分類（checksum → `ChecksumMismatch`、window/memory →
  `LimitExceeded`、dictionary → `Unsupported`、その他 → `CorruptData`
  等、design doc の対応表どおり）。`impl Read`/`impl Write` では
  `io::Error` 経由でも `source()` で `CompressionError` に到達可能。
- `default = []`・`zstd` 単独・`gzip,deflate,zstd` の feature matrix で
  check/lint/test が全緑。MSRV 1.89・edition 2024・`workspace.lints`・
  `-D warnings`・`publish = false` 継承・unsafe 不使用。
- zstd-safe の build 依存（`cc` C コンパイラ）が CI ubuntu lane と
  ローカル Windows で動くこと、MSRV（rust-version 1.64）と license
  （BSD-3-Clause）が要件に合うことを確認して PR に記録。
- 同一入力・同一設定の round-trip byte 一致を保証。Backend version 間
  の byte 列完全一致は契約に含めない。

## Acceptance Criteria

- [ ] AC1（Single/Concatenated/Skippable）: 単一 frame・連結 frame
      （WholeInput で全 frame 復号・正確な frame 数）・skippable frame
      の accept（skip・unit 非計上）/ reject（`Unsupported`）を Golden/
      Negative で検証。
- [ ] AC2（Checksum 混在）: checksum あり/なし frame が混在する連結
      stream で `IntegritySummary { checked_units, unchecked_units }`
      が正確。checksum 破損 → `ChecksumMismatch`。
- [ ] AC3（Window/破損/切断/末尾）: `max_window_log` 超過 frame →
      `LimitExceeded`、corrupt body → `CorruptData`、truncated →
      `TruncatedInput`、末尾 trailing byte → `TrailingData`、
      wrong magic → `InvalidFormat`、reserved FHD bit → `InvalidFormat`、
      DID≠0 → `Unsupported` を検証。
- [ ] AC4（Budget 厳密）: `max_decoded_bytes` / `max_compressed_bytes` /
      `max_units` のちょうど一致許容・+1 超過拒否・大量小 frame の累積
      制限・EncodeLimits の input/output cap。`finish`・Error 契約は
      gzip/deflate と同一（format 固有 option との混同なし）。
- [ ] AC5（境界/観測）: `compressed_bytes_consumed` が論理消費を正確に
      計測（SingleUnit で後続 byte 未消費・caller prefetch 非混同）。
      Frame 数が receipt で正確。`Decoder::finish` が `VerifiedEnd`
      のみ成功・未読出力を捨てない。
- [ ] AC6（feature/DAG/監査）: feature matrix compile、backend 型非
      公開、MSRV 1.89、unsafe なし、全 API nonRT、ruviform-* 依存ゼロ、
      `cc` build 依存が CI で問題ないこと、license/依存木を監査・記録。

## Deferred Design Concerns

- Preset dictionary / raw content dictionary の decode・encode
  対応 | 実需確認後 | 本 Issue では DID≠0 frame を `Unsupported`、
  DID 非宣言の raw-dict frame は復号結果として `CorruptData`（形式仕様
  上の不可分性を文書化）。
- `zstd`（高水準 crate）への依存切替 | 低水準で足りない要件が出たら |
  現段階では `zstd-safe` が必要十分（byte 精密・Drop 副作用なし）。
- Skippable frame 書込・magic-less frame・seekable format | 実需確認後。
- window 宣言の厳密 byte 比較（mantissa 含む windowSize 計算）は
  本 Issue で採用 — 将来 `max_window_bytes` 系 option を別設計で追加可。

## Sources and Relations

- 正本: Issue #301 本文（作業範囲・Acceptance・完了条件/対象外）。
- 設計基準: Issue #285（IO-03）v0.1/v0.2 合意追補（Zstd Frame 固有の
  skippable・window・checksum 混在の契約）、Issue #282（親設計）、
  Issue #288（IO-06 Cross-Consumer 受入）。
- 形式仕様: RFC 8878（Zstandard Compression Format）。
- 実装基盤: Issue #300 / `docs/specs|designs|plans/test-specs/
  issue-300-*`（共有語彙・feature/lane・借用契約の先例）、
  `docs/designs/issue-333-impact-based-ci.md`（lane 規約）。
- Backend: `zstd-safe` 8.2.0（BSD-3-Clause・rust-version 1.64。
  採用理由は `crates/ruviform-io-compression/Cargo.toml` コメントと
  design doc に記録）。

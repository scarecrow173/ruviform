---
id: "IMPL-rAJqQxI07EYCwOdCi-Qyd"
type: "impl"
status: "completed"
title: "Issue #286 IO-04 ZIP/TAR Archive — 設計正本照合・契約試験棚卸し slice"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/286"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-286-io-04-archive-acceptance-recheck.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-archive/tests/zip_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/tar_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_zip.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_tar.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/extract_staging.rs"
      - type: "file"
        path: "docs/designs/issue-286-io-04-zip-tar-archive.md"
      - type: "file"
        path: "docs/designs/issue-304-io-archive-extract.md"
      - type: "file"
        path: "docs/test-specs/issue-304-io-archive-extract.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
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
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
    - "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
    - "docs/designs/issue-304-io-archive-extract.md"
    - "docs/test-specs/issue-304-io-archive-extract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
    - "docs/test-specs/issue-304-io-archive-extract.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/302"
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #286 IO-04 ZIP/TAR Archive — 設計正本照合・契約試験棚卸し slice

## Summary

Issue #286（IO-04）の設計正本と main 入り済み実装の照合
スライス。`ruviform-io-archive`（#302 ZIP / #303 TAR /
#304 Secure Extraction で merge 済み）の Public API・
所有権/終了契約・backend 隠蔽・safe extraction を Issue
設計と突合し、headless で検証可能な未充足の契約試験のみを
追加した。実装コードの変更は不要と判断した（設計不整合は
文書側の記述ずれのみ）。

## Context

先行 slice（PR #520）は spec→design→plan→task→test-spec の
文書チェーンのみを作成し、実装試験を `NotRun` として記録
していた。その後 #302/#303/#304 の実装が merge され、
`tests/{zip_reader,zip_writer,tar_reader,tar_writer,
extract_zip,extract_tar,extract_staging}.rs`（本 slice
追加前で計 139 本）と `src/extract/{path,session,fs}.rs` の
`#[cfg(test)]` unit test が契約を pin している。本スライスは
その棚卸しと残件試験の追加。

## Audit findings（設計 ↔ 実装の照合結果）

- **Public API**: `zip`/`tar`/`extract` module は
  `ArchiveError(Kind)`・`ArchiveName`・`EntryKind`・
  `ArchiveEntryInfo`・`ArchiveReadLimits`・Receipt 系・
  `SafeRelativePath`・`ExtractionSession`/`ExtractedTree`/
  `ExtractionRecoveryHandle` のみを公開。`zip`/`tar`/
  `flate2`/`crc32fast` の型・error は `Cargo.toml` 依存に
  存在するが公開 signature に現れない（backend 非露出を
  目視 + signature 確認で監査。`source()` 連鎖も
  `ArchiveError`/`io::Error` のみ）。
- **seek 差異**: `ZipArchiveReader<R: Read + Seek>`・
  `ZipArchiveWriter<W: Write + Seek>`（indexed）と
  `TarArchiveReader<R: Read>`・`TarArchiveWriter<W: Write>`
  （streaming、Seek 非要求 — `Vec<u8>` sink で実証済み）
  の型 signature が設計どおり分離。
- **所有権/終了**: Entry 排他借用・明示 `finish()` のみが
  完了証拠・`Drop` は poison/best-effort で非証拠、
  TAR `Ready`/`EntryOpen`/`Pending`/`Failed` 状態機械と
  `discard_pending`/`discard_remaining`/`finish` の drain
  検証、Writer の unfinished-entry poison — 全て試験済み。
- **Receipt**: `ArchiveReadReceipt`/`EntryReadReceipt`・
  `EntryChecksum`（Verified/Unverified/NotApplicable）・
  `EntryDisposition` は measured 値のみ記録。
- **Secure Extraction**: `SafeRelativePath` policy
  （traversal/絶対/drive/UNC/予約名/NTFS 禁止文字/
  非 UTF-8）・canonical（NFC+casefold）衝突・kind/budget
  拒否・隔離 staging root 排他作成・identity 照合 cleanup・
  foreign-swap 拒否・`RecoveryTreeState` — 実装・試験済み。
- **Compression 合成**: `tar.gz` の 2 層独立 finish を
  `tar_reader.rs` の gzip 合成試験で pin。`tar.zst` は
  同一機構（IO-03 decoder を内側 stream として被せる）で
  個別試験は未追加 — gzip で契約は pin 済み。
- **文書ずれ（修正済み）**: #304 design/test-spec の
  `changes.added` と Verification Notes が実在しない
  `tests/extract_path.rs`/`extract_secure.rs`/
  `src/extract/failure.rs` を列挙していた（実装では
  `extract_zip.rs`/`extract_tar.rs`/`common/extract.rs` に
  分割・failure 型は `session.rs` 内）。#286 design の
  snapshot も同じ旧ファイル名を転記していた。実績の
  ファイル構成に合わせて修正。

## Implementation

### Changes

追加した契約試験（全て headless・実 FS temp dir のみ）:

- `tests/zip_reader.rs`
  - `verify_all_surfaces_the_first_entry_failure`:
    verify 済み entry + CRC 破損 entry で `verify_all` が
    部分 receipt ではなく `ChecksumMismatch` を返し、
    entry 単位の verified 状態が正直に残ること（ZIP-01/
    WRT 系）。
  - `into_inner_returns_the_underlying_reader_without_
    verifying`: inner stream の返却は完了証拠でないこと
    （TAR `into_inner` 契約の ZIP 側 mirror・OWN/WRT）。
- `tests/tar_reader.rs`
  - `solaris_x_header_applies_pax_records`: typeflag `X`
    （Solaris）の extended header が `x` と同じ PAX record
    grammar で effective field を上書きすること（TAR-01）。
  - `discard_pending_enforces_the_total_budget`:
    `Pending` drain が `max_total_uncompressed_bytes` を
    途中遮断し `LimitExceeded` + `Failed` latch すること
    （OWN-04・budget の実測途中遮断）。
- `tests/extract_tar.rs`
  - `pax_path_override_extracts_under_the_effective_name`:
    PAX `path` record が ustar header 名を完全に置き換え、
    effective name で展開されること（TAR-01 × SEC-01）。
  - `pax_path_override_still_passes_path_policy`:
    hostile な `path` record（`../evil.txt`）が header 名と
    同じ `SafeRelativePath` policy で `InvalidInput` 拒否
    されること（SEC-01/02 — extension 由来名の迂回防止）。
- `tests/extract_staging.rs`
  - `recovery_handle_reports_absent_and_cleanup_succeeds`:
    記録済み tree が外部から消えた場合 `inspect` が
    `Absent` を返し、retry cleanup が no-op 成功すること
    （REC-01/02・EXT-02）。
- `tests/extract_zip.rs`
  - `nested_archive_payload_is_extracted_opaquely`:
    ZIP-in-ZIP が再帰展開されず byte 列として書き出される
    こと（nested archive 非対応の明示・SEC 系）。

## Documentation reconciliation

- `docs/designs/issue-304-io-archive-extract.md`:
  `changes.added` を実績ファイル（`extract_zip.rs`/
  `extract_tar.rs`/`extract_staging.rs`/`common/extract.rs`）
  に修正し `failure.rs`/`extract_path.rs`/`extract_secure.rs`
  の幻のパスを除去。Verification Notes を実ファイル構成
  （`path.rs` unit test + extract_* negative matrix +
  session/fs mock）に合わせて更新。
- `docs/test-specs/issue-304-io-archive-extract.md`:
  `changes.added` を同様に実績へ修正。
- `docs/designs/issue-286-io-04-zip-tar-archive.md`:
  snapshot の test ファイル列挙を実績へ修正し、本 slice の
  追加試験を記録。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

canonical 検証（Windows 11・mise 管理 toolchain・
`origin/main` = `8ff39973` merge 後）:

- `mise run fmt-check` — pass。
- `mise run check`（`cargo check --workspace --all-targets`）
  — pass。
- `mise run lint`（`cargo clippy --workspace --all-targets
  -- -D warnings`）— pass。
- `mise run test`（`cargo test --workspace`）— pass
  （197 test binary 全て `0 failed`）。
- `mise run test-rt`（`cargo test --workspace rt_`）— pass。
- `mise run test-io-msrv`（`rust@1.89.0` で io-file/
  io-archive/io-structured/io-compression/io-wav/io-e2e）
  — pass。新規 8 試験も MSRV で実行・通過。
- 補助（io-archive scoped）: `check-io-file`・
  `lint-io-file`・`test-io-file`・`check-linux`
  （x86_64-unknown-linux-gnu type-check+clippy）— 全て pass。
- 補助証拠（非 canonical）: `cargo test -p
  ruviform-io-archive` 単独実行も pass。

## Risks

- OS/FS 別 TOCTOU 実機検証（Linux/macOS 実機・unix backend
  の実 FS 試験）は `check-linux` が type-check/lint のみで
  未実施 — unix backend の実行試験は Linux CI lane 依存の
  残存リスク。
- `tar.zst` 合成の個別試験は未追加（gzip と同一機構で
  2 層独立 finish は pin 済みだが、zstd backend 固有の
  差異は未検証）。
- 両 Consumer（AudioLens/Synvelis）の独立受入は
  #288 cross-consumer gate / #294 consumer-verify の範囲で
  本 slice では NotRun。

## Follow-ups

- unix backend の実 FS 実行試験（OS/FS 別 TOCTOU 実機検証）
  — Linux CI lane で実施。
- 両 Consumer（AudioLens/Synvelis）の独立受入 —
  #288 cross-consumer gate / #294 consumer-verify の範囲。
- `tar.zst` 合成の個別試験 — gzip で契約は pin 済みだが、
  必要に応じて zstd backend 向けに追加。
- Archive→`StagedFile` publish の E2E — #306
  `ruviform-io-e2e` の範囲（本 crate の `ExtractedTree`
  は publish を行わない設計）。
- Fuzz 実走・性能/互換性実測・MSRV/OS matrix の個別実施
  — `test-io-msrv` lane と CI matrix に委譲。
- 既存の範囲外 broken relation（#243/#313/#410 系の doc
  graph エラー）— 本 slice では修正しない（別途対応）。

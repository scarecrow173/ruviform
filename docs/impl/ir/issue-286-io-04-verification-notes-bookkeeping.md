---
id: "IMPL-034VNnxPkVZ7DW5CUalpn9"
type: "impl"
status: "completed"
title: "Issue #286 IO-04 ZIP/TAR Archive — Verification Notes 1:1 監査・簿記"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-286-io-04-verification-notes-bookkeeping.md"
    modified:
      - type: "file"
        path: "docs/designs/issue-286-io-04-zip-tar-archive.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-286-io-04-archive-acceptance-recheck.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
    - "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
  verified-by: []
  references: []
  defers:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #286 IO-04 ZIP/TAR Archive — Verification Notes 1:1 監査・簿記

## Summary

`docs/designs/issue-286-io-04-zip-tar-archive.md` の
Verification Notes 未監査項目（`[ ]` 11 項目）を、設計正本の
各節本文・`crates/ruviform-io-archive` の実装・
`tests/{zip_reader,zip_writer,tar_reader,tar_writer,extract_zip,
extract_tar,extract_staging}.rs`・IO-E2E（`ruviform-io-e2e`）・
IO-03 coverage・`Cargo.toml`/crate README・#302/#303/#304 設計・
#288 matrix・Issue/PR 番号へ 1:1 で照合した。11 項目全てが
実在する証拠へ解決できたため全て `[x]` へ更新し、本 IR に
照合表を記録した。`tar_reader.rs` 固有の真の gap は確認
されず、テスト追加は行わなかった（duplicate test 禁止の
指示どおり）。外部 gate（consumer compatibility=#288/#294・
consumer Issues・OS lanes=#288・performance/fuzz）は全て
NotRun/委譲のまま残し、Issue #286 は close しない。

## Context

- Issue: [#286](https://github.com/scarecrow173/ruviform/issues/286)
  （親 [#282](https://github.com/scarecrow173/ruviform/issues/282)）
- 対象文書: `docs/designs/issue-286-io-04-zip-tar-archive.md`
  （IO-04 設計正本。docs チェーンは spec/design/test-spec/
  plan/task で PR #520・#548 経由で main 統合済み）
- 実装: `crates/ruviform-io-archive`（#302 PR #352・#303
  PR #357・#304 PR #362 で merge 済み）
- 先行する受入再照合: `issue-286-io-04-archive-acceptance-
  recheck.md`（headless 契約試験 8 本追加済み）
- 先例: Issue #285 の同型 slice
  （`issue-285-io-03-compression-contract-audit.md`、
  PR #575 で Verification Notes 全項目 `[x]` 化の前例）

## Verification Notes 照合表（項目 → 証拠 → verdict）

判定基準: 各項目が要求する節内容・参照先が設計正本に実在し、
参照する test-spec rule・#288 matrix 行・Issue/PR・設計節・
試験が全て実在すること。「実測」は本 worktree での文書照合と
canonical 検証（後述）の実走を指す。

| # | 項目（要約） | 証拠 | verdict |
| --- | --- | --- | --- |
| 1 | Reader/Writer/Entry モデル節: ZIP `Read + Seek` indexed・TAR `Read`/`Write` streaming の形式別 API 族・共通 format 中立型・「形式が実在する情報のみ記録」・#302/#303 設計参照 | 本書 L228-254「形式別 API 族」表: `ZipArchiveReader<R: Read + Seek>`/`ZipArchiveWriter<W: Write + Seek>`（indexed、L238）・`TarArchiveReader<R: Read>`/`TarArchiveWriter<W: Write>`（streaming、L239）・正本列が `docs/designs/issue-302-…`/`issue-303-…` を参照。L256-270「Format 中立の共通型」: `ArchiveName`/`EntryKind`/`ArchiveEntryInfo`（`uncompressed_size`/`compressed_size` は `Option<u64>`「形式が実在する宣言値のみ」L269、`link_name` は TAR `1`/`2` のみ `Some`）。実装一致: `src/zip/read.rs:69` `R: Read + Seek`・`src/zip/write.rs:155` `W: Write + Seek`・`src/tar/read.rs:105` `R: Read`・`src/tar/write.rs:148` `W: Write`。試験: `tests/zip_reader.rs::round_trip_stored_and_deflate_read_finish`/`verify_all_reports_exact_counts`、`tests/tar_reader.rs::round_trip_reads_all_entries_and_receipts` | Pass → `[x]` |
| 2 | Backend 隠蔽節: 外部 `zip`/`tar` 非採用・内部 Parser + `flate2` rust_backend/`crc32fast`/`unicode-normalization`/`ruviform-io-file` vfs の非公開依存・Serde 非依存・`std::io` ベース | 本書 L385-428: L388/L394「外部 `zip`/`tar` crate — 未採用」・L399-401 `flate2`（`rust_backend`）/`crc32fast`/`unicode-normalization`・L404-408 `ruviform-io-file` 内部 `vfs` 共有・L409-410「Serde 必須依存なし」「`std::io` trait/型のみ」・L417-418「`std::io::{Read, Write, Seek}` のみを要求」。`Cargo.toml` 実測一致: `crc32fast = "1"`・`flate2`（`default-features=false`+`rust_backend`）・`unicode-normalization = "0.1"`・`ruviform-io-file` path 依存・serde/`zip`/`tar` crate なし・`default = []`。crate README「Backend 隠蔽」節（L124-129）一致 | Pass → `[x]` |
| 3 | 所有権/状態機械節: Entry 排他借用・明示 `finish` のみが完了証拠・TAR `Ready`/`EntryOpen`/`Pending`/`Failed`・`discard_pending`/`discard_remaining`・Writer poison | 本書 L285-368: L287-290「排他的に `&mut` 借用」「明示 `finish(self)` のみが完了証明」・L321-330 TAR 状態機械（`Ready`/`EntryOpen`/`Pending`/`Failed`/`Finished`）・L324/326/337-339 `discard_remaining`/`discard_pending`・L353-358 Writer `Ready -> EntryOpen -> Ready -> Finished`・失敗/未 finish Drop は `Failed`/poison。試験: `tests/tar_reader.rs::dropped_entry_creates_pending_until_discard`/`discard_pending_without_pending_is_invalid`/`discard_pending_io_failure_latches_failed`/`discard_pending_enforces_the_total_budget`/`failed_reader_rejects_every_operation`/`discard_remaining_drains_and_reports_discarded`/`finish_while_pending_is_rejected`、`tests/zip_writer.rs::incomplete_entry_drop_poisons_archive`/`unfinished_entry_dropped_without_write_still_poisons`、`tests/tar_writer.rs::dropped_entry_writer_poisons_archive`/`short_payload_finish_is_size_mismatch_and_poisons` | Pass → `[x]` |
| 4 | Receipt 節: `ArchiveReadReceipt`/`EntryReadReceipt` の typed summary・`EntryChecksum`/`EntryDisposition`・`verified: bool` 非丸め・真正性 ≠ 整合性 | 本書 L370-383: L372-375 `EntryReadReceipt`（`info`/実測 `uncompressed_bytes`/`checksum: EntryChecksum`/`disposition: EntryDisposition`）・L376-378 `ArchiveReadReceipt`（`entries_total`/`entries_verified`/`entries_unverified`/`payload_bytes`）「`verified: bool` 一値に丸めない」・L379-383「ZIP CRC32 は破損検出であり真正性ではない」「TAR payload には内容 checksum が存在しない」。実装: `src/receipt.rs`・`pub use receipt::{ArchiveReadReceipt, EntryChecksum, EntryDisposition, EntryReadReceipt}`（`src/lib.rs:115`）。試験: `tests/zip_reader.rs::verify_all_reports_exact_counts`、`tests/tar_reader.rs::finish_drains_unread_entries_as_unverified` | Pass → `[x]` |
| 5 | Secure Extraction 節: 信頼モデル（Untrusted 既定）・Path Policy・canonical 衝突・kind/budget・OS/FS 別保護レベル表・TOCTOU 境界・Cleanup/Recovery | 本書 L451-585: L459-463 Untrusted 既定・緩和 Option なし・L476-501 `SafeRelativePath` Path Policy（区切り/component/Windows 禁止/長さ）・L495-501 canonical key（`char::to_lowercase`+NFC）衝突拒否（ZIP plan phase 事前全件・TAR 逐次）・L503-514 kind/budget 拒否（宣言事前+実測途中の両層）・L516-526 OS/FS 別保護レベル表（unix dirfd/`O_NOFOLLOW`・Windows `NtCreateFile`/`OBJ_DONT_REPARSE`・その他 `Unsupported`）・L536-544 TOCTOU 正直な境界・L549-572 Cleanup/Recovery（`NodeRecord` identity 照合・`ExtractionRecoveryHandle`/`RecoveryTreeState`）・L574-585 `ExtractedTree` 保証境界。試験: `tests/extract_zip.rs`（`rejects_dangerous_and_divergent_names`/`rejects_case_fold_collision`/`rejects_unicode_normalization_collision`/`rejects_symlink_and_special_entries`/budget 系）、`tests/extract_tar.rs`（`rejects_symlink_and_hardlink_entries`/`pax_path_override_still_passes_path_policy` 等）、`tests/extract_staging.rs`（`cleanup_retains_a_foreign_file_at_a_recorded_name`/`cleanup_refuses_to_delete_a_swapped_root_tree`/`a_junction_at_the_root_is_never_followed_or_deleted`/`recovery_handle_reports_absent_and_cleanup_succeeds` 等） | Pass → `[x]` |
| 6 | Format 範囲表の受理/`Unsupported` が `Cargo.toml` 実装・crate README・#302/#303/#304 設計に一致 | 本書 L435-441 範囲表 ↔ 実測一致: ZIP read（classic Stored/Deflate・非暗号化・UTF-8 flag）/write（Stored/Deflate・classic field）・`Unsupported`（暗号化 bit0・未知 method・multi-disk・ZIP64-required・ZIP64/data descriptor/comment 出力）— `tests/zip_reader.rs::encrypted_entry_is_unsupported`/`unknown_method_is_unsupported`/`multi_disk_eocd_is_unsupported`/`multi_disk_cd_entry_is_unsupported`/`zip64_eocd_is_unsupported`、#302 設計 L122-123/L156-157/L220-223/L253-254/L263-264/L328。TAR read（ustar/GNU magic/PAX `x`/`g`/`X`/GNU `L`/`K`/base-256）・`Unsupported`（sparse `S`・special `3`/`4`/`6`・未知 typeflag）— `tests/tar_reader.rs::special_file_typeflags_are_unsupported`/`sparse_and_unknown_typeflags_are_unsupported`、#303 設計 L146-151。TAR write（USTAR+必要時 PAX `x`・`add_directory`・file のみ、GNU/base-256/link/special/sparse 出力なしの非対称）— `tests/tar_writer.rs::ustar_prefix_split_used_for_splittable_long_names`/`unsplittable_long_name_goes_through_pax`/`size_beyond_octal_range_goes_through_pax`、#303 設計 L254-258/L277-280。extract（`File`/`Directory` のみ）— `tests/extract_tar.rs::rejects_symlink_and_hardlink_entries`/`special_typeflags_are_rejected_by_the_reader`、#304 設計 L309/L364-365。crate README L42-44/L69-73/L84-87 一致 | Pass → `[x]` |
| 7 | Compression 組合せ節: tar.gz/tar.zst の 2 層 finish 独立・ZIP 内部 codec の capability 閉じ込め・IO-03 への必須依存なし | 本書 L672-697: L682-689「TAR 層の `finish` は decode 済み stream 内の TAR 終端のみを証明し、圧縮 stream の検証は IO-03 decoder の `finish`（`VerifiedEnd`+`DecodeReceipt`）に分離」・L690-692「ZIP 内部 Codec は `ZipCompressionMethod` に閉じ込め、IO-03 への必須依存を作らない」。`Cargo.toml`: `ruviform-io-compression` は dev-dependencies（`gzip` feature）のみで必須依存なし。試験: `tests/tar_reader.rs::tar_gz_layers_finish_independently`/`tar_gz_bad_trailer_surfaces_through_tar_finish`、IO-E2E `tests/archive_compression_staged.rs::tar_gzip_staged_round_trip_with_layered_completion_proofs`/`tar_zstd_staged_round_trip`/`truncated_gzip_member_fails_finish_before_publish` | Pass → `[x]` |
| 8 | 終了/公開契約節: Entry→Archive→Compression→StagedFile publish の finish 連鎖分離・`ExtractedTree` ≠ Directory Publish・無条件 Atomic Directory Publish 非保証 | 本書 L699-725: L704-708「Archive の `finish()` 成功後のみ安全な公開を認める。Entry/Archive/Compression のいずれの `finish` 成功も publish を代替しない」・L709-712「`ExtractedTree` は…単一 File の `StagedFile` publish とは別物」「無条件の Atomic Directory Publish を保証しない」・L716-719 中断/破損時は正式公開拒否+Cleanup・L720-725 Consumer 固有 schema を Archive 層へ混入しない。試験: IO-E2E `tests/staged_lifecycle.rs`・`tests/archive_compression_staged.rs`（finish 連鎖の layered completion proofs） | Pass → `[x]` |
| 9 | 証拠項目対応表の API/ZIP/TAR/SEC/EXT/INT/COMP/DEP/OWN/WRT/REC/OS 各行が実在する test-spec rule/節・#288 matrix 行（A-N1〜A-X5）・Issue/PR 番号へ解決できること | 本書 L784-808 対応表の全参照を実測解決: test-spec rule — `issue-302` R1-R11 全実在（`docs/test-specs/issue-302-io-archive-zip.md` L65-99）、`issue-303` R1-R13 全実在（`issue-303-io-archive-tar.md` L70-123）、`issue-304` R1-R11 全実在（`issue-304-io-archive-extract.md` L75-123）、`issue-306` R6 実在（`issue-306-io-composition-e2e.md` L101-108「ZIP/TAR→Secure Extraction」）。#288 matrix 行 — A-N1/A-N2/A-X1〜A-X5/C-X4/E-N1/E-X1 全実在（`docs/designs/issue-288-…` L317/L323-329/L345-346）。設計節参照 — #302「Index 構築」「Reader 状態機械」「Data and Control Flow」、#303「実効値の解決順位」「Reader 状態機械」「TAR 構造と解析」、#304「Cleanup（verified 降下削除）」全実在。Issue/PR — #295/#296/#300/#301/#302/#303/#304/#305/#306/#288/#219/#294 全実在（gh 照合）、PR #352/#357/#362 MERGED、AL #139/#143 CLOSED、SV #82/#25 CLOSED・#84 OPEN、SV #17 OPEN（PAP-14 は Issue #286 本文 L43/L227 が直接参照）。`check-linux`/`test-io-file`/`test-io-e2e` mise task・CI ubuntu+windows lane・`consumer-verify.yml` 実在 | Pass → `[x]` |
| 10 | Consumer 利用面節: AL #139/#143・SV #82/#84/#25・SV Project #17 の Issue 番号と責務分界（schema/version は Consumer 所有・`ExtractedTree` ≠ Project Transaction） | 本書 L727-774: L734「AL-IO-01 #139 — CLOSED・AL-IO-02 #143 — CLOSED」・L750「SYN-IO-01 #82 — CLOSED・SYN-IO-02 #84 — OPEN・#25 — CLOSED」・L755「SV Project #17 PAP-14 が親要求」— gh 実測で全 Issue 実在・状態一致（AL #139/#143 CLOSED、SV #82/#25 CLOSED、SV #84/#17 OPEN）。責務分界: L730-732「schema/version/migration/canonical signed bytes/manifest semantics は各 Consumer が所有」・L758-762「`ExtractedTree` は Directory Publish ではないため、SV の正式 commit/CAS/HEAD 更新は Consumer 側の別契約」・L720-725（終了/公開契約節）「SV の Project Snapshot/HEAD/Revision/Recovery…を Archive 層へ混入しない」 | Pass → `[x]` |
| 11 | 「設計合意と実装/検証の分離」節が存在し、未実施項目を `not run`/`NotRun`/`Unsupported` と記録する方針を明記 | 本書 L850-871「Design Done / Implementation Done の分離」節が存在。L856-858「本書の存在は『設計が文書化された』ことのみを意味し、Consumer 実測・OS 別実機・性能比較の完了を意味しない」・L862-863「未実施は `not run`/`NotRun`/`Unsupported` と記録する」・L864-871 受入条件の各項が #302/#303/#304 検証済み範囲への索引と cross-consumer gate 段階の要件に分離されている | Pass → `[x]` |

## Implementation

### Changes

- `docs/designs/issue-286-io-04-zip-tar-archive.md`:
  Verification Notes の `[ ]` 11 項目を全て `[x]` へ更新
  （項目文・他節は無変更。#285 前例と同じく証拠は本 IR の
  照合表へ集約）。
- `docs/impl/ir/issue-286-io-04-verification-notes-bookkeeping.md`:
  本 IR（照合表・検証・NotRun 記録）。
- `crates/ruviform-io-archive/tests/tar_reader.rs`: 変更なし
  — 全項目が既存試験で充足され、固有 gap は確認されなかっ
  たため（duplicate test 追加禁止の指示どおり）。

### Decisions During Implementation

- 判定は「項目が要求する内容が節に実在し、参照先が全て解決
  できること」を基準とし、#285 の同型 slice（PR #575）と
  同じく `[x]` 化 + 証拠の IR 集約を採用した。
- 項目 10 の「`ExtractedTree` ≠ Project Transaction」は
  文言の逐語一致ではなく責務分界の実体（`ExtractedTree` は
  Directory Publish でなく、SV の commit/CAS/HEAD は Consumer
  側契約）で判定した — 対応表・Consumer 節・Deferred 節の
  3 箇所が同一境界を一貫して記述している。
- 項目 9 の `issue-306` E-N1/E-X1 表記は #288 matrix の行 ID
  （E-N1/E-X1 が `test-spec issue-306` を根拠として参照）への
  解決として確認した。
- 外部 gate は全て NotRun/委譲のまま: consumer compatibility
  （#288・#219/#294 consumer-verify・AL #139/#143・SV
  #82/#84/#25 実 repo 実測）、OS lanes（#288・CI 既存
  ubuntu/windows/macos lane・`check-linux` は unix backend の
  型/lint 補助のみ）、performance/fuzz（#288 該当 gate・
  CI lane）。本 slice は headless Windows での文書照合と
  canonical 検証のみ。

## Related Experiments

### Adopted

なし — 本 slice は既知手順（#285 同型 slice の前例）に従う
機械的な監査・簿記であり、探索的実験は行わなかった。

### Rejected

なし。

## Validation

canonical 検証（`mise.toml` タスク経由・実走結果は PR 本文と
Issue コメントに記録）:

| コマンド | OS | toolchain | 結果 |
| --- | --- | --- | --- |
| `mise run fmt-check` | Windows 11 x64 | cargo 1.99.0 | exit 0 |
| `mise run check` | 同上 | 同上 | exit 0 |
| `mise run lint` | 同上 | 同上 | exit 0（`-D warnings`・10m04s） |
| `mise run test` | 同上 | 同上 | exit 0（全 workspace test + doctest。初回実行は `ruviform-plugin-export` `l1_video_runner_e2e` が子 process frame 待ち timeout で 3 件 failed — 並行 worker の rustc 高負荷下の flake で、単独再実行 20/20 pass・workspace 再実行で全 pass を確認。本 slice は docs のみで無関係） |
| `mise run build` | 同上 | 同上 | exit 0（12m49s） |

補助照合（canonical = mise）: `gh` による Issue/PR 実在・
状態照合（#286/#282/#295/#296/#300-#306/#288/#219/#294・
PR #352/#357/#362・audio-lens #139/#143・synvelis
#82/#84/#25/#17）、test-spec rule・#288 matrix 行・設計節・
mise task・CI workflow の grep 実測。

## Risks

- 本 slice は文書照合であり、IO-04 の OS/FS 実機挙動・
  Consumer 実測・性能/fuzz を新たに検証したわけではない。
  それらは #288・#219/#294・各 Consumer Issue が Owner の
  まま残る。
- Verification Notes の `[x]` は「節内容と参照の実在性」の
  監査結果であり、各契約の将来の実装変更に対する再監査を
  意味しない（設計 drift は Risks 節 #1 の mitigation どおり
  test-spec pin に依存）。

## Follow-ups

- Issue #286 の close 判断は Coordinator/Owner — 外部 gate
  （#288・#219/#294・Consumer 実測・OS lane・性能/fuzz）が
  残るため本 slice では close しない。
- `docs/impl/ir/README.md` index は worker spec の編集範囲外
  のため未更新（`doc_maintenance`/doc-status 運用で追随）。

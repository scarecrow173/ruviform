---
id: "DESIGN-ODmc6p5Cp7Kbq0u35OrAiR"
type: "design"
status: "approved"
title: "Issue #306 汎用 I/O 合成 E2E — 試験専用クレート構成・境界試験設計"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/staged_lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/structured_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/archive_compression_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/extraction.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_staged.rs"
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
        path: "AGENTS.md"
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
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-304-io-archive-extract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-composition-e2e.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 汎用 I/O 合成 E2E — 試験専用クレート構成・境界試験設計

## Overview

`crates/ruviform-io-e2e` は公開 API を持たない試験専用クレート。
`src/lib.rs` は crate doc のみとし、全試験を `tests/` 配下の
integration test として置く。Consumer と同じ公開面だけを使い、
IO-01〜05 の層間境界契約を実 OS の temp dir 上で pin する。

### クレート新設の判断

5 つの合成境界を単一の既存クレート `tests/` に置く案を検討したが、
`TAR→compression→StagedFile`・`ZIP/TAR→extract`・`WAV→StagedFile`
のいずれも単一クレートの責務に収まらず、io-wav や io-archive へ
意味論的に逆方向の dev-dependencies を置く設計は DAG の読みにくさ
を招く。`ruviform-ui-test`（Issue #312、試験専用 opt-in クレート）
の先例に倣い、葉側 consumer クレートとして新設する。
production クレートは本クレートに依存しない。

### 依存構成（dev-dependencies）

| 依存 | feature | 用途 |
| --- | --- | --- |
| `ruviform-io-file` | — | StagedFile/SealedFile/Publish 契約 |
| `ruviform-io-structured` | `json,csv,file` | write_file/read_file 境界 |
| `ruviform-io-compression` | `gzip,zstd` | Encoder/Decoder 合成 |
| `ruviform-io-archive` | — | ZIP/TAR writer/reader/extract |
| `ruviform-io-wav` | — | WavMuxer/WavEncoder |
| `ruviform-io-symphonia` | — | 公開前/後の独立 decode oracle |
| `ruviform-media-io` | — | StreamInfo/Frame/Packet/Demuxer/Decoder |
| `ruviform-media-core` | — | MediaTime/MediaTimestamp |
| `ruviform-audio-core` | — | AudioBuffer/ChannelLayout/SampleFormat |
| `serde` | `derive` | 構造化文書の試験型 |

feature unification は leaf crate の dev-deps のみで完結し、
production クレートの feature 集合を変更しない。

## Test File Map

| file | 境界 | 正常系 | 否定系 |
| --- | --- | --- | --- |
| `tests/staged_lifecycle.rs` | StagedFile→SealedFile→publish→再Open | publish receipt（durability evidence）・readback bytes・再 Open 照合 | CreateNew 競合→`NotPublished`+`DestinationExists`・Replace Owner 差替え・abort/drop の temp cleanup |
| `tests/structured_staged.rs` | Structured→StagedFile | json/csv `write_file`→`read_file` round-trip | Encode 失敗→destination 不 publish+temp なし・Publish 失敗→`Publish` variant が `PublishFailure` を保持・strict EOF（末尾混入→`TrailingData`） |
| `tests/archive_compression_staged.rs` | TAR→Gzip/Zstd→StagedFile | tar→enc→&mut StagedFile→seal→readback→dec→tar read→publish→再 Open 復元 | 切り詰め gzip→`finish` 非 VerifiedEnd→abort→destination 不在・未 finish entry writer drop→archive poisoned→finish 失敗→不 publish |
| `tests/extraction.rs` | ZIP/TAR→Secure Extraction | publish 済み zip→`verify_all`→`extract_zip`→`ExtractedTree` 内容照合・tar.gz→`extract_tar` | `../` entry→`InvalidInput`+staging 残存なし・casefold 衝突→`InvalidInput`・budget 超過→`LimitExceeded` |
| `tests/wav_staged.rs` | WAV→Finalize→Seal→Readback→Publish→Reopen | `WavMuxer<&mut StagedFile>`→`finalize` receipt→`into_sink` 借用解除→`seal`→`open_readback`→symphonia decode PCM 照合→publish→公開 file 再 decode | truncated payload→独立 decode が full payload に到達しない→abort→destination 不在・二重 `finalize` 拒否 |

## 合成契約の要点（unit 試験と重複しない観点）

- **完成証明の直列化**: 各層の `finish`/`finalize` は独立証明。
  TAR finish ≠ compression finish ≠ publish。未完了層がある bytes
  を seal/publish しない呼び出し順序を試験が実演し、未完了のまま
  進んだ場合に下層検証（decoder `finish`・readback decode）が
  失敗して publish が抑止されることを示す。
- **検証は publish 前に行う**: `SealedFile::open_readback` が提供
  する sealed temp の bytes を format 層で独立検証してから
  `publish` する。検証失敗時は `SealedFile::abort`（または drop）
  で temp を回収し destination を作らない。
- **失敗分類の層越え保存**: `DocumentFileWriteError::Publish` が
  `PublishFailure`（effect/phase/recovery handle）を失わず保持
  することを実 OS の CreateNew 競合で示す。`ExtractionFailure`
  が `ArchiveError` + optional recovery handle を返すこと。
- **Owner 差替えの atomic 契約**: staging 後に外部が destination
  を作成した場合、`CreateNew` は `NotPublished`（先入主 bytes
  保全）、`Replace` は staged bytes で置き換わることを実 OS
  primitive で確認する。
- **Adapter 系 fault（Applied/NotApplied/Unknown・DirSync 後失敗・
  Indeterminate・Temp 残留）**: `#[cfg(test)]` MockFileOps 前提の
  ユニット試験が既存正本（`ruviform-io-file/src/staged.rs` 内
  `publish_not_applied_destination_exists`・
  `dir_sync_failure_reports_published_not_ok`・
  `publish_indeterminate_*` 等）。実 OS で deterministic に誘発
  できない分類を合成層で偽装しない — 本クレートは実 OS で
  再現可能な fault class のみを扱い、mock 正本への参照を
  test-spec に記録する。

## OS/FS と主張境界

- 実施環境は Windows NTFS（開発機）。`NotRun` として扱うもの:
  macOS APFS/Linux ext4/XFS・Network/Cloud FS・性能測定・無制限
  fuzz・`Indeterminate`/`DirSync` 失敗の実 OS 再現。CI の既存
  lane（ci.yml の verify）が他 OS で同一試験を走らせる場合はその
  結果を別途参照する。
- Readback の保証は `SealedFile::open_readback` が返す
  `Read + Seek` reader に限定 — writable raw File は公開されない。
- `ExtractedTree` は不変 snapshot/Directory Publish ではない
  （io-archive #304 の既存契約）。Consumer 側の正式 commit/CAS/
  directory replace 意味論は試験対象外。

## RF #305 候補の scratch 検証

- scratch worktree（`git worktree add` で TEMP 配下、他者
  worktree/main を変更しない）に PR #391 head `82256cf…` を
  checkout し:
  1. `grep -rn "StagedOutput\|PublishOptions" crates/ruviform-media-io/src/` → 公開 API 不在の証跡。
  2. `cargo test -p ruviform-io-wav`（`mise exec --` 経由）で候補
     自身の staged 試験を実走。
  3. 本クレートの `wav_staged.rs` 相当の合成を候補でも再演し、
     旧 API 撤去後も `WavMuxer<&mut StagedFile>` + seal + readback
     + publish が同一公開面で成立することを確認（scratch 側には
     本クレートを members へ追加する uncommitted 変更のみ。
     成果物は main 側の worktree に commit しない）。
- SHA・command・exit code・結果を `evidence/rf306/` 配下の
  report に記録する。

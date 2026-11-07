---
id: "SPEC-lYVG6Gr07aTIBC8BSS0WKW"
type: "spec"
status: "approved"
title: "Issue #306 汎用 I/O 合成 E2E — StagedFile/Structured/Compression/Archive/WAV 境界試験 (IO-IMP-12)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/282"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/specs/issue-296-io-file-os-backend.md"
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
    - "docs/specs/issue-301-io-compression-zstd.md"
    - "docs/specs/issue-302-io-archive-zip.md"
    - "docs/specs/issue-303-io-archive-tar.md"
    - "docs/specs/issue-304-io-archive-extract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 汎用 I/O 合成 E2E — StagedFile/Structured/Compression/Archive/WAV 境界試験 (IO-IMP-12)

## What Should Be Built

IO-01〜05（Issue #283〜#287 → 実装 Issue #295〜#304）で main に
実装済みの公開 API を、**Consumer と同じ公開面だけ**で層横断に
合成する試験専用クレート `crates/ruviform-io-e2e` を新設する。
個別クレートのユニット/形式試験の鏡写しではなく、「層間の
所有権・状態・境界契約」— 完成証明（`finish`/`finalize`）→
`seal` → 公開前 Readback 検証 → `publish` → 再 Open — を重点的に
pin する。

対象の合成境界（Issue #306 作業範囲・Issue #288 受入表に対応）:

1. **Structured → StagedFile**: `json::write_file` /
   `csv::write_file`（`feature = "file"`）による
   serialize→seal→publish→`read_file` 再 Open。Encode 失敗時は
   destination が publish されず temp が残らないこと、Publish
   失敗時は `DocumentFileWriteError::Publish` が
   `PublishFailure`（effect/phase/recovery）を失わず保持すること。
2. **TAR → Gzip/Zstd → StagedFile**: `TarArchiveWriter` →
   compression `Encoder` → `&mut StagedFile` の Write 層合成。
   各層の `finish` が独立の完成証明であり、Encoder の未 finish・
   切り詰め bytes が `Decoder::finish` の VerifiedEnd 失敗として
   検出され、検証未完の bytes を publish しないこと。
3. **ZIP/TAR → Secure Extraction**: publish 済み archive を再
   Open し、`ZipArchiveReader::verify_all`/`TarArchiveReader` →
   `ExtractionSession::extract_zip`/`extract_tar` →
   `ExtractedTree` の実 FS 検証。Path Traversal・大小文字衝突・
   budget 超過の否定経路で Staging が残存しないこと。
4. **WAV → Finalize → Seal → Readback → Publish → Reopen**:
   `WavMuxer<&mut StagedFile>`（`MediaSink` blanket impl）→
   `finalize()` の `WavReceipt` → `into_sink` で借用解除 →
   `seal` → `open_readback` → `ruviform-io-symphonia` 独立
   decode で PCM 照合 → `publish` → 公開 file の再 decode。
   `finalize` 未実施（`into_sink`/`Drop` の best-effort のみ）を
   完成証拠にしないこと。
5. **File 合成/実 OS Fault**: `CreateNew` 競合 →
   `NotPublished`+`DestinationExists`（先入主 bytes の保全）、
   `Replace` による Owner 差替え、abort/drop の temp cleanup、
   `PublishReceipt` の durability evidence。Applied/NotApplied/
   Unknown・DirSync 後失敗・Indeterminate・Temp 残留の
   adapter 系 fault matrix は `ruviform-io-file` の
   `#[cfg(test)]` MockFileOps ユニット試験が既存の正本であり、
   本クレートでは**実 OS で再現可能な fault class のみ**を合成
   境界で検証する（重複実装しない）。

## Why Now

IO-IMP-01〜10 が main に揃い、IO-IMP-11（Issue #305・旧 Media
`StagedOutput`/`PublishOptions` 撤去候補 PR #391）の採否判定に
先立ち、#306 の合成 Gate が必要とする「Ruviform 内統合 Fixture
と試験」が不在。`WavMuxer<&mut StagedFile>` は `MediaSink` の
blanket impl により main でも候補でも同一の公開面で成立するため、
候補 merge を開始条件にせず main で先行して契約を pin できる。
Consumer 側（AudioLens #139/#143・Synvelis #82/#84）の受入接続は
既存 `consumer-verify.yml` の `*_extra_tasks` 入力が接続点であり、
本クレートが提供する `mise run test-io-e2e` はそのままでは
Consumer repo で実行されないが、Ruviform 側 Gate の正本 command
として PR/本チェーンに記録する。

## Users and Value

- RF #294/#219: 既存 CI/Workflow が参照する Ruviform 側合成 Gate
  の実行 command と結果分類（pass/fail/not_run/unsupported）の
  根拠を提供する。
- Consumer: AudioLens/Synvelis が同一の公開 API で実装する
  保存・展開経路と同じ層構成が、Ruviform 側で実 OS を通して
  検証済みであること。
- 後続実装: IO-IMP-11 候補 revision でも同一試験 suite がそのまま
  実行できる（旧 API 非依存）。

## Scope

### In Scope

- `crates/ruviform-io-e2e` 新設（公開 API なし・`src/lib.rs` は
  crate doc のみ。`publish.workspace = true` 継承で現行
  `publish = false`）。
- `tests/common/mod.rs`（既存 `ruviform-io-archive/tests/common`
  と同型の RAII `TestDir`・補助関数 — tempfile 依存は追加しない）。
- 上記 5 境界の合成試験ファイル 5 本。
- workspace `members`/`crates/README.md`/`Cargo.lock`/`AGENTS.md`
  クレート一覧の最小更新、`mise.toml` の `test-io-e2e` task 追加。
- RF #305 候補（PR #391 head `82256cf…`）の scratch checkout での
  補助検証: 旧 API 不存在の grep 証跡 + 候補の io-wav dev-dep
  `ruviform-io-file` を使う `WavMuxer<&mut StagedFile>` 試験実走。
  実施した SHA・command・結果を evidence として記録する。

### Out of Scope

- 公開 API・feature・backend の変更、旧 Media API の撤去
  （RF #305 担当）、Consumer repo への編集。
- 新しい CI Workflow・証拠 YAML/JSON スキーマ・集約 DB・
  Dashboard（Issue #288/#294 の既存機構を使う）。
- `#[cfg(test)]` MockFileOps の公開・fault-injection API の新設
  （mock 系 fault matrix は io-file ユニット試験の既存正本を
  参照し、合成層では実 OS fault のみを扱う）。
- 無制限 fuzz・全 OS/全 FS 組合せ・Network/Cloud FS・性能
  benchmark（未実測を Pass にしない。実施した OS/条件だけを
  記録する）。
- macOS/Linux 実 OS 実行（本環境は Windows のみ。CI の既存
  lane 結果に委ね、`NotRun` を記録する）。

## Acceptance Criteria

- [ ] `cargo test -p ruviform-io-e2e` が 5 境界の合成試験を含み
      全 pass（実施 OS = Windows NTFS の実測結果を記録）。
- [ ] 完成証明なし（未 finish/未 finalize/切り詰め）の bytes が
      publish されないことを、destination 不存在で示す否定経路が
      存在する。
- [ ] `DocumentFileWriteError::Publish` が `PublishFailure` の
      effect/phase を保持することを CreateNew 競合で実 OS 検証。
- [ ] Path Traversal・canonical/casefold 衝突・budget 超過の否定
      fixture で staging が残存しない（`ExtractionFailure` 経路）。
- [ ] `mise run test-io-e2e`・`mise run lint`・`mise run fmt-check`
      が pass し、`cargo test --workspace` に試験が含まれる。
- [ ] RF #305 候補 scratch での検証結果（SHA/command/結果）を
      evidence として記録。未実施項目は NotRun/Blocked と理由を
      明記する。
- [ ] OS/FS 別 Supported/Unsupported は実測範囲だけを主張し、
      macOS/Linux 実 OS・Network/Cloud FS・性能は未実測として
      記録する。

## Non-Functional Requirements

- **非 RT 維持**: 本クレートは全試験が blocking FS/Parser/
  Compression I/O を行う。RT 層への依存・RT 試験への混入は
  しない（既存の `rt_*` 命名規則に入れない）。
- **依存方向**: 葉側 consumer として io-file/-structured/-
  compression/-archive/-wav/-symphonia/media-io/domain crates を
  dev-dependencies で参照する。production クレートから本クレートへ
  依存させない（`ruviform-ui-test` の opt-in 専用クレート先例と
  同型）。
- **Backend 隠蔽維持**: 試験は公開 API のみを使い、backend 型・
  内部 mock に触れない。

## Open Questions

- DirSync 後失敗・`Indeterminate`・Temp 残留の adapter 系分類は
  実 OS で deterministic に再現できないため、io-file mock ユニット
  試験（`staged.rs` 内 `dir_sync_failure_reports_published_not_ok`
  等）を既存の正本証拠として test-spec で参照する。公開 fault
  API の追加は行わない。

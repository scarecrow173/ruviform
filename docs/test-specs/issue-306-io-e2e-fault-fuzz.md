---
id: "TSPEC-034UtBs1z6PAEjR3T4EiV2"
type: "test-spec"
status: "approved"
title: "Issue #306 IO E2E slice 2 テスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/structured_formats_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/fault_boundary.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/negative_fuzz.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/os_fs_probe.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/rooted_boundary.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/mod.rs"
      - type: "file"
        path: "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
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
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  refines:
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E slice 2 テスト仕様

## Purpose

第 1 slice（`docs/test-specs/issue-306-io-composition-e2e.md`）が
pin しなかった Issue #306 受入条件の残り — Runtime Fault
Injection の実 OS 可能 class・Negative/Fuzz・リソース上限・
OS/FS 実測記録 — を `ruviform-io-e2e` の fixture として固定する。
本 spec は fixture が保証すべき契約を定め、実装方法は定めない。

## Feature

StagedFile/Structured/Archive/Extraction/RootedDir の公開 API を
層横断で合成した fixture が、(a) 実 OS で誘発可能な publish fault
を層越えの分類・回復所有権ごと正しく伝播し、(b) 敵対的・破損した
archive/document 入力を typed error で拒否し未検証 bytes を
publish せず、(c) OS/FS capability を実測値として記録すること。

## Rules

- F1（Structured 全 format 境界）: `yaml::write_file`/`toml::
  write_file` が `StagedFile` 経由で publish され `read_file` で
  等値復元される。`xml::read_file` が publish 済み XML bytes を
  event 列として parse する。
- F2（reopen 境界の tamper 検出）: publish 済み JSON 文書の末尾に
  外部が byte を追加した場合、`json::read_file` は
  `DocumentFileReadError::Decode`（`DocumentErrorKind::TrailingData`）
  で拒否する。torn/truncated publish 済み文書も `Decode` で拒否
  される（形式ごとに期待 kind を assert）。
- F3（fault 伝播の保持）: `write_file` の `Create` 失敗
  （missing parent）は `DocumentFileWriteError::Create`、
  `Publish` 失敗（directory destination・CreateNew 競合）は
  `DocumentFileWriteError::Publish` で返り、内側 `PublishFailure`
  の `effect`/`phase`/`kind` と `take_recovery_handle` が層を
  越えて失われない。cleanup 後 temp は残存しない。
- F4（合成 publish 競合）: TAR→gzip→`&mut StagedFile` で作った
  合成 bytes を seal した後に外部が destination を作成した場合、
  `publish(CreateNew)` は `NotPublished`+`DestinationExists`、
  先入主 bytes は保全され、retained temp は recovery handle で
  回収できる。
- F5（archive 敵対 entry）: ZIP central directory の
  `external_attrs` を unix symlink mode に patch した entry は
  `EntryKind::SymbolicLink` として分類され、`extract_zip` が
  `ArchiveErrorKind::InvalidInput` で拒否し staging root を残さ
  ない。TAR symlink/hardlink/special・traversal・casefold・budget
  は第 1 slice R6 が既存正本（重複追加しない）。
- F6（resource budget）: 高圧縮率 entry（ZIP bomb 相当）に対し
  `ArchiveReadLimits` の `max_entry_uncompressed_bytes`/
  `max_total_uncompressed_bytes`/`max_entries`/`max_path_depth` が
  `LimitExceeded`（または reader 層の対応分類）で拘束し、宣言値・
  実測値のどちらが先に超過しても出力は budget 内に留まる。
- F7（document 敵対入力）: `<!DOCTYPE>`（内部 subset・外部
  identifier）・未定義 general entity を含む XML は `xml::read_file`
  が `DocumentFileReadError::Decode`（`Unsupported`）で拒否する。
  external entity の fetch・DTD 展開は行われない。
- F8（破損 fixture の typed error）: 破損 ZIP（bad magic・
  truncated central directory・payload CRC 不一致）・破損 TAR
  （header checksum 不一致・truncated）・破損 GZIP（bad magic・
  途中破損 deflate）・破損 JSON（invalid UTF-8）が panic せず
  `ArchiveErrorKind`/`DocumentErrorKind`/io error の typed error
  で失敗する。
- F9（mutation sweep の publish gate）: 有効 tar.gz composite に
  対する決定的 mutation（切詰め・構造領域 byte 反転）の全件で、
  staged bytes の独立 verify（gzip `Decoder::finish` + TAR
  reader `finish`）が失敗し、呼び出し側は `abort` で destination
  を作らない。panic は test failure として検出される。
- F10（directory 境界）: `RootedDir` が staged publish 済み
  tree と `ExtractionSession` 産出 tree の file を `NoFollow` で
  読み、`read_dir` で列挙し、`RelativePath` の `..`/絶対形を
  `InvalidInput` で拒否する。tree 内に植えた link
  （Windows junction / unix symlink）が `WithinRoot` 解決で
  root 外を指す場合 `EscapesRoot` で拒否される。link を作成
  できない環境では当該 assert を skip し `eprintln!` で記録する
  （試験したつもりを作らない）。
- F11（OS/FS 実測）: `os_fs_probe` が publish durability ladder
  を実 FS で実行し、測定した `DurabilityEvidence` のみを主張
  する。実施環境は Windows NTFS のみを本 slice の実測として
  記録し、macOS/Linux・Network/Cloud FS は `NotRun`/
  `Unsupported` と区別する。
- F12（誘発不能 fault の正直な扱い）: `Indeterminate`・DirSync
  後失敗・`TempState::Unknown` は本 fixture で偽装しない。
  `ruviform-io-file` の `#[cfg(test)]` mock 試験
  （`src/staged.rs`）を正本として参照記録する。

## Examples

- `yaml::write_file(dest, &doc, &YamlWriteOptions::default(),
  PublishOptions::new(CreateNew, FileSync))` → `yaml::read_file`
  で等値。toml 同型。
- `StagedFile` で `<root><a>v</a></root>` を publish →
  `xml::read_file` が `XmlEvent::Start`/`Text`/`End` を返す。
  `<!DOCTYPE x [<!ENTITY a "b">]>` 入り文書を publish →
  `xml::read_file` が `Decode(Unsupported)`。
- `json::write_file` で publish 後に `OpenOptions::append` で
  `x` を追加 → `json::read_file` が `Decode(TrailingData)`。
- `json::write_file(missing_dir/doc.json, …, CreateNew)` →
  `Err(DocumentFileWriteError::Create(_))` で `FileErrorKind::Io`。
- `json::write_file(dir_path, …, Replace)`（destination が dir）
  → `Err(DocumentFileWriteError::Publish(f))` で
  `f.effect()==NotPublished`、`take_recovery_handle` で cleanup。
- writer 産出 ZIP の CD `external_attrs` を `0o120777 << 16` に
  patch → publish → 再 Open → `extract_zip` が `InvalidInput`、
  `failure.staging_root` 不在。
- 1MiB の 0x00 payload を Deflate で ZIP 化（compressed は数
  KiB）→ `max_entry_uncompressed_bytes` を小さくした reader で
  `LimitExceeded`。
- tar.gz composite を seal → verify 合格 → publish。mutant
  （末尾 8 byte 欠落・gzip header 反転・tar header name 反転）
  各件で verify 失敗 → abort → destination 不在。

## Guarantees

- 全否定系で「拒否の typed 分類」と「staging/destination/temp の
  非残存（または recovery handle による明示回収）」を同時に
  assert する。
- 実 OS で再現不能な分類は assert せず、文書上 `NotRun` として
  区別する（Pass に見せない）。
- link 依存 assert は作成失敗時に skip し、skip を出力に残す。

## Non-goals

- 単体試験の重複: format parser の全 error kind・compression
  limit・io-file mock fault matrix・io-archive extract_staging
  の失敗注入系は各クレートの正本試験に委ね、本 spec は層間
  契約のみを対象とする。
- coverage-guided fuzzing・長時間 soak・性能計測。
- Consumer repo での実行・cross-repo gate 自体の実装。

## Risk

- fixture が公開 API の変更に追随できず腐る → 変更は leaf
  dev-deps のみで production API に触れないため影響は test の
  み。API 変更時は test の fail として検出される。
- OS/FS 差で link 作成・dirsync が失敗 → skip/失敗を正直に記録
  し、Unsupported を Pass にしない。

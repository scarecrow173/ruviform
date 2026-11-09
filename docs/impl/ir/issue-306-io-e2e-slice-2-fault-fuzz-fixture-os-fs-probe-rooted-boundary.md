---
id: "IMPL-034UudZrMUImnIREPB6lU1"
type: "impl"
status: "completed"
title: "Issue #306 IO 合成 E2E slice-2 — Fault/Fuzz fixture・OS/FS probe・Rooted boundary"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
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
        path: "docs/specs/issue-306-io-e2e-fault-fuzz.md"
      - type: "file"
        path: "docs/designs/issue-306-io-e2e-fault-fuzz.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
      - type: "file"
        path: "docs/plans/issue-306-io-e2e-fault-fuzz.md"
      - type: "file"
        path: "docs/tasks/issue-306-f1-fixture-extension.md"
      - type: "file"
        path: "docs/tasks/issue-306-f2-verification-handoff.md"
      - type: "file"
        path: "evidence/rf306/fault-fuzz-report.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-306-f1-fixture-extension.md"
    - "docs/tasks/issue-306-f2-verification-handoff.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-306-io-composition-e2e.md"
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
    - "docs/impl/ir/issue-296-ruviform-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
    - "evidence/rf306/fault-fuzz-report.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  verified-by: []
  references: []
  defers:
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected:
      - "TAR terminator mutant: writer.finish は 1024 bytes (2 zero block) を出力するが reader は単一 zero block を終端受理するため 512 bytes 欠落は mutant にならず、1024 bytes 除去に修正"
      - "truncated TAR payload read: TarEntryReader は typed ArchiveError を io::Error に包んで返すため、helper の一律 Io 分類をやめ entry.finish() で真の kind を再取得"
      - "Windows delete-pending: open handle 保持中の directory は try_exists に残るため、RootedDir/file handle を scope 解放してから cleanup 非存在 assert"
---

# Issue #306 IO 合成 E2E slice-2 — Fault/Fuzz fixture・OS/FS probe・Rooted boundary

## Summary

`ruviform-io-e2e` に 29 件の合成試験を追加し、計 60 件とした。
Structured 追加境界（YAML/TOML/XML・publish 後 tamper）、公開
API のみで誘発可能な fault 境界（Create/Encode/Publish 分類・
recovery handle 所有権・publish 競合・TAR+gzip publish 前独立
verify）、negative/fuzz fixture（ZIP symlink/special・ZIP bomb・
XML DOCTYPE/entity・corrupt/truncated archive・決定的 mutation
sweep）、実 FS durability probe、`RootedDir` 境界を検証した。
本番コード・公開 API・`#[cfg(test)]` mock は無変更。

## Context

- 上流: [spec](../../specs/issue-306-io-e2e-fault-fuzz.md)・
  [design](../../designs/issue-306-io-e2e-fault-fuzz.md)・
  [plan](../../plans/issue-306-io-e2e-fault-fuzz.md)・
  [test-spec](../../test-specs/issue-306-io-e2e-fault-fuzz.md)
- 前段 slice: [IR](issue-306-io-composition-e2e.md)（31 件の
  合成試験）。本 slice は残る fault/fuzz・OS/FS・rooted 境界を
  担う。
- 証拠: [evidence/rf306/fault-fuzz-report.md](../../../evidence/rf306/fault-fuzz-report.md)

## Implementation

### Changes

- `Cargo.toml`: `ruviform-io-structured` dev-dependency features
  に `yaml`・`toml`・`xml` を追加（既存 `json`・`csv`・`file` を維持）。
- `tests/common/mod.rs`: `dir_link`/`file_link`（link 生成失敗は
  呼出側で skip 判定）、`crafted_tar_entry`（byte-exact ustar
  header で traversal/symlink/special/corrupt fixture を生成 —
  公開 writer はこれらを出力できない）、ZIP central-directory
  `external_attrs` patch helper を追加。
- `tests/structured_formats_staged.rs`: YAML/TOML `write_file→
  publish→read_file::<T>` 往復、published XML bytes の event
  parse、published JSON への byte 追加で
  `DocumentFileReadError::Decode` + `TrailingData`、truncate で
  型付き decode 拒否。
- `tests/fault_boundary.rs`: missing-parent create →
  `DocumentFileWriteError::Create`（`FileErrorKind::Io`）・
  dir 競合 → `Publish`、`PublishFailure` の effect/phase/kind/
  durability・`take_recovery_handle` 所有権、CreateNew 競合で
  先勝ち bytes 保全・temp cleanup、TAR→gzip を publish 前に
  reader 層で独立 verify する composite gate。
- `tests/negative_fuzz.rs`: ZIP `external_attrs` patch で
  `EntryKind::SymbolicLink`/special を誘発し extraction 拒否・
  staging 非残存、ZIP bomb（`max_entries` は `ZipArchiveReader::
  new` で bind、declared/streamed total budget）、XML
  DOCTYPE（internal/external subset）・未定義 general entity
  拒否、corrupt checksum・truncated TAR・corrupt gzip・
  invalid UTF-8 JSON、gzip/TAR の決定的 mutant sweep で publish
  gate が unverified composite を通さないこと。
- `tests/os_fs_probe.rs`: CreateNew/Replace ×
  BestEffort/FileSync/FileAndDirectorySync の 6 通りを実 FS で
  probe。`DurabilityEvidence` が要求 level と一致、または
  `PublishFailureKind::Unsupported` + `NotPublished` で正直に
  拒否されることのみを assert（silent downgrade 検出）。
- `tests/rooted_boundary.rs`: publish 済み・extraction 済み
  tree に `RootedDir` を適用。NoFollow read・`read_dir`・
  locator 文法（絶対/`..`/`\`/`.`/重複 separator/colon/NUL）
  の `InvalidInput`、link escape の `WithinRoot`→`EscapesRoot`/
  `NoFollow`→`NotRegularFile`。link 生成不能時は `eprintln!`
  skip。
- `README.md`: 責務・主張境界を更新し、証拠 report を正本に
  指定。
- `evidence/rf306/fault-fuzz-report.md`: 実測値・NotRun/
  Unsupported 一覧・#219/#294 引き渡し条件を記録。

### Decisions During Implementation

- 敵対的 fixture は公開 writer が生成できない bytes
  （symlink/special entry・traversal・破損 checksum）を
  `tests/common` の byte-level helper で構築した。本番 API・
  `#[cfg(test)]` mock には触れない。
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` は
  実 OS で正直に誘発不能なため fixture 化せず、
  `ruviform-io-file/src/staged.rs` の `#[cfg(test)]` MockFileOps
  試験を正本として証拠 report に明記した（捏造しない）。
- `ZipArchiveReader::new` が `max_entries` を indexed open 時に
  enforce する実装を確認し、entry-count budget 試験は reader/
  extraction どちらの typed limit 失敗でも受理する形にした
  （実装詳細に結合しない）。
- Windows の delete-pending semantics により、extraction tree
  検証では `RootedDir` handle を cleanup 前に解放する scope を
  設けた（metadata 上の試験修正であり契約変更ではない）。

## Related Experiments

なし（実装方針は plan 確定済み。試験側の修正経緯は
`metadata.experiments.rejected` と証拠 report「発見・修正した
試験側の問題」に記録）。

## Validation

全て `mise` タスクで実走（exit 0）:

- `mise run fmt-check`（`cargo fmt --all -- --check`）
- `mise run check-io-e2e`（`cargo check -p ruviform-io-e2e
  --all-targets`）
- `mise run lint-io-e2e`（`cargo clippy -p ruviform-io-e2e
  --all-targets -- -D warnings`）
- `mise run test-io-e2e`（60 tests 全 pass）
- `mise run check` / `mise run lint` / `mise run test`
  （workspace 全体）
- `mise run test-rt`（`rt_` name filter。io-e2e は非 RT で
  対象外）

OS/FS 実測: Windows 11 x64 / NTFS のみ。durability ladder
6/6 で `DurabilityEvidence` が要求 level と一致
（`evidence/rf306/fault-fuzz-report.md` に実値を転記）。
macOS/Linux・Network/Cloud FS・性能は NotRun/Unsupported。

## Risks

- link 依存試験（`rooted_dir_link_escape_is_refused`）は
  link 生成不能環境で skip となる。本環境（Windows）では
  junction 作成に成功し実測済みだが、権限のない CI では
  skip 表示を確認すること。
- `os_fs_probe` の evidence 一致 assert は現行 Windows NTFS
  backend の実測に基づく。他 OS/FS で挙動が異なる場合は
  `Unsupported` 分岐の検証として機能する設計だが、当該環境で
  の実測は未実施。

## Follow-ups

- AudioLens #139/#143・Synvelis #82/#84 の consumer 実測は
  #219/#294 cross-repo gate の責務（NotRun。引き渡し条件は
  `evidence/rf306/fault-fuzz-report.md`）。
- macOS/Linux 実機・Network/Cloud FS での durability 再実測は
  実環境が用意された時点で実施。
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` の
  統合レベル証拠が必要になった場合は、`ruviform-io-file` の
  mock 注入点を公開 fixture 化する別 Issue で扱う。

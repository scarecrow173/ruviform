---
id: "DESIGN-034UtzREeBZLceakda7vIS"
type: "design"
status: "approved"
title: "Issue #306 IO E2E slice 2 — Fault/Fuzz fixture 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
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
        path: "docs/impl/ir/issue-306-io-e2e-fault-fuzz.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/mod.rs"
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
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-304-io-archive-extract.md"
  refines:
    - "docs/designs/issue-306-io-composition-e2e.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E slice 2 — Fault/Fuzz fixture 設計

## Context

第 1 slice の `ruviform-io-e2e` は 31 件の合成試験で 5 境界
（File lifecycle・Structured json/csv・TAR→compression→StagedFile・
Secure Extraction・WAV finalize→publish）を pin 済み。本 slice は
Issue #306 受入条件の残りを、**既存公開 API の消費側 fixture**
としてのみ追加する。内部 mock（`#[cfg(test)]` `PlatformFileOps`・
`vfs::mock`）は外部 crate から到達不能であり、公開面に fault
注入点は存在しない — したがって本 slice の fault fixture は
「実 OS で誘発可能な fault class のみ」を対象とし、誘発不能な
分類（`Indeterminate`・DirSync 後失敗・`TempState::Unknown`）は
io-file mock 正本への参照として記録に留める（第 1 slice spec の
Open Questions 決定を継承）。

## Scope

### In Scope

- `crates/ruviform-io-e2e/tests/` への 5 fixture file 追加と
  `tests/common/mod.rs` の helper 拡張。
- `Cargo.toml` dev-deps の `ruviform-io-structured` feature を
  `json,csv,file` → `json,yaml,toml,csv,xml,file` へ拡張
  （leaf crate dev-deps のみ。production feature 集合は不変）。
- OS/FS 実測の記録（test + evidence report + impl record）。
- #219/#294 向け handoff 記述（Test Command/Fixtures/判断基準）。

### Out of Scope

- 公開 API 変更・mock 公開・fault-injection API 新設。
- 無制限 fuzz・fuzzing 基盤・新 CI framework・証拠 schema。
- Consumer repo 編集・macOS/Linux 実機・Network/Cloud FS・性能。
- `ruviform-observability`/`ruviform-ui-test`（別 dispatch 稼働中）。

## Components and Boundaries

### Fixture file 構成

| file | 境界 | 内容 |
| --- | --- | --- |
| `tests/structured_formats_staged.rs` | Structured(全 format)→StagedFile→reopen | yaml/toml `write_file`→`read_file` round-trip、xml `read_file`（event 列）on publish 済み bytes、publish 後外部改竄 → `Decode` 拒否（TrailingData/構文） |
| `tests/fault_boundary.rs` | File/Structured/Archive→publish fault 伝播 | `write_file` の `Create`（missing parent）/`Publish`（directory destination・CreateNew 競合）失敗分類の層越え保持、recovery handle 到達、temp 非残存。TAR→gzip→StagedFile 合成の publish 競合 |
| `tests/negative_fuzz.rs` | Parser/Extraction/Compression negative + bounded mutation | ZIP symlink entry（CD `external_attrs` patch）・ZIP bomb budget・XML DOCTYPE/entity・破損 ZIP/TAR/GZIP/JSON の typed error・決定的 mutation sweep で publish gate を検証 |
| `tests/os_fs_probe.rs` | OS/FS capability 実測 | durability ladder の実 FS probe（測定値のみ主張） |
| `tests/rooted_boundary.rs` | Directory 境界（RootedDir） | publish 済み tree・extraction 産出 tree の no-follow 読取、植え付け link の `EscapesRoot` 拒否（作成不能時は skip） |

### `tests/common/mod.rs` への追加 helper

- `dir_link`/`file_link` — io-file `tests/common` と同型
  （unix symlink・Windows `mklink /J` junction・`symlink_file`
  best-effort）。作成失敗は `Err` を返し呼び出し側が skip 判定。
- `patch_zip_external_attrs` — writer 産出 ZIP の central
  directory record の `external_attrs`（offset +38..42）を unix
  mode へ書換える。CRC32 は payload のみを覆うため patch は
  整合性検査に影響しない。`crc32fast` 等の新規依存は不要。
- publish 後改竄（`Published-then-tamper`）は `std::fs::OpenOptions::append` で
  実施（公開 file への外部 write はテスト境界内の fault 模擬）。

## Data and Control Flow

- **Structured 拡張**: `yaml::write_file`/`toml::write_file` は
  `json::write_file` と同一の `write_document` compose
  （create→encode→seal→publish）。`xml` は read のみ（event API）
  のため「StagedFile で publish した bytes を `xml::read_file` が
  parse」+「敵対 XML を publish して `read_file` が `Decode` で
  拒否」の 2 向きで境界を pin する。
- **Fault 伝播**: `DocumentFileWriteError::{Create,Encode,Publish}`
  の variant 保存を各 fault class で assert。`Publish` variant は
  内側 `PublishFailure` の `take_recovery_handle` まで到達して
  cleanup する（handle が層を越えて失われないことの証明）。
- **Negative/Fuzz**: 破損 fixture は writer/手工 craft で決定的に
  生成し、期待する `*ErrorKind` を assert。mutation sweep は
  有効 composite（tar.gz）の切詰め・header 領域 byte 反転を
  決定的に適用し、「独立 verify が失敗 → abort → destination
  不在」を全 mutant で確認する（panic は test failure として
  検出される）。
- **OS/FS probe**: `StagedFile` publish ladder を temp dir で実行
  し `DurabilityEvidence` を assert。対応不能環境では失敗が
  正直に出る（Supported の偽装をしない）。

## Non-Functional Constraints

- **非 RT**: 全 fixture は blocking FS/Parser I/O。`rt_*` 命名・
  RT lane への混入なし。
- **DAG**: io-e2e は葉側 consumer のまま。dev-deps feature 拡張は
  leaf 内で完結し、production crate の feature 集合・依存方向を
  変えない。
- **License/MSRV**: workspace 継承のまま変更なし。新規 code は
  MSRV 1.89 互換の std/既存 API のみ（`let ... else`、`?` 等、
  既存試験と同じ構文範囲）。
- **実行時間**: mutation sweep・probe は秒オーダーに収める
  （各 mutant は小さい fixture、probe は 6 publish のみ）。

## Risks and Trade-offs

- **ZIP `external_attrs` patch の脆さ**: CD record の offset 計算を
  EOCD 走査で行い、writer 産出物の既知 layout に依存する。
  代替（手組み ZIP + CRC 実装）は複雑さが増すため patch 方式を
  採用。patch 位置を assert 付きで検証し、将来 writer layout が
  変われば test が fail して検出される（silent wrong にはならない）。
- **link 作成の権限依存**: Windows `symlink_file` は
  `SeCreateSymbolicLinkPrivilege` が必要で失敗しうる。io-file
  既存試験と同じく junction（`mklink /J`、非特権）を dir link に
  使い、file link は失敗時 skip。skip は `eprintln!` で記録し、
  「試験したつもり」を作らない。
- **mutation sweep の判定粒度**: tar payload に checksum がない
  等 format 固有の性質上、「全 byte 反転が error」は保証できない。
  sweep は構造領域（header/EOCD/trailer）に限定し、payload 領域の
  変異は形式ごとの正本試験に委ねる — 本 fixture は「検証未完
  bytes を publish しない」gate のみを pin する。
- **OS/FS 差**: dirsync 非対応 FS で `os_fs_probe` が fail するのは
  意図通り（実測の正直な反映）。CI lane の OS 差は evidence に
  OS 別で記録する。

## Deferred Design Concerns

- `Indeterminate`/DirSync 失敗/`TempState::Unknown` の合成層検証 |
  実 OS 誘発不能・公開 fault API なし | fault-injection 公開面の
  将来追加 | mock 正本依存の分類盲点。
- 真の fuzzing（coverage-guided・長時間） | bounded fixture が本
  slice の責務 | 別 Issue での fuzzing 基盤整備 | parser の未知
  panic/過剰 resource が残る可能性。

## Verification Notes

- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
      `test-io-e2e` pass
- [ ] `mise run check` / `lint` / `test`（workspace 非回帰）pass
- [ ] evidence report に OS/FS 実測・NotRun/Unsupported を記録

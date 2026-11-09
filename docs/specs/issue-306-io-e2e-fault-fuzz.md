---
id: "SPEC-034UtxqBI7DX73fpCfjAok"
type: "spec"
status: "approved"
title: "Issue #306 IO E2E slice 2 — Fault/Fuzz fixture・OS/FS 実測・CI handoff"
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
    - "docs/plans/issue-306-io-composition-e2e.md"
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-304-io-archive-extract.md"
  refines:
    - "docs/specs/issue-306-io-composition-e2e.md"
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

# Issue #306 IO E2E slice 2 — Fault/Fuzz fixture・OS/FS 実測・CI handoff

## What Should Be Built

Issue #306（IO-IMP-12 共通 I/O 合成 E2E・Fault/Fuzz）の第 2 slice。
第 1 slice（`docs/specs/issue-306-io-composition-e2e.md`、PR merge
済み）で確立した `crates/ruviform-io-e2e` 試験専用クレートに対し、
Issue 受入条件の残り — **層間 fault 伝播・Negative/Fuzz・リソース
上限・OS/FS 実測記録・CI handoff** — を既存公開 API の消費側
fixture として追加する。公開 API・内部実装は一切変更しない。

追加する合成境界:

1. **Structured 全 format の StagedFile 境界**: `yaml`/`toml` の
   `write_file`/`read_file`（`feature = "file"` 合成）と `xml` の
   `read_file`（event API）を publish 済み bytes 経路で検証。
   publish 後に外部が末尾 byte を追加した文書が `read_file` で
   `Decode(TrailingData)` に拒否される reopen 境界（第 1 slice
   test-spec R3 の残件）を pin する。
2. **合成 fault 伝播（実 OS 再現可能 class のみ）**:
   `write_file` の `Create`/`Publish` 失敗が `FileError`/
   `PublishFailure`（effect/phase/recovery handle）を失わず保持
   すること、TAR→gzip→StagedFile 合成 bytes の publish 競合で
   `NotPublished`+`DestinationExists`+temp retained→recovery
   cleanup、missing parent・directory destination 等の実 OS fault
   を層越えで検証する。`Indeterminate`/`DirSync` 後失敗/`TempState::
   Unknown` は実 OS で deterministic に誘発不能のため対象外
   （`ruviform-io-file` の `#[cfg(test)]` MockFileOps ユニット試験が
   既存正本 — 第 1 slice spec の Open Questions 決定を維持）。
3. **Negative/Fuzz fixture（bounded）**: ZIP symlink entry
   （central directory `external_attrs` の unix mode 細工）→
   Secure Extraction 拒否、ZIP bomb 相当（高圧縮率 entry）→
   `ArchiveReadLimits` で `LimitExceeded`、XML `<!DOCTYPE>`/未定義
   entity → `Unsupported`、破損 ZIP/TAR/GZIP/JSON の typed error
   分類、決定的 mutation sweep で「検証未完 bytes を publish しない」
   gate を pin。無制限 fuzz・専用 fuzzer 基盤は作らない。
4. **Directory 境界**: `RootedDir`（io-file `rooted` capability）が
   staged publish 済み tree と Secure Extraction 産出 tree を
   no-follow で読めること、植え付けた link（Windows junction /
   unix symlink、作成不能時は正直に skip）が `EscapesRoot` で
   拒否されることを層横断で検証する。
5. **OS/FS 実測 probe**: `os_fs_probe.rs` が publish durability
   ladder（`FileSync`/`FileAndDirectorySync`/`BestEffort` ×
   `CreateNew`/`Replace`）を実 FS で実行し、測定した
   Supported/Unsupported を test 結果として残す。Windows NTFS の
   実測のみを本 slice の実施環境として記録し、macOS/Linux 実機・
   Network/Cloud FS は `NotRun`/`Unsupported` と正直に記録する。

## Why Now

第 1 slice で 5 合成境界の正常系・基本否定系は pin 済みだが、
Issue #306 受入条件の「Runtime Fault Injection」「Negative/Fuzz・
リソース上限」「OS/FS 別 Supported/Unsupported の実測範囲記録」
「#219/#294 への Test Command/Fixtures/判断基準の引き渡し」は
未着手のまま残っている。本 slice はそれらを fixture 層で閉じ、
Consumer 受入 Gate（#219/#294）が参照する Ruviform 側正本 command
`mise run test-io-e2e` の証拠範囲を確定させる。

## Users and Value

- RF #294/#219: consumer-verify が参照する Ruviform 側 Gate の
  Test Command・Fixture 一覧・pass/fail/not_run/unsupported 判断
  基準を本チェーンと evidence report に明記する。
- Consumer（AudioLens #139・Synvelis #82）: 同一公開 API で実装する
  保存・展開経路が、fault・敵対入力・OS/FS 差の境界で Ruviform 側
  実測済みであることの根拠。
- 後続 slice: #305 旧 API 撤去接続・cross-repo 実測の前提となる
  fixture 資産。

## Scope

### In Scope

- `crates/ruviform-io-e2e` の `tests/` への fixture 追加
  （`structured_formats_staged.rs`・`fault_boundary.rs`・
  `negative_fuzz.rs`・`os_fs_probe.rs`・`rooted_boundary.rs`）、
  `tests/common/mod.rs` への共有 helper 追加（link 生成・ZIP
  central directory patch 等）。
- `crates/ruviform-io-e2e/Cargo.toml` dev-dependencies の feature
  拡張（`ruviform-io-structured` に `yaml,toml,xml` 追加）。
  新規外部依存は追加しない（CRC32 は writer 産出 ZIP の
  `external_attrs` patch で済ませ、`crc32fast` 直接依存は不要）。
- `crates/ruviform-io-e2e/README.md` の責務・検証節の更新。
- `evidence/rf306/fault-fuzz-report.md` — 実コマンド・実値・
  OS/FS 実測範囲・NotRun/Unsupported の記録。
- 本 doc チェーン（spec/design/test-spec/plan/task/impl）。

### Out of Scope

- 各 IO crate の公開 API・内部実装の変更（fixture は既存 API の
  消費側としてのみ実装）。`#[cfg(test)]` MockFileOps の公開や
  fault-injection API の新設。
- 無制限 fuzz・専用 fuzzing 基盤・新 CI framework・証拠
  YAML/JSON スキーマ・Dashboard（Issue #288/#294 の既存機構を使う）。
- Consumer repo（AudioLens/Synvelis）への編集・実測
  （#219/#294 cross-repo gate の責務。本 slice は handoff 記述のみ）。
- macOS/Linux 実機・Network/Cloud FS・性能 benchmark — 実測範囲
  外として `NotRun`/`Unsupported` を記録するのみ。
- `crates/ruviform-observability/`・`crates/ruviform-ui-test/`
  （別 dispatch 稼働中のため不可触）。

## Requirements

- 全 fixture は公開 API のみを消費し、backend 型・内部 mock・
  `#[doc(hidden)]` の `vfs` module に触れない。
- 否定系は「拒否の分類（`ArchiveErrorKind`/`DocumentErrorKind`/
  `PublishEffect` 等）+ staging/destination 非残存」の両方を assert
  する。観測できない分類を推測で記録しない。
- 実 OS で再現不能な fault class（`Indeterminate`・DirSync 後失敗・
  `TempState::Unknown`）は合成層で偽装せず、io-file mock 正本への
  参照と `NotRun` 記録に留める。
- OS/FS 別 Supported/Unsupported は実測した OS/FS のみを主張する。
  Windows NTFS 実測 + CI 既存 lane（ubuntu/windows/macos の
  `test-ci`）が走らせる範囲を区別して記録する。
- `#[non_exhaustive]` enum の variant 追加・API 変更は行わない。
  追加する dev-dependency feature は leaf crate の dev-deps に
  閉じ、production feature 集合を変えない。

## Acceptance Criteria

- [ ] `cargo test -p ruviform-io-e2e` が新規 fixture を含め全 pass
      （実施 OS = Windows NTFS の実測結果を記録）。
- [ ] `yaml`/`toml`/`xml` の StagedFile 境界 round-trip と、
      publish 後改竄 → `read_file` 拒否の reopen 境界が存在する。
- [ ] 合成 fault 伝播試験が `Create`/`Publish` 失敗の分類保持・
      recovery handle・temp 非残存を層越えで検証する。
- [ ] ZIP symlink・ZIP bomb budget・XML entity/DOCTYPE・破損
      fixture・mutation sweep が typed error/拒否を assert し、
      未検証 bytes の publish 抑止を示す。
- [ ] `RootedDir` が publish 済み/extraction 産出 tree を読み、
      link 経由 escape を拒否する（link 作成不能環境では正直に
      skip）。
- [ ] `os_fs_probe` が durability ladder を実 FS で実行し、
      測定値のみを主張する。
- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
      `test-io-e2e` / `check` / `lint` / `test` が pass。
- [ ] #219/#294 向けの Test Command・Fixtures・判断基準を
      evidence report と impl record に明記する。

## Deferred Design Concerns

- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` の実 OS
  再現 | 実 OS で deterministic に誘発不能 | fault-injection
  公開 API が将来設けられた場合 | 合成層の分類網羅が mock 正本
  依存のまま残る。
- macOS/Linux 実機・Network/Cloud FS・性能 | 本環境は Windows のみ
  | CI 既存 lane または将来の実機 dispatch | OS/FS 差異が未検出の
  まま残る。
- Consumer 実測（AudioLens #139・Synvelis #82） | #219/#294
  cross-repo gate の責務 | consumer-verify lane 整備後 | 両 Consumer
  の片側 pass を合格と誤認するリスク（本 slice では NotRun と明記）。

## Sources and Relations

- Issue #306 本文（受入条件の正本）・Issue #288 IO-06 v0.1・
  Issue #282 横断設計正本。
- 第 1 slice 文書チェーン: spec/design/plan/test-spec/tasks/impl
  （`docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  issue-306-*.md`）— 本 slice はその `refines` 関係。
- `crates/ruviform-io-e2e/README.md` — 既存 fixture の責務境界と
  「mock fault matrix は io-file ユニット試験が正本」の既存決定。

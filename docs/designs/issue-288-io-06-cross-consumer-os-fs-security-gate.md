---
id: "DESIGN-034UtFFzzM3wAegGBSycbJ"
type: "design"
status: "approved"
title: "Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
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
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "https://github.com/scarecrow173/ruviform/issues/288"
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
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/300"
    - "https://github.com/scarecrow173/ruviform/issues/301"
    - "https://github.com/scarecrow173/ruviform/issues/302"
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/25"
  defers: []
  deferred-by: []
---

# Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計

## Context

Issue #288 は、IO-01〜05（File / Structured / Compression /
Archive / Migration — Issue #283〜#287 の設計 Issue、実装 Issue
#295〜#306）の実装を Ruviform・AudioLens・Synvelis の 3 者へ
導入する際の **検証範囲と合否基準を定義する Gate 設計** を
要求する。Issue 本文の「合意済み設計 v0.1（簡素版・
2026-11-04）」が要求の正本であり、本書はそれを docs チェーンの
正本設計書として文書化する。

**本書が固定するもの / 固定しないもの**

- 固定する: Gate の判定語彙・受入 matrix の構造と各行の判定
  基準・否定系判定表・Owner/境界の接続表・Consumer 要件・
  Breaking Change 追跡導線・証拠の参照先。
- 固定しない: 各 Gate の実走結果（実装段階の要件）、新規 CI
  機構、証拠集約基盤。Issue #288 は「独自の証拠 YAML/JSON、
  集約 DB、CI フレームワークは作らない」を明示しており、本書も
  既存 GitHub Actions・PR・Issue への参照のみで構成する。

**実装の現状（2026-11-09 時点の snapshot）**

- IO-IMP-01〜12（#295〜#306）は main に merge 済み。
  `ruviform-io-file` / `-structured` / `-compression` /
  `-archive` / `-e2e` クレートと `ruviform-io-wav`・
  `ruviform-io-symphonia` が存在する。
- IO-05 の旧 `StagedOutput`/旧 `PublishOptions` は互換層なしで
  撤去済み（#305・PR #391 merge）。`ruviform-media-io` は
  Domain Core 3 依存のみを維持する。
- 合成 E2E 試験クレート `ruviform-io-e2e` は main に存在し
  `mise run test-io-e2e` が Gate command（#306・PR #395 merge）。
- Cross-Repo 検証機構: `consumer-verify.yml`（#294）が候補
  Ruviform SHA × Consumer 候補 ref の tuple 証拠を記録する。
  日次 trigger は `daily-ci.yml`（#294 tuple mode・#367 scheduler）。
- Consumer 側: AudioLens #139/#140/#141/#143 は CLOSED、
  Synvelis #82/#84 は OPEN、Synvelis #25（WAV E2E）は CLOSED
  （Issue 状態は変動し得る snapshot として記録）。

文書チェーン: [spec](../specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)・
[plan](../plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md)・
[test-spec](../test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)・
[task](../tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md)。

## Scope

### In Scope

- IO-01〜05 の受入 matrix（正常系・否定系 × 入力/障害条件 ×
  Capability/OS/FS）の判定表設計と結果記録フォーマット。
- publish/recovery・安全性・resource budget・Media/PCM/RT・
  Consumer 非回帰の否定系判定表。
- IO-01〜05 テスト要件と Owner・OS/FS/脅威モデル境界を実装
  Issue へ接続する対応表。
- Ruviform/AudioLens/Synvelis の影響・Golden・回帰・性能・
  Dependency 更新要件。
- 旧 API Breaking Change の採用手順・戻し方の追跡導線。
- 証拠導線（既存 Actions/PR/Issue 参照）と設計/実装の分離。

### Out of Scope

- matrix の実走・テスト実装・fixture 作成（#306 系・#219/#294・
  Consumer repo が Owner）。
- 新規 CI Workflow・証拠スキーマ・集約 DB・Dashboard。
- AudioLens/Synvelis repo の変更・実測。
- macOS/Linux 実機検証（環境外。CI lane 結果の参照に委ねる）。

## Components and Boundaries — 責務分担

| 領域 | Owner | 本書との関係 |
| --- | --- | --- |
| 汎用 I/O Gate の定義・対象 Fixture・OS/FS 保証・Consumer 別合否の追跡 | **Issue #288（本書）** | 判定基準の正本 |
| Cross-Repo CI 運用・継続追従の正本 | **Issue #219**（`docs/specs/issue-219-consumer-compat-contract.md`） | 影響分類 `impacted/no impact/unknown` と証拠要件の語彙を共有 |
| `consumer-verify.yml` の候補 tuple・lane 拡張・CI 新設/Trigger/鍵管理 | **Issue #294** | 実行機構の Owner。本書は lane 結果を受入項目へ対応付けるのみ |
| IO-01〜05 のテスト要件・実装 | **#295〜#306**（IO-IMP-01〜12）と各 test-spec | 各 Gate 行の根拠・実行 command の出所 |
| Consumer 固有の実装・Golden/E2E/復旧・依存更新 | **AudioLens #139/#140/#141/#143、Synvelis #82/#84/#25** | Consumer 受入の実施 Owner |
| 旧 API 廃止の Breaking Change 管理 | **#287（設計）・#305（実装・merge 済み）** | 採用/戻し手順の追跡対象 |

原則: 本書は **判定の定義** を所有し、**実行** は既存機構
（`cargo test`/`mise run test-*`/`consumer-verify.yml`/各
Consumer CI）に委ねる。CI の新設・Trigger・鍵管理を #288 へ
重複移管しない（Issue #288 明記）。

## 判定語彙と最小判定ルール

Issue #288「最小判定ルール」を正本として固定する。独自スキーマ・
証拠 DB は作らない。

### 語彙

- **Consumer 影響分類**: `impacted` / `no impact` / `unknown`
  （#219 の契約と同一。根拠を既存 PR/Issue に記載する）。
- **Gate 結果**: `pass` / `fail` / `blocked` / `not run` /
  `unsupported`。
  - `unsupported`: 当該 OS/FS が必要保証を提供できないと
    **正しく拒否した場合の否定テスト成功**のみを意味する。
    Consumer に必須の Capability が欠ける場合、その Consumer の
    採用 Gate は通過しない。
  - `not run` / `unknown` を PASS 扱いしない。
  - `blocked`: 既知のベースライン不具合等で Gate を実行完了
    できない場合。実態を維持し合格に付け替えない。
- **ベースライン vs 新規退行**: 既知不具合（例: AudioLens #141
  の Export 不具合 — CLOSED 済みだが同種の先例）と今回の変更由来の
  退行を分けて記録する。

### Consumer 独立判定

同一候補 Ruviform Revision でも AudioLens と Synvelis は
**独立して判定**する。無関係の Consumer 作業は一律停止しないが、
**両 Consumer に影響する Breaking Change は両方の候補で必要
Gate が通らなければ採用完了としない**（Issue #288 明記）。

### 結果記録フォーマット

各 matrix 行・否定系判定表の行の結果は、**実行 venue ×
Consumer 候補**の組ごとに `pass`/`fail`/`blocked`/`not run`/
`unsupported` で記録する。記録先は新規スキーマを設けず、
次の既存場所に限定する。

- Ruviform 単体 Gate: 該当 PR の Checks（`ci.yml` lane）と
  mise task の実行結果。PR/Issue 本文に `pass`/`fail`/
  `blocked`/`not run`/`unsupported` と対象 OS/FS・toolchain
  を根拠付きで記載する。
- Consumer 候補 tuple Gate: `consumer-verify.yml` run の
  evidence artifact（`consumer-evidence-*.json`）と Job
  Summary、対応する PR/Issue。lane 単位の status を tuple
  単位で読まず、AudioLens/Synvelis を独立に記録する。
- OS/FS 別実測: 実施した OS/FS・mount 条件・toolchain を
  テストログまたは実施 Issue/PR に記録する。保証判定に
  必要で既存 CI から追えない値だけを残し、独自の証拠記録
  形式は設けない（Issue #288 明記）。
- `unsupported` は「当該 OS/FS で必要保証を提供できないと
  正しく拒否した否定テストの成功」としてのみ記録し、
  Consumer 必須 Capability の欠落を `unsupported` で
  免除しない。

## OS/FS・Capability 軸

### 実測候補 OS/FS

Issue #288 は Linux ext4/XFS・macOS APFS・Windows NTFS を実測
候補とする。`docs/designs/issue-296-io-file-os-backend.md` の
capability matrix（設計時点の主張）を Gate の OS/FS 軸の土台と
し、**実測で検証された範囲だけを Supported と主張する**。

| OS/FS | CreateNew | Replace | FileSync | DirSync | 備考 |
| --- | --- | --- | --- | --- | --- |
| Linux ext4/XFS/btrfs | renameat2 / link fallback | rename | fsync | dir fsync | CI lane で検証対象 |
| Linux vfat/exFAT | renameat2（VFS 層保証） | rename | fsync | dir fsync（失敗あり得る→Err+Published） | FS 依存 |
| Linux NFS | renameat2/link（server 依存・Unsupported になり得る） | rename | fsync | dir fsync | **未実測 → Supported としない** |
| macOS APFS | renamex_np / link fallback | rename | fsync | dir fsync | **実機未検証 → NotRun** |
| Windows NTFS | MoveFileExW(0) | MoveFileEx 系 | FlushFileBuffers | BackupSemantics+FlushFileBuffers | 本 slice 作業環境・実測済み範囲 |
| Windows FAT32/exFAT | MoveFileExW(0) | MoveFileEx 系 | FlushFileBuffers | DirSync 失敗し得る→Err+Published | FS 依存 |
| Windows ReFS | MoveFileExW(0) | MoveFileEx 系 | FlushFileBuffers | FS 依存 | **未実測** |
| 上記以外の unix/その他 OS | link+unlink または Unsupported | rename / Unsupported | fsync / Unsupported | Unsupported | Capability 不在は Unsupported |

### 原則

- **Fault Injection と実 OS/FS 実測は区別する**。`MockFileOps`
  系の adapter fault matrix（`ruviform-io-file` の `#[cfg(test)]`
  ユニット試験が正本）と、実 FS 上の再現可能な fault（CreateNew
  競合・Disk Full 等）は別の証拠として記録する。#306 の合成 E2E
  は後者のみを扱う（重複実装しない）。
- **API の Sync 成功を全種の電源断への保証と呼ばない**。
  FileSync/DirectorySync の Receipt evidence（`FileSynced`/
  `FileAndDirectorySynced`）は要求達成の観測値であり、
  crash-consistency の保証範囲は各 OS/FS の実測でのみ主張する。
- **Network/Cloud FS**（NFS/SMB 等）は OS・Server・Mount 条件と
  保証範囲を個別に扱い、**未実測を Supported としない**。
- Feature 単独/組合せ・MSRV 1.89・依存 DAG・ライセンス/
  セキュリティは Ruviform のテスト・監査（`mise run lint-ci`・
  `test-ci`・feature matrix task）で確認する。Parser/
  Compression/Archive・ブロッキング I/O を RT Core へ混入しない
  （DAG は `docs/designs/ruviform-architecture-design-baseline.md`
  と各クレート README が正本）。

## 受入 matrix（Acceptance Matrix）

Issue #288 の受入スコープ表を、判定可能な粒度へ展開する。
各行は **入力・障害条件 → 判定基準 → 証拠/根拠** を持つ。
「証拠/根拠」列の test-spec・Issue・command は既存の正本への
参照であり、本書はそれらを複写しない。

### IO-01 File（Issue #283 → #295/#296）

| # | 系統 | 入力・障害条件 | Capability/OS/FS 軸 | 判定基準（pass 条件） | 証拠/根拠 |
| --- | --- | --- | --- | --- | --- |
| F-N1 | 正常 | `StagedFile::create` → write/seek → `seal` → `open_readback` → `publish` → `PublishReceipt` | 全対応 OS/FS | 型状態遷移が成立し、Receipt が要求 durability の実績 evidence を持つ | test-spec `issue-295` R1–R3、`issue-296` R1–R2、`mise run test-io-file` |
| F-N2 | 正常 | `PublishMode::CreateNew` / `Replace` の各 mode | CreateNew 非対応 FS では Unsupported を返す | CreateNew は原子的排他・Replace は Create-or-Replace（CAS なし）を receipt/分類で区別 | test-spec `issue-296` R3–R4 |
| F-N3 | 正常 | `Durability::{BestEffort,FileSync,FileAndDirectorySync}` | FS が DirSync を提供しない環境 | 各要求で `None`/`FileSynced`/`FileAndDirectorySynced` の実績 evidence。要求不達を Ok で偽装しない | test-spec `issue-295` R3/R5、`issue-296` R5 |
| F-X1 | 否定 | 既存宛先への CreateNew 競合（複数プロセス同時を含む） | 全対応 OS/FS | ちょうど 1 つが成功、他は `NotPublished`+`DestinationExists`、先入主 bytes 保全 | test-spec `issue-296` R3・Examples（8 子プロセス競合） |
| F-X2 | 否定 | Disk Full / write 途中失敗 | 実 FS | `NotPublished` または適切な分類。宛先は publish されず temp は RecoveryHandle 経由で回収可能 | test-spec `issue-295` R4/R7 |
| F-X3 | 否定 | publish 途中失敗・rename 後 DirSync 失敗 | 実 FS + Mock（DirSync 強制不能分） | `Published`+`FileSynced` 等の effect を正しく分類し Ok にしない。`Indeterminate` は自動 cleanup/再 publish しない | test-spec `issue-295` R3/R4/R7（Mock 駆動分は `issue-296` R5 で pin） |
| F-X4 | 否定 | abort/drop/verify 失敗時の temp 残留 | 実 FS | `abort`→`CleanupReceipt`、失敗時 `CleanupFailure`+`RecoveryHandle`、`Indeterminate` temp を無条件削除しない | test-spec `issue-295` R6/R7、`issue-305` R6 |

### IO-02 Structured（Issue #284 → #297/#298/#299）

| # | 系統 | 入力・障害条件 | Capability/OS/FS 軸 | 判定基準（pass 条件） | 証拠/根拠 |
| --- | --- | --- | --- | --- | --- |
| S-N1 | 正常 | JSON/YAML/TOML の Serde 型付き read/write round-trip | format feature 有効時 | 値・構造・Unicode が再構成される | test-spec `issue-297` Rule 1 |
| S-N2 | 正常 | CSV Record streaming（quoted/newline/dialect/header） | `csv` feature | record 単位の増分消費・dialect 指定が仕様どおり | test-spec `issue-298` G1–G13 |
| S-N3 | 正常 | XML Event streaming（Namespace/Attribute/CDATA） | `xml` feature | event 単位の増分消費・namespace 解決が仕様どおり | test-spec `issue-299` Golden/Positive |
| S-N4 | 正常 | `write_file`/`read_file`（`file` feature）で StagedFile 合成 | `file` feature | Encode→seal→publish / open→parse の失敗軸が `DocumentFile*Error` で分離 | test-spec `issue-297` Rule 6、`issue-298` S5、`issue-299` S5 |
| S-X1 | 否定 | 不正構文・末尾余剰（TrailingData）・型不一致 | 全 format | `Syntax`/`DataModel`/`TrailingData` に分類、silent truncation なし | test-spec `issue-297` Rule 2/4 |
| S-X2 | 否定 | 過大データ（`max_bytes`/`max_depth`/`max_records`/`max_events` 等） | 全 format | `LimitExceeded`、ちょうど上限は成功し得る（境界包含）、超過 byte を返さない | test-spec `issue-297` Rule 3、`issue-298` N1–N5、`issue-299` N1–N6 |
| S-X3 | 否定 | CSV field 数/型不一致・非 UTF-8・UTF-16 BOM | `csv` | `DataModel`/`InvalidEncoding` に分類 | test-spec `issue-298` N7–N11 |
| S-X4 | 否定 | YAML 複数文書・YAML alias | `yaml` | trailing/未対応構文を受理しない（backend 保証範囲を超えて主張しない） | test-spec `issue-297` Rule 2/4・backend 保証範囲の明記 |
| S-X5 | 否定 | XML DTD internal/external subset・未定義 entity・非 UTF-8 encoding・mismatched tag | `xml` | `Unsupported`/`Syntax`/`InvalidEncoding` に分類（DTD/外部 entity は既定で拒否） | test-spec `issue-299` N7–N18 |
| S-X6 | 否定 | backend 型の公開 API 露出 | 全 format | `serde` trait のみ公開許可、`serde_json`/`csv`/`quick_xml` 等の型は非露出 | test-spec `issue-297` Rule 5、`issue-298` S8、`issue-299` S7 |

### IO-03 Compression（Issue #285 → #300/#301）

| # | 系統 | 入力・障害条件 | Capability/OS/FS 軸 | 判定基準（pass 条件） | 証拠/根拠 |
| --- | --- | --- | --- | --- | --- |
| C-N1 | 正常 | Gzip/Raw Deflate の encode→decode round-trip、明示 `finish()` | `gzip`/`deflate` feature | byte 一致・`VerifiedEnd` のみ成功・`finish` 冪等 | test-spec `issue-300` R1–R3 |
| C-N2 | 正常 | Zstd single/concatenated/skippable frame | `zstd` feature | frame 単位 receipt（units/consumed/integrity）が正確 | test-spec `issue-301` AC1 |
| C-X1 | 否定 | Truncated input・CRC/Frame Checksum 不一致・Trailing data | 全 format | `TruncatedInput`/`ChecksumMismatch`/`TrailingData` に分類 | test-spec `issue-300` R4–R5、`issue-301` AC2/AC3 |
| C-X2 | 否定 | 上限ちょうどと +1 超過（`max_decoded_bytes`/`max_compressed_bytes`/`max_units`/`max_window_log`） | 全 format | ちょうどは成功、+1 は `LimitExceeded`/`InvalidInput`、超過 byte を返さない | test-spec `issue-300` R7、`issue-301` AC3/AC4 |
| C-X3 | 否定 | Window/Frame 数・先読み境界（`SingleUnit`/`WholeInput`・BufRead prefetch 非混同） | 全 format | 論理消費量と物理 prefetch を混同しない・caller に残 byte を返す | test-spec `issue-300` R6、`issue-301` AC5 |
| C-X4 | 否定 | 未 `finish`/`Drop` のみの出力を完成証拠にしない | Encoder/Decoder 合成 | `finish` 成功なしに publish/VerifiedEnd を主張しない | test-spec `issue-300` R1–R2、`issue-301`（Encoder finish receipt） |

### IO-04 Archive（Issue #286 → #302/#303/#304）

| # | 系統 | 入力・障害条件 | Capability/OS/FS 軸 | 判定基準（pass 条件） | 証拠/根拠 |
| --- | --- | --- | --- | --- | --- |
| A-N1 | 正常 | ZIP/TAR Entry の index/stream read・`verify_all`/finish | `archive` | 検証完了 receipt（entries_total/verified/payload_bytes）が正確 | test-spec `issue-302` R6、`issue-303` R7 |
| A-N2 | 正常 | publish 済み archive の再 Open → verify → `ExtractionSession` 隔離展開 → `ExtractedTree` | `archive`+`file` | trusted parent 内の排他 staging・検証済み entry のみ公開 | test-spec `issue-304` R5/R6、`issue-306` R6 |
| A-X1 | 否定 | Path Traversal（`..`/絶対/drive/UNC/予約名/NTFS 禁止文字/非 UTF-8） | extract | `SafeRelativePath` が書込前に拒否、staging が残存しない | test-spec `issue-304` R1、`issue-306` R6 |
| A-X2 | 否定 | Link entry（symlink/hardlink/special）・重複・casefold/NFC-NFD 衝突 | extract | `Unsupported`/`InvalidInput` で拒否、File/Directory のみ受理 | test-spec `issue-304` R2/R3 |
| A-X3 | 否定 | Zip Bomb・entry/total/metadata/path budget 超過 | archive+extract | 宣言 size の事前拒否と実測の途中遮断の両層で `LimitExceeded`、超過 byte が FS に出ない | test-spec `issue-302` R1/R7、`issue-303` R4/R5、`issue-304` R4 |
| A-X4 | 否定 | 構造破損（Truncated/Malformed/ChecksumMismatch/Overlap/Inconsistent）・未読 TAR Entry（Pending）・TOCTOU | archive | backend 保証範囲の分類で拒否。lexical check のみで race-safe を主張しない | test-spec `issue-302` R2–R4、`issue-303` R1/R6、`issue-304`（TOCTOU は脅威モデル境界を参照） |
| A-X5 | 否定 | extract 途中 cancel/失敗時の staging cleanup | extract | `abort`/失敗経路で staging が残存しない・既存 destination 無変更 | test-spec `issue-304` R6、`issue-306` R6 |

### IO-05 Migration（Issue #287 → #305・#306）

| # | 系統 | 入力・障害条件 | Capability/OS/FS 軸 | 判定基準（pass 条件） | 証拠/根拠 |
| --- | --- | --- | --- | --- | --- |
| M-N1 | 正常 | WAV encode → `finalize()`/`WavReceipt` → `into_sink` で借用解除 → `seal` → `open_readback` 独立 decode 検証 → `publish` → 必要に応じ reopen | `io-wav`+`io-file` | 完成証明（明示 finalize）なしに publish しない。`WavMuxer<&mut StagedFile>` が `MediaSink` で成立 | test-spec `issue-305` R2–R4、`issue-306` WAV 境界 |
| M-N2 | 正常 | `MediaSource`/`MediaSink`/Demux/Decode/Mux/Encode の公開契約継続 | media-io | Codec finalize と File publish が別責務のまま。Media 契約の非破壊 | test-spec `issue-305` R7、spec `issue-305` |
| M-X1 | 否定 | 旧 `StagedOutput`/旧 `PublishOptions` の公開・参照残存 | workspace 全体 | コード/Docs/Examples/Rustdoc/依存 DAG で旧 API 参照 0 件（歴史資料を除く）。`ruviform-media-io` 依存は Domain Core3 のみ | test-spec `issue-305` R1、Issue #287/#305 Acceptance Gate |
| M-X2 | 否定 | Publish 失敗時の情報喪失 | media-io→io-file | `RenderError::Publish` が `PublishFailure`（effect/phase/durability/recovery）を保持し `Io` に丸めない。`DestinationExists` 写像 | test-spec `issue-305` R5/R6 |
| M-X3 | 否定 | Media/PCM/RT 回帰（WAV golden・PCM precision・4GiB 境界・ZeroFrames・NaN/Inf） | media/audio | 既存試験が変更なしに緑。RT callback で I/O/Parser/Compression を呼ばない | test-spec `issue-305` R7、architecture baseline DAG |

### 合成 E2E（Issue #306 → `ruviform-io-e2e`）

| # | 系統 | 入力・障害条件 | Capability/OS/FS 軸 | 判定基準（pass 条件） | 証拠/根拠 |
| --- | --- | --- | --- | --- | --- |
| E-N1 | 正常 | Structured→StagedFile・TAR→Gzip/Zstd→StagedFile・ZIP/TAR→extract・WAV→publish の層横断合成 | 公開 API のみ・実 OS temp dir | 層間の所有権・状態・境界契約が pin される（finish/finalize→seal→readback→publish→reopen） | test-spec `issue-306`、`mise run test-io-e2e` |
| E-X1 | 否定 | 完成証明なし・切り詰め・検証未完 bytes の publish 阻止 | 実 OS | destination 不存在で否定経路を示す | test-spec `issue-306`（spec Acceptance Criteria） |
| E-X2 | 否定 | 実 OS で再現可能な fault class（CreateNew 競合・Replace 差替え・abort/drop cleanup） | 実 OS/FS | adapter 系 fault matrix（Mock 正本）と重複せず実 FS のみ検証 | spec `issue-306` 境界 5・Open Questions |

## 否定系判定表（Negative Judgment Tables）

Issue #288 が要求する 5 領域の否定系を、判定規則として固定する。
各表の「判定」は Gate 結果語彙（`pass`/`fail`/`blocked`/
`not run`/`unsupported`）を使う。

### 1. publish/recovery

| 条件 | 期待分類 | pass 判定 | fail/blocked 判定 | 根拠 |
| --- | --- | --- | --- | --- |
| CreateNew 競合 | `NotPublished`+`DestinationExists`・先入主保全 | 宛先 bytes 不変・temp が RecoveryHandle で回収可能 | 先入主破壊・情報喪失・自動削除 | `issue-295` R4/R7、`issue-296` R3 |
| FileSync/DirSync 失敗 | `Err`+正しい phase/effect（`Published`/`NotPublished`） | 要求不達を Ok で偽装しない・未公開と誤報しない | `Ok` 偽装・effect 誤分類 | `issue-295` R3、`issue-296` R5 |
| rename 後 DirSync 失敗 | `Err{Published, FileSynced}` | 公開済みの可能性を隠さない | 一律 rollback 済みと誤報 | `issue-296` R5・design capability matrix |
| Indeterminate | 自動 cleanup/再 publish なし・RecoveryHandle 保持 | 情報喪失なしで Consumer へ伝播 | 自動削除・`Io` 丸め | `issue-295` R4/R7、`issue-305` R5 |
| Cleanup 失敗 | `CleanupFailure`+`RecoveryHandle`・`cleanup()` 再試行 | publish 成功を覆さない receipt | temp 放置の黙殺 | `issue-295` R6/R7 |

### 2. 安全性（Security / Threat Model）

| 条件 | 期待分類 | pass 判定 | fail 判定 | 根拠 |
| --- | --- | --- | --- | --- |
| Archive Path Traversal/Link/重複/casefold 衝突 | 書込前拒否・staging 非残存 | `SafeRelativePath`+canonical 衝突検出・限定 kind | traversal bytes が FS に出る | `issue-304` R1–R3、`issue-306` R6 |
| Zip Bomb/展開 budget | `LimitExceeded`・途中遮断 | 宣言事前拒否+実測遮断の両層 | 上限超過 byte の出力 | `issue-302` R1/R7、`issue-304` R4 |
| XML DTD/外部 entity・YAML alias・CSV/XML encoding | `Unsupported`/`InvalidEncoding` | backend 保証範囲を超えた主張をしない | DTD/外部参照の黙った受理 | `issue-299` N7–N10/N17、`issue-297` Rule 4/5 |
| Structured 過大/深さ/複雑度入力 | `LimitExceeded` | `InputLimits` 各軸で境界包含 | 無制限メモリ/時間消費 | `issue-297` Rule 3、`issue-298`/`issue-299` negative |
| TOCTOU（展開先の外部書換え） | race-safe を主張しない | lexical check の限界を脅威モデルとして明記 | path check のみで敵対者安全と主張 | `issue-304` design・Issue #282 不変条件 |
| CRC 等は整合性検出であり真正性保証ではない | — | 文書・API が真正性を主張しない | CRC を認証と誤記 | Issue #282 不変条件・`issue-301`/`issue-302` receipt 語彙 |

脅威モデル境界: Ruviform は「File の安全な操作・Format の
byte encoding/decoding・Archive/Compression の安全な
Streaming」までを所有し、Consumer の Schema/Project/
Dataset 意味論・認証・暗号は各 Consumer が所有する
（Issue #282 方針 8）。

### 3. resource budget

| 条件 | 期待分類 | pass 判定 | 根拠 |
| --- | --- | --- | --- |
| `max_bytes`/`max_records`/`max_events`/`max_entries` 等の上限 | `LimitExceeded` | ちょうど上限は成功・+1 は拒否・超過 byte を返さない | `issue-297` R3、`issue-298`/`issue-299` N 系、`issue-300`/`issue-301`/`issue-302`/`issue-303`/`issue-304` budget 系 |
| streaming 増分消費（全入力一括読みでない） | 実装が BufRead/Read 単位で消費 | S1 系試験で pin | `issue-298` S1、`issue-299` S1、`issue-300`/`issue-301` 境界 |
| record/event buffer reuse・capacity 単調非減 | peak memory の有界性 | S2 系試験で pin | `issue-298` S2、`issue-299` S2 |
| 性能比較（速度/Peak Memory/Allocation） | 変更前と同一条件で比較・許容差は比較前に定義 | 暗黙の精度・性能犠牲なし | Issue #288「OS/Filesystemと性能」・下記 Consumer 性能要件 |

### 4. Media/PCM/RT 非回帰

| 条件 | 期待分類 | pass 判定 | 根拠 |
| --- | --- | --- | --- |
| WAV finalize→seal→readback→publish→reopen | 完成証明→独立検証→publish | `issue-305` R2–R4、`issue-306` WAV 境界 |
| PCM precision・4GiB 境界・ZeroFrames・NaN/Inf・golden bytes | 既存契約不変 | `issue-305` R7・既存 WAV 試験が緑 |
| `MediaSource`/`MediaSink`/Codec 公開契約 | 非破壊 | `issue-305` R7・spec Acceptance |
| RT callback で I/O/Parser/Compression を呼ばない | DAG で保証 | architecture baseline・`issue-305` R7・各 crate README |
| 旧 API 参照 0 件 | 撤去完了 | `issue-305` R1・workspace grep |

### 5. Consumer 非回帰

| 条件 | 期待分類 | pass 判定 | 根拠 |
| --- | --- | --- | --- |
| AudioLens: fsutil→StagedFile・Manifest/Dataset/Index/Chunk Commit・Stage Resume | Consumer 候補で build/test/Golden 緑 | AL #143（CLOSED）+ consumer-verify lane + AL 固有 Golden | Issue #288 Consumer 固有受入 |
| AudioLens: f64/PCM・時刻/Provenance/Validity・Artifact Hash/Canonical Bytes・Export portable path・CLI/headless stdout 非汚染・Cancel/Recovery | Consumer 固有受入 | AL 側 Issue/PR の証拠 | 同上 |
| Synvelis: Snapshot/Schema/Revision/HEAD CAS・外部変更/再 Open/Crash Recovery・AutoSave/Undo | Consumer 候補で build/test 緑 | SV #82/#84（OPEN）+ consumer-verify lane + SV 固有 Golden | 同上 |
| Synvelis: WAV PCM/Riff finalize/再 Open・ZIP/TAR Backup・Secure Import・Audio RT callback nonblocking/Xrun/Deadline | Consumer 固有受入 | SV #25（CLOSED・WAV E2E）+ SV #84 | 同上 |
| 片方 PASS/片方 not_run の集約結果を「両 Consumer 受入済み」としない | 独立判定 | consumer-verify の status を tuple/lane 単位で読む | Issue #288「実行の割当て」・`issue-294` test-spec |

## Owner・境界接続表（Implementation Issue Mapping）

IO-01〜05 のテスト要件・OS/FS/脅威モデル境界を、実装 Issue と
Consumer Issue へ接続する索引。**これが Issue #288 の
「テスト要件と Owner・OS/FS/脅威モデルの境界が実装 Issue へ
接続されている」の対応表**である。

| IO | 設計 Issue | 実装 Issue（merge 済み） | test-spec（正本） | 主な検証入口 | OS/FS・脅威モデル境界 | Consumer 接続先 |
| --- | --- | --- | --- | --- | --- | --- |
| IO-01 File | #283 | #295（契約/Mock）・#296（実 OS backend） | `issue-295-io-file-staged-publish.md`・`issue-296-io-file-os-backend.md` | `mise run test-io-file`・`check-linux`（unix backend 補助） | OS primitive（renameat2/renamex_np/MoveFileExW/link+unlink）・dir sync の FS 依存・NFS/ネットワーク FS の不確定性 | AL #143（fsutil 移行）・SV #84（DirectoryBundleStore） |
| IO-02 Structured | #284 | #297（JSON/YAML/TOML）・#298（CSV）・#299（XML） | `issue-297`・`issue-298`・`issue-299`（test-specs） | `mise run test-io-structured`（feature matrix） | backend 型非公開・InputLimits・DTD/encoding 拒否 | AL Manifest/Dataset（#139/#143）・SV Snapshot schema（#82/#84） |
| IO-03 Compression | #285 | #300（Gzip/Deflate）・#301（Zstd） | `issue-300`・`issue-301`（test-specs） | `mise run test-io-compression`（feature matrix） | frame/member 境界・checksum=整合性（真正性でない）・window/budget | tar.gz 合成・Backup 圧縮（SV #84）・Dataset 圧縮（AL） |
| IO-04 Archive | #286 | #302（ZIP）・#303（TAR）・#304（Secure Extract） | `issue-302`・`issue-303`・`issue-304`（test-specs） | `mise run test-io-file`（io-archive 含む）・`test-io-e2e` | Path Policy・canonical 衝突・budget・TOCTOU 限界・link/special 拒否 | SV ZIP/TAR Backup・Secure Import（#82/#84）・AL archive 要件（#139） |
| IO-05 Migration | #287 | #305（旧 API 撤去・merge 済み PR #391） | `issue-305-media-io-staged-file-cutover.md` | `cargo test --workspace`・旧 API grep 0 件 | Media 責務と File 責務の分離・RT 非混入 | AL #143・SV #84・SV #25（WAV E2E） |
| 合成 E2E | —（#288 配下の実装接続） | #306（`ruviform-io-e2e`・merge 済み PR #395） | `issue-306-io-composition-e2e.md` | `mise run test-io-e2e` | 実 OS temp dir・公開 API のみ・Mock fault は io-file 正本へ委譲 | 両 Consumer の保存経路と同じ層構成の Ruviform 側 Gate |

### Consumer Issue 接続（snapshot 2026-11-09）

| Consumer | Issue | 状態 | 役割 |
| --- | --- | --- | --- |
| AudioLens | #139 AL-IO-01 移行影響・回帰 | CLOSED | Manifest/Dataset/File Utility 移行の設計・要件 |
| AudioLens | #140 Git 依存→Cargo.lock 正本移行 | CLOSED | lockfile 主体依存・単一 Ruviform Revision/GUI vendor 整合 |
| AudioLens | #141 Export 絶対パス不具合 | CLOSED | **ベースライン不具合の先例**（新規退行と区別する対象） |
| AudioLens | #143 AL-IO-02 fsutil/Manifest/Dataset→StagedFile | CLOSED | 実装移行の受入 |
| Synvelis | #82 SYN-IO-01 ProjectStore/HEAD/Backup 適合境界 | OPEN | Snapshot/HEAD/Revision/CAS の設計・受入 |
| Synvelis | #84 SYN-IO-02 DirectoryBundleStore/Snapshot/HEAD→StagedFile | OPEN | 実装移行の受入（Ruviform 単一 File Publish ≠ Project Transaction/HEAD CAS） |
| Synvelis | #25 WAV Export E2E | CLOSED | WAV PCM/Riff finalize/再 Open の既存 E2E |

## Consumer 受入要件（Ruviform/AudioLens/Synvelis）

Issue #288「Consumer 固有の受入」を要件として固定する。
実施は各 Consumer repo が Owner であり、本書は要件の定義のみ。

### AudioLens

- `al-job::fsutil` → 新 File 保存、Manifest/Schema/Index/Chunk
  Commit、Stage Resume の移行と非回帰。
- f64/PCM・時刻・Provenance/Validity・Artifact Hash/Canonical
  Bytes・Export portable path の精度保持。
- CLI/headless・stdout 非汚染、Cancel/障害時 Recovery。
- lockfile 主体への依存移行（#140・CLOSED）と単一 Ruviform
  Revision/GUI vendor の整合。
- 既知 Export 不具合（#141・CLOSED）はベースライン問題として
  新規退行と区別する。

### Synvelis

- Project Snapshot/Schema/Revision/HEAD の CAS・外部変更・
  再 Open/Crash Recovery、AutoSave/Undo。
- WAV PCM/Riff Finalize/再 Open（#25・CLOSED の既存 E2E を
  非回帰の基準にする）。
- ZIP/TAR Backup と Secure Import の Project 整合。
- Audio RT callback の nonblocking・Xrun/Deadline。
- **Ruviform 単一 File Publish を Project 全体の Transaction
  や HEAD の CAS とみなさない**（Issue #288 明記）。

### 両 Consumer 共通

- 候補 Ruviform Revision で各 Consumer の対応候補を個別に
  build/test し、Lockfile・Golden・必要 OS/FS・性能・各
  Consumer 固有受入結果を確認する。
- 恒久 SHA 固定・旧 API Facade での非互換隠蔽を移行策にしない。
- 速度/Peak Memory/Allocation は変更前と同一条件で比較し、
  許容差は実測・製品要件により**比較前**に定める。

## Breaking Change 追跡導線（旧 StagedOutput/PublishOptions）

Issue #287 の決定（2026-11-04・ユーザー合意）により、旧
`StagedOutput`/旧 `PublishOptions` の互換 API・Facade・
Deprecated 共存は**作らない**。本節はその採用手順と戻し方の
追跡導線を固定する。

### 採用手順（記録済みの経緯）

1. **設計**: IO-05 #287 で API 対応表（旧→新）と Acceptance
   Gate を確定。
2. **Ruviform 実装**: #305 で旧 `staged.rs`/re-export/専用
   テスト・Docs を削除し、WAV Output・Samples・Tests・
   README・ADR・Rustdoc を新 API へ移行。PR #391 で main merge
   済み（`StagedOutput` 撤去を `git log`/grep で確認可能）。
3. **Consumer 協調**: AL #143（CLOSED）・SV #84（OPEN）で
   依存更新と旧 API 使用箇所を置換。汎用 fsutil/単一 File
   Publish 部分は Ruviform へ統合、Job/Dataset/Project 固有
   意味論は各 Consumer に残す。
4. **Cross-repo release Gate**: 候補 Ruviform SHA × 両 Consumer
   候補 Branch を consumer-verify.yml（#294）で検証。
   恒久 SHA 固定・旧 API 保持・一方 Consumer の精度退行を
   移行策にしない。

### 戻し方（Rollback）

- ロールバックは**変更単位・依存更新と連動**して準備する
  （#287 明記）。恒久的な旧 API 復活・Facade 再導入は
  Breaking Change 管理の方針に反するため、戻しは「Ruviform
  側の revert PR + Consumer 側の依存戻し」の組合せで行い、
  いずれも #287/#305 と各 Consumer Issue で追跡する。
- 旧 API が存在しないことの完了時検証: コード/Docs/Examples/
  公開 Rustdoc/依存 DAG で `StagedOutput`・旧 `PublishOptions`
  の削除を確認（#287 Acceptance Gate・`issue-305` R1）。

### 追跡先まとめ

| 段階 | 追跡先 |
| --- | --- |
| 設計・API 対応表・Acceptance Gate | Issue #287・`docs/specs|designs|test-specs/issue-305-*` |
| Ruviform 実装 | Issue #305・PR #391・`docs/tasks/issue-305-*` |
| Consumer 移行 | AL #143・SV #84（+ 関連 AL #139/#140、SV #82/#25） |
| Cross-repo 採用判定 | #219・#294・consumer-verify.yml run/PR |
| 本 Gate 定義 | 本書（#288） |

## 証拠導線（Evidence Trail）

Issue #288 の要求どおり、**既存 GitHub Actions・PR・Issue を
参照するだけで必要な検証を追える**構成とし、専用 YAML・
新 Dashboard・重複レポートは作らない。

| 検証種別 | 既存の証拠先 | 参照方法 |
| --- | --- | --- |
| Ruviform 単体（unit/negative/fault-injection） | `ci.yml`・`mise run test-io-*`/`lint-ci`/`test-ci`・各 PR | PR の Checks・mise task 名・test-spec の Rules ID |
| 合成 E2E | `mise run test-io-e2e`・`crates/ruviform-io-e2e`・PR #395 | Ruviform PR checks・`issue-306` test-spec |
| Consumer 候補 tuple 検証 | `consumer-verify.yml`（`workflow_dispatch`）・`ci/consumer-verify` evidence JSON・Job Summary | run URL・artifact の `consumer-evidence-*.json`（tuple/lane/status） |
| 日次候補 trigger | `daily-ci.yml`・#367 scheduler・#294 tuple mode | run URL・Issue 記録 |
| Consumer 固有受入 | 各 Consumer repo の CI/PR/Issue（AL #139 系・SV #82/#84/#25） | Consumer 側 PR/Issue の証拠 |
| OS/FS 別実測 | CI の OS lane（`check-linux` 等）・実機実行記録 | test-spec の「未実施・制約」節・impl 記録 |
| 文書・関係の追跡 | 本 docs チェーン・`docs/{specs,designs,plans,tasks,test-specs}/README.md` index | doc-status lint/audit |

判定時に必要で既存 CI から追えない値（OS・FS・Toolchain 等）
だけをテストログに残す。証拠の二重転記は不要。

### Gate 種別ごとの実行割当て

Issue #288「実行の割当て」に従い、検証種別を既存の実行 venue
へ割当てる。#288 内で別の実行基盤を実装しない。

| Gate 種別 | 割当て先 |
| --- | --- |
| 通常 PR（Public API/旧 API 削除/DAG・Feature） | Ruviform PR の `ci.yml` + `mise run lint-ci`/`test-ci`/feature matrix task |
| 影響のある共有 I/O 変更 | 既存 Cross-Repo Workflow（#219/#294）の候補 Ruviform SHA × 両 Consumer 候補 Branch の Check/Test/Golden |
| OS/FS 実測 | 既存 CI の OS lane（`check-linux` 等）・実機実行記録。未実施は `not run` |
| Fuzz | 各 crate の deterministic fuzz 試験（`issue-298`/`issue-299` S6 等）を既存 test task で実行。無制限 fuzz は対象外 |
| 性能 | 変更前と同一条件での比較を導入 Gate として扱い、許容差は比較前に定義（Consumer 性能要件節参照） |
| Crash Recovery / publish・recovery 否定系 | `issue-295`/`issue-296` test-spec の Mock/実 FS 試験 + `issue-306` 合成 E2E。電源断実機試験は対象外（`issue-296` Non-goals） |

## Design Done / Implementation Done の分離

- **本 slice（docs）**: Gate の判定語彙・matrix・否定系判定表・
  接続表・Consumer 要件・Breaking Change 導線・証拠導線の
  **設計合意**を文書化する。ここで「設計が決まった」ことと
  「テストが通った」ことを混同しない。
- **実装段階（別 dispatch/Issue）**: 各必須 Gate の**実走
  PASS** を要求する。matrix の実走・consumer-verify の候補
  run・Consumer 実測・macOS/Linux 実機はいずれも本 slice の
  対象外であり、未実施は `not run`/`NotRun`/`Unsupported` と
  正直に記録する。
- Issue #288 の受入条件「実装段階では各必須 Gate の実走 PASS
  を要求し、設計の合意をテスト完了と混同しない」を、本書の
  存在だけでは満たさない — 満たすのは実装段階の各 Gate 実走
  完了である。

## Risks and Trade-offs

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | matrix の行が test-spec/Issue の変動で陳腐化 | med | med | 各行は正本への参照のみ持ち、複写しない。test-spec/Issue 更新時に本書の参照を見直す（doc-status の relation 監査） |
| 2 | `unsupported` を「対象外の成功」と誤読し Consumer 必須 Capability の欠落を見逃す | med | high | 判定語彙節で `unsupported` の限定意味を固定。Consumer 必須 Capability 欠落は採用 Gate 不通過と明記 |
| 3 | 片方 Consumer のみ PASS の集約結果を「両方受入」と誤認 | med | high | Consumer 独立判定・tuple/lane 単位の status 読みを明記（#294 test-spec の status 導出と整合） |
| 4 | OS/FS 未実測を Supported と主張 | med | med | capability matrix に実測状態列を持たせ、未実測は NotRun/Unsupported と記録 |
| 5 | 既知ベースライン不具合と新規退行の混同 | low | med | 分離記録の規則を判定語彙に明記（AL #141 を先例として記録） |
| 6 | docs のみの変更が「Gate 完了」と誤解される | low | med | Design/Implementation Done 分離節で明示 |

## Deferred Design Concerns

- matrix 実走結果の集約・可視化 | Issue #288 は独自証拠基盤を
  作らない方針 | #294 consumer-verify 拡張・#219 運用確定時 |
  実走結果が Issue/PR に散在し追跡コストが問題化した場合 |
- Network/Cloud FS の保証範囲実測 | 実機・環境未確保 | Consumer が
  該当 FS をサポート対象に含める判断時 | 未実測のまま Supported
  誤認のリスク |
- macOS/Linux 実機 OS/FS 別 Gate 実走 | 本 slice 環境は Windows
  のみ | CI 既存 OS lane または実機環境が利用可能に
  なった時点 | APFS/ext4 の DirSync 等の OS 差異が未検証の
  まま残る |
- 性能許容差の具体値 | 実測・製品要件が前提 | Consumer が性能
  Gate を正式に要求した時点 | 暗黙の性能犠牲を見逃すリスク |

## Verification Notes

- [ ] 受入 matrix が IO-01〜05 の正常系/否定系を Issue #288
      表の粒度で網羅し、各行が判定基準と既存正本への参照を
      持つこと。
- [ ] 否定系判定表 5 領域（publish/recovery・安全性・
      resource budget・Media/PCM/RT・Consumer 非回帰）が
      存在し、判定語彙と整合すること。
- [ ] Owner/境界接続表の Issue 番号・test-spec パス・mise
      task 名が実在すること（作成時に `docs/`・`mise.toml`・
      Issue state を確認済み）。
- [ ] 証拠導線が既存 Actions/PR/Issue のみを参照し、新規
      YAML/Dashboard/重複レポートを追加していないこと。
- [ ] 「実装段階の実走 PASS 要求・設計合意≠テスト完了」が
      明記されていること。

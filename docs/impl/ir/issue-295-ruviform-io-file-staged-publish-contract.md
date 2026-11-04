---
id: "IMPL-034RwfDSIjD2OHsi69VaSc"
type: "impl"
status: "completed"
title: "Issue 295 ruviform-io-file staged publish contract"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/295"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/options.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/outcome.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/platform.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/mock.rs"
      - type: "file"
        path: "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
      - type: "file"
        path: "docs/specs/issue-295-io-file-staged-publish-contract.md"
      - type: "file"
        path: "docs/designs/issue-295-io-file-staged-publish.md"
      - type: "file"
        path: "docs/plans/issue-295-io-file-staged-publish.md"
      - type: "file"
        path: "docs/test-specs/issue-295-io-file-staged-publish.md"
      - type: "file"
        path: "docs/tasks/issue-295-t1-crate-and-contract.md"
      - type: "file"
        path: "docs/tasks/issue-295-t2-ledger-verify.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "crates/README.md"
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
  implements:
    - "docs/tasks/issue-295-t1-crate-and-contract.md"
    - "docs/tasks/issue-295-t2-ledger-verify.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/plans/issue-295-io-file-staged-publish.md"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/296"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-295-io-file-staged-publish.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/296"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 295 ruviform-io-file staged publish contract

## Summary

IO-IMP-01（Issue #295）として新規独立クレート `crates/ruviform-io-file` を
新設し、IO-01（Issue #283）v0.4 の staged-publish 公開契約を型状態で実装
した。`StagedFile`（`Write + Seek + Send`・temp 書込み面）→ `seal` →
`SealedFile`（Write/Seek 非保持・`open_readback(&mut self)` の `Read +
Seek` borrow・`publish`・`abort`）の所有権遷移、`PublishOptions` の
mode/visibility/durability 独立3軸、`PublishReceipt` / `PublishFailure`
（`PublishEffect` 三分類・`PublishPhase`・`DurabilityEvidence`・
`RecoveryHandle`）を確立し、private `PlatformFileOps` 境界 +
`#[cfg(test)]` Fault Inject Mock 駆動で決定的な publish 分類パイプライン
を unit test 行列（26 件）で検証した。実 OS atomic publish・durability
実装は IO-IMP-02（Issue #296）へ明示的に委譲し、本クレートは契約確立に
留めている。

## Context

- 実施タスク: [T1](../../tasks/issue-295-t1-crate-and-contract.md) /
  [T2](../../tasks/issue-295-t2-ledger-verify.md)
- 上流: [plan](../../plans/issue-295-io-file-staged-publish.md)、
  [design](../../designs/issue-295-io-file-staged-publish.md)、
  [spec](../../specs/issue-295-io-file-staged-publish-contract.md)、
  親設計 Issue #283（IO-01 v0.4 契約 — v0.4 が正本、旧 `writer() -> &mut
  File` 案は禁止）
- 契約保証の正本:
  [test-spec](../../test-specs/issue-295-io-file-staged-publish.md)
- 境界: 実 OS adapter・temp 命名・fsync/fdatasync・rename/replace 実装は
  Issue #296。旧 `ruviform-media-io::StagedOutput` の撤去と消費者移行は
  Issue #305。構成/OS/障害/消費者受入は Issue #306。AudioLens #143・
  Synvelis #84 は下流消費者（本 Issue は契約起点のみ）。
- 制約: 依存なし（std のみ）・`default = []`・全 API 非 RT・MSRV 1.89・
  公開 enum は `#[non_exhaustive]`・unsafe 不使用・`File`/clone 可能
  ハンドル非公開。

## Implementation

### Changes

- `crates/ruviform-io-file/Cargo.toml` — workspace package 継承・
  `[features] default = []`・依存なし・`lints workspace`。
- `src/lib.rs` — crate doc（ライフサイクル・保証対要求の区別・IO-IMP-01
  スコープの明示・非 RT 契約・`no_run` 使用例 2 件）、モジュール宣言、
  公開再エクスポート。
- `src/error.rs` — `FileErrorKind`（`Io`/`Unsupported`/`InvalidInput`）・
  `FileError`（normalized kind + type-erased source）。`io::ErrorKind::
  Unsupported`/`InvalidInput` の正規化規約。
- `src/options.rs` — `PublishMode`（`CreateNew`/`Replace`）・
  `VisibilityRequirement`（`RequireAtomic`/`BestEffort`）・`Durability`
  （`BestEffort`/`FileSync`/`FileAndDirectorySync`）・`PublishOptions`
  （3 独立軸・`new` は RequireAtomic を既定、`with_visibility` で緩和）。
- `src/outcome.rs` — `OperationId`（診断用 correlation id）・
  `PublishEffect`（`NotPublished`/`Published`/`Indeterminate`）・
  `PublishPhase`（Validate/FileSync/Close/Publish/DirectorySync/Cleanup）・
  `DurabilityEvidence`（要求値ではなく観測証跡）・`TempState`・
  `PublishFailureKind`・`PublishFailure`（`take_recovery_handle` で
  recovery 所有権を移譲。Drop は temp を削除しない）。
- `src/platform.rs` — private `PlatformFileOps` trait（write/flush/seek・
  `open_readback`・`validate_publish`・`sync_file`・`prepare_for_publish`・
  `publish`・`sync_directory`・`cleanup_temp`・`inspect_temp`）、
  `ApplyOutcome`/`TempDisposition`/`PublishAttempt`/`CleanupAttempt`
  分類語彙、`ReadSeek` object 面、`staged_file` factory（IO-IMP-02 まで
  `Unsupported` を返す stub）。
- `src/receipt.rs` — `RecoveryInfo`（非破壊診断 snapshot）・
  `RecoveryObservation`・`RecoveryHandle`（inspect/cleanup・Drop は削除
  しない）・`CleanupReceipt`/`CleanupFailure`（handle を返して retry 可能）・
  `PublishReceipt`（cleanup 未完了時は receipt に handle を保持）。
- `src/staged.rs` — `StageOwner`（`Option<Box<dyn PlatformFileOps>>` +
  `RecoveryInfo` + `cleanup_on_drop`、唯一の Drop 実装）、`StagedFile`
  （`create`・`Write`/`Seek` 実装・`seal`・`abort`）、`SealedFile`
  （`open_readback`・`publish`・`abort`）、`ReadbackFile<'_>`（`Read +
  Seek` の非 File borrow・position 0 開始）、`publish_pipeline`（validate →
  file sync → close → publish → dir sync → cleanup の決定的分類）、
  unit test 行列 26 件・`Write + Seek + Send` const 静的検証。
- `src/mock.rs` — `#[cfg(test)]` `MockFileOps`（仮想 FS: temp/dest/呼出
  ログを `Arc<Mutex>` 共有・物理適用と報告を分離できる `MockPublish`）、
  `MockScript`（各段階の注入・消費後は happy path 既定）。
- `Cargo.toml` — members に `crates/ruviform-io-file` をメディア I/O より
  下位の汎用ファイル I/O 基盤として登録。`Cargo.lock` 再生成。
- `crates/README.md` — 「汎用ファイル I/O（非 RT 専用）」セクション新設・
  クレート数を実態（33）に更新。
- `AGENTS.md` — クレート数記載を 33 に同期（従来値 29 は過去追加分未反映
  の stale だったため併せて訂正）。
- `docs/` — spec・design・plan・test-spec・task×2・本 IR と生成索引を
  doc-driven-dev-graph チェーンで整備。

### Decisions During Implementation

- **`PublishEffect` 三分類は adapter 自身の報告に立脚**: `publish` op は
  `Result` ではなく `ApplyOutcome`（`Applied`/`NotApplied`/`Unknown`）+
  `TempDisposition` + 任意の `io::Error` を返す契約とし、ambiguous OS
  エラーが `NotPublished` に正規化されて嘘の「未適用」を出さない構造に
  した。physical 適用と report を分離できる Mock で「適用済みだが結果
  不明 → Indeterminate」を再現検証。
- **`Ok(PublishReceipt)` は要求保証の全達成のみ**: dir sync 失敗は
  `Published` + `Err`（receipt ではなく failure で evidence =
  `FileSynced`）— 要求 durability を達成しなかった成功は一切返さない。
- **Indeterminate/失敗の temp は自動削除しない**: `RecoveryHandle` が
  adapter object を保持し、その Drop は削除しない（明示 `cleanup()` のみ
  削除）。一方、`StagedFile`/`SealedFile` の `cleanup_on_drop` Drop は
  未 publish の temp のみ best-effort 掃除し、Indeterminate 残存物には
  関与しない。
- **`StageOwner` は `ops: Option<Box<dyn>>`**: Drop 型からフィールドを
  move できない制約に対し、`Option::take` で adapter を outcome の
  `RecoveryHandle` へ移譲する（`StagedFile`/`SealedFile` 自体に Drop は
  実装せず `StageOwner` に集約 — review-protocol で検出した P1 の修正）。
- **`SealedFile` は `Write`/`Seek` を実装しない・`ReadbackFile` は
  `&mut` borrow**: readback は同一 staged object の `Read + Seek` で
  position 0 開始。匿名 path 再 open（TOCTOU）は契約上の安全な実装と
  偽装せず、adapter が安全な read surface を提供できなければ
  `Unsupported`。compile-fail doctest で `Write`/`Seek` 呼出禁止を検証。
- **`ReadSeek` は private・`reader()` は `impl Read + Seek`**: trait object
  の multi-bound upcast は不安定なため `ReadSeek` supertrait 経由で内部
  object を運び、公開 accessor は opaque `impl Read + Seek + '_` を返す
  （`private_interfaces` 回避・File 型も clone 可能 handle も露出しない）。
- **`RecoveryInfo` は診断 snapshot に限定**: `OperationId`/`temp_path` は
  lock・CAS token・identity 証明ではないと rustdoc で明記。handle 移譲で
  二重削除・置換済み temp identity の誤削除を構造防止。
- **enum variant の `dead_code` は `cfg_attr(not(test), allow)`**: variant
  構築は adapter 側の責務で、crate 内唯一の adapter が test Mock のため
  lib ビルドでは未構築警告になる — 実 OS adapter 着工（#296）までの
  計画済み境界として対象限定の allow を明記コメント付きで適用。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64（`x86_64-pc-windows-msvc`、mise pin rustc 1.99.0、
MSRV 要求は 1.89）・Git Bash。`git fetch origin` → `git merge origin/main`
（Already up to date — 分岐点 = main tip）後に task の `## Verification`
全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `mise run fmt` → `mise run fmt-check` | PASS・差分なし |
| 型チェック | `mise run check`（`cargo check --workspace --all-targets`） | PASS・exit 0 |
| lint | `mise run lint`（`cargo clippy --workspace --all-targets -- -D warnings`） | PASS・0 warnings（新クレート由来の clippy 指摘 8 種は全て実修正で解消） |
| テスト | `mise run test`（`cargo test --workspace`） | PASS・全クレート緑。`ruviform-io-file` = unit 26 件 + doctest 10 件（内 compile_fail 2 件）全合格 |
| compile-time 契約 | const fn 静的検証 + `compile_fail` doctest | `StagedFile: Write + Seek + Send`・`ReadbackFile: Read + Seek` を const 検証、`SealedFile` への `write_all`/`seek` 呼出が compile_fail であることを確認 |
| ワークスペース台帳 | `crates/README.md`・`AGENTS.md`・`Cargo.lock` | 計33クレートで整合・lockfile に `ruviform-io-file` 0.1.0 登録 |

未実施: `mise run verify-ci`（dev プロファイルの fmt-check/check/lint/test
を全て実施済み。CI プロファイル再検証は同等ゲートの再実行のため今回は
割愛）。実 OS での Atomic `CreateNew`・fsync/directory sync 達成・temp 命名
・復旧処理は Issue #296 のゲートに委ね、本 Issue では主張しない。

## Risks

- `StagedFile::create` は OS backend 未実装のため現行では常に
  `FileErrorKind::Unsupported` を返す — 公開契約と Mock 検証のみが本
  Issue の成果物であり、実ファイルを作れるのは #296 以降（rustdoc で
  `no_run` + 明示記載済み）。
- `PlatformFileOps` の分類語彙は #296 の OS adapter が初の実利用者 —
  `cfg_attr(not(test), allow(dead_code))` を `ApplyOutcome`/
  `TempDisposition` に限って適用済み（#296 で除去すべき既知の暫定措置）。
- 公開 enum は全て `#[non_exhaustive]` — variant 追加余地を残すが、
  消費者側の `match` は `..` arm が必須になる点に注意。

## Follow-ups

- Issue #296（IO-IMP-02）: 実 OS `PlatformFileOps` 実装（atomic
  `CreateNew`/`Replace`、file/directory sync、temp 命名・identity、
  匿名ハンドル readback or 明示 `Unsupported`）と `dead_code` 暫定 allow
  の除去。
- Issue #305: 旧 `ruviform-media-io::StagedOutput`・旧 `PublishOptions` の
  撤去と WAV readback→publish の本 API への移行（compat layer なし）。
- Issue #306: 構成・OS・fault・security・consumer 受入ゲート。
- AudioLens #143・Synvelis #84: 本契約を起点に各 consumer 側で移行実装
  （現時点では契約未消費 — 影響分類は下流作業の前提確立のみ）。

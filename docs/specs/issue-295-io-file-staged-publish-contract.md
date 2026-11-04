---
id: "SPEC-034RoR8tcezgggBLlNk7od"
type: "spec"
status: "approved"
title: "Issue #295 ruviform-io-file — StagedFile 型状態・公開契約・Mock Publish (IO-IMP-01)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
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

# Issue #295 ruviform-io-file — StagedFile 型状態・公開契約・Mock Publish (IO-IMP-01)

## What Should Be Built

独立クレート `crates/ruviform-io-file` を新設し、Workspace に登録する。
IO-01（Issue #283）の **v0.4 合意済み契約**を実装基準として、型状態
（type-state）による安全な staged publish の公開 API・結果/失敗モデル・
回復所有権を確立する。

公開面:

- `StagedFile::create(destination) -> Result<StagedFile, FileError>` —
  宛先の sibling な一時ファイルへの書き込み面を開く。`StagedFile` は
  `std::io::Write + Seek + Send` を実装し、内部の OS ファイルハンドルや
  clone 可能な Handle を一切公開しない。
- `StagedFile::seal(self) -> SealedFile` — 書込終了の型遷移。
  `SealedFile` は `Write`/`Seek` を実装せず、`publish`/`abort`/
  `open_readback` のみを持つ。
- `SealedFile::open_readback(&mut self) -> Result<ReadbackFile<'_>, FileError>`
  — 書込権限を与えない `Read + Seek` の借用 Reader。Reader は Temp と
  同一の対象を指し、borrow 生存中は `publish(self)` が所有権上移動できない
  （borrow 契約で publish を禁止）。安全な Readback を提供できない環境は
  `Unsupported` を返し、匿名 Path 再 Open による TOCTOU を安全と偽装しない。
- `SealedFile::publish(self, PublishOptions) -> Result<PublishReceipt, PublishFailure>`
  — `PublishOptions` は独立 3 軸: `PublishMode::{CreateNew,Replace}`、
  `VisibilityRequirement::RequireAtomic`（初期はこれのみ）、
  `Durability::{BestEffort,FileSync,FileAndDirectorySync}`。
- `PublishReceipt` — 公開成功時のみ。`OperationId`・宛先・**実績**の
  `DurabilityEvidence`・公開後の Temp 状態・残留時の `RecoveryHandle`。
- `PublishFailure` — `kind`・`phase`（`PublishPhase`）・`effect`
  （`PublishEffect::{NotPublished,Published,Indeterminate}`）・
  到達済み `DurabilityEvidence`・`RecoveryInfo`・`Option<RecoveryHandle>`・
  `source()`。`take_recovery_handle` で Handle を移譲可能。
- `StagedFile::abort` / `SealedFile::abort` →
  `Result<CleanupReceipt, CleanupFailure>`。未公開の Drop は
  best-effort cleanup。公開結果が不明（Indeterminate）な残留物を
  Destructor で無条件削除しない。`RecoveryHandle` の Drop も削除しない。

OS 操作は private `PlatformFileOps` 抽象越しにのみ行い、本 Issue では
Fault Inject Mock のみで全契約を駆動する。実 OS 実装
（Atomic CreateNew・Atomic Replace・File/Directory Sync・Temp identity）は
IO-IMP-02（Issue #296）へ委譲し、本 Issue ではその達成を主張しない。

## Why Now

共通 I/O 基盤チェーン（Issue #282）の起点。AL #143・SYN #84・RF #305/#306
が `StagedFile`/`SealedFile`/`PublishReceipt`/`PublishFailure` の
公開契約を前提とするため、型と結果モデルを先に固定しないと後続の
実装 Issue（#296・#297・#300・#302）が着手できない。旧
`ruviform-media-io::StagedOutput` は IO-05（#287）で互換層なしに廃止
されるため、正本となる File API の契約が必要。

## Users and Value

- AudioLens: `al-job::fsutil` の durable write / atomic rename / completion
  marker の移行先（AL-IO-02 / audio-lens #143）。
- Synvelis: `DirectoryBundleStore` の Snapshot→HEAD 保存、WAV 公開前
  Reopen/Verify（synvelis #25・#82・#84）。
- Ruviform 内部: `ruviform-io-wav` 等のバックエンドが Format `finish()` 後に
  使う汎用保存プリミティブ（#305 の移行先）。

## Scope

### In Scope

- `crates/ruviform-io-file` 新設・Workspace 登録・`crates/README.md` /
  AGENTS.md のクレート台帳更新。
- 上記公開 API（型・所有権・Result・Error source）のコンパイル可能な定義と
  rustdoc（典型使用例・保証境界・non-RT 明記）。
- private `PlatformFileOps` trait と Fault Inject Mock による契約駆動
  （`Applied/NotApplied/Unknown` × `Removed/Retained/Unknown`、
  各段階の故障注入）。
- `Write+Seek+Send` の静的検証、`SealedFile` への書込禁止の
  compile-fail テスト、Mock 行列の unit test。
- `StagedFile::create` は実 OS バックエンド未着のため
  `FileErrorKind::Unsupported` を返す状態で公開（IO-IMP-02 で実装）。
  公開シグネチャ・所有権・ドキュメント契約は本 Issue で確定する。

### Out of Scope

- 実 OS 実装（`renameat2`/`renamex_np`/`MoveFileExW` 等、capability
  probing、実機 FS 試験）→ IO-IMP-02（Issue #296）。
- 旧 `StagedOutput`/旧 `PublishOptions` の削除・Consumer 移行 → IO-05
  （#287）/ IO-IMP-11（#305）。
- Consumer 独自 Schema/意味論（Manifest・HEAD/Revision・CAS・Journal）。
- Media/Codec/Serde 依存、ネットワーク/リモート FS、複数ファイル
  トランザクション、汎用 Byte Stream 規格。
- 既存 CI #219/#294 の再実装。

## Requirements

- `default = []`（feature 最小）、依存クレートなし（std のみ）、non-RT のみ。
- MSRV 1.89・edition 2024・`workspace.lints` 継承。
- クレート外から到達可能な全公開 enum に `#[non_exhaustive]`。
- `unsafe` 不使用。`Write+Seek` は標準 trait のみで独自 Byte Stream trait
  を作らない。
- `Write::flush`・File Sync・Directory Sync・Atomic Visibility・
  Power-failure durability を別の保証として明示し、要求値を実績として
  偽装しない。
- `exists()`→`rename()` を原子的 CreateNew の根拠にしない。
- `Indeterminate` 時の自動再 publish・自動 cleanup・rollback を行わない。

## Acceptance Criteria

- [ ] AC1: `StagedFile: Write+Seek+Send`、`seal()`→`publish()`/`abort()`/
      `open_readback()` の所有権遷移がコンパイル可能な公開 API として存在し、
      典型使用例が rustdoc にある。
- [ ] AC2: `SealedFile` への `Write` 呼出しがコンパイル不可であること
      （compile-fail doctest または静的検証）、`StagedFile` が `Write+Seek+Send`
      を満たす静的検証がある。
- [ ] AC3: Mock で `NotPublished`/`Published`/`Indeterminate` × Temp 状態
      （`Removed`/`Retained`/`Unknown`）× 同期段階（FileSync/Publish/DirSync/
      Cleanup 失敗）の組合せが期待どおり分類され、`Ok` が要求達成時のみ返る
      ことを unit test で検証する（虚偽の成功を出さない）。
- [ ] AC4: Drop/Abort/Cleanup 失敗時の所有権（`CleanupFailure` が Handle を
      保持し再試行可能）・Indeterminate 時に自動削除されないことを unit test
      で検証する。
- [ ] AC5: Media/Codec/Serde 依存なし・non-RT・MSRV 1.89・`default=[]`・
      rustdoc・`mise run lint`/`test` が通る。
- [ ] AC6: 実 OS での Atomic CreateNew・Durability 達成は本 Issue で主張せず、
      `create` の Unsupported 応答と rustdoc で IO-IMP-02 への委譲を明示する。

## Deferred Design Concerns

- `PublishMode` の条件付き置換（Revision/CAS 前提の Conditional Replace）|
  IO-01 v0.3 で留保・Consumer 側 Revision 管理の実需が固まってから |
  現行の無条件 Create-or-Replace で誤用するリスク。
- Hard-link による CreateNew 代替・OS 別 capability matrix | IO-IMP-02 の
  実測待ち | 未検証の保証を Supported と誤記するリスク。
- `RecoveryHandle` のクラッシュをまたぐ再取得・永続 Journal | Consumer 側
  責務（Synvelis/AudioLens の保存モデル）| 診断値を同一性証拠と誤認する
  リスク。

## Sources and Relations

- 正本: Issue #295 本文（作業範囲・Acceptance・横断レビュー追補）。
- 設計基準: Issue #283（IO-01）v0.2/v0.3/v0.4 合意済み追補 + 最終レビュー
  追補（Readback Reader）、Issue #282（統合設計正本・4 独立クレート方針）。
- 先行実装の参考: `crates/ruviform-media-io/src/staged.rs`（旧
  `StagedOutput`。IO-05/#287 で互換層なし廃止予定のため参照のみで依存しない）。

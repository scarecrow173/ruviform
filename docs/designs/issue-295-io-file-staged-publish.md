---
id: "DESIGN-034RoR9SDtSG2iiZbvNGKT"
type: "design"
status: "approved"
title: "Issue #295 ruviform-io-file — StagedFile type-state・PublishOptions・Receipt/Failure・PlatformFileOps/Mock 設計"
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
      - type: "file"
        path: "crates/ruviform-io-file/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/options.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/outcome.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/platform.rs"
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
  depends-on:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/media-io-backend-abstraction.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #295 ruviform-io-file — StagedFile type-state・PublishOptions・Receipt/Failure・PlatformFileOps/Mock 設計

## Context

Issue #295（IO-IMP-01）は共通 I/O 基盤チェーンの起点であり、IO-01
（Issue #283）の v0.4 合意済み契約をコンパイル可能な Rust 公開契約へ
固定する。実 OS バックエンド（atomic rename・directory sync・capability
probing）は IO-IMP-02（Issue #296）が private `PlatformFileOps` 抽象の
実装として追加する。本設計の主眼は「OS を切り離した契約駆動」:

- 公開型（`StagedFile`/`SealedFile`/`PublishOptions`/`PublishReceipt`/
  `PublishFailure`/`RecoveryHandle` 等）の所有権・Result 形状を確定する。
- private `PlatformFileOps` による OS 操作の完全抽象化 — `Applied /
  NotApplied / Unknown` と `Removed / Retained / Unknown` の分類語彙を
  アダプタ契約として定義する。
- Fault Inject Mock で全失敗行列を決定的に駆動し、虚偽の成功を返さない
  分類ロジック（パイプライン）を検証する。

`StagedFile::create` は既定の OS バックエンド未実装のため
`FileErrorKind::Unsupported` で失敗する状態で公開し、rustdoc で
IO-IMP-02 への委譲を明示する（虚偽の利用可能性を示さない）。

## Scope

### In Scope

- 新クレート `crates/ruviform-io-file`（std のみ・依存ゼロ・`default = []`）。
- 公開型: `StagedFile`・`SealedFile`・`ReadbackFile<'a>`・`PublishOptions`
  （`PublishMode`/`VisibilityRequirement`/`Durability`）・`PublishReceipt`・
  `PublishFailure`（`PublishFailureKind`/`PublishPhase`/`PublishEffect`）・
  `DurabilityEvidence`・`TempState`・`OperationId`・`RecoveryInfo`・
  `RecoveryHandle`・`RecoveryObservation`・`CleanupReceipt`・`CleanupFailure`・
  `FileError`/`FileErrorKind`。
- private `PlatformFileOps` trait・`ApplyOutcome`/`TempDisposition`/
  `PublishAttempt`/`CleanupAttempt` 分類語彙・`StageOwner` パイプライン。
- `#[cfg(test)]` の Fault Inject Mock（in-memory temp/dest + 段階別注入）。
- unit test 行列・compile-fail doctest・静的 trait 検証。
- ワークスペース登録（root `Cargo.toml` members・`crates/README.md`・
  `Cargo.lock`・AGENTS.md クレート数）。

### Out of Scope

- 実 OS の `PlatformFileOps` 実装・OS/FS capability matrix・実機検証
  （IO-IMP-02 / Issue #296）。
- 旧 `StagedOutput` 廃止・Consumer 移行（IO-05 / IO-IMP-11）。
- Mock のクレート外公開（`#[cfg(test)]` 限定。必要になれば別 Issue で
  testing support 面を設計する）。

## Components and Boundaries

### クレート配置

`ruviform-io-file` は依存ゼロ（std のみ）の汎用 I/O 基盤であり、
`crates/README.md` に新設する「汎用 I/O 基盤（非 RT 専用）」層に配置する。
メディア I/O バックエンド（`ruviform-io-wav` 等）や利用側層が下向きに依存
できる。上位層への依存・RT 経路からの呼出しは禁止。

### モジュール構成

| ファイル | 責務 |
| --- | --- |
| `src/lib.rs` | crate-level doc（保証境界・non-RT・IO-IMP-02 委譲）と re-export |
| `src/error.rs` | `FileError`/`FileErrorKind` — create/readback/inspect 系の失敗 |
| `src/options.rs` | `PublishOptions`・`PublishMode`・`VisibilityRequirement`・`Durability` |
| `src/outcome.rs` | `OperationId`・`PublishEffect`・`PublishPhase`・`DurabilityEvidence`・`TempState`・`PublishFailureKind`・`PublishFailure` |
| `src/receipt.rs` | `PublishReceipt`・`RecoveryInfo`・`RecoveryObservation`・`RecoveryHandle`・`CleanupReceipt`・`CleanupFailure` |
| `src/staged.rs` | `StagedFile`・`SealedFile`・`ReadbackFile`・内部 `StageOwner` と publish パイプライン |
| `src/platform.rs` | private `PlatformFileOps`・`ApplyOutcome`・`TempDisposition`・`PublishAttempt`・`CleanupAttempt`・既定 backend factory（現時点は Unsupported） |
| `src/mock.rs` | `#[cfg(test)]` の in-memory `MockFileOps` と注入スクリプト |

### 公開型の所有権構造

```text
StagedFile ─seal()→ SealedFile ─publish(opts)→ PublishReceipt (Ok)
                                         └─→ PublishFailure  (Err)
StagedFile / SealedFile ─abort()→ CleanupReceipt | CleanupFailure
SealedFile ─open_readback(&mut)→ ReadbackFile<'_> (Read+Seek, borrow 中は publish 不可)
PublishFailure / PublishReceipt ─take_recovery_handle()→ RecoveryHandle
RecoveryHandle ─inspect()→ RecoveryObservation / ─cleanup()→ CleanupReceipt | CleanupFailure
```

- `StageOwner`（private）が `Option<Box<dyn PlatformFileOps>>`・
  `RecoveryInfo`・`cleanup_on_drop` フラグを保持し、
  `StagedFile`→`SealedFile` の型遷移で値ごと引き継がれる。
  `Drop` は `StageOwner` にのみ実装する（`cleanup_on_drop` 時に
  best-effort `cleanup_temp`）。`StagedFile`/`SealedFile` は `Drop` を
  実装しないため、`seal`/`publish`/`abort` で `owner` を値移動できる。
  `StageOwner` が `Drop` 型であるため、publish パイプライン内では
  `ops.take()` で所有権を `RecoveryHandle` へ移譲する（フィールドの
  直接 move は不可）。これにより二重 cleanup・循環所有は起きない。
- `RecoveryHandle` は `Box<dyn PlatformFileOps>` と `RecoveryInfo` を所有し、
  Drop で自動削除しない（契約: 公開結果不明の残留物を無条件削除しない）。
- `CleanupFailure` は必ず `RecoveryHandle` を返す（再試行可能）。

### `PlatformFileOps`（private adapter 契約）

```rust
pub(crate) trait PlatformFileOps: Send {
    // 書込面（temp）: std::io エラーは io::Error のまま返す
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()>;
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64>;

    // Readback: Temp と同一対象への Read+Seek Reader を借用。
    // 安全に提供できない場合は Err(io::ErrorKind::Unsupported)。
    fn open_readback(&mut self) -> Result<&mut dyn ReadSeek, io::Error>;

    // publish 前検証（capability/要求の充足）— phase: Validate
    fn validate_publish(&self, options: &PublishOptions) -> io::Result<()>;
    // 公開前ファイル同期 — phase: FileSync
    fn sync_file(&mut self) -> io::Result<()>;
    // ハンドル整理（close/flush 等）— phase: Close
    fn prepare_for_publish(&mut self) -> io::Result<()>;
    // 原子 publish 試行 — phase: Publish
    fn publish(&mut self, mode: PublishMode) -> PublishAttempt;
    // 公開後ディレクトリ同期 — phase: DirectorySync
    fn sync_directory(&mut self) -> io::Result<()>;
    // temp 削除（abort/drop/公開後残留）— phase: Cleanup
    fn cleanup_temp(&mut self) -> CleanupAttempt;
    // RecoveryHandle::inspect 用の temp 状態照会
    fn inspect_temp(&self) -> io::Result<TempDisposition>;
}

pub(crate) struct PublishAttempt {
    pub effect: ApplyOutcome,      // Applied | NotApplied | Unknown
    pub temp: TempDisposition,     // Removed | Retained | Unknown
    pub error: Option<io::Error>,  // OS エラー詳細（あれば source へ）
}
pub(crate) struct CleanupAttempt {
    pub temp: TempDisposition,
    pub error: Option<io::Error>,
}
```

- アダプタの失敗語彙は `io::Error` に正規化し、`io::ErrorKind::Unsupported`
  → `FileErrorKind::Unsupported`/`PublishFailureKind::Unsupported`、
  `AlreadyExists` → `PublishFailureKind::DestinationExists`、その他 → `Io`
  に写像する。OS 固有 error 型は公開しない。
- `publish` の戻り値は `Result` ではなく `PublishAttempt`:
  「呼出し自体が失敗」ではなく「公開効果の分類」を返す設計で、
  `Unknown` を `Indeterminate` に正規化し虚偽の `NotPublished` を避ける。
- `TempDisposition` は内部語彙。公開側は `TempState::{Absent,Retained,
  Unknown}` に写像する（Removed→Absent）。

### publish パイプライン（StageOwner）

基本順序（IO-01 v0.4 §4）: 要求/Capability 検証 → 必要な File Sync →
Handle 整理 → Atomic Publish → 必要な Directory Sync → 残留 Temp Cleanup
→ Receipt/Failure。

| 段階 | 条件/結果 | 結果 |
| --- | --- | --- |
| Validate | `validate_publish` Err | `Err(Failure{phase:Validate, effect:NotPublished, evidence:None})` |
| FileSync | `durability >= FileSync` かつ `sync_file` Err | `Err(Failure{phase:FileSync, effect:NotPublished, evidence:None})` |
| FileSync | 成功 | evidence = `FileSynced` |
| Close | `prepare_for_publish` Err | `Err(Failure{phase:Close, effect:NotPublished, evidence})` |
| Publish | `effect=NotApplied` | `Err(Failure{phase:Publish, effect:NotPublished, kind: DestinationExists または Io})` |
| Publish | `effect=Unknown` | `Err(Failure{phase:Publish, effect:Indeterminate, info.temp_state=attempt.temp})` |
| Publish | `effect=Applied` | 次段へ |
| DirectorySync | `durability == FileAndDirectorySync` かつ `sync_directory` Err | `Err(Failure{phase:DirectorySync, effect:Published, evidence:FileSynced})` |
| DirectorySync | 成功 | evidence = `FileAndDirectorySynced` |
| Cleanup | `attempt.temp=Removed` | `Ok(Receipt{temp_state:Absent, evidence})` |
| Cleanup | `attempt.temp∈{Retained,Unknown}` → `cleanup_temp` → Removed | `Ok(Receipt{temp_state:Absent, evidence})` |
| Cleanup | `cleanup_temp` → Retained/Unknown | `Ok(Receipt{temp_state:同じ, evidence, recovery:Some(handle)})` |

不変条件:

- `Ok(PublishReceipt)` は要求した公開・同期がすべて達成された場合のみ
  （dir sync 失敗時に Ok + 低い evidence を返さない。`Err(effect=Published)`）。
- `NotPublished` は OS 契約上「公開されていない」が確定した場合のみ
  （`NotApplied` か publish 未到達）。曖昧さは `Indeterminate`。
- `DurabilityEvidence` は要求値ではなく成功した同期段階のみ記録する。
- Failure の `RecoveryHandle` は常に付随（`Option` 型だが現行は常に Some。
  将来 temp を持たない失敗面ができた場合に None を許す余地を残す）。

### Readback（横断追補）

- `SealedFile::open_readback(&mut self) -> Result<ReadbackFile<'_>, FileError>`。
  `ReadbackFile<'a>` は `&'a mut dyn ReadSeek`（private trait
  `ReadSeek: Read + Seek`）を抱える薄い型で、`Read + Seek` のみ実装。
  `Write` 実装なし・`File`/`&File`/clone 可能 handle の露出なし。
- Reader の初期位置は 0（rustdoc で契約化）。Temp と同一対象を指すことは
  adapter 契約の一部（`open_readback` が `&mut dyn ReadSeek` を返す設計で、
  匿名 Path 再 Open による TOCTOU を構造的に排除）。
- borrow 生存中は `SealedFile` が `&mut` 借用されているため
  `publish(self)`/`abort(self)`/`open_readback` の再呼出しはコンパイル不可。
- アダプタが安全な readback を提供できない場合は `Err(Unsupported)`。

### `StagedFile::create` と既定 backend

```rust
pub fn create(destination: impl AsRef<Path>) -> Result<Self, FileError>
```

- destination が file name を持たない（root/dir/末尾 `..` 等）場合は
  `FileErrorKind::InvalidInput`。
- その後 private `platform::staged_file(destination)`（既定 backend
  factory）を呼ぶ。本 Issue では backend 未実装のため
  `io::ErrorKind::Unsupported` → `FileErrorKind::Unsupported` を返す。
  rustdoc に「OS backend は IO-IMP-02（#296）で実装。それまで
  `create` は `Unsupported` を返す」と明記する。
- テストは crate-private の `StagedFile::with_ops(Box<dyn PlatformFileOps>,
  RecoveryInfo)`（`#[cfg(test)]`）で Mock を注入し、型状態・パイプラインを
  全経路駆動する。

## Data and Control Flow

```text
create(dest) ──Unsupported──> Err(FileError)     [本 Issue の公開入口]
   │
   └─(test: with_ops)──> StagedFile ──Write+Seek──> temp bytes
        │ seal()                                  │
        ▼                                         │
     SealedFile ─open_readback(&mut)─> ReadbackFile (pos=0, Read+Seek)
        │
        ├─abort()─────────────> cleanup_temp ─> CleanupReceipt | CleanupFailure(+handle)
        │
        └─publish(opts)─> Validate→FileSync→Close→Publish→DirSync→Cleanup
                          ├─ 各 Err ─> PublishFailure{phase,effect,evidence,info,handle,source}
                          └─ 全成功 ─> PublishReceipt{op_id,dest,evidence,temp_state,handle?}
        │
     Drop(未 publish)──> cleanup_on_drop=true ─> best-effort cleanup_temp
     Drop(RecoveryHandle / 結果不明)──> 削除しない
```

## Non-Functional Constraints

- 依存ゼロ（std のみ）・`default = []`・`unsafe` 不使用・MSRV 1.89・
  edition 2024・`workspace.lints` 継承。
- 全 API non-RT（ファイル I/O・alloc・blocking を許容）。RT 経路からの
  呼出しは契約違反として rustdoc に明記。
- 全公開 enum に `#[non_exhaustive]`（ADR 正本）。
  `PublishOptions`/`PublishReceipt`/`PublishFailure`/`RecoveryInfo`/
  `RecoveryObservation`/`CleanupReceipt`/`CleanupFailure` は
  `#[non_exhaustive]` struct + private フィールド + accessor（将来軸追加の
  余地を残す）。
- `OperationId` はプロセス内単調採番の相関 ID（Lock/CAS/同一性証明ではない）。
- `RecoveryInfo` の `temp_path` は診断値であり、後日の同一性証拠にしない。

## Risks and Trade-offs

- `create` が Unsupported のまま公開 → 「死んだ API」に見えるリスク。
  対策: rustdoc と `FileErrorKind::Unsupported` で意図を明示し、
  Mock 駆動の契約検証が Issue の主目的であることを Docs/Test で一致させる。
- Mock が `#[cfg(test)]` のため外部 crate から再利用不可 → IO-IMP-02 が
  同クレート内で実 OS impl を追加する際に必要になれば、その Issue で
  testing support 面を設計する（YAGNI）。
- `prepare_for_publish`（Close 段階）は OS の close エラー検出能力を
  過大評価しないため、失敗 = `NotPublished` 扱いの明示段階に留める。
- `ReadbackFile` が `&mut dyn ReadSeek` を返すため、実 adapter は
  「temp と同一対象への read handle」を保持している必要がある
  （Windows の共有モード・close 要件は IO-IMP-02 の検証事項）。

## Deferred Design Concerns

- 実 OS backend（renameat2/renamex_np/MoveFileExW・dir FD・File ID 同一性
  検証・capability probing）| IO-IMP-02 の実機検証待ち | 未検証保証の
  Supported 誤記リスク。
- Mock/testing support の crate 外公開 | 下流 crate（io-wav #305 等）の
  テスト実需が出てから | 公開面の安易な拡張リスク。
- `VisibilityRequirement` の将来 variant（NonAtomic 許可等）| Consumer
  実需が出てから | `RequireAtomic` を暗黙弱体化するリスク。

## Verification Notes

- [ ] `cargo check`/`clippy`/`test`（mise タスク経由）が新クレート込みで緑。
- [ ] Mock 行列: `PublishEffect` × `TempDisposition` × 失敗段階の分類が
      test-spec 表と一致。
- [ ] compile-fail doctest で `SealedFile` への `Write`/`Seek` 呼出し不可・
      `StagedFile` の `Write+Seek+Send` を静的検証。
- [ ] `create` が `FileErrorKind::Unsupported` を返すことを unit test で確認。

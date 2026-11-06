---
id: "DESIGN-eVPQpm7tyMUS2gxitU5KtN"
type: "design"
status: "approved"
title: "Issue #305 Media I/O 旧 StagedOutput 撤去・StagedFile 移行設計"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/287"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/sink.rs"
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/render.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/render.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-wav/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/src/muxer.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/README.md"
      - type: "file"
        path: "crates/ruviform-io-wav/tests/contract.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted:
      - type: "file"
        path: "crates/ruviform-media-io/src/staged.rs"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  implemented-by:
    - "docs/plans/issue-305-media-io-staged-file-cutover.md"
  depends-on:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
    - "docs/designs/issue-198-sequential-offline-render.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 Media I/O 旧 StagedOutput 撤去・StagedFile 移行設計

## Overview

`ruviform-media-io` の旧 `staged` モジュールを互換層なしで撤去し、
Ruviform workspace 内の全 call site を `ruviform-io-file` の型状態
契約へ移行する。IO-05（#287）で確定した非互換対応表が直接の設計
入力であり、本書はそれを `ruviform-audio::render` と
`ruviform-io-wav` の具体的な型・所有権・エラー写像へ落とす。

破られた不変条件: File Publish の実装・契約正本が 2 箇所
（`ruviform-media-io::staged` と `ruviform-io-file`）に存在し、
旧実装は非原子 `exists()→rename()`・clone 可能な `&mut File` 露出・
過大な失敗保証を含む。正本を `ruviform-io-file` に一本化し、
media-io には FS 実装を残さない。

## 移行対応表（IO-05 #287 確定・正本）

| 旧 API | 新 API | 備考 |
| --- | --- | --- |
| `StagedOutput::create(dest)` | `StagedFile::create(dest)` | Temp 同一性・所有権を新契約で管理 |
| `sink() -> &mut File` | `&mut StagedFile`（`Write+Seek+Send`） | raw File handle は公開しない |
| `PublishOptions::no_overwrite()` | `PublishMode::CreateNew` | 原子 non-replace。対応不能 OS/FS は Unsupported |
| `PublishOptions::replace()` | `PublishMode::Replace` | Create-or-Replace、CAS なし |
| `sync_before_publish=true` | `Durability::FileSync` | Directory Sync とは別軸 |
| `sync_before_publish=false` | `Durability::BestEffort` | 明示的な弱い要求 |
| `publish() -> Result<()>` | `seal().publish() -> Result<PublishReceipt, PublishFailure>` | 三分類 + Recovery |
| `abort(self)` | `abort(self) -> Result<CleanupReceipt, CleanupFailure>` | cleanup 結果を確認 |

## 設計決定

### D1. `ruviform-media-io` からの撤去範囲

- `src/staged.rs` を削除、`lib.rs` の `mod staged` と
  `pub use staged::{PublishOptions, StagedOutput}` を撤去。
- `sink.rs` module doc の `StagedOutput` 言及を、durability の責務が
  `ruviform-io-file` の `StagedFile`/`SealedFile` 契約へ移った旨へ更新。
- `ruviform-media-io` は Domain Core3 依存のみを維持し、
  `ruviform-io-file` への依存を追加しない（IO-05 確定:
  合成は Consumer/出力呼び出し側の責務）。
- 互換型・type alias・Deprecated・Facade を作らない。
  `staged` という module 名自体も残さない。

### D2. `ruviform-audio::render` の型移行

`RenderOutput` / `render` / `run_pipeline` の変更:

```text
staged: StagedOutput                       → staged: StagedFile
publish_options: media_io::PublishOptions  → publish_options: io_file::PublishOptions
make_muxer: FnOnce(File) -> Result<Box<dyn Muxer>, MediaError>
    → for<'a> FnOnce(&'a mut StagedFile) -> Result<Box<dyn Muxer + 'a>, MediaError>
verify: Option<FnOnce(&Path, &OutputFacts) -> Result<(), MediaError>>
    → Option<FnOnce(&mut SealedFile, &OutputFacts) -> Result<(), MediaError>>
publish: FnOnce(StagedOutput, PublishOptions) -> Result<(), MediaError>
    → FnOnce(SealedFile, PublishOptions) -> Result<PublishReceipt, PublishFailure>
```

- muxer は `&mut StagedFile` 借用で構築する（`WavMuxer<&mut
  StagedFile>`）。旧来の `staged.sink()?.try_clone()` 経路は
  raw handle 露出に依存するため廃止。muxer は `finish()` 後に drop
  し、借用終了後に `staged.seal()` で `SealedFile` へ遷移する。
- `verify` は `&mut SealedFile` を受け、内部で
  `sealed.open_readback()`（`Read + Seek` の借用 Reader）を使う。
  `ReadbackFile` は `Send`/`'static` ではないため、symphonia 等の
  `MediaSource` を要求する独立 decode には readback から読んだ
  bytes で `Cursor<Vec<u8>>` などの独立 source を構成する
  （検証対象は seal 済み Temp の内容で同一）。
- `&mut SealedFile` では `publish`/`abort`（両者 `self` 消費）を
  hook から呼べない。さらに driver は verify 前後で
  `sealed.operation_id()` を照合し、`mem::swap` による別 sealed
  file 差し替え（receipt と publish 先の不整合）を `Verify` 失敗
  として拒否する。
- `dest_existed()` に相当する早い拒否は
  `publish_options.mode == PublishMode::CreateNew &&
  staged.destination().exists()` で render 冒頭に行う。
  これは節約のための preflight であり、原子性の根拠には使わない
  （原子保証は `PublishMode::CreateNew` の publish 側）。
- `RenderReceipt` に `pub publish: PublishReceipt` を追加する。
  `PublishReceipt` は非 Clone のため `RenderReceipt` の
  `#[derive(Clone)]` を除去する（DurabilityEvidence・TempState・
  残留時の RecoveryHandle を失わないことが本 Issue の契約）。
- publish 失敗時: `failure.kind() == DestinationExists` を
  `RenderError::DestinationExists` へ、それ以外を
  `RenderError::Publish { source: PublishFailure }` へ写像する。
  `PublishFailure` の `source: MediaError` 化・`ErrorKind::Io`
  への丸めは行わない。

### D3. publish 失敗後の Temp 処理

`render` の既存契約「失敗時に Temp を残さない」を、io-file の
正直な分類と両立させる:

- `PublishEffect::NotPublished`（DestinationExists を含む）:
  `take_recovery_handle()` → `cleanup()` を best-effort で実行し、
  Temp を除去する。
- `PublishEffect::Published`（DirSync 失敗等）:
  Destination は既に公開済み。Temp 残留の可否は Receipt/Failure が
  保持する情報どおりとし、`Indeterminate` 同様に自動削除しない
  — `RenderError::Publish.source` 内の `PublishFailure` が
  `RecoveryHandle` を保持したまま呼び出し側へ返る。
- `PublishEffect::Indeterminate`: 公開の成否が不明 —
  自動 cleanup しない（io-file 契約そのもの）。

`abort`/`Drop` 経路: `SealedFile`・`StagedFile` の Drop は
best-effort cleanup であり、verify 失敗や cancel 時の Temp は
Drop で除去される（`cleanup_on_drop = true`）。

### D4. `ruviform-io-wav` の変更

- `tests/contract.rs` の staged 系試験を `StagedFile` API へ移行:
  `create` → `open_muxer(&mut staged, ...)`（`WavMuxer<&mut
  StagedFile>`）→ `finalize()` → `seal()` → `publish()`。
- 新規 E2E `finalize` → `seal` → `open_readback` 独立 decode →
  `publish` → 公開後再 Open を実機 FS で追加する。
- `Cargo.toml` dev-dependencies に `ruviform-io-file` を追加
  （backend の公開依存 DAG は `ruviform-media-io` +
  domain core のまま維持）。
- `lib.rs`/`muxer.rs`/README.md の `StagedOutput::publish` 言及を
  `ruviform-io-file` の `SealedFile::publish` へ更新。

### D5. 依存 DAG

```text
ruviform-media-io   ──> media-core, audio-core, video-core（不変）
ruviform-io-wav     ──> media-io, media-core, audio-core（不変）
                    └─ dev: io-symphonia, io-file（新規・dev のみ）
ruviform-audio      ──> rt, audio-core, media-core, graph,
                        audio-analysis, observability, media-io, exec,
                        io-file（新規・下向き・render のみ非 RT 使用）
```

`ruviform-io-file` は汎用ファイル I/O 層（非 RT 専用）で
engine より下位。`media-io` ↔ `io-file` 間の相互依存は双方なし
（IO-05 確定）。

## 検証計画

- `mise run fmt` / `check` / `lint` / `test`（workspace 全体）。
- `cargo tree -p ruviform-media-io` で 3 edge の確認（補助証拠）。
- Workspace 内の `StagedOutput`/旧 `PublishOptions` 検索ゼロ確認
  （`docs/` 歴史資料は対象外）。
- test-spec: `docs/test-specs/issue-305-media-io-staged-file-cutover.md`。

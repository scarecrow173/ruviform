---
id: "DESIGN-034WXOSFa35dv10W4LdOri"
type: "design"
status: "approved"
title: "Issue #306 G07/G08 fault seam・corpus replay 設計"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/fault_seam.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/corpus_replay.rs"
      - type: "file"
        path: "evidence/rf306/fault-seam-fuzz-report.md"
      - type: "file"
        path: "docs/designs/issue-306-fault-seam-fuzz-design.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/mock.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-fault-seam-fuzz.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-fault-seam-fuzz.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-fault-seam-fuzz.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G07/G08 fault seam・corpus replay 設計

## Context

`ruviform-io-file` の publish pipeline は全ての OS 相互作用を
private trait `PlatformFileOps`（`src/platform.rs`）へ集約し、
`StagedFile`/`SealedFile` の型状態 lifecycle が
`ApplyOutcome`/`TempDisposition` を公開語彙
（`PublishEffect`/`PublishPhase`/`PublishFailureKind`/`TempState`/
`DurabilityEvidence`/`RecoveryHandle`）へ正規化する。fault 分類の
網羅試験は `#[cfg(test)]` の `MockFileOps`（`src/mock.rs`）が担うが、
`cfg(test)` は crate 外に出ないため `ruviform-io-e2e` のような
consumer crate からは注入不能であり、「公開 API 経路で分類契約が
観測可能」という結合証拠が欠けている（G07）。

一方 G08 は Issue が要求する「固定 Seed/Time Budget/Crash・Leak/
回帰入口」を、長時間 fuzz ではなく bounded・再現可能な replay
として実装する必要がある。

## Scope

### In Scope

- `ruviform-io-file` への opt-in fault 注入 seam
  （`fault-injection` feature + `#[doc(hidden)]` module + facade）。
- `ruviform-io-e2e` の `tests/fault_seam.rs` 契約試験と
  `tests/corpus_replay.rs` bounded replay 試験、opt-in mise task。
- 判定・実行証拠の `evidence/rf306/` 記録。

### Out of Scope

- 公開 API・既定 feature build・RT 面の変更。
- `PlatformFileOps`/`MockFileOps` 等 crate-private 型の公開。
- structured/compression/archive/wav 各層への seam 波及。
- 実 OS fault の強制再現・長時間 fuzz・新規 CI lane。

## Components and Boundaries

### G07 seam（`ruviform-io-file`）

判定結果: **seam は安全に設けられる**。根拠:

- `PlatformFileOps` は既に「platform backends are interchangeable
  implementation details」として設計された private 境界であり、
  mock はその正当な実装の一つ。seam はこの境界を外部試験へ開く
  だけで pipeline の責務分界を変えない。
- `vfs` module の `#[doc(hidden)] pub` 前例（io-archive が消費）が
  「workspace 内部・semver 非対象」の既存パターン。
- feature gate により既定 build・`cargo check --workspace`・
  production consumer には一切出ない（`publish = false` で
  semver 配布も無い）。

構成:

- `Cargo.toml`: `[features] fault-injection = []` を追加
  （default = [] のまま。理由をコメントで記録）。
- `src/lib.rs`: `#[cfg(test)] mod mock;` を
  `#[cfg(any(test, feature = "fault-injection"))]
  #[doc(hidden)] pub mod mock;` へ変更。`cfg(test)` 側は従来の
  unit test 利用を維持し、feature 側だけが外部へ出る。
- `src/mock.rs`: 既存 `MockFs`/`MockFileOps`/`MockScript` は
  `pub(crate)` のまま再利用。`MockFs` のみ `pub` 化（field は
  全て std 型: `Option<Vec<u8>>`・`Vec<&'static str>`）。
  facade として std 型のみの公開面を追加:
  - `TempReport`（`Removed`/`Retained`/`Unknown`、`#[non_exhaustive]`）
  - `PublishReport`（`Applied`/`NotApplied`/`Unknown`）
  - `PublishScript { report, temp, error: Option<io::ErrorKind>,
    physically_applied }`
  - `FaultScript`（builder: `validate_error`/`sync_file_error`/
    `prepare_error`/`sync_directory_error`/`readback_error`/
    `publish`/`cleanup`/`inspect` — 各 `io::ErrorKind` または
    report 型のみを取り、内部で `MockScript` へ写像）
  - `staged_file(destination, script, initial_temp,
    existing_dest) -> (StagedFile, Arc<Mutex<MockFs>>)`:
    `MockFileOps` + `RecoveryInfo` を組み立て
    `StagedFile::with_ops` へ渡す。`with_ops` の cfg を
    `any(test, feature = "fault-injection")` に拡張
    （`pub(crate)` のまま — facade が crate 内で呼ぶ）。
- `PlatformFileOps`・`ApplyOutcome`・`TempDisposition`・
  `PublishAttempt`・`CleanupAttempt` は **pub(crate) のまま** —
  facade が内部で写像するため公開 signature に出ない
  （`private_interfaces` を構造的に回避）。

### G07 契約試験（`ruviform-io-e2e/tests/fault_seam.rs`）

`ruviform_io_file::mock::{staged_file, FaultScript, PublishScript,
PublishReport, TempReport, MockFs}` を消費し、公開 API のみで
publish pipeline を駆動する。`MockFs` は temp/dest bytes と stage
call log の観測証拠として使う（assertion の主対象は公開結果型）。

### G08 corpus-replay（`ruviform-io-e2e/tests/corpus_replay.rs` +
`mise.toml`）

- 決定的 PRNG: `splitmix64` を test file 内に実装（依存追加なし、
  ~20 行）。seed は `const CORPUS_SEED: u64` で固定・文書化。
- corpus 生成: 各形式の **writer 正規 bytes を種**に、PRNG で
  byte flip / truncate / 拡張の mutation を施す。seed bytes は
  公開 writer（`json::to_writer` 等・`gzip::Encoder`・
  `ZipArchiveWriter`/`TarArchiveWriter`）で決定的に生成。
- replay: `json/yaml/toml/csv::from_reader`（`InputLimits` で
  byte cap）、`xml::XmlEventReader` の event drain、
  `gzip/zstd::Decoder`（`DecodeLimits::with_max_decoded_bytes`
  で cap + `finish()` verified-EOF）、`ZipArchiveReader`/
  `TarArchiveReader`（`ArchiveReadLimits` 縮小 + entry drain/
  `verify_all`/`finish`）。
- bound: format 毎の case 数・入力 byte cap・decoded byte cap・
  全体 deadline（`Instant`、超過で fail-loud）。`#[ignore]` で
  opt-in、`mise run test-io-e2e-corpus` が
  `cargo test -p ruviform-io-e2e --test corpus_replay -- --ignored`
  を実行。
- 形式別除外: `deflate`（io-e2e の compression features は
  gzip,zstd — deflate は io-compression 自身の matrix lane が正本）、
  WAV（Issue #305 の golden/fault lane が正本・G08 列挙外）、
  secure extraction・StagedFile publish FS 経路（FS bound・
  negative_fuzz/fault_boundary/fault_seam が正本）、持続 fuzz。

## Data and Control Flow

- G07: `FaultScript` → `staged_file()` → `MockFileOps` →
  `StageOwner` → `publish_pipeline`（validate→sync_file→prepare→
  publish→sync_directory→cleanup）→ 公開 `PublishReceipt`/
  `PublishFailure`/`RecoveryHandle` → test assertion + `MockFs`
  観測。
- G08: `CORPUS_SEED` → `Rng` → per-format seed bytes → mutation
  case 列 → reader replay → outcome 分類（Ok / typed Err kind /
  budget-capped）→ histogram を `eprintln!` で証拠出力。

## Non-Functional Constraints

- 非 RT 専用（既存契約を維持）。seam は blocking FS I/O を伴わない
  mock のみ。
- 既定 build への影響ゼロ: feature 無しでは `mock` module 自体が
  compile されない（`cfg(any(test, feature))`）。
- `cargo test --workspace` では feature unify により io-file が
  `fault-injection` 付きで build される — seam code は MSRV
  1.89・`-D warnings` clean を要求。
- G08 の実行時間は数秒〜十数秒を目標とし、deadline 超過は
  fail-loud（silent skip しない）。

## Risks and Trade-offs

- seam が「test-only」を超えて production に漏れる risk →
  non-default feature + doc(hidden) + `publish = false` の三重で
  封じ込め。feature を有効化する consumer は自ら opt-in する。
- mock の facade が `MockScript` の語彙を単純化しすぎて表現不能に
  なる risk → G07 が必要とする 3 class（Indeterminate・DirSync
  後失敗・TempState::Unknown）+ phase 別 NotPublished を全て
  表現できる最小の型集合に留め、表現不能な将来要求はその時に
  拡張する（YAGNI）。
- corpus replay が「seed への byte mutation」では parser の深い
  構造を踏まない risk → writer 正規 bytes を種にすることで
  magic/構造が正しい入力への局所破壊を優先し、structured 系は
  `InputLimits`/`ArchiveReadLimits`/`DecodeLimits` の typed
  budget 拘束も同時に検証する。深い構造 fuzz は持続 fuzz lane
  （Deferred）へ送る。
- `#[ignore]` 試験が「走らないまま忘れられる」risk → mise task
  を正本入口として evidence に実行記録を残し、docs/README に
  opt-in 手順を明記。

## Deferred Design Concerns

- 実 OS での Indeterminate/DirSync 失敗/TempState::Unknown 再現
  | OS が fault を公開しない | OS capability 変化・特権 fault
  injection 基盤 | mock 契約と実 FS 挙動の差分が未検証のまま。
- structured 層への seam 波及 | io-structured の API 変更を要し
  本 slice 範囲外 | consumer 要求時 | `write_file` 経路の
  Indeterminate 伝播が未結合。
- corpus の外部ファイル replay（minimized reproducer 置き場）
  | 生成 corpus で十分・YAGNI | 実運用で再現 case の保存需要が
  出た場合 | 発見 bug の再現入力を git 管理できない。

## Verification Notes

- [ ] `cargo check/clippy/test -p ruviform-io-file`（feature 無し）
      で既定 build 不変を確認。
- [ ] `cargo test -p ruviform-io-e2e` で `fault_seam` 契約試験が
      deterministic に pass。
- [ ] `mise run test-io-e2e-corpus` が bounded に完走し証拠を残す。
- [ ] `mise run fmt-check`/`check-io-file`/`lint-io-file`/
      `test-io-file`/`check-io-e2e`/`lint-io-e2e`/`test-io-e2e` pass。

---
id: "SPEC-034WXOCYjbhvL8XtAwcQYk"
type: "spec"
status: "implemented"
title: "Issue #306 G07/G08 — fault seam 判定・契約試験 + bounded corpus-replay 入口"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/fault_seam.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/corpus_replay.rs"
      - type: "file"
        path: "evidence/rf306/fault-seam-fuzz-report.md"
      - type: "file"
        path: "docs/specs/issue-306-fault-seam-fuzz.md"
      - type: "file"
        path: "docs/designs/issue-306-fault-seam-fuzz-design.md"
      - type: "file"
        path: "docs/test-specs/issue-306-fault-seam-fuzz.md"
      - type: "file"
        path: "docs/plans/issue-306-fault-seam-fuzz-plan.md"
      - type: "file"
        path: "docs/tasks/issue-306-fault-seam-fuzz-task.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-fault-seam-fuzz.md"
      - type: "file"
        path: "docs/impl/exp/issue-306-fault-seam-fuzz.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/306"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  refines:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G07/G08 — fault seam 判定・契約試験 + bounded corpus-replay 入口

## What Should Be Built

Issue #306（IO-IMP-12）の残件のうち headless に実装可能な 2 件 —
**G07**（publish fault の `Indeterminate` / DirSync 後失敗 /
`TempState::Unknown` の決定的検証可能範囲の判定と契約試験）と
**G08**（固定 seed・bounded・再現可能な corpus-replay の opt-in
入口）— を閉じる slice。

### G07: fault seam 判定と契約試験

第 2 slice（`docs/specs/issue-306-io-e2e-fault-fuzz.md`）は
`Indeterminate`・publish 後 DirSync 失敗・`TempState::Unknown` を
「実 OS で決定的に誘発不能」として `Unsupported` に固定し、
`ruviform-io-file` の `#[cfg(test)]` `MockFileOps` ユニット試験を
正本とした。しかし `#[cfg(test)]` mock は crate 外から到達不能
であり、公開 API 経路での分類契約（`PublishEffect` /
`PublishPhase` / `PublishFailureKind` / `TempState` /
`RecoveryHandle`）が consumer 視点で観測可能であることの結合証拠は
無い。G07 はこの隙間を埋める:

1. `ruviform-io-file` に **test-only・非公開の fault 注入 seam**
   を安全に設けられるかを技術的に判定する。判定基準は「公開 API
   （semver 面・既定 feature build）を変えない」「RT 面を侵さない」
   「本番 build に fault 経路を残さない」。
2. 安全に可能なら seam を実装し、`ruviform-io-e2e` の契約試験で
   `Indeterminate`（`ApplyOutcome::Unknown` →
   `PublishEffect::Indeterminate`、temp 非自動削除、recovery
   handle の inspect/cleanup）・publish 後 DirSync 失敗
   （`PublishEffect::Published` + `PublishPhase::DirectorySync` +
   `DurabilityEvidence::FileSynced`）・`TempState::Unknown`
   （publish/cleanup/inspect の `TempDisposition::Unknown` →
   `TempState::Unknown`）を **決定的に** 検証する。
3. seam が安全に設けられない部分は `Unsupported` + mock-oracle
   （io-file ユニット試験）+ OS capability の根拠を
   `evidence/rf306/` と設計注記に確定する。

### G08: bounded corpus-replay 入口

長時間 fuzz ではなく、**固定 seed・bounded・再現可能**な
corpus-replay を `mise.toml` の opt-in タスクとして追加する:

- `ruviform-io-e2e` に `#[ignore]` 付きの corpus-replay 試験を追加
  し、通常の `test-io-e2e` lane には載せない。
- seed・case 数・byte budget・時間 budget・timeout・形式別除外を
  明記する。決定的 PRNG（依存追加なし）で writer 正規 bytes を
  種に mutation した corpus を各 reader へ replay し、panic しない
  こと・typed error 分類・budget 拘束を検証する。
- 追加した入口を Windows で 1 回実行し、記録を
  `evidence/rf306/` に残す。

## Why Now

Issue #306 の受入条件に G07/G08 が未チェックで残っている。
G07 は「実 OS で誘発不能」という前 slice の結論を、test-only seam
の可否という技術判定として再検討する残件であり、G08 は Issue が
「決定的 mutation Pass とは別に、固定 Seed/Time Budget の
Corpus/実環境入口」を要求する残件である。両者とも headless・
Windows ローカルで完結し、CI lane の追加なしに opt-in で実施できる。

## Users and Value

- Issue #306 / #288 Gate: G07 の判定結果（seam 可否・契約試験範囲・
  Unsupported 根拠）と G08 の replay 入口・実行証拠を、Case 別の
  Pass/Fail/NotRun/Unsupported 判定材料として渡す。
- `ruviform-io-file` consumer: publish 分類契約
  （`PublishEffect`/`PublishPhase`/`TempState`/`RecoveryHandle`）が
  注入 fault 下でも嘘をつかないことの結合証拠。
- parser/codec 保守者: 再現可能な corpus-replay 入口で回帰を
  bounded に検出する手段。

## Scope

### In Scope

- `crates/ruviform-io-file`: test-only fault 注入 seam
  （非 default feature + `#[doc(hidden)]` module + facade API。
  公開 API・既定 build・RT 面は不変）と、その seam を使う
  io-e2e 契約試験のための最小限の内部変更。
- `crates/ruviform-io-e2e`: `tests/fault_seam.rs`（G07 契約試験）・
  `tests/corpus_replay.rs`（G08 replay）・dev-deps feature 指定・
  README/lib.rs の記述更新。
- `mise.toml`: G08 opt-in タスク 1 件の追加のみ。
- `evidence/rf306/`: G07 判定・契約試験結果・G08 実行記録の証拠
  文書。
- 本 doc チェーン（spec/design/test-spec/plan/task/ir/exp）。

### Out of Scope

- G01 実 4GiB・G02 Network/Cloud FS・G03 特殊 FS・実故障操作・
  実測容量/性能閾値・持続 Fuzz — Issue が Deferred/外部 gate と
  指定する範囲。
- `ruviform-io-structured`/`io-compression`/`io-archive`/`io-wav`
  の公開 API・内部変更（seam は io-file のみ。structured の
  `write_file` 等は内部で `StagedFile::create` を呼ぶため注入面を
  持たず、G07 契約試験は `StagedFile` 層に限定する）。
- 新規 CI Workflow・PR 必須 lane・外部 fuzzing 基盤
  （cargo-fuzz/AFL 等）・新規外部依存。
- `Cargo.toml`（workspace root）・`Cargo.lock`・docs index 以外の
  生成物の手編集。

## Requirements

- seam は **opt-in feature + `#[doc(hidden)]`** に限定し、既定
  feature・`publish = false` の配布面・公開 API シグネチャを変え
  ない。`PlatformFileOps` 等の crate-private 型を公開 signature
  に出さない。
- seam は `#[cfg(test)]` mock を複写しない — 既存 `MockFileOps` を
  再利用し、外部に出すのは std 型のみの facade に留める。
- G07 契約試験は公開型（`StagedFile`/`SealedFile`/`PublishOptions`/
  `PublishReceipt`/`PublishFailure`/`RecoveryHandle`）のみを消費し、
  mock の内部状態は観測証拠としてのみ使う。NotRun/Unsupported を
  Pass へ繰り上げない。
- G08 は固定 seed・case 数・byte budget・時間 budget・timeout を
  持ち、通常 `test-io-e2e` では実行されない（`#[ignore]` + opt-in
  mise タスク）。panic・無制限ループ・無制限メモリ確保を許さない。
- 形式別除外（deflate・WAV・secure extraction・publish FS 経路）
  を理由付きで明記する。
- 各 Case は検証対象 SHA/OS・FS/Version/Feature/Profile・期待値・
  実測値・Pass/Fail/NotRun/Unsupported・ログを記録する。

## Acceptance Criteria

- [ ] G07 の seam 可否判定が技術的根拠付きで確定し、可能な範囲の
      契約試験（`Indeterminate`・DirSync 後失敗・`TempState::
      Unknown` を含む）が `cargo test -p ruviform-io-e2e` で
      deterministic に pass する。不可能な範囲は `Unsupported`
      + mock-oracle + OS capability の根拠が evidence/design に
      記録される。
- [ ] seam が公開 API・既定 build・RT 面を変えないことが
      `cargo check/clippy/test -p ruviform-io-file`（feature 無し）
      と io-e2e lane の両方で確認される。
- [ ] G08 の opt-in タスクが `mise.toml` に追加され、seed/budget/
      timeout/形式別除外が明記され、Windows で 1 回実行した記録が
      `evidence/rf306/` に残る。
- [ ] `mise run fmt-check` / `check-io-file` / `lint-io-file` /
      `test-io-file` / `check-io-e2e` / `lint-io-e2e` /
      `test-io-e2e` が pass（波及時は io-structured/io-compression
      lane も）。
- [ ] 変更が波及する crate の check/lint/test が pass し、
      workspace 非回帰を確認する。

## Deferred Design Concerns

- `Indeterminate`/DirSync 後失敗/`TempState::Unknown` の **実 OS**
  再現 | 実 OS で deterministic に誘発不能（前 slice と同じ結論）
  | OS/FS が fault を公開した場合・特権 fault injection 基盤の
  導入時 | 実 FS 保証と mock 保証の差分が未検証のまま残る。
- structured 層（`write_file`）への fault seam 波及 | seam を
  `StagedFile::create` 呼出し側へも通すには io-structured の
  API 変更が必要で本タスク範囲外 | consumer が structured 層の
  fault 契約を要求した場合 | structured 経路の Indeterminate
  伝播は mock 正本 + StagedFile 層契約試験の合成でしか保証
  されない。
- G08 の長時間・持続 fuzz | bounded replay が本 slice の範囲 |
  Issue #306 の外部 gate・将来の dedicated fuzz lane | 未知の
  長尾 bug が残る。

## Sources and Relations

- Issue #306 本文（受入条件の正本）・Issue #288 IO-06 v0.1・
  Issue #282 横断設計正本。
- 前 slice 文書チェーン: `docs/{specs,designs,plans,tasks,
  test-specs,impl/ir}/issue-306-io-e2e-*.md` — 本 spec はその
  `refines` 関係（G07/G08 残件の再検討）。
- `evidence/rf306/fault-fuzz-report.md` — 前 slice の
  Unsupported 判定と mock 正本の記録。
- `crates/ruviform-io-file/src/{mock,platform,staged}.rs` —
  `#[cfg(test)]` MockFileOps・`PlatformFileOps` seam・
  publish pipeline 分類の実装正本。

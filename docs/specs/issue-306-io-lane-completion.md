---
id: "SPEC-034XCP64zCIaWAnqXYkbFt"
type: "spec"
status: "implemented"
title: "Issue #306 G01/G05 — 実4GiB opt-in lane + 性能 baseline lane"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/large_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/benches/io_composition.rs"
      - type: "file"
        path: "evidence/rf306/large-bench-report.md"
      - type: "file"
        path: "docs/specs/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/designs/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/plans/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/tasks/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/impl/exp/issue-306-io-lane-completion.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-e2e-cases.md"
    - "docs/specs/issue-306-fault-seam-fuzz.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  refines:
    - "docs/specs/issue-306-io-e2e-cases.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-e2e-cases.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G01/G05 — 実4GiB opt-in lane + 性能 baseline lane

## What Should Be Built

Issue #306（IO-IMP-12）の Stage-1 環境非依存 slice の残り 2 件 —
**G01**（実 4GiB ファイルの staged publish 境界を実 FS で検証する
opt-in lane）と **G05**（IO 合成経路の性能 baseline を criterion
bench として実測・記録する lane）— を `ruviform-io-e2e` に追加する。

### G01: 実 4GiB 境界 opt-in 試験

`crates/ruviform-io-e2e/tests/large_staged.rs` を新設し、全試験を
`#[ignore]` 付きの opt-in とする（通常 `test-io-e2e` lane には載ら
ない）。公開 API（`StagedFile`/`SealedFile`/`PublishOptions`/
`PublishReceipt`/`PublishFailure`/`RecoveryHandle`）と実 FS のみを
消費し、本番 API は変更しない。

- 対象サイズは **4GiB−1 / 4GiB / 4GiB+1** の 3 境界。
- 各サイズで StagedFile write → seal → readback checksum →
  publish（`CreateNew` / `Replace` 両経路）→ reopen 照合、および
  途中失敗 → `RecoveryHandle` cleanup を検証する。
- temp 配置ドライブの空き容量を preflight（`set_len` + 末尾書き込み
  + sync の probe file）で確認し、容量不足は `StorageFull` 相当の
  正直な skip = **NotRun** とする。skip 理由（要求量・観測結果・
  temp path/FS）は stdout と `evidence/rf306/` に明記し、
  Pass/Fail に偽装しない。
- checksum は新規依存を足さず std のみ（chunked FNV-1a）で実装する。
- 途中失敗の注入は `tests/wav_fault.rs` の `&mut StagedFile` fault
  wrapper 先例に倣う（本番 API 不変）。
- 実行は `mise run test ruviform-io-e2e --test large_staged --
  --ignored --nocapture`（汎用 `mise run test` 経由。専用 task は
  `mise.toml` 変更禁止のため追加しない）。

### G05: 性能 baseline lane

`crates/ruviform-io-e2e/benches/io_composition.rs` を criterion
bench（`harness = false`）として新設し、`mise run bench`
（`cargo bench --workspace`）で実走する。Format / 入力サイズ /
Feature / Profile / FS / OS・baseline revision・回帰閾値・計測ブレ・
許容不能時の判定を事前に定義し（bench 冒頭コメント + design +
test-spec に pin）、実測値を SHA/OS/FS/toolchain 付きで
`evidence/rf306/` に記録する。

- 計測対象: staged write / seal / publish / readback、
  TAR→gzip（+zstd）→StagedFile 合成 throughput、wall time、
  counting `GlobalAlloc` による peak alloc（bench 内自前実装・
  新規依存なし）。
- `benches/rt_observability_poc.rs` の構成先例（BenchmarkGroup・
  `Throughput`・実行開始時の環境 metadata を stdout に 1 回出力・
  `iter_batched_ref` + `BatchSize` で fixture 準備を計測窓外に置く
  方法）に倣う。
- peak alloc を測る bench は GlobalAlloc 差替えの副作用が他 bench
  へ漏れないよう、計測対象区間のみ有効化する。

## Why Now

Issue #306 の Issue 詳細（2026-11-13「IO-02/03/04設計Close後の
実装・検証残件」— 現行正本）で G01 は「実 4GiB ファイル
publish・境界 = not run（M-X3・PCM 4GiB 境界の実測部分）」、
G05 は「性能比較 = not run・閾値未決定」として残っている
（`evidence/rf288/io06-gate-matrix.md` の matrix の穴 1・5）。
G07/G08 は前 slice で main 統合済みであり、本 slice は Stage-1
環境非依存 slice の最後の実装残件である。両者とも headless・
Windows ローカルで完結し、CI lane・`mise.toml` の変更なしに
opt-in/既存 bench 入口で実施できる。

## Users and Value

- Issue #306 / #288 Gate: G01 の実 FS 実測（境界サイズごとの
  期待成功/明示的拒否・Recovery・容量不足時の NotRun 判定）と
  G05 の baseline 実測値（環境 metadata・throughput・peak alloc・
  計測ブレ）を、Case 別の Pass/Fail/NotRun/Unsupported 判定材料
  として渡す。
- `ruviform-io-file` consumer: 4GiB 境界での staged publish が
  型状態・実 FS 保証を破らないことの結合証拠。
- 性能回帰監視: baseline revision・閾値・計測ブレを事前定義した
  記録により、将来の回帰比較の基準点を得る。

## Scope

### In Scope

- `crates/ruviform-io-e2e/tests/large_staged.rs`（G01 opt-in 試験）。
- `crates/ruviform-io-e2e/benches/io_composition.rs`（G05 bench）。
- `crates/ruviform-io-e2e/Cargo.toml`: criterion dev-dep
  （`0.8.2`・`default-features = false`、ruviform-rt と同形）と
  `[[bench]]` 登録。依存追加理由をコメントで記録。
- `Cargo.lock`: io-e2e の依存一覧への criterion 追加分
  （生成物としてコミット。`0.8.2` は既収録のため新規 version
  解決なし）。
- `crates/ruviform-io-e2e/src/lib.rs` / `README.md`: lane 記述・
  検証節の更新（文末の未実測一覧を実測結果で更新）。
- `evidence/rf306/large-bench-report.md`: G01 実走/NotRun と
  G05 baseline の証拠記録。
- 本 doc チェーン（spec/design/test-spec/plan/task/ir/exp）。

### Out of Scope

- G02 Network/Cloud FS・G03 非既定 FS・G04 macOS/Linux 実機・
  G06 consumer gate・持続 fuzz・最終受入 — Issue open 継続の
  後続ゲート。
- `mise.toml`・`.github/workflows/`・`docs/development-environment.md`
  の変更（bench lane 対象一覧の drift は証拠・PR 本文に記録）。
- `ruviform-io-file` 等他クレートの公開 API・内部変更
  （dev-dep 先は読み取りのみ）。
- WAV RIFF 実ファイル 4GiB variant — `wav_golden` の discard-sink
  検証（classic RIFF 4GiB 境界）が既存根拠。実ファイル variant を
  本 lane に追加するかは design で判断し、実装しない場合は
  NotRun として正直に記録する。
- 新規 CI Workflow・PR 必須 lane・外部 fuzzing 基盤・新規外部
  依存（criterion 以外）。
- `*-msrv` 系 lane（ユーザー指定により Daily CI 委譲）。

## Requirements

- G01 は `#[ignore]` opt-in であり、通常 `test-io-e2e` lane の
  意味を変えない。容量不足は Pass/Fail に偽装せず NotRun とし、
  skip 理由を stdout と evidence に明記する。
- G01/G05 は公開 API と実 FS のみを消費し、本番コード・公開
  signature を変更しない。fault 注入は `&mut StagedFile` wrapper
  等の試験側実装に限定する。
- G05 は Format/入力サイズ/Feature/Profile/FS/OS・baseline
  revision・回帰閾値・計測ブレ・許容不能時の判定を**実装前に**
  定義し、実測値と環境 metadata を `evidence/rf306/` に記録する。
- checksum・PRNG・temp 管理・peak alloc 計測は新規依存なしの
  std/自前実装とする。criterion のみ dev-dep 追加（bench 限定、
  `[dependencies]` に昇格しない）。
- `unsafe` は bench の counting `GlobalAlloc` に限定し、
  `#[expect(unsafe_code)]` + SAFETY コメントの局所許可とする。
- NotRun/Unsupported を Pass へ繰り上げない（POLICIES §3・
  Issue 検証方法どおり Requirement×Result を直交記録）。

## Acceptance Criteria

- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
  `test-io-e2e` / `test-io-e2e-corpus` が全て exit 0。
- [ ] `mise run test ruviform-io-e2e --test large_staged --
  --ignored --nocapture` を実走し、各 case の Pass または
  NotRun（skip 理由付き）を `evidence/rf306/` に記録する。
- [ ] `mise run bench` を実走し、`io_composition` の実測値を
  SHA/OS/FS/toolchain/feature/profile 付きで `evidence/rf306/` に
  記録する。
- [ ] bench の計測対象・入力・閾値・判定基準が bench 冒頭・
  design・test-spec に事前定義されている。
- [ ] 実装しない範囲（WAV RIFF 実ファイル variant 等）を
  NotRun/Deferred として正直に記録し、Pass 繰り上げがない。

## Deferred Design Concerns

- Consumer 固有の性能閾値（f64/RT/Long-media/Project-save）は
  各 Consumer Issue（#127/#28/#84・#294）で独立評価 — 本 slice の
  baseline は閾値の事前定義と実測記録まで。
- SMB/Cloud/特殊 FS・持続 Fuzz・macOS/Linux 実機 — Issue の
  Deferred/外部 gate。

## Sources and Relations

- Issue #306（IO-IMP-12）Issue 詳細 2026-11-13「IO-02/03/04設計
  Close後の実装・検証残件」G01/G05 行・受入条件・検証方法。
- `evidence/rf288/io06-gate-matrix.md`「実 4GiB ファイル
  publish・境界 = not run」「性能比較 = not run」「matrix の穴
  1・5」。
- `evidence/rf306/fault-seam-fuzz-report.md`（G07/G08 の記録形）。
- `docs/specs/issue-306-io-e2e-cases.md`（Case 体系・Gate 対応）。
- `docs/designs/issue-295-io-file-staged-publish.md` /
  `issue-296-io-file-os-backend.md`（StagedFile/publish 契約）。

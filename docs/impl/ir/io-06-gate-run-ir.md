---
id: "IMPL-kQ7mXv2nLp8wRtYb5jHc3d"
type: "impl"
status: "completed"
title: "Issue #288 IO-06 Gate matrix 実走記録（rf288 slice）"
created: "2026-11-12"
updated: "2026-11-12"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "evidence/rf288/io06-gate-matrix.md"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/lanes.log"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/lanes-rerun.log"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/run-lanes.sh"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/run-lanes-rerun.sh"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/check-linux.log"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/al/consumer-evidence-audiolens.json"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/sv/consumer-evidence-synvelis.json"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/al.zip"
      - type: "file"
        path: "evidence/rf288/lane-run-raw/sv.zip"
      - type: "doc"
        path: "docs/impl/ir/io-06-gate-run-ir.md"
    modified:
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/io-06-gate-run-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-306-io-e2e-slice-4-feature-matrix-lane.md"
    - "docs/impl/ir/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/impl/ir/issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md"
    - "evidence/rf306/report.md"
    - "evidence/rf306/fault-fuzz-report.md"
    - "evidence/rf306/feature-matrix-report.md"
    - "evidence/rf288/io06-gate-matrix.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #288 IO-06 Gate matrix 実走記録（rf288 slice）

## Summary

Issue #288（IO-06 Cross-Consumer・OS/FS・Security・性能・互換性
Gate）の受入 matrix を、Windows 11 x64 / NTFS 実機での `mise run`
lane 実走と既存証拠（`evidence/rf306/`・ci.yml run・
consumer-verify.yml run）へ対応付け、セルごとの判定を
`evidence/rf288/io06-gate-matrix.md` へ記録した。
**コード・テスト・設定ファイルを一切変更しない検証・証拠
slice** であり、`crates/`・`mise.toml`・`.github/`・
`Cargo.toml`/`Cargo.lock` は無変更。

## Context

- 対象 Issue: [Issue #288](https://github.com/scarecrow173/ruviform/issues/288)。
  Gate 判定語彙・matrix 構造・否定系判定表の正本は
  `docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`
  （PR #510 merge 済み）。
- 文書チェーン: [spec](../../../docs/specs/io-06-gate-run.md)・
  [design](../../../docs/designs/io-06-gate-run-design.md)・
  [plan](../../../docs/plans/io-06-gate-run-plan.md)・
  [task](../../../docs/tasks/io-06-gate-run-task.md)・
  [test-spec](../../../docs/test-specs/io-06-gate-run.md)。
- 前提証拠: `evidence/rf306/` 3 本（IO-IMP-12 slice-1〜4 の
  実走記録）・ci.yml run 42937526176・consumer-verify.yml run
  42937524742（いずれも main `9865cd57`・success）。

## Implementation

### Changes

- `evidence/rf288/io06-gate-matrix.md`（新規）: matrix 記録本体。
  全 18 lane の exit code・OS・toolchain・test 数、第 1 run
  （salvage `lanes.log`）の infra flake 診断と第 2 run
  （`lanes-rerun.log`）の確定値、design 正本行 ID 準拠の
  受入 matrix（IO-01〜05 + 合成 E2E 全行）、否定系判定表
  5 領域、OS/FS capability 軸、Consumer 独立判定、残存
  NotRun/Unsupported/Blocked 一覧、matrix の穴 7 件。
- `evidence/rf288/lane-run-raw/`（新規・salvage + 再実走ログ）:
  `lanes.log`（前セッション実走の生ログ・io 系 12 lane の
  exit= 記録あり）、`lanes-rerun.log`（本セッション再実走）、
  `run-lanes.sh`/`run-lanes-rerun.sh`（実走スクリプト）、
  `al/`・`sv/` の consumer-verify evidence JSON（run
  42937524742 artifact の展開物）、`al.zip`/`sv.zip`（同
  artifact の原本 archive。各 `consumer-evidence-*.json`
  1 件を内包し、展開済み JSON と byte 一致を `unzip -p|diff`
  で確認済み — provenance として保存）。
- `docs/impl/ir/README.md`: 本 IR の row を末尾へ追加
  （index は tool 非管理のため手動追記 — 先行 IR と同じ運用）。
- 本 IR（新規）。
- `crates/`・`mise.toml`・`.github/`・`Cargo.toml`/
  `Cargo.lock`・他 docs: **無変更**。

### Decisions During Implementation

- **exit= 記録済み lane は再実走せず、記録のない lane と
  flake 診断が必要な lane のみ再実走**: io 系 12 lane は
  `lanes.log` の `exit=0` をそのまま採用。`test`/`test-rt`/
  `test-io-msrv` は exit 記録がなかったため再実走。
  `check`(exit 101)/`lint`(exit 66) は記録があるが、失敗
  signature が rustc キャッシュ破損（`only metadata stub
  found for rlib dependency core`）と無診断即時終了であり
  infra flake と診断 — コード起因でないことを同一 log 内の
  `cargo test --workspace` 成功で裏付けた上で、確定値を得る
  ため再実走した。両 run の事実を matrix に併記し、flake を
  隠さず PASS に付け替えていない。
- **merge は lane 実走後・docs のみ差分**: lane 実走時の
  base `588d29a2` から `origin/main` `1b78607c` への差分は
  `docs/designs/issue-286-*` と IR 1 件の docs 簿記のみで
  コード差分ゼロのため、実走結果は merge 後 HEAD へそのまま
  適用できると判断（matrix に両 SHA を明記）。
- **consumer-verify の tuple は revision を明示して転記**:
  run 42937524742 の candidate `9865cd57` は現 main の祖先
  であり「最新 main での consumer 受入」とは記録しない
  （design risk #2 対応）。AL/SV を独立に判定し、consumer
  固有受入の not_run/unsupported lane をそのまま転記した。
- **文書チェーンは spec→design→plan→task→test-spec→IR**:
  検証・証拠 slice だが受入条件確定済みのため doc-driven
  フローの内部文書を作成（`docs/discovery/` は本 slice の
  relations に含まれず作成しない）。

## Related Experiments

なし（既存 lane の実測記録と docs 簿記のみ。新規の技術的
探索は行っていない）。

## Validation

全て `mise run` タスクで実走（exit code は `mise run` 自体の
終了コード）。環境: Windows 11 x64 (10.0.26300) / NTFS /
rustc 1.99.0 (b940084d7 2026-09-28) / cargo 1.99.0。

- `mise run fmt-check` — Pass (exit 0)（lanes.log L1–3）。
- `mise run check-io-file` / `lint-io-file` /
  `test-io-file` — Pass (exit 0)。test は **315 passed /
  0 failed / 1 ignored**（io-file + io-archive。ignored は
  `create_new_contention_multi_process` が spawn する
  `child_create_new_once` の既知の意図的 `#[ignore]`）。
- `mise run check-io-structured` / `lint-io-structured` /
  `test-io-structured` — Pass (exit 0)。test は **251 passed /
  0 failed / 0 ignored**（feature matrix 4 組合せ）。
- `mise run check-io-compression` / `lint-io-compression` /
  `test-io-compression` — Pass (exit 0)。test は **330 passed /
  0 failed / 0 ignored**（5 組合せ）。
- `mise run check-io-e2e` / `lint-io-e2e` / `test-io-e2e` —
  Pass (exit 0)。test は **77 passed / 0 failed / 0 ignored**。
- `mise run check` — 第 1 run exit 101（infra flake:
  rustc metadata stub 破損）→ 第 2 run **Pass (exit 0)**。
- `mise run lint` — 第 1 run exit 66（無診断即時終了・同上
  flake と診断）→ 第 2 run **Pass (exit 0、40.20s)**。
- `mise run test` — 第 2 run **Pass (exit 0)**。
  **3772 passed / 0 failed / 10 ignored**（201 result 行全て
  ok）。
- `mise run test-rt` — 第 2 run **Pass (exit 0)**。
  **290 passed / 0 failed / 0 ignored**。
- `mise run test-io-msrv` — 第 2 run **Pass (exit 0)**。
  `rust@1.89.0` (29483883e 2025-08-04) で **742 passed /
  0 failed / 1 ignored**（54 result 行全て ok。ignored は
  gate spawned `#[ignore]` helper の既知の意図的 ignore）。
- `mise run check-linux` — **Pass (exit 0)**（補助 lane。
  `x86_64-unknown-linux-gnu` target で io-file/io-archive の
  check+clippy。必須 lane 一覧外だが unix cfg 経路の静的
  証拠として実施・記録）。
- `git fetch origin main && git merge origin/main` —
  fast-forward `588d29a2..1b78607c`（docs 簿記のみ）。
- 補助証拠（canonical = mise ではない grep）: 旧 API 不在
  確認（`StagedOutput` 0 件・media-io 旧 `PublishOptions`
  0 件・deps = Domain Core3 のみ）。

## Risks

- 第 1 run の check/lint 失敗は infra flake と診断したが、
  flake の再発可能性は残る（rustc キャッシュ破損は環境依存）。
  matrix には両 run を併記し、flake を隠していない。
- CI/consumer-verify の参照 SHA（`9865cd57`）は現 main
  `1b78607c` の祖先。間の差分は docs 簿記のみだが、
  「最新 main での consumer 受入」とは記録していない。
- workspace `test` の ignored 10 件は failed=0 を確認した上で
  の記録であり、内訳の個別監査は本 slice 対象外。

## Follow-ups

- matrix の穴 7 件（実 4GiB・Network/Cloud FS・非既定 FS
  個別実測・macOS/Linux 実機・性能比較・consumer 固有受入残・
  fault fixture 化）は `evidence/rf288/io06-gate-matrix.md`
  「matrix の穴」節に列挙。新規 Issue 起票は Coordinator が
  行う。
- Issue #288 の Open/Close 判定は Coordinator が行う。
- macOS/Linux 実機・consumer repo 側受入は別 owner・別環境の
  後続作業。

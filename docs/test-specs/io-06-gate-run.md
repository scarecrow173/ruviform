---
id: "TSPEC-hCescNtTTgZy1xetgEJlap"
type: "test-spec"
status: "approved"
title: "IO-06 Gate matrix 実走証拠の記録"
created: "2026-11-11"
updated: "2026-11-12"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/designs/io-06-gate-run-design.md"
      - type: "doc"
        path: "docs/plans/io-06-gate-run-plan.md"
      - type: "doc"
        path: "docs/tasks/io-06-gate-run-task.md"
      - type: "doc"
        path: "docs/test-specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/impl/ir/io-06-gate-run-ir.md"
      - type: "file"
        path: "evidence/rf288/io06-gate-matrix.md"
    modified: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/io-06-gate-run-design.md"
  derived-by: []
  verifies:
    - "docs/specs/io-06-gate-run.md"
    - "docs/designs/io-06-gate-run-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# IO-06 Gate matrix 実走証拠の記録

## Purpose

本 slice の成果物 — `evidence/rf288/io06-gate-matrix.md` と
文書チェーン — が、[design](../designs/io-06-gate-run-design.md)
の記録規則どおりに Issue #288 受入 matrix を誠実に記録して
いることを検証する。対象 spec は
[spec](../specs/io-06-gate-run.md)。各 lane が実際に検証する
I/O 契約自体の正しさは対象外（それは issue-295〜306 系
test-spec と `mise run test-io-*` の実走が担う）。

## Feature

IO-06 Gate matrix 実走証拠の記録（Issue #288 slice）

## Rules

- **R1（全セル記録）**: `evidence/rf288/io06-gate-matrix.md`
  は issue-288 design 正本の受入 matrix 全行（F-N1〜F-X4・
  S-N1〜S-X6・C-N1〜C-X4・A-N1〜A-X5・M-N1〜M-X3・E-N1〜E-X2）
  と否定系判定表 5 領域・OS/FS capability 軸の各項目に対し、
  判定（`pass`/`fail`/`blocked`/`not run`/`unsupported`）と
  根拠（lane 名・exit code・OS・toolchain・参照先）を持つ。
- **R2（PASS の誠実さ）**: `pass` 判定は実際に実行され exit
  code 0 で終了した lane、または参照先の記録（`evidence/rf306/`・
  CI run・PR）が pass を示すものに限る。`not run`・`unknown`・
  未確認の参照を PASS 扱いしない。`unsupported` は「正しく
  拒否した否定テストの成功」または「実測環境が存在しない
  FS/OS 領域」の意味でのみ使い、Consumer 必須 Capability の
  欠落を免除しない。
- **R3（実走 lane の記録完全性）**: spec が列挙する Windows
  実走 lane（fmt-check・check・lint・test・test-rt・
  check/lint/test-io-file・check/lint/test-io-structured・
  check/lint/test-io-compression・check/lint/test-io-e2e・
  test-io-msrv）それぞれについて、実行の有無・exit code・
  OS・toolchain が matrix または付随記録に存在する。
  `test-io-msrv` のみ toolchain 可用性条件付きであり、
  未実施の場合は理由付き `not run` が許容される。
- **R4（参照の実在性）**: matrix が参照する PR 番号・Issue
  番号・CI run ID・evidence パス・mise task 名・test-spec
  パスが実在する。存在しない参照を根拠として記載しない。
- **R5（重複実走の回避と鮮度）**: `evidence/rf306/` と同一
  内容の lane を根拠に使う場合は参照として記録し、再実走した
  場合は新しい exit code・SHA を記録する。参照 SHA と実走
  SHA を区別して記録する。
- **R6（Consumer 独立判定）**: AudioLens/Synvelis の consumer
  gate 結果は独立に記録し、片側の pass を両 Consumer 受入と
  みなさない。consumer-verify の tuple は ruviformRev・
  consumer resolvedSha・run URL を持つ。
- **R7（matrix の穴の列挙）**: どの lane にも対応しない必須
  セルがあれば、PR 本文と Issue #288 comment に列挙される。
  「穴がない」場合もその旨を明示する。
- **R8（残存 NotRun の引き渡し）**: macOS/Linux 実機・
  Network/Cloud FS・実 4GiB・consumer gate・fixture 化対象
  （Indeterminate・DirSync 後失敗・TempState::Unknown）が
  `not run`/`unsupported`/`blocked` として理由付きで残り、
  Issue #288 comment と PR 本文に記載される。

## Examples

- **Example 1（pass の記録）**: F-N1（StagedFile 正常系）が
  `mise run test-io-file` exit 0・315 tests pass・
  Windows 11 x64 / NTFS / rustc 1.99.0 を根拠に `pass` と
  記録される。
- **Example 2（CI 参照の記録）**: Linux ext4 セルが ci.yml
  run（例: run ID・headSha・ubuntu-latest verify job の
  test-ci 成功）を根拠に `pass` と記録され、FS 種別が
  runner 既定である旨が明記される。
- **Example 3（not run の記録）**: 実 4GiB publish が lane
  不在で `not run`（理由: lane 未実装・scope 外）と記録され、
  PASS に付け替えられない。
- **Example 4（unsupported の記録）**: Network/Cloud FS の
  durability セルが実測環境なしで `unsupported`/`not run`
  と記録され、Supported 主張がない。

## Risks

- **R-A（参照 SHA の陳腐化）**: CI/consumer-verify の参照
  SHA が最新 main と異なる場合、判定の鮮度を誤認する恐れ。
  → matrix に run ID・headSha・日時を明記する規則（R5）で
  緩和。
- **R-B（consumer tuple の誤読）**: consumer-verify が
  片側のみ成功・または古い Ruviform Revision の tuple で
  あっても「consumer 受入済み」と誤読する恐れ。→ R6 で
  独立記録と tuple 転記を要求。
- **R-C（文書のみの合格化）**: 証拠文書の存在をもって Gate
  合格と誤認する恐れ。→ test-spec は「記録の正しさ」のみを
  検証し、lane 自体の合否は `mise run` の実走結果を根拠と
  する（R2/R3）。

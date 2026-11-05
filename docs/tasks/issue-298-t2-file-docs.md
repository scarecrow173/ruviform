---
id: "TASK-034DgWopBD0NzsxoZGVxdm"
type: "task"
status: "todo"
title: "Issue #298 T2 — file feature 合成・試験仕上げ・README・impl record"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/298"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-io-structured/src/file.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/README.md"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/impl/ir/issue-298-io-structured-csv.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-298-io-structured-csv.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-298-t1-csv-module.md"
    - "docs/designs/issue-298-io-structured-csv.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/tasks/issue-297-t3-file-feature.md"
  refines:
    - "docs/plans/issue-298-io-structured-csv.md"
  refined-by: []
  derives-from:
    - "docs/plans/issue-298-io-structured-csv.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #298 T2 — file feature 合成・試験仕上げ・README・impl record

## 目的

`csv`+`file` 合成の動作確認、deterministic fuzz 補強、crate/ repo
README 更新、impl record 作成。

## 対象変更

- `src/file.rs`: `open_document`/`write_document` 利用箇所の cfg に
  `feature = "csv"` を追加（本体ロジック不変）。
- `tests/structured.rs`: `csv`+`file` の read_file/write_file
  round-trip・失敗軸分離テスト、deterministic fuzz テスト。
- `crates/ruviform-io-structured/README.md`: feature matrix に
  `csv` 行追加・CSV 固有の制約（BOM・encoding・formula injection・
  memory/boundedness・non-RT）を記述。
- `crates/README.md`: `ruviform-io-structured` の CSV feature
  記述を反映。
- `docs/impl/ir/issue-298-io-structured-csv.md`: 実施・検証・
  未検証を記録する impl record。

## 受入基準

- [ ] `read_file`/`write_file` が `DocumentFileReadError`/
  `DocumentFileWriteError` で失敗軸分離しつつ round-trip する。
- [ ] deterministic fuzz が複数 seed で panic なく分類される。
- [ ] README に CSV 制約（BOM・encoding・formula injection・
  memory・non-RT）が明記されている。
- [ ] impl record に実行 command・exit code・未検証項目が記録
  されている。

## 完了条件

- `mise run fmt`/`check`/`lint`/`test` 全緑（csv+file 全 feature）。
- feature matrix（default・csv・csv+file・all-features）が
  コンパイル・テストで成立する。
- `DocumentFormat::Csv`/`DocumentPosition.record` が公開 API に
  反映され、backend 型が露出しないことを確認済み。

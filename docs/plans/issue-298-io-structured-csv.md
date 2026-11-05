---
id: "PLAN-034MOVaPAnnuuoKzGdYDxq"
type: "plan"
status: "approved"
title: "Issue #298 ruviform-io-structured — CSV Module 実装計画（IO-IMP-04）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/298"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/src/csv.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-structured/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/README.md"
      - type: "file"
        path: "crates/ruviform-io-structured/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/input.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/file.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/designs/issue-298-io-structured-csv.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/specs/issue-298-io-structured-csv.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/plans/issue-297-io-structured-json-yaml-toml.md"
  refines:
    - "docs/designs/issue-298-io-structured-csv.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-298-io-structured-csv.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-298-io-structured-csv.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #298 ruviform-io-structured — CSV Module 実装計画（IO-IMP-04）

## 概要

設計書 `DESIGN-034k6xAsDaYM7wuYe0CKOF` に基づき、`csv` feature/
module を既存 scaffold に載せる。実装は 2 task に分け、各 task で
検証可能な単位にする。

## Task 分解

### T1: csv module 本体（`src/csv.rs` + 共有拡張）

`docs/tasks/issue-298-t1-csv-module.md`

- `Cargo.toml`: `csv = ["dep:csv", "dep:csv-core"]` feature・両 dep
  を `=1.4.0`/`=0.1.13` の optional・default-features=false で追加。
- `src/error.rs`: `DocumentFormat::Csv`・`DocumentPosition.record`
  フィールド追加・`with_parts`/`with_limit` cfg に `csv` 追加。
- `src/lib.rs`・`src/input.rs`・`src/file.rs`: cfg 条件へ `csv` 追加。
- `src/csv.rs`: 全公開 API（Options・`CsvRecord`・`CsvReader`・
  `CsvWriter`・`from_reader`/`to_writer`/`read_file`/`write_file`）・
  `csv_core::Reader` 駆動 loop・bounded record 組立て・trim・
  BOM probe・typed deserialize/writer。
- `tests/structured.rs` への unit test 追加（AC1/AC2/AC3 系）。

### T2: file feature 合成・試験仕上げ・README

`docs/tasks/issue-298-t2-file-docs.md`

- `read_file`/`write_file` の `file` feature テスト・round-trip。
- deterministic fuzz（seeded LCG による dialect/破断 CSV）。
- `crates/ruviform-io-structured/README.md` feature matrix・CSV
  制約（BOM・encoding・formula injection・memory・non-RT）記述。
- `crates/README.md` の module 一覧に CSV を反映。
- feature matrix・MSRV 補助検証の実施と記録。

## 実装順序

1. `Cargo.toml` + `error.rs` + `lib.rs`/`file.rs` cfg で土台。
2. `src/csv.rs` を Options → `CsvRecord` → `CsvReader` →
   `CsvWriter` → 一発 API の順で実装。
3. `tests/structured.rs` に CSV セクション追加（golden・negative・
   dialect・reusable buffer・position・typed・file）。
4. `README.md`/`crates/README.md` 更新・impl record 作成。
5. `mise run fmt`/`check`/`lint`/`test` + feature matrix + MSRV
   補助検証 → merge origin/main → 最終検証 → push → PR → required
   checks watch。

## 検証計画

| 検証 | 方法 |
| --- | --- |
| fmt/check/lint/test | `mise run fmt`/`check`/`lint`/`test` |
| feature matrix | `cargo check/test` を `mise exec -- cargo ...` で feature 組合せ（default・csv・csv+file・all-features）— canonical でない補助検証として明記 |
| MSRV 1.89 | `mise exec rust@1.89.0 -- cargo check -p ruviform-io-structured --all-features`（環境に 1.89 がある前提、なければ記録） |
| 公開 API 監査 | `cargo doc` build 後に rustdoc source grep / `csv.rs` の pub signature 目視 + 将来の `cargo public-api` 相当で backend 非露出を確認 |
| peak memory / streaming | `Read` を切断して逐次消費を観測する test・record buffer 再利用 test・ベースライン throughput を impl record に記録 |
| fuzz | deterministic seeded LCG fuzz test（external corpus ではなく再現可能な内部生成）— 外部 fuzz harness があればそちらも記録 |
| `file` 合成 | `read_file`/`write_file` round-trip test |

## リスクと対策

- **`csv` crate の内部 buffering との重複**: `csv::Reader` 層を使わず
  `csv_core::Reader` + 自前 BoundedRead + 自前 data/ends buffer に
  することで、全確保点を制御下に置く。`csv` 側の record 組立て
  コードは参考実装としてのみ利用。
- **`ByteRecord` への typed 詰め替え**: `read_typed` だけが
  `ByteRecord` を使う。内部的に fields slice を構築して
  `ByteRecord::from(&fields[..])` → `deserialize` — backend の
  `ErrorKind` を kind 別に正規化して silent truncation を排除。
- **BOM probe での over-read**: `BoundedRead` から最大 4 byte を先に
  読み残りを `prefix` buffer に保持し、reader 駆動 loop の先頭で
  prefix から消費してから `src` に移る。underlying reader を
  余分に消費しない（probe は BoundedRead 内の共通契約内）。
- **trim の in-place 実装**: field 単位で data を memmove するが、
  record サイズ上限内のため O(record) で済む。
- **`csv` の MSRV**: 1.73 なので 1.89 に対し余裕あり。`Cargo.lock`
  で他 feature との版整合が取れるかを `cargo check --all-features`
  で実測。

## Done 判定

- AC1–AC5（spec）を test・実測・記録で全て満たす。
- docs/tasks/issue-298-t1-csv-module.md・issue-298-t2-file-docs.md
  を done として index 再生成。
- impl record（`docs/impl/ir/issue-298-...md`）に command・exit
  code・未検証を記録。

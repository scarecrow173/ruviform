---
id: "IMPL-034V4ltq3DWHBuZVbOLTjH"
type: "impl"
status: "completed"
title: "Issue 284 IO-02 Structured Formats 設計正本照合・契約試験棚卸し slice"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/284"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/tests/api_surface.rs"
      - type: "file"
        path: "docs/impl/ir/issue-284-io-02-structured-formats-slice.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
      - type: "file"
        path: "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
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
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/impl/ir/issue-297-ruviform-io-structured-serde-json-yaml-toml.md"
    - "docs/impl/ir/issue-298-io-structured-csv.md"
    - "docs/impl/ir/issue-299-io-structured-xml.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
  verified-by: []
  references: []
  defers:
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 284 IO-02 Structured Formats 設計正本照合・契約試験棚卸し slice

## Summary

`ruviform-io-structured` の実装済み公開 API・limits/security・
error 分類を Issue #284 採用済み設計（Serde trait のみ公開
例外・backend 型/Error/feature/version 非公開・Format 別
API・low-level/File Convenience 分離・non-RT）と照合し、
設計正本の Verification Notes 8 項目を実文書で確認済みに
更新した。契約試験の棚卸しで特定した headless 検証可能な
不足分 — 横断 Public API 監査・unknown-field の Consumer
制御・数値精度 round-trip・正規化 `Display` の no-leak
（JSON 以外）・file error `source()` chain — を
`tests/api_surface.rs` 新設と `tests/structured.rs` への
試験追加で閉じた。crate の `src/` 実装は無変更（設計不整合は
確認されなかった）。

## Context

- 上流: [spec](../../specs/issue-284-io-02-structured-formats-serde-boundary.md)・
  [design](../../designs/issue-284-io-02-structured-formats-serde-boundary.md)・
  [plan](../../plans/issue-284-io-02-structured-formats-serde-boundary.md)・
  [task](../../tasks/issue-284-io-02-structured-formats-serde-boundary.md)・
  [test-spec](../../test-specs/issue-284-io-02-structured-formats-serde-boundary.md)
  の issue-284 チェーン（前段 docs slice で approved/done）。
- 実装正本: #297（JSON/YAML/TOML）・#298（CSV）・#299（XML）
  の IR と per-format test-spec の audit 規則
  （`issue-297` Rule 5・`issue-298` S8・`issue-299` S7）。
- 本 slice は docs-only 設計 slice の後続で、実装との照合と
  契約試験棚卸し・不足試験の追加を担う。Consumer 回帰・
  #288 Integration Gate・OS/MSRV matrix の consumer 側受入・
  Fuzz 実走（cargo-fuzz 等）は本 slice の対象外。

## Implementation

### Changes

- `crates/ruviform-io-structured/tests/api_surface.rs`（新規）:
  公開型の `std::any::type_name` canonical path が
  `ruviform_io_structured::` 名前空間に留まり backend crate
  名（serde_json/serde_saphyr/saphyr/quick_xml/csv_core 等）を
  含まないことを全公開型で機械確認。`DocumentError` の
  `Send`・正規化 `Display` 接頭辞、Serde bound のみで
  consumer 側 round-trip helper が組み立てられること、
  file error enum の型名も同規則で pin。feature ごとに
  `#[cfg]` で gate。
- `crates/ruviform-io-structured/tests/structured.rs`（追加）:
  - fixtures: `StrictNameDoc`（`#[serde(deny_unknown_fields)]`）
    と `Floats`（f64 精度 probe: 0.1 / 0.30000000000000004 /
    DBL_MAX / -2.5e-10）を追加。
  - JSON: unknown-field は既定で無視・`deny_unknown_fields` で
    `DataModel`、f64 bit-exact round-trip、256 段 nesting が
    backend 固定上限で `Syntax`（`LimitExceeded` に偽装
    しない）であることを `serde_json::Value` への deserialize
    で確認。
  - YAML/TOML: 同型の unknown-field・float precision 試験と、
    marker 文字列を含む malformed 入力で正規化 `Display` が
    入力本文を echo しないこと（ERR-02）を確認。
  - CSV: typed record 失敗時の `Display` no-leak。
  - XML: mismatched tag の streaming 失敗時の `Display`
    no-leak。
  - file（JSON+file）: `DocumentFileReadError`/`WriteError`
    の `Open`/`Decode`/`Encode`/`Publish` 各 variant が
    `Error::source()` で内側原因（io::Error / DocumentError /
    PublishFailure）を保持することを 4 軸で確認。
- `docs/designs/issue-284-…-serde-boundary.md`: Verification
  Notes の 8 項目を文書実体と照合して `[x]` 化し、
  「検証根拠」節に `api_surface.rs` の機械 audit を追記。
- `docs/impl/ir/README.md`: 本記録を手動索引へ追加
  （`new_impl_record.js` は同 README を hand-curated として
  skip するため）。

### Decisions During Implementation

- **API audit は `type_name` ベースの型レベル pin とした**:
  test-spec が要求する「公開 signature に backend 型名がない」
  監査を、外部ツール（cargo-public-api 等）非依存で実現する
  ため、公開型の canonical path 文字列検査を採用。public
  `pub use`/`pub type` alias による backend 型の再露出は
  `type_name` に backend crate path として現れるため検出
  できる。signature 引数・戻り値の backend 型混入は per-format
  test-spec の目視/grep 規則が引き続き担う（本 test はその
  補完であり置き換えではない）。
- **`IgnoredAny` では JSON 深度上限を probe できない**:
  serde_json の `deserialize_ignored_any` は深度 accounting
  を通らず 256 段 nesting を受理したため、実再帰を強制する
  `serde_json::Value`（test 内部利用 — 公開 API 境界とは無関係）
  へ deserialize 先を変更した。backend 固定上限の超過が
  `Syntax` で `LimitExceeded` に偽装されないことを確認。
- **no-leak 試験は format ごとに 1 本ずつ**: JSON 既存試験に
  加え YAML/TOML/CSV/XML に marker 文字列ベースの
  `Display` 検査を追加し、ERR-02（backend 診断は `source()`
  のみ・`Display` は正規化面）を全経路で pin した。
- **`src/` 無変更**: 照合の結果、公開 API・limits・error
  分類・file publish 分離は採用済み設計と一致しており、
  設計不整合に起因する実装変更は不要と判断した。

## Related Experiments

### Adopted

なし（実験ログは作成していない）。

### Rejected

なし。

## Validation

実施済み（canonical = mise task、Windows 11 x64・
rust 1.99.0 toolchain）:

- `mise run fmt-check` — pass（`cargo fmt --all -- --check`）。
- `mise run check-io-structured` — pass（default / `xml` /
  `xml,file` / `--all-features` の 4 組合せ）。
- `mise run lint-io-structured` — pass（同 4 組合せ・
  `clippy --all-targets -D warnings`）。
- `mise run test-io-structured` — pass（同 4 組合せ。
  `--all-features` で integration 129 tests + doctest 12
  全 pass。追加試験: api_surface.rs 9 tests・structured.rs
  新規 12 tests）。
- `mise run test-io-msrv` — pass（rust@1.89.0 で IO 系 6
  crate 一括 + io-compression deflate。io-structured は
  io-e2e dev-dep 経由の feature unify で全 format + file
  が MSRV build に載る）。
- `mise run test-rt` — pass（`cargo test --workspace rt_`
  フィルタ。io-structured は non-RT で rt_ 試験を持たない
  が workspace lane として実走）。

NotRun / Unsupported（本 slice では未実施・理由付き）:

- 両 Consumer（AudioLens/Synvelis）回帰・Golden/byte 互換・
  性能比較 — cross-consumer gate（#288・#219/#294）の実走段階。
  headless 環境では検証不能。
- macOS/Linux 実機での file feature 経路 — Windows のみの
  環境。OS 別 matrix は #288 gate 側。
- cargo-fuzz 等の外部 fuzzer 実走 — seeded deterministic
  fuzz（既存 `deterministic_*_fuzz`・`seeded_fuzz_never_panics`）
  のみ。長時間 fuzz は別 Issue/CI lane。
- `cargo public-api` 等の外部 API diff ツール — test-spec が
  非要求（手書き audit test で代替）。

## Risks

- `type_name` audit は公開「型」の path のみを検査し、
  関数 signature 内の backend 型（仮に将来混入した場合）を
  完全には網羅しない。per-format test-spec の目視/grep
  規則との併用が前提。
- float precision 試験は serde backend の round-trip 保証に
  依存する。backend 更新（`=` pin の更新 PR）で精度表現が
  変わった場合は試験が検出する設計だが、Consumer 側の
  byte 互換までは保証しない（Consumer 責務）。

## Follow-ups

- #288 Integration Gate での OS/MSRV matrix・license/security
  audit の継続監視（本 slice は headless 契約試験のみ）。
- Consumer 側 Golden/byte 互換/性能の独立検証は AL/SV Issue・
  consumer-verify tuple（#294）の段階。
- io-structured 専用 CI lane の配線は #306 系の整備状況に
  追従（現行は `test-io-structured` task が local/CI-shared
  入口）。

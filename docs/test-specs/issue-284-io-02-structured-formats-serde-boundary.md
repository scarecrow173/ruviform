---
id: "TSPEC-034UyJjriooXu9EsvaMpgj"
type: "test-spec"
status: "approved"
title: "Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/284"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
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
    - "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計

## Purpose

Issue #284 の設計正本（[design](../designs/issue-284-io-02-structured-formats-serde-boundary.md)）
が「Serde 公開境界・Format/Version 契約・Resource Budget・
Error 契約・Feature/Backend 確定値・Consumer 利用面・証拠項目
対応を、実在する per-format 正本への参照で構成し、設計合意と
実装/検証の完了を混同していない」ことを検証する。対象 spec は
[spec](../specs/issue-284-io-02-structured-formats-serde-boundary.md)、
実施記録は [task](../tasks/issue-284-io-02-structured-formats-serde-boundary.md) /
[plan](../plans/issue-284-io-02-structured-formats-serde-boundary.md) を参照。
コード変更を伴わない docs 成果物のため、検証は文書検査
（link/relation 解決・参照先の実在確認・内容の突合・lint）で
行う。本 test-spec は横断設計文書の正しさを対象とし、
per-format の実走テスト自体の合否・Consumer 実測は対象外
（実装/受入段階の要件）。

## Feature

IO-02 Structured Formats 横断設計文書（Serde 境界・
Format/Version 契約・Consumer 利用面・証拠項目対応）

## Rules

- **R1（Serde 境界の完全性）**: 設計文書は公開 API に許可
  される外部型を `serde::Serialize`/`serde::de::
  DeserializeOwned`・`std::io` 標準 trait/型・Ruviform 独自型
  に限定し、backend（serde_json/serde-saphyr/toml/csv/
  csv-core/quick-xml）の型・Error・feature 名・version の
  非公開、`Error::source()` 経由のみの診断到達、
  `Display` が backend 文字列・入力 snippet を含めない
  こと、自作 Serializer trait/derive macro/共通
  DOM/`DocumentValue`/FormatRegistry を作らないことを
  明記する。検証根拠として `issue-297` Rule 5・
  `issue-298` S8・`issue-299` S7・#288 matrix S-X6 への
  参照を持つ。
- **R2（Format/Version 契約）**: 5 Format（JSON/YAML/TOML/
  CSV/XML）の API 族（typed document・record streaming・
  event streaming）・単一文書 Strict EOF/TrailingData 契約・
  schema/version/migration/canonical signed bytes/manifest
  semantics の Consumer 所有・`Format A→共通 DOM→Format B`
  の自動変換を提供しないことが記述される。
- **R3（参照の実在性）**: 設計文書・spec・plan・task が
  参照する Issue 番号（#282〜#306・#409・#219・#294・
  AL #139/#143・SV #82/#84/#25）・PR 番号（#354/#361/#363）・
  docs パス（per-format spec/design/test-spec・IO-01・
  #288・#306・#409）・crate 名（`ruviform-io-structured`・
  `ruviform-io-file`・`ruviform-io-e2e`）・mise task 名
  （`test-io-structured`/`check-io-structured`/
  `lint-io-structured`）が作成時点で実在する。存在しない
  Issue/文書/command を根拠として記載しない。
- **R4（Resource Budget 3 層）**: hard limit
  （`InputLimits::max_bytes`・既定 16 MiB・`BoundedRead`
  が唯一の authoritative 実装・境界包含）・parser budget
  （backend 保証範囲を検証したうえでの公開・未対応は
  `Unsupported`/非公開）・advisory（CPU/deserializer
  heap/無限 stream を厳密保証しない）の 3 層が区別され、
  「保証できない制限を黙って無視しない」「byte 上限は
  解析コストの保証ではない」が明記される。
- **R5（Error 契約）**: `DocumentErrorKind` の 7 variant
  （Io/InvalidEncoding/Syntax/DataModel/TrailingData/
  LimitExceeded/Unsupported）・`DocumentFormat` 5 variant・
  `DocumentPosition`/`DocumentLimit` の「判明した成分のみ」
  原則・`DocumentFileReadError`（Open/Decode）・
  `DocumentFileWriteError`（Create/Encode/Publish）の
  失敗軸分離と `PublishFailure` 全情報（effect/phase/
  recovery）保持が記述される。
- **R6（Feature/Backend 確定値）**: feature matrix
  （`default=[]`・`json`/`yaml`/`toml`/`csv`/`xml`/`file`・
  format feature と `file` の独立組合せ）と backend 選定表
  （serde_json 1.0.154・serde-saphyr 2.0.0・toml
  1.1.8+spec-1.1.0・csv 1.4.0+csv-core 0.1.13・quick-xml
  0.42.0・全て `=` pin）が `crates/ruviform-io-structured/
  Cargo.toml` および per-format 設計の「Backend 選定」節と
  一致し、MSRV が全て workspace MSRV 1.89 以下である。
- **R7（Consumer 利用面）**: AudioLens（Manifest・
  schema.json・index.json・Job 再開・Artifact hash・数値
  精度・Dataset binary chunk 非移管）と Synvelis
  （Project Snapshot Serde 型・旧ファイル互換・
  Revision/HEAD/Recovery 分離・Ruviform 単一 Publish ≠
  Project Transaction/HEAD CAS）が、schema/version を
  Ruviform に移さず利用できる面として記述され、AL
  #139/#143・SV #82/#84/#25 の Issue 番号が含まれる。
- **R8（証拠項目対応）**: Issue #284 の OWN/CSV/XML/DOC/
  ERR/COMP 6 項目と Design Acceptance Gate 7 項目の全てが、
  実在する per-format test-spec の rule/節・#288 matrix
  行（S-N1〜S-X6）・実装 Issue/PR へ対応付けられている。
- **R9（設計/実装分離）**: 「実装段階の検証（Consumer 実測・
  OS 別実機・性能比較）と本 slice の文書合意を混同しない」
  旨が明記され、未実施項目は `not run`/`NotRun`/
  `Unsupported` と記録する方針が示される。
- **R10（変更範囲）**: 差分が `docs/` の新規
  `issue-284-*` 文書と生成 index のみであり、`crates/`・
  `Cargo.toml`・`Cargo.lock`・`mise.toml`・`.github/` に
  変更がない。

## Examples

- Given Issue #284 v0.2-6 の OWN-01/02（Borrowed
  Reader/Writer・Streaming 終了と部分出力・所有権/メモリ
  解放）、When 証拠項目対応表を検査する、Then `issue-297`
  Rule 1/6・`issue-298` S1–S5・`issue-299` S1–S5・#288
  matrix S-N4 への参照を持つ行が存在する。
- Given `serde_json::Value` が公開 API の戻り値として
  現れない設計、When Serde 境界節を検査する、Then
  「backend 型/Error/feature/version は非公開・診断は
  `source()` の `dyn Error` のみ・自作 Serializer/DOM
  なし」が明記され `issue-297` Rule 5 等への参照を持つ
  （R1）。
- Given consumer-verify の Synvelis lane が `not run`、
  When Consumer 受入を判定する、Then 「両 Consumer 受入
  済み」とは記録されず `not run` のまま残る — 本書は
  Consumer 独立判定と設計/実装分離を明記している（R7/R9）。
- Given 本 PR の diff、When `git status`/`git diff --stat`
  を確認する、Then 変更は `docs/` の `issue-284-*` 新規
  文書と `docs/*/README.md` index のみ（R10）。

## Guarantees

- 横断契約の記述は per-format 正本（#297/#298/#299 の
  設計・test-spec）への参照で構成され、詳細の複写による
  drift を避ける構造。
- Serde 境界・budget・error 分類の検証は既存 test-spec の
  audit 規則（backend 非露出 grep・kind 分類・境界包含）
  に pin されており、本書はその索引を提供する。
- Backend 選定の版・MSRV・ライセンスは `Cargo.toml` の
  pin と per-format 設計の記録に一致し、独自の未検証な
  主張を追加しない。
- 証拠項目対応表の各行は実在する正本への参照のみを持ち、
  存在しない Issue/文書/command を根拠にしない。
- Consumer の schema/version/canonical bytes 所有と、
  Ruviform 単一 File Publish ≠ Project Transaction/HEAD
  CAS の責務分界が #288 と同一原則で記述される。

## Non-goals

- per-format の実走テスト合否（`issue-297`/`298`/`299`
  test-spec と `mise run test-io-structured` が正本）。
- Consumer repo での Golden/byte 互換/性能の実測
  （#219/#294 consumer-verify・AL/SV Issue が Owner）。
- macOS/Linux 実機・Network/Cloud FS での file feature
  経路の実測。
- 新 Format・backend 更新・schema/version/migration の
  Ruviform 側実装。
- `crates/`・`Cargo.toml`・`mise.toml`・`.github/` の変更。

## Risk

- 本 test-spec が検証するのは横断設計文書の正しさで
  あり、IO-02 の Consumer 受入（COMP-01/02）は #288 gate
  と consumer-verify の実走段階で別途検証される。設計
  文書が完璧でも Consumer 実測が `not run` なら Issue
  #284 の受入は未完である — この分離を崩すと
  「設計合意＝受入完了」の誤認が起きる。
- 参照先 Issue の状態（OPEN/CLOSED）は変動する。本書は
  2026-11-09 時点の snapshot（SV #82 は #288 記録時の
  OPEN から CLOSED へ変動）を記録しており、将来の状態
  変化で接続表の「役割」記述が古くなる可能性がある —
  snapshot 日付の明記で緩和する。

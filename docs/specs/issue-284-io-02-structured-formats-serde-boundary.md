---
id: "SPEC-034Uyb75wxxSWrPfyvFEgh"
type: "spec"
status: "approved"
title: "Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/specs/issue-296-io-file-os-backend.md"
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-409-scoped-enumeration.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/25"
  defers: []
  deferred-by: []
---

# Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計

## What Should Be Built

IO-02「Structured Formats — Serde 公開・JSON/YAML/TOML/CSV/XML
Adapter 契約」（Issue #284、親 #282）の **正本設計文書** を
`docs/` チェーン（spec → design → test-spec → plan → task）
として作成する。具体的には
`docs/designs/issue-284-io-02-structured-formats-serde-boundary.md`
を新設し、Issue #284 本文の合意済み設計（v0.1・v0.2、
2026-11-04）と最終整合性レビュー追記（実装 Issue 分解
#297/#298/#299）を、次の構成要素として文書化する。

1. **Serde 公開境界**: 公開 API に許可される外部型は
   `serde::{Serialize, de::DeserializeOwned}` のみ。具体
   Backend（serde_json / serde-saphyr / toml / csv / csv-core /
   quick-xml）の型・Error・feature 名・version は非公開で、
   診断は `Error::source()` の `dyn Error` 経由のみ。自作
   Serializer trait・derive macro・共通 DOM/`DocumentValue`/
   FormatRegistry を作らない。
2. **Format/Version 契約**: JSON/YAML/TOML の Serde 型付き
   `from_reader`/`to_writer`/`read_file`/`write_file`、CSV の
   Record streaming、XML の Event streaming という Format 別
   API 族。単一文書 API の Strict EOF・TrailingData 契約、
   schema/version/migration/canonical signed bytes/manifest
   semantics は各 Consumer が所有し、`Format A→共通 DOM→B`
   の自動変換を提供しない。
3. **Resource Budget 契約**: `InputLimits::max_bytes` の hard
   limit（`BoundedRead` が唯一の authoritative 実装）と、
   Format 別 parser budget（YAML depth/alias、XML
   depth/attribute/event、CSV record/field）の「実際に強制
   できるものだけを公開し、保証できないものは
   `Unsupported`/`LimitNotGuaranteed` 相当で正直に扱う」
   方針。CPU 時間・deserializer heap・無限 stream の終了は
   advisory であり厳密保証を主張しない。
4. **Error 契約**: `DocumentError`/`DocumentErrorKind`/
   `DocumentFormat`/`DocumentPosition`/`DocumentLimit` の正規化
   分類と、File Convenience の `DocumentFileReadError`/
   `DocumentFileWriteError` が Encode 失敗と Publish 失敗
   （`PublishFailure` の effect/phase/recovery 情報）を分離して
   保持する契約。
5. **Consumer 利用面**: AudioLens（Manifest・schema.json・
   index.json、Job 再開・Artifact hash・数値精度、Dataset の
   独自 binary chunk 意味論は移管しない）と Synvelis
   （Project Snapshot の Serde 型、旧ファイル読み込み互換、
   Revision/HEAD/Recovery の責務分離）が Ruviform の型を
   schema/version を移さずに利用できる面。
6. **他 IO gate との証拠項目対応**: Issue #284 の
   OWN/CSV/XML/DOC/ERR/COMP 証拠項目と Design Acceptance
   Gate を、既存の per-format test-spec（#297/#298/#299）・
   合成 E2E（#306）・cross-consumer gate（#288 の IO-02
   matrix 行 S-N1〜S-X6）へ対応付ける索引。

## Why Now

- IO-02 の実装は main に揃っている: `ruviform-io-structured`
  クレートが #297（JSON/YAML/TOML・PR #354）・#298（CSV・
  PR #361）・#299（XML・PR #363）で merge 済み。ただし
  Issue #284（設計 Issue）自体の正本設計文書は未作成で、
  Format 横断の Serde 境界・version 契約・Consumer 利用面・
  証拠項目対応が per-format 文書（#297〜#299）に分散した
  ままである。
- IO-06 cross-consumer gate（#288）の受入 matrix は IO-02 を
  S-N1〜S-X6 の行で参照しており、その根拠となる設計正本が
  #284 に求められている。
- 合成 E2E（#306・`ruviform-io-e2e`）と旧 API 撤去
  （#305）は main に存在し、Structured→StagedFile 境界の
  実 OS 証拠が得られている。#284 の設計文書はこれらの実装
  成果を消費して「設計として何が固定され、何が Consumer
  責務か」を一箇所に記録する役割を持つ。

## Users and Value

- **Ruviform maintainer / IO-02 Owner**: Serde 境界・Format
  別契約・resource budget・error 分類の正本を得て、後続の
  Format 追加・backend 更新・Consumer 統合の判断基準を
  固定できる。
- **AudioLens / Synvelis Owner**: Manifest/Project schema を
  Ruviform に移さずに typed I/O を利用する面と、byte 互換・
  数値精度・publish 失敗分類の責務分界を確認できる。
- **#288（IO-06 gate）Owner**: IO-02 matrix 行（S-N1〜S-X6）
  の根拠となる設計正本と証拠項目の対応を辿れる。
- **後続の実装・保守担当**: per-format 詳細は #297/#298/#299
  の設計・test-spec を正本として参照し、本書では横断契約のみ
  を確認できる。

## Scope

### In Scope

- `docs/` 配下の文書チェーン（spec/design/test-spec/plan/task）
  と生成 index のみ。**コード変更なし**。
- Issue #284 本文（合意済み設計 v0.1・v0.2・最終整合性
  レビュー追記）の要求事項の文書化。
- IO-01（#295/#296）・per-format 実装（#297/#298/#299）・
  合成 E2E（#306）・旧 API 撤去（#305）・rooted capability
  （#409）・cross-consumer gate（#288）の main 上の設計・
  実装成果を消費した、Serde 境界・Format/Version 契約・
  Consumer 利用面・証拠項目対応の記述。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/` の変更（本 slice は docs 設計専用）。
- per-format の詳細 API・backend 内部・test 規則の再定義
  （#297/#298/#299 の設計・test-spec が正本。本書は横断契約と
  索引のみを所有する）。
- AudioLens/Synvelis repo の変更・実測（cross-repo gate、
  #219/#294 の consumer-verify が Owner）。
- 新規 CI Workflow・証拠 YAML/JSON スキーマ・Dashboard の
  作成。
- Format A→共通 DOM→Format B の自動変換、Schema/version/
  migration/canonical signed bytes の Ruviform 側実装
  （各 Consumer の責務）。

## Requirements

- Serde 公開境界: `Serialize`/`DeserializeOwned` 以外の具体
  Backend 型・Error・feature 名・version を公開 API に露出
  させない契約を、根拠（実装・test-spec の audit 規則）と
  共に明文化する。
- Format/Version 契約: 5 Format の API 族（typed document /
  record streaming / event streaming）・単一文書 Strict EOF・
  format/version・schema・canonical bytes の Consumer 所有を
  区別して記述する。
- Resource Budget: hard limit（`max_bytes`）・parser budget・
  advisory の 3 層と、「保証できない制限を黙って無視しない」
  規則を明文化する。
- Error 契約: `DocumentError` 系の分類語彙と File Convenience
  の失敗軸分離（Encode vs Publish・`PublishFailure` 保存）を
  明文化する。
- Consumer 利用面: AudioLens/Synvelis が schema/version を
  Ruviform に移さず利用できる面と、Dataset binary chunk・
  Project Snapshot/HEAD/Recovery の責務分界を記述する。
- 証拠項目対応: Issue #284 の OWN/CSV/XML/DOC/ERR/COMP と
  Design Acceptance Gate の各項目を、既存 test-spec・#288
  matrix 行・実装 Issue/PR へ対応付ける索引を設計文書に
  含める。
- 「設計の合意 ≠ テスト完了」を明記し、実装済み部分と
  未検証・Consumer 側未実施部分を分離して記録する。

## Acceptance Criteria

- [ ] Serde 境界・Format/Version 契約・Resource Budget・
      Error 契約・Consumer 利用面・証拠項目対応を含む正本
      設計文書が `docs/designs/` に存在する。
- [ ] 公開 API に Serde trait 以外の具体 Backend 型を露出
      しない契約と、その検証根拠（per-format test-spec の
      backend 非露出規則）が対応付けられている。
- [ ] Backend 選定表（採用版・MSRV 1.89 整合・ライセンス・
      メンテ・Feature 構成）が per-format 設計への参照を伴い
      記述されている。
- [ ] Low-level（`from_reader`/`to_writer`/streaming）と
      File Convenience（`file` feature・StagedFile publish
      合成）の分離と依存 DAG（`io-structured` → `io-file`
      一方向・optional・逆依存なし・nonRT）が明文化されて
      いる。
- [ ] Issue #284 の証拠項目（OWN/CSV/XML/DOC/ERR/COMP）と
      Design Acceptance Gate が既存 test-spec（#297/#298/
      #299・#306）と #288 matrix 行（S-N1〜S-X6）へ対応付け
      られている。
- [ ] 両 Consumer の Manifest/Project schema が Ruviform に
      移動されず利用可能である旨と、責務分界が明記されて
      いる。
- [ ] 「設計合意と実装/検証の分離」が明記され、未実施項目は
      `not run`/`NotRun`/`Unsupported` と記録する方針が
      明示されている。
- [ ] 変更が `docs/` のみであり、`crates/`・`Cargo.toml`・
      `Cargo.lock`・`mise.toml`・`.github/` に差分がない。

## Deferred Design Concerns

- Consumer 側の実測・移行（AL Manifest/Dataset の Golden・
  byte 互換、SV Snapshot/HEAD の旧ファイル読み込み互換）|
  cross-repo gate（#219/#294 consumer-verify）と各 Consumer
  Issue が Owner であり本 slice の対象外 | Consumer 候補
  tuple の実走・Consumer repo の受入実施時 | 設計どおりの
  互換性が実測で裏付けられないまま採用されるリスク。
- macOS/Linux 実機での file feature 経路の実測 | 本 slice の
  作業環境は Windows のみ | CI 既存 OS lane または実機環境が
  利用可能になった時点 | OS/FS 差異（dir sync 等）が
  Structured file publish で未検証のまま残る。
- Format 追加・backend 更新時の契約維持 | 新 Format や
  backend メジャー更新の要求が出た時点 | 公開 API 境界・
  budget 保証・error 分類の再監査が必要になるリスク。

## Sources and Relations

- 正本 Issue: [#284](https://github.com/scarecrow173/ruviform/issues/284)
  （IO-02 合意済み設計 v0.1・v0.2・2026-11-04・最終整合性
  レビュー追記 — 実装 Issue #297/#298/#299）。
- 親設計正本: [#282](https://github.com/scarecrow173/ruviform/issues/282)
  （汎用 I/O 基盤 横断設計）。
- 実装 Issue: IO-IMP-03/04/05 = #297/#298/#299（merge 済み、
  PR #354/#361/#363）。IO-01 = #295/#296。IO-05 = #305。
  合成 E2E = #306。rooted capability = #409。
- Gate 接続: #288（IO-06 cross-consumer gate — 本書の証拠
  項目の一部が同 gate の IO-02 matrix 行の根拠となる）。
- Consumer Issue: AudioLens #139/#143、Synvelis #82/#84/#25
  （状態は 2026-11-09 時点の snapshot）。
- 参照設計文書: `docs/designs/issue-297-…`・`issue-298-…`・
  `issue-299-…`（per-format 正本）、`docs/designs/
  issue-295-…`・`issue-296-…`（IO-01 File 契約）、
  `docs/designs/issue-306-…`（合成 E2E）、`docs/designs/
  issue-409-…`（rooted capability）、`docs/designs/
  issue-288-…`（cross-consumer gate）。

---
id: "TSPEC-034ZkLmNpQrStUvWxYzEf3"
type: "test-spec"
status: "approved"
title: "Issue #692 テスト仕様 — 中立契約 port の fail-closed・codec 往復・digest・analyzer"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/specs/issue-692-script-port.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-692-script-port.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-692-script-port.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-692-script-port.md"
    - "docs/specs/issue-692-script-port.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-692-script-port.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-692-script-port.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #692 テスト仕様 — 中立契約 port の fail-closed・codec 往復・digest・analyzer

## Purpose

`crates/ruviform-script` の中立契約 port が、design 完全形の型一式・
両 registry の Fail-Closed resolve・codec 往復・canonical digest・
analyzer port・HostApiSurface 中立性を、fake fixture による
deterministic 検証で保証する。本 slice の対象は中立契約のみであり、
実 Luau backend/Analyzer・synvelis 接続・実 VM/FFI・実 OS・GUI は
対象外（後続 slice / consumer gate / CI lane）。

## Feature

RF-SCRIPT-00 中立契約 port crate（headless）— `crates/ruviform-script`

## Rules

- **R1 Fail-Closed resolve（backend）**: `ScriptBackendRegistry::resolve`
  は未知 language → `UnknownLanguage`、未知 backend → `UnknownBackend`、
  既知 ID 同士の矛盾で候補 0 件 → `NoCandidates`、`contract_version`
  非互換 → `UnsupportedContractVersion`、capability/artifact 不足 →
  `MissingCapability`、候補 2 件以上 → `AmbiguousBackend` を型付き
  エラーで返す。最近似フォールバック・遅延ロード・feature 排他は
  行わない。重複 `ScriptBackendId` の `register` は
  `RegistryError::DuplicateBackend` で拒否。
- **R2 Fail-Closed resolve（analyzer）**: `AnalyzerRegistry::resolve`
  は C1 と同一規則を analyzer 語彙（`UnknownAnalyzer`/
  `AmbiguousAnalyzer{Vec<AnalyzerId>}`）で行い、未登録での
  `analyze` は `AnalyzerError::Unavailable`（F-18）。重複
  `AnalyzerId` は `AnalyzerRegistryError::DuplicateAnalyzer`。
- **R3 codec 往復**: `dematerialize(materialize(v)) == v` が
  `ScriptValue` 全 variant（`Absent`/`Null`/`Bool`/`Integer`（2^53 超
  含む）/`Number`/`String`/`Bytes`/`Array`/`Record`/`Time`/
  `ResourceId`、ネスト含む）で成立。未知 `$rv` tag・field 欠落・
  型不一致・`$rv` 非文字列は `CodecError` で fail-closed。
- **R4 canonical digest**: `canonical_value` は Record key を byte 順
  ソート・`Absent` field 省略・`Time`/`ResourceId`/`Bytes` を `$rv`
  tag 構造で展開・`Number` を `canonical_number` で正規化する。
  `definition_digest`/`source_digest` は `"sha256:v1:<hex>"` 形式。
  同一意味の payload（構築順序差・`Number(1.0)`/`Integer(1)` の
  数値正規化吸収）は同一 digest、意味差は別 digest。トップレベル
  `Absent` は `CanonicalError`。
- **R5 analyzer 契約**: `ScriptAnalyzer::analyze(&self, …)` が
  `AnalyzeRequest` を処理し `AnalyzeResult{source_revision,
  diagnostics, completions}` を返す。未対応 capability 要求は
  `AnalyzerError::Unsupported`、`verify_result_revision` と registry
  `analyze` は `source_revision` 不一致を `StaleResult` で拒否
  （F-14）。Analyzer 成否は実行 Grant を意味しない。
- **R6 HostApiSurface 中立性**: `validate_neutral` は
  `os.`/`fs.`/`net.`/`shell.`/`process.`/`env.`/`sys.`/`danger.`
  prefix の binding 名を `HostSurfaceError` で拒否し、中立名は受理。
- **R7 品質契約**: vendor 型を署名に出さない・`unsafe` なし・全公開
  enum `#[non_exhaustive]`・全 API 非 RT・fake は test 専用で本番
  経路に残さない。

## Examples

- **E1 backend resolve 正常系**: fake backend 2 件（luau/rhai）を
  登録し `language`/`backend`/`required_capabilities`/`artifact_kinds`
  を満たす requirement で `resolve` → 一意な `&dyn ScriptBackend`。
- **E2 fail-closed 行列**: `DuplicateBackend`・`UnknownLanguage`・
  `UnknownBackend`・`NoCandidates`・`AmbiguousBackend`・
  `UnsupportedContractVersion`・`MissingCapability` を個別 case で
  検証（理由区別ごと）。
- **E3 codec 全 variant 往復**: `ScriptValue` 全 variant・ネスト・
  2^53 超 `Integer`・`Time{domain,timebase,count}`・`ResourceId`・
  `Bytes`・`Absent`/`Null` 区別を `materialize`→`dematerialize` で
  等値確認。未知 tag `{"$rv":"bogus"}`・`{"$rv":123}`・field 欠落は
  `CodecError`。
- **E4 digest 安定性/差異**: 同一意味 payload（BTreeMap 構築差・
  `Number(1.0)` vs `Integer(1)`）→ 同一 `DefinitionDigest`。
  値差・`Absent`/`Null` 差 → 別 digest。`source_digest` は同一 text
  で一致・異 text で不一致。
- **E5 analyzer**: `AnalyzerRegistry` fail-closed resolve・fake
  analyzer の `analyze(&self)` が `AnalyzeResult` を返す・
  `wanted.completion` 未対応で `Unsupported{completion}`・
  `verify_result_revision`/registry `analyze` が stale
  `source_revision` を `StaleResult` で拒否・空 registry の
  `analyze` が `Unavailable`。
- **E6 host surface**: `validate_neutral` が `os.exec`/`fs.read`/
  `net.http`/`shell.run`/`process.spawn`/`env.get`/`sys.info`/
  `danger.raw` を拒否し、`action.invoke`/`project.query` 等の中立名を
  受理。
- **E7 session 契約**: fake backend の `open_session` が
  `UnsupportedBudget`/`LanguageMismatch`/`SourceTooLarge` を返し、
  happy path で `evaluate`→`ScriptResult`・`cancel`→`Cancelled`・
  `close` 冪等を確認。

## Guarantees

- 両 registry が Fail-Closed で解決し、未知・曖昧・Version/Capability
  不一致を型付きエラーで拒否する（暗黙フォールバックなし）。
- codec が `ScriptValue` 全 variant を vendor 中立 tag record 経由で
  往復等値にし、未知 tag を fail-closed で拒否する。
- canonical 表現が一箇所（`canonical_value`）に集約され、同一意味の
  定義が同一 `DefinitionDigest` を生成する。
- analyzer port が `&self` で `AnalyzeResult` を返し、stale revision
  を `StaleResult` で検出する。
- `HostApiSurface` が中立契約面のみを受理し、OS/FS/Network/Shell 系
  binding を拒否する。
- vendor 型・unsafe・実 VM/FFI・RT 経路・GUI を含まない headless
  crate であることが build/lint/test で確認できる。

## Non-goals

- 実 Luau/Rhai backend・実 Analyzer・mlua/rhai/vendored Luau/native
  bridge・外部依存追加 — s2 以降の slice。
- synvelis thin adapter 接続・`synvelis-automation` registry 移行 —
  consumer gate / consumer 側後続。
- Host binding handler 実行 seam・OS/FS/Network/Shell binding・
  feature 実行時排他・RT 文脈検知機構・GUI。
- 実 VM/FFI・実 OS・3OS 横断・`*-full`/`*-msrv`/Feature Matrix —
  CI lane / Daily Full CI 委譲（macOS/Linux build は NotRun）。
- `poc/`・`ffi-poc/` の変更・consumer リポジトリの変更・Issue #692
  の close・AC checkbox 更新。

## Risk

- `SourceRange{start,end}` を position 統合形にしたことで backend が
  offset を計算する責務を負う — non-RT のため許容範囲だが、実 backend
  実装時に offset 未対応の診断源があれば再検討（design D5 記録済み）。
- `canonical_number` の数値正規化で `Number(1.0)`/`Integer(1)` が
  同一 digest になる — backend 間の数値表現差吸収が目的の意図的仕様
  であり、型差を digest で区別したい consumer は payload に型 tag を
  含める（design D12 記録済み）。
- `AnalyzerCapabilities::capability_ids()` の導出語彙
  （`analyze.diagnostics`/`analyze.types`/`analyze.completion`）は
  新規 — consumer が要求する語彙と齟齬があれば s2 で調整
  （`CapabilityId` は opaque で非破壊的に拡張可能）。
- fake fixture が本番経路に残ると「fake でない backend」の証拠に
  ならない — `tests/` 専用配置・本番 API に fake を出さない設計で
  担保（design D14）。

---
id: "SPEC-034ZkLmNpQrStUvWxYzAb1"
type: "spec"
status: "approved"
title: "Issue #692 RF-SCRIPT-00 script port crate first slice — 中立契約型一式 + registry/analyzer port + canonical digest"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "https://github.com/scarecrow173/ruviform/issues/558"
    - "docs/designs/issue-558-script-contract-design.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-692-script-port.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-692-script-port.md"
  depends-on:
    - "docs/designs/issue-558-script-contract-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-692-script-port.md"
    - "docs/test-specs/issue-692-script-port.md"
    - "docs/discovery/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-692-script-port.md"
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  defers: []
  deferred-by: []
---

# Issue #692 RF-SCRIPT-00 script port crate first slice — 中立契約型一式 + registry/analyzer port + canonical digest

## What Should Be Built

Ruviform workspace に製品 Script backend registry crate
`crates/ruviform-script` を新設し、RF-SCRIPT-00 中立契約
（`docs/designs/issue-558-script-contract-design.md`、approved）の
design 完全形を headless に実装する:

- **C1 識別子/Registry**: `ScriptLanguageId`/`ScriptBackendId`/
  `ScriptBackendVersion`（`impl_version`+`contract_version` 分離）/
  `BackendCapabilities`/`BackendDescriptor`/`ScriptRequirement`/
  `ScriptBackendRegistry`（`register`/`resolve`/`list`）。resolve は
  Fail-Closed — `NoMatchingBackend`（理由 `UnknownLanguage`/
  `UnknownBackend`/`UnsupportedContractVersion`/`MissingCapability`/
  `NoCandidates` 区別）と `AmbiguousBackend`。最近似フォールバック・
  遅延ロード・feature 実行時排他は禁止。
- **C2 Runtime Port**: `ScriptBackend` trait（`descriptor()`・
  `open_session(&SessionConfig)`）/`ScriptSession` trait
  （`evaluate`/`cancel`/`close` — Send/Sync を無条件に要求しない）/
  `SessionConfig`（`budget`/`host_bindings`/`artifact_kinds`/`source`/
  `placement_hint`/`resume`）/`SessionError`/`ScriptResult`/
  `ScriptDiagnostic`/`SessionId`。
- **C3 値モデル**: `ScriptValue` 閉じた variant 集合（`Absent`/`Null`/
  `Bool`/`Integer(i64)`/`Number(f64)`/`String`/`Bytes`/`Array`/
  `Record(BTreeMap)`/`Time{domain: TimeDomainId, timebase: Rational,
  count: i64}`/`ResourceId{domain: ResourceDomainId, kind, value}` —
  Absent≠Null・Integer 厳密）/`ScriptError`+`ScriptErrorKind`/
  `SourcePosition`/`SourceRange`/`ScriptSource`/`SourceDigest`
  （`"sha256:v1:"` 形式）/`SourceRevision`/`ScriptArtifact`/
  `ArtifactKind`。
- **C4 Budget/Cancel**: `ExecutionBudget`（全項目 `Option`・
  `None`=無制限）/`CancelGranularity`/`CancelToken`/`BudgetKind`/
  `PlacementHint`。
- **C5 Analyzer Port**: `ScriptAnalyzer` trait（`analyze(&self,
  request: &AnalyzeRequest)` — `&self` + 内部可変性が natural）/
  `AnalyzerCapabilities`/`AnalyzerDescriptor`/`AnalyzerRequirement`/
  `AnalyzerRegistry`（本 crate が所有 — synvelis-automation のものは
  後日移行。resolve は C1 と同一 fail-closed 規則）/`AnalyzeRequest`/
  `AnalyzeResult`/`Diagnostic`/`Completion`/`AnalyzerError` +
  `verify_result_revision` 相当の stale 検出 helper。
- **C6 Host API 記述子**: `HostBinding`/`HostSignature`/`HostEffect`/
  `HostApiSurface`（中立契約に OS/FS/Network/Shell binding を含めない。
  `validate_neutral` 相当の中立性検証を持つ）。
- **canonical digest utils + codec**: `canonical_value`（canonical
  表現の authoritative 実装は一箇所）/`definition_digest`/`source_digest`/
  `sha256_hex` + typed payload codec（`materialize`/`dematerialize`
  往復、未知 tag は fail-closed）。
- **fake fixture 検証**: deterministic test double による registry
  fail-closed 行列・codec 往復・digest 安定性/差異・analyzer
  fail-closed/stale・HostApiSurface 中立性の検証。test/fixture 専用で
  本番公開面のデフォルトに fake を残さない。

本 slice は Issue #692 の first slice であり、Issue 全体の受入条件
（AC2 実 Luau backend/Analyzer、AC3 synvelis thin adapter 接続）の
残りは後続 slice / consumer gate へ hand-off する。Issue #692 は
継続 open — 本 slice では close しない。

## Why Now

- `docs/designs/issue-558-script-contract-design.md`（approved）・
  `poc/script-coexist`（#559）・`ffi-poc`（#560）が全て merge 済みで、
  中立契約の型・意味論・Fail-Closed 規則が確定している。consumer
  （synvelis #271〜#274）が接続できる製品契約面が次の gate。
- synvelis 側は `synvelis-script` が consumer 簡約ミラーを保持しており、
  thin adapter 接続（synvelis#271/#272/#273/#274 gate）には Ruviform 側の
  製品契約 crate が先に必要。
- PoC で検証済みの意味論（canonical digest 一致・fail-closed resolve・
  codec 往復）を、本番公開面として vendor 型・unsafe・実 VM/FFI を
  含まない headless crate へ固定する。

## Users and Value

- **Synvelis（consumer）**: `synvelis-script` thin adapter が本 port を
  経由して fake ではない backend を利用できる契約面を提供する
  （接続自体は synvelis 側 Issue の gate）。
- **Ruviform 自身**: 後続 slice（s2 以降の実 Luau backend/Analyzer
  port）が実装すべき契約の authoritative な置き場となり、Backend/
  Analyzer 実装と consumer の双方が同一の型・Fail-Closed 語彙を共有する。
- **AudioLens**: 現時点で Script 非要求（AL #127）。将来適用時の
  影響評価は別途。

## Scope

### In Scope

- `crates/ruviform-script/` の新設（headless 契約 crate、
  `#![forbid(unsafe_code)]`、vendor 型・実 VM/FFI 非含有、
  全公開 enum `#[non_exhaustive]`、全 API 非 RT）。
- C1〜C6 の中立契約型一式 + `ScriptBackendRegistry`/`AnalyzerRegistry`
  の Fail-Closed resolve。
- `canonical_value`/`definition_digest`/`source_digest`/`sha256_hex`
  と `materialize`/`dematerialize` codec（`$rv` tag record 方式）。
- `tests/` の fake backend/analyzer fixture による契約検証
  （registry fail-closed 行列・codec 往復・digest・analyzer・
  HostApiSurface 中立性）。
- Workspace 登録（root `Cargo.toml` members、`crates/README.md` 索引、
  `Cargo.lock` 再生成 — 新規クレート分のみ）と docs chain・
  `evidence/rf692/`。
- AC4 記録: mlua vendored vs native bridge の依存選定は s2 slice で
  RF-SCRIPT-02（ffi-poc）の版整合・3OS 配布結果を参照して決定する旨を
  design 文書と PR 本文へ正直に記録（本 slice では選定しない）。

### Out of Scope

- 実 Luau/Rhai backend・実 Analyzer 実装・mlua/rhai/vendored Luau/
  native bridge 等の外部依存追加（s2 以降の slice — 追加時は依存承認を
  `orca orchestration ask` で先に取る）。
- synvelis thin adapter（`synvelis-script`）の本 port への接続 —
  synvelis#271/#272/#273/#274 の consumer gate。
- `poc/`・`ffi-poc/` の移動・削除・変更（参照のみ）。
- consumer リポジトリ（synvelis・audio-lens）内の一切の変更。
- serde/serde_json/sha2 以外の依存追加・default feature 拡大・
  新規 feature/optional 依存。
- `mise.toml`・`mise.ci.toml`・`mise.lock`・`.github/`・`POLICIES.md`・
  `AGENTS.md`・`apm.yml`・`apm.lock.yaml`・`.gitattributes`・
  `.gitignore`・`.pre-commit-config.yaml` の変更。
- `docs/**/README.md`（docs index — Coordinator が統合時に再生成）。
- `unsafe` コード・実 VM/FFI 実行・OS/FS/Network/Shell Host binding・
  feature 実行時排他・RT 経路・GUI。
- `*-full`・`*-msrv`・Feature Matrix・3OS 横断の必須化・実行
  （Daily Full CI / 対象 SHA の CI lane 委譲）。macOS/Linux build は
  本環境（Windows）では実行不可 — CI lane へ委譲し NotRun と記録。
  実 Luau VM/FFI・実 OS・GUI 経路は本 slice 非含有 — NotRun/Deferred。
- Issue #692 の close・AC checkbox 更新（`Refs #692`、`Closes` 禁止）。

## Requirements

- vendor 型（mlua/rhai/FFI/consumer 具象型）を公開シグネチャに出さない。
- `unsafe` なし。実 VM/FFI 非含有。全 API 非 RT（RT 文脈検知機構は
  新規に作らない — C4 の RT 禁止は呼出し側責務の記述どおり）。
- 公開 enum は全て `#[non_exhaustive]`
  （`docs/adr/public-enum-non-exhaustive-policy.md`）。
- `ScriptSession`/`ScriptBackend`/`ScriptAnalyzer` の trait は
  Send/Sync を無条件に要求しない（配置は `BackendCapabilities` 広告 +
  `placement_hint` で consumer が選択）。
- resolve は両 registry とも Fail-Closed: 未知・曖昧・Version/
  Capability 不一致を型付きエラーで拒否し、暗黙フォールバック・
  遅延ロード・feature 実行時排他を行わない。
- canonical 表現の authoritative 実装は一箇所（`canonical_value`）。
  同一意味の定義は同一 `DefinitionDigest`、表層差（キー順序等）は
  digest に影響しない。
- codec は `materialize`/`dematerialize` の往復で `ScriptValue` 全
  variant（`Absent`/`Time`/`ResourceId`/大きな `Integer` 含む）を
  等値に戻し、未知 tag は fail-closed で拒否する。
- `HostApiSurface::validate_neutral` は OS/FS/Network/Shell 系の
  binding 名を拒否する（中立契約にそれらを含めない）。
- 依存は serde_json/sha2 のみ（Cargo.lock 既在 — 追加理由・ライセンスを
  crate `Cargo.toml` コメントと PR 本文に記録）。
- fake fixture は test/fixture 専用で、本番公開面のデフォルトに残さない。

## Acceptance Criteria

- [ ] `crates/ruviform-script` が workspace member として build でき、
  `mise run check|lint|test ruviform-script`・`mise run fmt-check`・
  `mise exec -- prek run --all-files` が Pass する。
- [ ] C1〜C6 の中立契約型一式が design 完全形で実装され、registry
  fail-closed 行列（重複/未知/0 件/曖昧/version 不一致/capability
  不足）が型付きエラーで検証される。
- [ ] `ScriptValue` 全 variant の codec 往復が等値で、canonical digest
  が同一意味 payload で一致・意味差で不一致になることが検証される。
- [ ] `AnalyzerRegistry` の fail-closed resolve・fake analyzer の
  `analyze(&self)` 往復・`source_revision` 不一致の stale 検出が
  検証される。
- [ ] `HostApiSurface` の中立性検証が OS/FS/Network/Shell 相当の
  binding を拒否することが検証される。
- [ ] AC4（mlua vendored vs native bridge 選定）は s2 slice へ
  Deferred と design 文書・PR 本文に正直に記録されている。
- [ ] Issue #692 の残受入（実 Luau backend/Analyzer = AC2、synvelis
  thin adapter 接続 = AC3）は本 slice 対象外として docs に明記され、
  実 Luau/実 OS/GUI 経路は NotRun/Deferred と記録されている。

## Deferred Design Concerns

- 実 Luau backend/Analyzer の製品 port（AC2 本体）・依存選定
  （mlua vendored vs native bridge）— s2 以降の slice。
- synvelis thin adapter 接続（AC3）— synvelis#271/#272/#273/#274
  consumer gate。
- `synvelis-automation` の `AnalyzerRegistry` の本 crate への移行 —
  consumer 側後続作業。
- Host binding の handler 実行 seam（記述子のみ契約化 — 実 backend が
  必要とする呼出し transport は backend/consumer 実装時に確定）。
- 3OS/`*-full`/`*-msrv`/Feature Matrix — Daily Full CI・対象 SHA の
  CI lane 委譲。macOS/Linux build は CI lane へ委譲（NotRun）。

## Sources and Relations

- `scarecrow173/ruviform#692`（本 Issue — OPEN、auto-close しない）、
  `scarecrow173/ruviform#558`（契約設計の起点 Issue）。
- `docs/designs/issue-558-script-contract-design.md`（approved —
  C1〜C7・Fail-Closed 判定表・Fault Matrix・Fixture 割当の正本）。
- `poc/script-coexist/src/`（#559 — 独立 workspace、意味論 port 元）、
  `ffi-poc/src/{contract,analyzer}.rs`（#560 — analyzer 契約 shape）。
- synvelis `crates/synvelis-script/src/{backend,analyzer,canon}.rs`
  （cross-repo READ-ONLY — `analyze(&self)`・`canonical_json`/
  `digest_of` 先例）。
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3・仮想優先・証拠要件）、
  `docs/adr/public-enum-non-exhaustive-policy.md`。

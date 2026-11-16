---
id: "DISC-034ZkLmNpQrStUvWxYzKl6"
type: "discovery"
status: "resolved"
title: "Issue #692 ruviform-script port crate — 調査記録"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/designs/issue-558-script-contract-design.md"
  changes:
    added: []
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
  related:
    - "docs/specs/issue-692-script-port.md"
    - "docs/designs/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  defers: []
  deferred-by: []
---

# Issue #692 ruviform-script port crate — 調査記録

## Summary

Issue #692 first slice（RF-SCRIPT-00 中立契約の製品 port crate
`crates/ruviform-script` 新設・headless）の実装前調査。契約正本・
PoC 移植元・consumer 先例・crate 雛形を実読し、design 完全形の採用・
serde 非露出・canonical 一箇所集約・fake fixture 境界の 4 論点を確定した
（confidence: high — 全て merge 済み成果物の実読に基づく）。

## Exploration Goal

- `docs/designs/issue-558-script-contract-design.md`（approved）が
  固定する C1〜C6 の型・不変条件・Fail-Closed 判定表を製品 crate へ
  どう写像するかを確定する。
- `poc/script-coexist/src/{contract,value,canonical,codec,analyzer}.rs`
  （#559、独立 workspace）の意味論 port 範囲と、PoC 形状から外す点
  （`ScriptSession: Send`、RT 模擬 guard、VM backend、`&mut self`
  analyzer）を確定する。
- `ffi-poc/src/{contract,analyzer}.rs`（#560）と synvelis
  `synvelis-script` ミラー（cross-repo READ-ONLY）の analyzer 契約
  shape（`analyze(&self)`、`AnalyzerError` 語彙、`verify_result_revision`）
  を製品側へどう統合するかを確定する。
- canonical 表現の authoritative 実装を一箇所に集約する方式
  （手書き `canonical_value` vs serde_json `Value` 経由）と、serde
  derive を公開契約型へ付すかを確定する。
- 依存を serde_json/sha2（Cargo.lock 既在）に限定できるか、fake
  fixture を本番公開面へ残さずに検証できるかを確定する。

## Findings

- **契約正本は design 完全形**: `Time{domain,timebase,count}`・
  `ResourceId{domain,kind,value}`・`HostApiSurface`・
  `SessionConfig.placement_hint`・各 `schema_version` を保持する。
  synvelis ミラーの簡約形（`Time(i64)` 等）は consumer 都合の簡約で
  正本ではない。`poc/script-coexist` の `value.rs`/`contract.rs` が
  design 完全形に最も近い移植元。
- **poc → 製品で外すもの**: `ScriptSession: Send`（poc contract.rs —
  C2 は Send/Sync を無条件要求しない。配置は `BackendCapabilities` 広告 +
  `placement_hint`）、`poc_simulate_rt_context`/`ensure_non_rt`
  （PoC 検証補助・RT 文脈検知機構は新規に作らない）、`backends/`・
  `main.rs`・`plan.rs`・`LuauAnalyzer`/`RhaiAnalyzer`・`HostHandler`
  （VM 実行側は s2 以降）。`AnalyzerRegistry::analyze(&mut self)`
  （ffi-poc 形状）→ `analyze(&self, request)`（synvelis mirror が
  `&self` 先例、内部可変性は実装責務）。
- **analyzer 契約は ffi-poc/synvelis shape が正本に近い**:
  `AnalyzeRequest{source,language,type_definitions,wanted,cursor}`・
  `AnalyzeResult{source_revision,diagnostics,completions}`・
  `AnalyzerError{Unavailable,Unsupported,StaleResult,InvalidRequest,
  VersionMismatch,Internal}`・`verify_result_revision`。
  `type_definitions` は design の `Option<DefinitionSnapshotRef>` に
  対応させ `Option<Vec<TypeDefinition>>` とする。
- **SourceRange/SourcePosition の統合**: poc は `SourcePosition
  {offset,line,col}` + `SourceRange{start:u64,end:u64}`（byte offset）、
  ffi-poc/synvelis は `SourcePosition{line,column}` +
  `SourceRange{start,end}`（position）。design C3 は
  `SourcePosition` = offset/line/col。統合形として
  `SourcePosition{offset:u64,line:u32,col:u32}`（0 始まり line・
  行内 byte col）+ `SourceRange{start:SourcePosition,end:SourcePosition}`
  を採用 — 両 PoC の情報を包含し、synvelis adapter の写像が直接的。
- **canonical は手書き `canonical_value` を authoritative に採用**:
  poc `canonical.rs` は `ScriptValue` から直接 canonical 文字列を生成し
  serde derive を要求しない。synvelis `canon.rs` は `Serialize` 経由の
  `canonical_json`/`digest_of`。製品では `ScriptValue` 直結の手書き
  canonical を正本とし、公開契約型へ serde derive を付さない
  （serde 型を署名へ露出しない — wire format は consumer が未決定）。
  serde_json は canonical 文字列 escape の内部利用に限定。
- **digest 形式**: `"sha256:v1:<hex>"`（`SourceDigest`/`DefinitionDigest`
  共通、design C3 の例示どおり）。`canonical_number` の数値正規化
  （整数値 f64 → 整数 10 進、-0.0→0、NaN/±Inf 特別形）を維持し、
  「backend 間の数値表現差を吸収」「同一意味の定義は同一 digest」の
  不変条件を保つ。
- **codec は tag record 方式を port**: `materialize`/`dematerialize`
  （`{"$rv":…}` 予約 key、未知 tag は fail-closed `Err`）。String error
  を型付き `CodecError`/`CanonicalError` へ格上げ。
- **依存は serde_json + sha2 のみ**: 両方 Cargo.lock 既在
  （serde_json 1.0.154、sha2 0.10.9 — `ruviform-plugin-dock-test`・
  `poc/script-coexist` 先例）。mlua/rhai 等の新規外部依存は本 slice
  非含有（ユーザー指定の承認済みリストに非該当）。
- **fake fixture 境界**: deterministic test double は `tests/` 側に置き、
  `ScriptBackendRegistry::new()` 等の本番経路へ fake をプリ登録しない。
  `#[cfg(test)]` は integration test から見えないため `tests/common`
  パターンを採用。
- **crate 配置**: `crates/README.md` の層構造では利用側層に近いが、
  consumer（synvelis）へ契約を提供する中立 port であり workspace 内
  path 依存を持たない。`docs/designs/ruviform-architecture-design-
  baseline.md` の層構造に照らし「利用側層」セクションへ配置するのが
  妥当（engine 非依存・consumer が実装する契約面）。
- **AC4 記録**: mlua vendored vs native bridge の選定は s2 slice
  （RF-SCRIPT-02 ffi-poc の版整合・3OS 配布結果を参照）で行い、
  本 slice では選定しない — `docs/adr/` は本 slice の lease 外のため
  design 文書と PR 本文へ正直に記録する。

## Evidence

- `docs/designs/issue-558-script-contract-design.md`（approved —
  C1〜C7・Fail-Closed 判定表 F-01〜F-18・Fault Matrix M-01〜M-13・
  Fixture 割当 FX-P/N）
- `poc/script-coexist/src/{contract,value,canonical,codec,analyzer,lib}.rs`
  （#559 — 独立 workspace、意味論 port 元）
- `ffi-poc/src/{contract,analyzer}.rs`（#560 — analyzer 契約の
  FFI 側 shape、`AnalyzerError`/`WantedCapabilities`/`TypeDefinition`/
  `Cursor`/`verify_result_revision` の語彙元）
- `G:/repositry/audio-products/synvelis/crates/synvelis-script/src/
  {backend,analyzer,canon}.rs`（cross-repo READ-ONLY — `analyze(&self)`・
  `canonical_json`/`digest_of`/`sha256_hex` の consumer 先例。
  一切変更しない）
- `crates/ruviform-ui-theme/Cargo.toml`・`crates/ruviform-audio-test/`
  （crate 雛形・test-only 宣言の先例）
- `docs/adr/public-enum-non-exhaustive-policy.md`（公開 enum 方針）

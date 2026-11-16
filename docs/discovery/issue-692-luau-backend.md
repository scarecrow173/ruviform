---
id: "DISC-qxX0PvQy0Qu34AwzZ69G5e"
type: "discovery"
status: "resolved"
title: "Issue #692 s2 Luau analyzer backend — 調査記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
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
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-692-luau-backend.md"
    - "docs/adr/issue-692-luau-dependency-selection.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "evidence/rf560/gap-report.md"
    - "evidence/rf560/revision-alignment.md"
    - "evidence/rf560/license-ledger.md"
  defers: []
  deferred-by: []
---

# Issue #692 s2 Luau analyzer backend — 調査記録

## Summary

Issue #692 second slice（`crates/ruviform-script-luau` 新設 —
`ruviform-script` C5 analyzer port を実装する実 Luau analyzer
backend）の実装前調査。s1 着地済みの中立契約 crate・ffi-poc
（#560、22/22 tests Pass 実績）・AC4 証拠群・consumer 側 interim
adapter を実読し、PoC→製品の型写像・依存選定・unsafe 境界の 3 論点を
確定した（confidence: high — 全て merge 済み成果物の実読に基づく）。

## Exploration Goal

- `crates/ruviform-script/src/{lib,analyzer,value,id}.rs`（s1、
  merge `49f42b72`）が公開する authoritative 契約型と、ffi-poc の
  contract mirror 語彙との差分（drift 点）を確定する。
- `ffi-poc/src/{analyzer,ffi,contract}.rs`・`native/rf_luau_bridge.
  {h,cpp}`・`build.rs`・`tests/{bridge,faults}.rs`・`vendor/luau/`
  の移植範囲と、製品側で意味論を変えるべき点を確定する。
- AC4（mlua vendored vs native bridge）の依存選定を、Cargo.lock 既在
  制約の下で独立に検証し ADR 記録の材料を揃える。
- `analyze(&self)` 契約と shim owner-thread affinity の整合
  （`Mutex`/`RefCell` 内部可変性・`Send`/`Sync` 非要求）を確定する。

## Findings

- **契約 crate は s1 で着地済み**: `ScriptAnalyzer`（`analyze(&self,
  &AnalyzeRequest)`）・`AnalyzerRegistry`（`AnalyzerRequirement` 付き
  fail-closed resolve + `analyze` convenience が wanted-capability
  検査と `verify_result_revision` を内包）・`AnalyzeRequest`/
  `AnalyzeResult`/`Diagnostic`/`Completion`/`TypeDefinition`/
  `WantedCapabilities`/`AnalyzerError`/`SourcePosition{offset,line,
  col}`/`SourceRevision`/`ScriptSource`/`CONTRACT_VERSION=1`。
  trait に Send/Sync 要求なし — backend 実装の Send/Sync は任意。
- **drift 点（s1 契約が authoritative）**:
  - `TypeDefinition` は `source: ScriptSource` + `schema_version`
    （PoC は `source: String` のみ）。`type_definitions` は
    `Option<Vec<_>>`（`None`=未提供 / `Some(vec![])`=空 snapshot）。
  - `cursor` は `Option<SourcePosition>`（`offset`/`line`/`col`）。
    shim の byte offset 引数には `offset` を渡す。
  - `Diagnostic` は `{severity, message, range: Option<SourceRange>,
    code: Option<String>}` — PoC の `kind`/`source_revision` 語彙は
    製品にない。severity は `DiagnosticSeverity::{Info,Hint,Warning,
    Error}`。
  - `Completion` は `CompletionKind::{Text,Method,Field,Keyword,
    Other}` の 5 variant — PoC の 10 variant から写像表が必要
    （property→Field、binding→Text、keyword→Keyword、他→Other）。
  - `SourcePosition` は `{offset,line,col}` — shim JSON は
    `{line,column}` のみ返すため offset は Rust 側で line/col→byte
    offset を再計算する（shim の `offsetToPosition` と同一の改行
    意味論: `\n`・`\r\n`・単独 `\r`）。
  - `AnalyzeResult.source_revision` は request の revision をエコー
    （F-14、`verify_result_revision` で consumer 検査可能）。
- **`analyze(&self)` + 内部可変性**: `BridgeHandle`（FFI ハンドル）は
  可変状態（Frontend + loaded definitions）を持つ。`Mutex` で包み、
  `NonNull` 由来で `Send`/`Sync` は自動的に非実装となる
  （`unsafe impl` 不要・thread-affine を型で正直に表現）。shim の
  owner-thread affinity（status 4 `THREAD_MISMATCH`）は shim 契約
  どおり — cross-thread 呼出は typed `AnalyzerError` で fail-closed。
- **AC4 依存選定（独立検証）**: `Cargo.lock` 既在は `mlua 0.12.2`→
  `mlua-sys 0.13.0`→`luau0-src 0.22.0+luau740`（Luau 0.740、
  `0.22.1+luau744` は yanked — `evidence/rf560/revision-alignment.md`）。
  `luau0-src` は `Analysis/` を同梱せず mlua API も解析面を露出しない
  ため、C5 `SyntaxAndType` diagnostics + completion には vendored
  `Luau.Analysis` + 狭小 C ABI が必須。(b) mlua のみでは capability
  充足不可。(c) vendored 全 tree 自前ビルドは consumer が mlua backend
  と同一リンクした際に Luau シンボル二重定義/ODR リスク。(d)
  ffi-poc vendor 直接参照は PoC/製品分離違反。**(a) native bridge
  （ffi-poc 方式）を採用** — Windows 22/22 Pass・版整合 fail-closed・
  同一 revision VM 共存が PoC で実証済み。
- **版整合不変条件**: vendored `Analysis/` の revision は
  `luau0-src` の Luau revision と同一でなければならない（現行両側
  0.740）。mlua-sys/luau0-src 更新時は再 vendor + build.rs lib 名再
  emit の再照合が必須（`revision-alignment.md` の 0.744→0.740 経緯）。
- **unsafe 境界**: ffi-poc の `#[expect(unsafe_code)]` + SAFETY
  コメント + `src/ffi.rs` 局在パターンを踏襲。`ShimString`（
  `rf_luau_free` Drop）・`BridgeHandle`（create/destroy RAII）・
  status→typed error 写像・create 時 version ネゴシエーションを
  意味論どおり移植。C++/Luau/vendor 型・生ポインタは公開 API に
  出さない（JSON + status code のみ越境）。
- **fault injection seam**: `RF_FFI_POC_FAIL_{INIT,ANALYZE,LOADDEF}`
  （status 7 `RF_LUAU_ERR_INJECTED`）は test-only seam として維持し、
  製品命名 `RF_LUAU_FAIL_{INIT,ANALYZE,LOADDEF}` へ改名する
  （shim コメントで test-only と明記 — design で判断記録）。
- **consumer 側形状（cross-repo READ-ONLY）**: synvelis
  `synvelis-script-luau` は mlua 0.12.2 interim runtime adapter
  （`ScriptBackend` eval 経路・analyze ではない）。thin adapter の
  本 port 接続は synvelis#271/#272/#273/#274 gate — 本 slice 非含有。
- **`crates/ruviform-script` は無変更で済む**: 必要な trait/型は
  s1 で完備（`AnalyzerError` 全 variant・`ScriptLanguageId::new` で
  `"luau"` 構成可）。`ScriptLanguageId::luau()` helper 等の最小追補は
  不要と判断し、契約 crate への変更・回帰検証を回避する
  （design に記録）。

## Evidence

- `crates/ruviform-script/src/{lib,analyzer,value,id}.rs`（s1 merge
  `49f42b72` — 契約 authoritative 形）
- `ffi-poc/src/{analyzer,ffi,contract,lib,ffi_boundary_tests}.rs`・
  `ffi-poc/native/rf_luau_bridge.{h,cpp}`・`ffi-poc/build.rs`・
  `ffi-poc/Cargo.toml`・`ffi-poc/tests/{bridge,faults}.rs`・
  `ffi-poc/vendor/luau/REVISION`（#560 — 移植元、READ-ONLY）
- `docs/designs/issue-558-script-contract-design.md`（approved — C5
  analyzer port 正本）・`docs/designs/issue-692-script-port.md`
  （s1 design — AC4 Deferred 記録）
- `evidence/rf560/{gap-report,platform-matrix,revision-alignment,
  license-ledger,build-link-run}.md`（AC4 選定の証拠 — PoC-proven /
  NotRun の正直な分類）
- `G:/repositry/audio-products/synvelis/crates/synvelis-script-luau/
  src/lib.rs`（cross-repo READ-ONLY — mlua interim adapter の形状
  確認のみ）
- `Cargo.lock`（mlua 0.12.2/mlua-sys 0.13.0/luau0-src 0.22.0+luau740/
  cc/find-msvc-tools/serde_json 既在を確認）
- `docs/adr/public-enum-non-exhaustive-policy.md`（公開 enum 方針）

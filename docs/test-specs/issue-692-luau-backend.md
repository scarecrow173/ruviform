---
id: "TSPEC-PwucpxWy6XmKiZkGa4xT4U"
type: "test-spec"
status: "approved"
title: "Issue #692 s2 Luau analyzer backend — テスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/knowledge/scripting-luau.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-692-luau-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-692-luau-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-692-luau-backend.md"
    - "docs/specs/issue-692-luau-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-692-luau-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-692-luau-backend.md"
  verified-by: []
  references:
    - "docs/specs/issue-692-luau-backend.md"
    - "ffi-poc/tests/bridge.rs"
    - "ffi-poc/tests/faults.rs"
    - "ffi-poc/src/ffi_boundary_tests.rs"
  defers: []
  deferred-by: []
---

# Issue #692 s2 Luau analyzer backend — テスト仕様

`scrates/ruviform-script-luau` の検証仕様。全テストは Windows
x86_64 実機で実行（macOS/Linux は NotRun — AC4 ADR /
`evidence/rf560/platform-matrix.md` 継承）。実 Luau.Analysis を
link する統合テストを含む。

## Feature: analyzer contract conformance

### Rule: descriptor は中立契約を正しく広告する

`LuauAnalyzer::connect(&LuauAnalyzerOptions::default())` の
descriptor が `analyzer_id=="luau-analysis"`、`language=="luau"`、
`contract_version==CONTRACT_VERSION`、`capabilities.diagnostics==
SyntaxAndType`、`capabilities.completion==false`（default）、
`capabilities.max_source_bytes==None`、`supported_languages==
["luau"]`、`version.implementation_version` に `luau0.740` を含む
こと。

### Rule: 有効 source の analyze は revision をエコーする

`AnalyzeRequest{source:{text:"return 1+1",revision:"rev-1",…},
wanted:{diagnostics:Some(SyntaxAndType),completion:None},
cursor:None,type_definitions:None}` → `Ok(AnalyzeResult)` で
`result.source_revision=="rev-1"`、`verify_result_revision` が
通り、error-severity diagnostic が存在しないこと。

### Rule: syntax error は構造化 Diagnostic を返す

構文エラー source（例: `local x = `）→ `Ok`（diagnostics は result
内）で severity `Error`・`code==Some("parse")`・`range` が
`Some(SourceRange)` で offset/line/col が byte 位置と整合する
こと（PoC bridge.rs の syntax error 系を製品型へ写像）。

### Rule: strict mode の type error は Diagnostic を返す

strict `local x: number = "s"` → severity `Error` の diagnostic
（PoC type error 系を写像）。`strict=false` では同 source が
type error を出さないことも確認（strict option の実効性）。

### Rule: completion は enable 時のみ動作する

`enable_completion=true` + cursor 指定の request →
`wanted.completion=Some(_)` で `completions` が非空、
`CompletionKind` 写像（property→Field 等）が spec 表どおり。
`enable_completion=false` で同 request → `Err(Unsupported{
capability:"analyze.completion"})`。

### Rule: type_definitions は request 経由で一時適用される

`Some(vec![TypeDefinition{package_name:"rvGreeter",
source:{text:"export type …",…},schema_version:1}])` → その後の
analyze で型解決された completion/diagnostics が変化（PoC
rfPocGreeter 相当を中立名へ改名）。`add_type_definition` で永続
ロード後は `type_definitions:None` の request でも効果が残る
こと。`BTreeSet` dedup により同名の重複ロードは shim を再呼出
しないこと（shim `load_definition` 呼出回数で検証するか、効果の
冪等性で確認）。

### Rule: schema_version 不一致は VersionMismatch で fail-closed

`source.schema_version != SOURCE_SCHEMA_VERSION`、
`TypeDefinition.schema_version != TYPE_DEFINITION_SCHEMA_VERSION`、
`TypeDefinition.source.schema_version` 不一致の各ケースで
`Err(VersionMismatch{field,expected,actual})`。

### Rule: cursor 不正は InvalidRequest

`cursor.offset > text.len()`、UTF-8 char-boundary でない offset
（マルチバイト文字途中）、completion 要求で `cursor:None` →
`Err(InvalidRequest{reason})`。

### Rule: oversize source は InvalidRequest

`max_source_bytes=Some(n)`（options で小さい値）に対し `len>n`
の source → `Err(InvalidRequest)`（shim status 3 経路）。

### Rule: 決定性 — 同一 request は同一 result

同じ `AnalyzeRequest` を 2 回 analyze → `AnalyzeResult` が
`PartialEq` で一致（diagnostics/completions の順序・内容が
決定的）。

### Rule: mlua VM との共存

同一 test binary で `mlua::Lua` の vendored Luau VM eval と
`LuauAnalyzer::analyze` が両方成功する（AC4 の共存実証 —
PoC coexistence の製品版）。

## Feature: FFI boundary

### Rule: 生 FFI 層が typed error に写像する

`src/ffi_boundary_tests.rs`（`#[cfg(test)]`、crate 内部）:
- invalid UTF-8 byte 列を shim へ → status 1 → `InvalidRequest`
- cursor offset 範囲外 → status 2 → `InvalidRequest`
- oversize → status 3 → `InvalidRequest`
- `enable_completion=false` で completion 要求 → status 5 →
  `Unsupported`
- `bridge_versions` が abi/contract/impl/luau version を報告
- create 時の ABI/contract/struct_size 不一致 → `VersionMismatch`
  （shim create を直接叩く fault 経路 — PoC 同型）

## Feature: fault injection

### Rule: RF_LUAU_FAIL_* env seam が Internal/Unavailable に写像

`tests/faults.rs`:
- `RF_LUAU_FAIL_INIT=1` → `connect` が `Err`（Unavailable or
  Internal — shim status に応じた写像を確認）
- `RF_LUAU_FAIL_ANALYZE=1` → `analyze` が `Err(Internal)`
- `RF_LUAU_FAIL_LOADDEF=1` → `add_type_definition` が `Err(Internal)`
- env は process 全体のため test は直列化（単一 `#[test]` 内で
  順序制御 or `serial_test` 相当 — 依存追加せず単一 test 関数で
  env 設定→実行→解除を逐次実施）

## Feature: registry integration

### Rule: AnalyzerRegistry が実 analyzer を fail-closed で解決する

`tests/registry.rs`:
- 空 registry の `resolve`/`analyze` → `UnknownLanguage`/`Unavailable`
- `LuauAnalyzer` 登録 → `AnalyzerRequirement{language:"luau",
  capabilities:…}` で resolve 成功 → `registry.analyze` が
  `Ok(AnalyzeResult)`（end-to-end）
- 同一 `analyzer_id` の重複登録 → `DuplicateAnalyzer`
- `connect_with_analyzer_id` で 2 instance（id A/B、同 language）
  登録 → hint なし resolve → `AmbiguousAnalyzer`、hint 指定 →
  正しい instance に解決
- `min_contract_version > CONTRACT_VERSION` の requirement →
  `UnsupportedContractVersion`
- `required_capabilities` が descriptor を超過 → `MissingCapability`
- `wanted.completion` 要求だが analyzer の `enable_completion=false`
  → `Unsupported{analyze.completion}`（registry.analyze の
  wanted 検査経路）
- stub `ScriptAnalyzer`（`contract_version=0`）→
  `UnsupportedContractVersion`；stub が誤った `source_revision`
  を返す → `StaleResult`（`verify_result_revision` 経路）

## Feature: unit（analyzer.rs 内）

### Rule: position_offset が shim と同一の改行意味論を持つ

行区切りは Luau Lexer 規約どおり `\n` のみ（`\r` は空白扱い —
単独 `\r` は改行にならず、`\r\n` は `\n` で 1 回計数）。
`\n`/`\r\n`/単独 `\r` 混在 source で line/col→offset 変換が
shim `offsetToPosition` の逆変換として正しいこと（例:
`"a\r\nb\rc\nd"` の line=2,col=0 → offset=7 等）。line 超過 →
`len` clamp。

### Rule: JSON→中立型写像の edge case

未知 severity → `Error`、未知 completion kind → `Other`、
`label` 欠落 completion は skip、`range`/`replace_range` の
`{line,column}` → `SourcePosition{offset,line,col}` 再構成、
malformed JSON → `Internal`（panic しない）。

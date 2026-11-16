---
id: "SPEC-WUXVb2PmGVTsr91ylmdD5J"
type: "spec"
status: "approved"
title: "Issue #692 s2 Luau analyzer backend — 仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-luau-backend.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-692-luau-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-692-luau-backend.md"
  depends-on:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-692-luau-backend.md"
    - "docs/test-specs/issue-692-luau-backend.md"
    - "docs/discovery/issue-692-luau-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-692-luau-backend.md"
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/issue-692-luau-dependency-selection.md"
  defers: []
  deferred-by: []
---

# Issue #692 s2 Luau analyzer backend — 仕様

## Scope

Issue #692 second slice（s2）。s1（RF-SCRIPT-00 中立契約、
`crates/ruviform-script`、merge `49f42b72`）が確立した C5 analyzer
port に対し、`crates/ruviform-script-luau` 新設クレートとして実 Luau
analyzer backend（AC2 の analyzer 側）を実装し、AC4 依存選定を ADR
記録する。RF-SCRIPT-02（ffi-poc）の実証済み native bridge
アーキテクチャを製品化する slice。

- **新設**: `crates/ruviform-script-luau/`（Cargo.toml / build.rs /
  native shim / vendor/luau / src）。
- **移植**: `ffi-poc`（#560、Windows 22/22 tests Pass）の analyzer
  経路を製品契約型へ写像して移植。`ffi-poc` 自体は READ-ONLY 参照で
  変更しない。
- **AC4**: 依存選定を `docs/adr/issue-692-luau-dependency-selection.md`
  に記録（AC4 checkbox 更新は行わない — Issue 側の受入管理）。
- **非含有**: 実 runtime backend（`ScriptBackend`/`ScriptSession`
  実装）は本 slice の対象外（Issue #692 AC2 の runtime 側と synvelis
  adapter は別 slice / 別 consumer gate）。`crates/ruviform-script`
  本体は変更しない。

## Non-goals

- Synvelis thin adapter 接続（AC3、synvelis#271/#272/#273/#274 gate）
  — consumer 側の責務。
- 実 VM / `ScriptBackend` runtime 実装（eval・budget 実行・artifact
  生成）— 仕様は確定済みだが実装は別 slice。`mlua` は shim が
  リンクする Luau VM 静的ライブラリ（mlua→mlua-sys→luau0-src の
  build script 産物）の供給元として **必須依存** — Rust API としては
  本 crate から直接使用しない（coexistence 検証にも兼用）。
- `ffi-poc` の変更・削除・統合 — PoC は証拠として凍結維持。
- 3-OS（macOS/Linux）実機検証、release-size/performance 計測、
  real OS/GUI 経路 — 本 slice は Windows 実機検証のみ
  （`evidence/rf560/platform-matrix.md` の NotRun 分類を継承）。
- Issue #692 AC checkbox 更新・Issue 自動 close。

## Contract surface（`crates/ruviform-script-luau`）

### `LuauAnalyzer`

- `LuauAnalyzer::connect(options: &LuauAnalyzerOptions)
  -> Result<Self, AnalyzerError>` — デフォルト analyzer id
  `"luau-analysis"` で shim 初期化 + version negotiation。
- `LuauAnalyzer::connect_with_analyzer_id(options, analyzer_id:
  AnalyzerId) -> Result<Self, AnalyzerError>` — 同一言語の複数
  instance を区別するための named 版（registry 曖昧性回避用）。
- `LuauAnalyzer::add_type_definition(&self, def: &TypeDefinition)
  -> Result<(), AnalyzerError>` — 型定義の永続ロード（schema
  version 検証 + shim `load_definition`）。
- `impl ScriptAnalyzer for LuauAnalyzer`:
  - `descriptor(&self) -> &AnalyzerDescriptor`
  - `analyze(&self, request: &AnalyzeRequest) ->
    Result<AnalyzeResult, AnalyzerError>`（`&self` — 内部は
    `Mutex` + owner-thread affinity で shim 可変状態を保持）

### `LuauAnalyzerOptions`

`ffi-poc` 同型の pub struct（全 field pub、builder なし）:

- `strict: bool` — Luau strict mode（既定 `true`）
- `enable_completion: bool` — completion 有効化（既定 `false`、
  capability 広告と一致）
- `max_source_bytes: u64` — 0=無制限。shim u32 上限超過は connect
  時に fail-closed 拒否
- `module_time_limit_sec: f64` — module time limit（既定 5.0、
  0=無制限）。負値・NaN は connect 時に `InvalidRequest` で拒否。
  制限超過時は Luau が `timeoutHits` を報告し部分診断しか得られ
  ないため、shim は成功を返さず status 6 → `AnalyzerError::
  Internal` で fail-closed（部分結果を完全な診断として返さない）
- `source_type_module: bool` — module として解析（既定 `true`）

### descriptor 仕様

- `analyzer_id`: `"luau-analysis"`（default）/ connect 引数
- `language`: `ScriptLanguageId::new("luau")`
- `version`: `ScriptBackendVersion::new("ruviform-script-luau/
  <crate_ver>+luau<rev>", CONTRACT_VERSION)` — shim 報告の
  `impl_version` + `luau_version` を合成
- `capabilities`: `diagnostics: SyntaxAndType`、`completion`:
  options.enable_completion、`max_source_bytes`:
  0→`None`/非 0→`Some(n)`、`supported_languages: ["luau"]`
- `contract_version`: `ruviform_script::CONTRACT_VERSION`（=1）

### `AnalyzeRequest` → shim 入力写像

| 中立契約 | shim 呼出 |
| --- | --- |
| `request.source.text` | `analyze` の `source` 引数（UTF-8 検証済み String） |
| `request.source.revision` | 結果 `source_revision` エコー + `verify_result_revision` 対象 |
| `request.cursor` `Some(SourcePosition{offset,..})` | byte `offset` を `i64` へ（範囲/char-boundary 検証後） |
| `request.cursor` `None` | cursor なし（completion は `Unsupported`） |
| `request.wanted.diagnostics` | `None`/`Unsupported`→空要求、`SyntaxAndType`/`SyntaxOnly`→全量（shim は severity 区別なしで全 diagnostics 返却 — capability 広告と整合） |
| `request.wanted.completion` | `Some(_)`→completion 有効要求（`enable_completion=false` 時 shim status 5 → `Unsupported{analyze.completion}`）、`None`→completion 無効 |
| `request.type_definitions` | `Some(defs)` → 各 `TypeDefinition` の `schema_version` + `source.schema_version` 検証後、未ロード `package_name` を shim `load_definition`（instance 内 `BTreeSet` dedup）。`None` → 追加ロードなし。ロード済みは analyzer 寿命中保持 |

### 結果写像

- `AnalyzeResult.source_revision` = `request.source.revision`（F-14
  エコー、PoC 踏襲）
- `diagnostics[]`: shim JSON → `Diagnostic{severity, message,
  range, code}`。severity 写像: `error`→`Error`、`warning`→
  `Warning`、`information`→`Info`、`hint`→`Hint`、未知→`Error`
  （fail-visible）。`code`: shim 文字列をそのまま `Some`（"parse"
  等）。`range`: shim `{start:{line,column},end:{...}}` →
  `SourceRange{start,end}` — `offset` は Rust 側で line/column →
  byte offset を再計算（shim `offsetToPosition` と同一改行意味論:
  Luau Lexer 規約どおり `\n` のみ — `\r` は空白扱いで単独 `\r` は
  改行にならず、`\r\n` は `\n` で 1 回計数）。JSON 欠落/型不一致
  field は `None`。
- shim JSON のトップレベル schema は fail-visible に検証する:
  非 object / `diagnostics` 非 array / completion 要求時
  `completions` 非 array / diagnostics の非 object エントリは
  `AnalyzerError::Internal`（silent な空成功にしない）。
- `completions[]`: shim JSON → `Completion`。`kind` 写像:
  `property`→`Field`、`binding`→`Text`、`keyword`→`Keyword`、
  `string`/`type`/`module`/`generated_function`/`require_path`/
  `hot_comment`/`unknown`/その他→`Other`。`label` 必須（欠落は
  skip）、`detail`/`insert_text`/`replace_range` は `Option` 写像、
  `deprecated`/`completion_context` は中立契約に対応 field がない
  ため破棄。
- `completion` 未要求時は `completions: vec![]`。

### error 写像（shim status → `AnalyzerError`）

| status | `AnalyzerError` |
| --- | --- |
| 1 INVALID_UTF8 | `InvalidRequest{reason}` |
| 2 CURSOR_RANGE | `InvalidRequest{reason}` |
| 3 SOURCE_TOO_LARGE | `InvalidRequest{reason}` |
| 4 THREAD_MISMATCH | `InvalidRequest{reason: "…different thread…"}` |
| 5 COMPLETION_UNSUPPORTED | `Unsupported{capability: "analyze.completion"}` |
| 6 INTERNAL | `Internal{message: "shim internal error (status 6): …"}` |
| 7 INJECTED | `Internal{message: "shim error (status 7): …"}`（test-only fault seam） |
| 8 VERSION | `VersionMismatch{field, expected, actual}` |
| その他/未知 | `Internal{message: "shim error (status N): …"}` |

`connect` 時の version negotiation 失敗（ABI/contract/struct_size/
impl_version 空）も `AnalyzerError` の typed variant に写像
（ffi-poc の `ConnectError` を `AnalyzerError` へ統合）。

### 事前検証（analyze 呼出前、shim 到達前に fail-closed）

- `request.source.schema_version != SOURCE_SCHEMA_VERSION` →
  `VersionMismatch{field:"source.schema_version",…}`
- `cursor.offset > text.len()` または非 char-boundary →
  `InvalidRequest{reason}`
- `cursor.line`/`col` は shim が offset のみ使用するため
  追加検証なし（offset が権威）
- `type_definitions` 各要素の `schema_version` /
  `source.schema_version` 不一致 → `VersionMismatch`
- `max_source_bytes != 0` かつ `text.len() > max` → shim status 3
  経由 `InvalidRequest`（事前にも同等検査を shim 到達前に実施 —
  shim 契約と二重防御ではなく単一の早期 reject として shim 側
  status 3 は defense-in-depth）

### thread affinity / Send・Sync

- shim の owner-thread affinity を維持: `connect` を呼んだ
  thread のみが `analyze`/`add_type_definition` を呼べる。
  `Mutex<Inner>` で内部可変性を提供するが `BridgeHandle` の
  `NonNull` により `Send`/`Sync` は自動的に非実装 — `Box<dyn
  ScriptAnalyzer>`（Send/Sync 非要求）として registry 登録可能。
- cross-thread 呼出は shim status 4 → `InvalidRequest`（shim 契約
  どおりの defense-in-depth。safe code では到達不能だが FFI 層の
  契約として維持）。

## ADR 参照

- `docs/adr/issue-692-luau-dependency-selection.md` — AC4 依存選定
  （native bridge 採用の決定記録）。
- `docs/adr/public-enum-non-exhaustive-policy.md` — 公開 enum は
  `#[non_exhaustive]`（本クレートの公開 enum は contract 型再利用
  のみで新規公開 enum なし — `LuauAnalyzerOptions` は struct）。
- `docs/adr/issue-558-script-backend-contract.md` — 中立契約の
  正本（C5 analyzer port）。

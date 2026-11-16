# ruviform-script

言語非依存の Script/Analyzer 中立契約 port クレート（Issue #692 /
RF-SCRIPT-00 first slice）。

`docs/designs/issue-558-script-contract-design.md`（approved）の
design 完全形を headless に実装する。script backend / analyzer の
実装者と consumer（Synvelis `synvelis-script` 等）が共有する型・
Fail-Closed 語彙・canonical digest・codec の authoritative な置き場。

## 責務

- **C1 識別子/Registry**: `ScriptLanguageId`/`ScriptBackendId`/
  `ScriptBackendVersion`/`BackendCapabilities`/`BackendDescriptor`/
  `ScriptRequirement`/`ScriptBackendRegistry` — resolve は fail-closed
  （未知・0 件・version 不一致・capability 不足・曖昧を型付きエラーで
  拒否。最近似フォールバック・遅延ロード・feature 実行時排他なし）。
- **C2 Runtime Port**: `ScriptBackend`/`ScriptSession` trait・
  `SessionConfig`/`SessionError`/`ScriptResult`/`ScriptDiagnostic`。
  Send/Sync を無条件に要求しない（`BackendCapabilities` が広告、
  `placement_hint` が要求）。
- **C3 値モデル**: `ScriptValue` 閉じた variant 集合（`Absent`≠`Null`・
  `Integer(i64)` 厳密・`Time{domain,timebase,count}`・
  `ResourceId{domain,kind,value}`）・`ScriptError`/`ScriptErrorKind`・
  `ScriptSource`/`SourceDigest`/`SourceRevision`/`ScriptArtifact`/
  `ArtifactKind`。
- **C4 Budget/Cancel**: `ExecutionBudget`（全 `Option`・`None`=無制限）・
  `BudgetKind`・`CancelGranularity`・`CancelToken`・`PlacementHint`。
  全 API 非 RT。
- **C5 Analyzer Port**: `ScriptAnalyzer`（`analyze(&self, …)`）・
  `AnalyzerRegistry`（backend registry と独立・同一 fail-closed 規則）・
  `AnalyzeRequest`/`AnalyzeResult`/`Diagnostic`/`Completion`/
  `AnalyzerError`・`verify_result_revision`（stale 検出 helper）。
- **C6 Host API 記述子**: `HostApiSurface`/`HostBinding`/`HostSignature`/
  `HostEffect` — 記述子のみ。`validate_neutral` が OS/FS/Network/Shell
  系 binding 名を拒否。handler 実行 seam は実 backend slice で確定。
- **canonical/digest/codec**: `canonical_value`（authoritative 一箇所）・
  `definition_digest`/`source_digest`/`sha256_hex`・`materialize`/
  `dematerialize`（`{"$rv":…}` tag record、未知 tag は fail-closed）。

## 層・依存境界

利用側層（engine と同順位・engine ではない）。workspace 内
`path` 依存を持たず、外部依存は `serde_json`（canonical 文字列
escape の内部利用のみ）と `sha2`（digest 計算）のみ — いずれも
Cargo.lock 既在。公開 API に serde 型・vendor 型を露出しない。
`#![forbid(unsafe_code)]`。

## 非目標（本 slice）

- 実 Luau/Rhai backend・実 Analyzer・mlua/rhai/vendored Luau/native
  bridge 等の外部依存 — s2 以降の slice。
- synvelis thin adapter 接続 — synvelis#271/#272/#273/#274 consumer gate。
- 依存選定（mlua vendored vs native bridge）— s2 slice で
  RF-SCRIPT-02（ffi-poc）の版整合・3OS 配布結果を参照して決定
  （AC4 記録。`docs/designs/issue-692-script-port.md` 参照）。

## 検証

```sh
mise run check ruviform-script
mise run lint ruviform-script
mise run test ruviform-script
mise run fmt-check
```

fake backend/analyzer fixture（`tests/`）で registry fail-closed 行列・
codec 往復・digest 安定性/差異・analyzer・host surface を検証する。
実 VM/FFI・実 OS・GUI 経路は対象外。

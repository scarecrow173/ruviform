# ruviform-script-luau

実 Luau analyzer backend（Issue #692 s2 / RF-SCRIPT-03）。
`ruviform-script`（RF-SCRIPT-00 中立契約）の C5 `ScriptAnalyzer`
port を、vendored Luau.Analysis（Luau 0.740）+ 狭小 C ABI shim
（`native/rf_luau_bridge.{h,cpp}`）で実装する。

## 責務

- **`LuauAnalyzer`**: `ScriptAnalyzer` 実装。`connect`/`connect_with_analyzer_id`
  で shim 初期化 + ABI/contract/struct_size version negotiation
  （fail-closed）。`analyze(&self)` は `Mutex` 内部可変性で shim の
  可変状態を保持し、owner-thread affinity を型で表現する
  （`NonNull` 由来で `Send`/`Sync` 非実装）。
- **`LuauAnalyzerOptions`**: strict / enable_completion /
  max_source_bytes / module_time_limit_sec / source_type_module。
- **型定義**: `add_type_definition`（永続）と `AnalyzeRequest.
  type_definitions`（request 同梱・package_name dedup）で
  `.d.luau` 相当の定義を shim へロードする。型定義の再定義
  セマンティクスは Luau の global binding 上書きに従う
  last-write-wins — request 同梱の同名 package は dedup で skip、
  `add_type_definition` の同名再ロードは宣言名 binding を上書き
  する（`tests/bridge.rs` の redefinition テストで固定）。
- **結果写像**: shim JSON → 中立 `Diagnostic`/`Completion`。
  severity/kind の写像表・line/col→byte offset 再計算
  （`position_offset` — shim `offsetToPosition` の逆変換）・
  `source_revision` エコー（F-14）を契約どおり行う。

## 層・依存境界

利用側層。`ruviform-script` への path 依存 + `serde_json`（shim JSON
パース）+ `mlua`（**link 順序のための必須依存** — shim がリンクする
Luau VM 静的 lib は mlua→mlua-sys→luau0-src の build script が供給
する。Rust API としては未使用。runtime `ScriptBackend` 実装は別
slice — AC4 ADR `docs/adr/issue-692-luau-dependency-selection.md`）。
build 依存は `cc` + `find-msvc-tools`。

公開 API は `LuauAnalyzer`/`LuauAnalyzerOptions`/`luau_language`/
`DEFAULT_ANALYZER_ID` のみ — Luau/vendor/C++/mlua 型・生ポインタ・
shim status code は公開面に出さない。`unsafe` は `src/ffi.rs` に
局所許可で隔離（`#[expect(unsafe_code)]` + SAFETY コメント）。

## 版整合不変条件（AC4）

`vendor/luau/Analysis` の revision は `luau0-src`（mlua-sys 経由）
の Luau revision と同一でなければならない（現行両側 0.740）。
mlua/luau0-src 更新時は vendor/luau の再 vendor と `build.rs` の
link lib 名再照合が必須。`Cargo.lock` 既在依存のみで構成。

## 非目標（本 slice）

- 実 runtime backend（`ScriptBackend`/`ScriptSession` 実装）— 別 slice。
- Synvelis thin adapter 接続 — synvelis#271/#272/#273/#274 consumer gate。
- 3-OS（macOS/Linux）実機検証・release-size/performance — Windows
  x86_64 のみ実測（`evidence/rf560/platform-matrix.md` の NotRun
  分類を継承）。

## test-only seam

shim は fault injection 用に `RF_LUAU_FAIL_INIT` / `RF_LUAU_FAIL_ANALYZE` /
`RF_LUAU_FAIL_LOADDEF` 環境変数を読む（設定時は対応操作が status 7
`INJECTED` → `AnalyzerError::Internal` を返す）。`tests/faults.rs`
専用の seam であり、consumer 環境でこれらを設定しないこと。

## 検証

```sh
mise run check ruviform-script-luau
mise run lint ruviform-script-luau
mise run test ruviform-script-luau
mise run fmt-check
```

`tests/`（bridge/faults/registry）で製品契約の統合・fault injection・
registry fail-closed 行列を、`src/ffi_boundary_tests.rs` で FFI 生
境界を検証する。実 Luau.Analysis の C++ ビルドを伴うため初回は
時間がかかる。

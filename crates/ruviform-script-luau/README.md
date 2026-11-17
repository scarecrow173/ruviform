# ruviform-script-luau

実 Luau backend（Issue #692 s2 / RF-SCRIPT-03 + Issue #716）。
`ruviform-script`（RF-SCRIPT-00 中立契約）の C5 `ScriptAnalyzer`
port を vendored Luau.Analysis（Luau 0.740）+ 狭小 C ABI shim
（`native/rf_luau_bridge.{h,cpp}`）で、C2 `ScriptBackend`/
`ScriptSession`（eval/budget/artifact 実行経路）を mlua 0.12.2
Rust API（vendored Luau 0.740 — 同一 revision）で実装する。

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

- **`LuauBackend`**: `ScriptBackend` 実装（Issue #716）。
  `open_session` で fail-closed 検証（language/digest/size/
  budget/artifact kind/host surface）後、session 毎に fresh
  sandboxed `Lua` を生成。`evaluate` は text-mode chunk を 1 回
  実行し、wall-time（interrupt deadline）/memory
  （`set_memory_limit`）/output（canonical size）budget・
  cooperative cancel（contract `CancelToken` → interrupt
  closure）・record payload artifact（`ScriptArtifact::new` が
  digest を計算）を中立契約型で返す。`max_instructions` は
  mlua に API がなく `UnsupportedBudget`。host binding は
  記述子の検証・受理のみ（実行 seam は契約に非存在）。

## 層・依存境界

利用側層。`ruviform-script` への path 依存 + `serde_json`（shim JSON
パース）+ `mlua`（vendored Luau VM — shim がリンクする Luau 静的
lib の供給元であり、runtime backend がその Rust API を使用する。
依存選定: `docs/adr/issue-692-luau-dependency-selection.md` +
`docs/adr/issue-716-runtime-backend.md`）。
build 依存は `cc` + `find-msvc-tools`。

公開 API は `LuauAnalyzer`/`LuauAnalyzerOptions`/`luau_language`/
`DEFAULT_ANALYZER_ID`/`LuauBackend` のみ — Luau/vendor/C++/mlua 型・
生ポインタ・shim status code は公開面に出さない。`unsafe` は
`src/ffi.rs` に局所許可で隔離（`#[expect(unsafe_code)]` + SAFETY
コメント）。runtime module（`src/runtime.rs`）は safe Rust のみ。

## 版整合不変条件（AC4）

`vendor/luau/Analysis` の revision は `luau0-src`（mlua-sys 経由）
の Luau revision と同一でなければならない（現行両側 0.740）。
mlua/luau0-src 更新時は vendor/luau の再 vendor と `build.rs` の
link lib 名再照合が必須。`Cargo.lock` 既在依存のみで構成。

## 非目標

- Synvelis thin adapter 接続 — synvelis#516（port 交換・binding
  適合）/ synvelis#267 AUTX-08（SCRIPT-RUN/OS E2E）consumer gate。
  実 OS/実 VM 経路の受入も同 gate へ委譲。
- host binding 実行 seam — 中立契約に handler 経路が存在しない
  ため本 backend は host call を実行しない（記述子の検証・受理
  のみ）。実 binding 適合は consumer gate。
- named entry points・evaluate 再実行・Resume VM 復元 —
  fail-closed（`Unsupported`/`Internal`）。
- `max_instructions` budget — mlua に命令数 budget API がなく
  `UnsupportedBudget` で拒否。
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

`tests/`（bridge/faults/registry/runtime）で製品契約の統合・
fault injection・registry fail-closed 行列・runtime backend の
open/evaluate/cancel/budget/artifact 行列を、
`src/ffi_boundary_tests.rs` で FFI 生境界を検証する。実
Luau.Analysis の C++ ビルドを伴うため初回は時間がかかる。

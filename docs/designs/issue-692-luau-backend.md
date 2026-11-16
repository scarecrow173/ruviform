---
id: "DESIGN-6X6G9umyA344m7SixphdpZ"
type: "design"
status: "approved"
title: "Issue #692 s2 Luau analyzer backend — 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-692-luau-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-692-luau-backend.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-692-luau-backend.md"
    - "docs/test-specs/issue-692-luau-backend.md"
    - "docs/discovery/issue-692-luau-backend.md"
    - "docs/adr/issue-692-luau-dependency-selection.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-692-luau-backend.md"
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/issue-692-luau-dependency-selection.md"
    - "ffi-poc/src/analyzer.rs"
    - "ffi-poc/src/ffi.rs"
    - "ffi-poc/native/rf_luau_bridge.h"
    - "ffi-poc/native/rf_luau_bridge.cpp"
    - "ffi-poc/build.rs"
    - "ffi-poc/tests/bridge.rs"
    - "ffi-poc/tests/faults.rs"
  defers: []
  deferred-by: []
---

# Issue #692 s2 Luau analyzer backend — 設計

## Overview

`ffi-poc`（RF-SCRIPT-02、#560）の Luau.Analysis native bridge を
`crates/ruviform-script-luau` として製品化し、s1 の
`ruviform-script` C5 `ScriptAnalyzer` 契約を実装する。公開 API は
中立契約型のみ — Luau/vendor/C++/FFI 生ポインタは crate 内部に
閉じる。analyzer と runtime は分離維持（本 slice は analyzer のみ
実装、`ScriptBackend` runtime は実装しない）。

依存選定は AC4 ADR（`issue-692-luau-dependency-selection.md`）で
(native bridge) を決定済み — 本設計はその実装形。

## Architecture

```
crates/ruviform-script-luau/
├── Cargo.toml            # ruviform-script path dep + serde_json + mlua(link) + cc/find-msvc-tools(build)
├── build.rs              # vendored Luau.Analysis C++ コンパイル（ffi-poc build.rs 踏襲）
├── native/
│   ├── rf_luau_bridge.h  # C ABI（shim 契約 — version/negotiation/status/JSON）
│   └── rf_luau_bridge.cpp# Luau.Analysis 呼出し + JSON 直列化 + fault injection seam
├── vendor/luau/          # Luau 0.740 vendored tree（Analysis/Ast/Common/Compiler/Config/…）
│   ├── REVISION
│   └── LICENSE 等
├── src/
│   ├── lib.rs            # crate doc + #[expect(unsafe_code)] 局所許可 + re-export + ffi_boundary_tests
│   ├── analyzer.rs       # LuauAnalyzer/LuauAnalyzerOptions/ScriptAnalyzer impl/JSON→中立型写像
│   ├── contract.rs       # shim JSON schema 定数 + status code 定数（ffi-poc contract.rs 踏襲）
│   └── ffi.rs            # unsafe FFI 層: extern "C" 宣言・ShimString・BridgeHandle・CreateOptions
└── tests/
    ├── bridge.rs         # 製品契約統合テスト（analyze/diagnostics/completion/typedef/version）
    ├── faults.rs         # RF_LUAU_FAIL_* fault injection（env seam）
    └── registry.rs       # AnalyzerRegistry end-to-end（fail-closed 行列 + 実 analyzer 登録）
```

`ffi-poc` は READ-ONLY 参照 — コピー元としてのみ使用し、PoC 側は
変更しない。`vendor/luau/` は `ffi-poc/vendor/luau` から byte 同一
コピー（Luau 0.740、`luau0-src 0.22.0+luau740` と同一 revision —
版整合不変条件）。

## Module design

### `src/ffi.rs` — unsafe FFI 層（唯一の unsafe 集中）

`ffi-poc/src/ffi.rs` を製品化して移植。公開面は crate 内 private
（`mod ffi;`、`pub(crate)` のみ）。

- `extern "C"` 宣言: `rf_luau_bridge_versions` /
  `rf_luau_bridge_create` / `rf_luau_bridge_destroy` /
  `rf_luau_bridge_analyze` / `rf_luau_bridge_load_type_definition` /
  `rf_luau_bridge_free`。
- `BridgeHandle(NonNull<c_void>)` — RAII。`Drop` で
  `rf_luau_bridge_destroy`。`Send`/`Sync` は `NonNull` により
  自動非実装（`unsafe impl` 不使用）。
- `ShimString` — shim が `rf_luau_free` で解放する文字列の RAII
  wrapper（`from_raw`/`Drop`）。
- `CreateOptions` — `strict`/`enable_completion`/
  `max_source_bytes: u32`/`module_time_limit_sec: f64`/
  `source_type_module` を `#[repr(C)]` で shim ABI に整合。
- `bridge_versions() -> BridgeVersions {abi_version,
  contract_version, impl_version, luau_version, struct_size}`。
- 各 FFI 呼出に SAFETY コメント（shim 契約: owner-thread、
  handle 生存中のみ、引数 UTF-8、戻り値は `rf_luau_free` で解放）。

### `src/contract.rs` — shim 契約定数

`ffi-poc/src/contract.rs` 踏襲。status code 定数
（`RF_LUAU_OK`=0 … `RF_LUAU_ERR_VERSION`=8）・JSON field 名・
version 期待値。製品では `AnalyzerError` variant への写像表を
`analyzer.rs` が持つ。

### `src/analyzer.rs` — 製品 analyzer

`ffi-poc/src/analyzer.rs` の `LuauAnalyzer` を中立契約へ写像。

- `LuauAnalyzerOptions` — ffi-poc 同型 pub struct（strict /
  enable_completion / max_source_bytes: u64 / module_time_limit_sec /
  source_type_module）。`Default` 実装あり。
- `LuauAnalyzer`:
  ```rust
  pub struct LuauAnalyzer {
      descriptor: AnalyzerDescriptor,
      state: Mutex<Inner>, // Inner { handle: BridgeHandle, loaded: BTreeSet<String>, owner: ThreadId }
  }
  ```
  `Mutex` は poison 時 `Internal` に写像。`NonNull` 由来で
  `Send`/`Sync` 非実装 → `Box<dyn ScriptAnalyzer>` 登録可・
  thread-affine を型で正直に表現。`analyze(&self)` は `MutexGuard`
  取得 → owner thread 検査 → shim 呼出。
- `connect`/`connect_with_analyzer_id`: `bridge_versions()` で
  `abi_version`/`contract_version`/`struct_size`/`impl_version` を
  fail-closed 検査 → `BridgeHandle::create` → descriptor 構築。
  `max_source_bytes > u32::MAX` は `InvalidRequest` で早期拒否。
- `analyze`:
  1. `source.schema_version` 検査 → `VersionMismatch`
  2. `type_definitions` 各要素の schema 検査 + 未ロード分を
     `load_definition`（`loaded` set で dedup）
  3. `cursor` 検証（offset ≤ len、char-boundary）→ byte offset `i64`
  4. `wanted` → shim flags（completion 要求時は shim が
     `enable_completion=false` で status 5 を返す経路を維持）
  5. `handle.analyze(source, cursor_offset, want_completion)` →
     JSON 文字列
  6. status → `AnalyzerError` 写像（spec の表どおり）
  7. JSON → `AnalyzeResult`（diagnostics/completions 写像、
     `source_revision` エコー）
- `position_offset(source, line, col) -> u64`: shim
  `offsetToPosition` の逆変換。改行意味論は Luau Lexer 規約に
  一致させる — 行区切りは `\n` のみ（`\r` は Lexer では空白扱いで
  単独 `\r` は改行にならず、`\r\n` は `\n` で 1 回計数）。
  `col` は行末で clamp、line 超過時は `len` に clamp。
- JSON パースは `serde_json::Value` で防御的に。トップレベル
  schema（object / `diagnostics` array / 要求時 `completions`
  array / diagnostics の object エントリ）は fail-visible に
  検証し違反は `Internal`。field 単位の欠落は `None`/skip、
  型不一致は `Internal{message}` で fail-visible。
  vendor error 文字列を `AnalyzerError.message` にそのまま載せない
  方針は維持 — status + 固定 prefix + shim message を `Internal`
  に含めるのは診断可能性のため許容、diagnostic 本文は中立
  `Diagnostic.message` に載る。

### `src/lib.rs`

- `#![forbid(unsafe_code)]` は付けない — `ffi.rs` が unsafe を含む
  ため crate ルートは `#![warn(unsafe_code)]`（workspace lint）に
  委ね、`ffi.rs` モジュール先頭に `#[expect(unsafe_code)]` +
  SAFETY コメントで局所許可（ffi-poc と同一パターン、
  AGENTS.md の unsafe ポリシー準拠）。
- `mod analyzer; mod contract; mod ffi;`（全て private）+
  `pub use analyzer::{LuauAnalyzer, LuauAnalyzerOptions};`。
- `#[cfg(test)] mod ffi_boundary_tests;`（ffi-poc 踏襲の FFI 境界
  単体テスト — invalid UTF-8/範囲外 cursor/oversize/completion
  disabled/version negotiation fault）。
- crate doc: 責務・unsafe 境界・非目標（runtime backend 非含有、
  AC3 consumer gate、3-OS NotRun）を明記。

### `build.rs` / `native/` / `vendor/`

- `build.rs`: `ffi-poc/build.rs` を製品化 — `cc::Build` で
  `vendor/luau/{Analysis,Ast,Common,Compiler,Config,Require}/src/*.cpp`
  + `native/rf_luau_bridge.cpp` を C++17 でコンパイル、
  `cargo:rustc-link-lib=static=rf_luau_bridge`（lib 名は shim
  `RF_LUAU_LIB_NAME` と一致必須）、`find-msvc-tools` で MSVC 環境
  検出、`cargo:rerun-if-changed` に vendor/native パス列挙。
- `native/rf_luau_bridge.{h,cpp}`: ffi-poc から移植。差分は
  - `RF_LUAU_IMPL_VERSION` を `"ruviform-script-luau/<ver>"` へ
  - fault injection env 名を `RF_FFI_POC_FAIL_*` →
    `RF_LUAU_FAIL_{INIT,ANALYZE,LOADDEF}` へ改名（test-only seam、
    shim コメントに明記）
  - それ以外の ABI/status/JSON schema は不変（contract_version=1
    維持 — shim は契約版を持たず Rust 側が `CONTRACT_VERSION` を
    descriptor に載せる）
- `vendor/luau/`: `ffi-poc/vendor/luau` から `cp -r` byte 同一
  コピー（REVISION=0.740、LICENSE 類保持）。`.gitattributes` の
  `eol=lf` 正規化は ffi-poc と同一適用。

## 依存・feature

```toml
[dependencies]
ruviform-script = { path = "../ruviform-script" }
serde_json = "1"          # Cargo.lock 既在
# mlua は「link 順序のための必須依存」: build.rs が emit する
# `cargo:rustc-link-lib=static=luau*` は mlua→mlua-sys→luau0-src
# (0.22.0+luau740) の build script が供給する Luau VM 静的 lib を指す。
# Rust API としては未使用（coexistence test が兼用）。runtime
# ScriptBackend 実装は別 slice — AC4 ADR 参照。
mlua = { version = "0.12.2", default-features = false, features = ["luau"] }

[build-dependencies]
cc = "1"                  # Cargo.lock 既在
find-msvc-tools = "0.1"   # Cargo.lock 既在（MSVC 検出）
```

- `mlua` は必須依存（link 供給）— dev-dependency では luau0-src が
  本番依存グラフに入らず、下流 consumer の最終リンクで luau*.lib が
  解決できない。公開 API には mlua 型を露出しない。
- feature 追加なし（default のみ）。`workspace = true` lints 継承。
- `Cargo.lock`: 新規 package `ruviform-script-luau` 追加のみ。
  `mlua`/`luau0-src`/`cc`/`serde_json` は既存 lock を再利用
  （`cargo check` で lock 更新 → 差分確認）。

## Fail-closed / unsafe 境界

- shim ABI version・contract version・struct_size・impl_version の
  不一致は `connect` で `VersionMismatch`/`Unavailable` に写像
  （version negotiation は PoC 実証済み機構を維持）。
- JSON 応答の malformed/型不一致は `Internal` で fail-visible
  （panic しない、vendor error を中立型に漏らさない）。
- `AnalyzerError` は全て typed variant — 文字列エラー・panic・
  unwrap なし（`expect` は lock poison 等の到達不能経路のみ、
  可能な限り `map_err` で typed 化）。
- `unsafe` は `ffi.rs` のみ、各 unsafe ブロックに SAFETY コメント、
  `#[expect(unsafe_code)]` で clippy `unsafe_code` warn を局所許可
  （workspace lint `unsafe_code = "warn"` + `-D warnings` ゲートと
  整合 — `expect` が未使用警告を出さないよう実際に unsafe を含む
  こと）。
- 公開 API に `Luau*`/`mlua`/`Luau.Analysis`/C++ 型/生ポインタを
  出さない（contract 型 + `LuauAnalyzer`/`LuauAnalyzerOptions`
  のみ）。

## Test design（詳細は test-specs）

- `tests/bridge.rs`: 製品契約の外部振る舞い — 有効 source 解析、
  syntax/type diagnostic、completion（kind 写像・replace_range）、
  type_definitions（request 経由 + `add_type_definition` 永続 +
  dedup）、cursor 検証、oversize、schema_version 不一致、
  determinism（同一 request → 同一 result）、mlua VM coexistence、
  descriptor/version 検査。
- `tests/faults.rs`: `RF_LUAU_FAIL_{INIT,ANALYZE,LOADDEF}` env
  seam で `Internal`/`Unavailable` 写像を検証（env は process
  全体 — `#[serial]` 相当の直列化または単一 test 内で順序制御）。
- `tests/registry.rs`: `AnalyzerRegistry` end-to-end — 空 registry
  fail-closed、実 `LuauAnalyzer` 登録→resolve→analyze、
  `connect_with_analyzer_id` による複数 instance の曖昧性回避、
  capability/contract_version mismatch、stub analyzer による
  stale revision 検出（`verify_result_revision` 経路）。
- `src/ffi_boundary_tests.rs`: FFI 生境界 — invalid UTF-8 byte
  列・cursor 範囲外・oversize・completion disabled・version
  negotiation fault（shim create を直接叩く PoC 同型テスト）。
- `analyzer.rs` `#[cfg(test)]`: `position_offset` の改行意味論
  （`\n`/`\r\n`/`\r` 混在）・JSON→中立型写像の edge case。

## Verification

```sh
mise run fmt-check
mise run check ruviform-script-luau
mise run lint ruviform-script-luau
mise run test ruviform-script-luau
mise exec -- prek run --all-files
```

`ruviform-script` は無変更のため同タスクは回帰確認として実行
（差分が出ないことの確認）。build は C++ vendor のため初回は
長時間（Luau.Analysis ~76 TU + shim）。Windows x86_64 実機のみ
— macOS/Linux は NotRun（`evidence/rf560/platform-matrix.md` 継承、
AC4 ADR に記録）。

## Rollback

crate 新設のみ — `crates/ruviform-script-luau` 削除 + workspace
members/crates README/Cargo.lock 差分 revert で完全に除去可能。
`ruviform-script`・`ffi-poc`・consumer への影響なし。

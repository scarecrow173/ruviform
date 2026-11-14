---
id: "IMPL-034Xgit7eLLg54Qn3vg7Vl"
type: "impl"
status: "completed"
title: "RF-SCRIPT-02 Luau.Analysis native bridge PoC"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "ffi-poc/"
      - type: "directory"
        path: "evidence/rf560/"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# RF-SCRIPT-02 Luau.Analysis native bridge PoC

## Summary

Issue #560 の headless PoC スライス。RF-SCRIPT-00 契約
（`docs/designs/issue-558-script-contract-design.md` C5）の
`ScriptAnalyzer`/`AnalyzerRegistry` をミラーした最小ローカル Port と、
vendored Luau.Analysis を呼ぶ狭小 C ABI FFI shim を `ffi-poc/` 隔離
crate として実装する。mlua の vendored Luau VM と同一バイナリで
リンクし、版整合・fault・stale revision の fail-closed を fixture で
実測する。本番公開 crate ではなく、採否は #558/#559 の統合 Gate で判断。

## Context

- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/560
- 契約正本: RF-SCRIPT-00（PR #621 で main 統合済み）—
  `docs/specs/issue-558-script-contract.md`、
  `docs/designs/issue-558-script-contract-design.md`、
  `docs/adr/issue-558-script-backend-contract.md`、
  `docs/test-specs/issue-558-script-contract.md`
- 隣接 PoC: Issue #559 の `poc/`（Luau/Rhai VM 共存）とは別ディレクトリ。
- 本スライスは headless Native Bridge PoC のみ。real-OS 3OS 配布実行・
  製品 Binary 統合・Consumer 受入は統合 Gate（#558/#559/#294・synvelis）
  の責務であり、ここでは NotRun/Deferred として記録する。

## Implementation

### Changes

- `ffi-poc/` 新設（隔離 PoC crate `ruviform-ffi-poc`、workspace member 登録）:
  - `Cargo.toml`: `serde_json`（shim JSON の parse）、`mlua` with `luau`
    feature（VM 共存証明用、default-features=false）、build-deps `cc` +
    `find-msvc-tools`。
  - `build.rs`: vendored `Analysis/src/*.cpp`（76 TU）+ `native/rf_luau_bridge.cpp`
    を C++17 で静的ライブラリ化。define は `luau0-src` の Build と同一
    （`LUAI_MAXCSTACK=1000000`/`LUA_VECTOR_SIZE=3`/`LUA_API=extern "C"`/
    debug 時 `LUAU_ENABLE_ASSERT`）。Windows MSVC では vcvars 未初期化環境向けに
    `find-msvc-tools` で検出した INCLUDE/LIB/PATH を build script プロセスへ適用。
  - `native/rf_luau_bridge.h` / `.cpp`: C ABI shim。opaque handle、
    status code、UTF-8 検証、cursor 範囲検査、owner-thread 検査、
    環境変数 fault injection、JSON 出力。C++ 型・Luau 型・例外は境界を越えない。
  - `src/contract.rs`: 契約型ミラー（`AnalyzeRequest`/`AnalyzeResult`/
    `Diagnostic`/`Completion`/`AnalyzerCapabilities`/`ScriptBackendVersion`
    （impl/contract 分離）/`AnalyzerError`（Unavailable/Unsupported/
    StaleResult/InvalidRequest/VersionMismatch/Bridge/Internal））。
  - `src/ffi.rs`: extern "C" 宣言と unsafe 隔離層。shim 文字列は
    `ShimString` が `rf_luau_free` で一度だけ解放。
  - `src/analyzer.rs`: `ScriptAnalyzer` trait、`LuauAnalyzer`、
    `AnalyzerRegistry`（Fail-Closed 解決・stale revision を Consumer 側で拒否）。
  - `src/ffi_boundary_tests.rs`（cfg(test)）: 生バイト/版不一致/
    thread 不一致など安全 API で表現不能な fault fixture。
  - `tests/bridge.rs`: FX-P-08 正常/構文/型/Completion 更新、FX-N-13/14/24、
    cursor/サイズ、Analyzer 単独利用、mlua VM 共存。
  - `tests/faults.rs`: 環境変数 fault injection（init/analyze/loaddef）。
  - `vendor/luau/`: Luau 0.740 を upstream tag `0.740`
    （commit `c0e346edd89066b44dca174c9f54ce84c746a540`）から vendored。
    `Analysis/`・`Ast`・`Bytecode`・`CodeGen`・`Common`・`Compiler`・
    `Config`・`Inliner`・`Require`・`VM` の include/src と MIT license
    （`LICENSE.txt`/`lua_LICENSE.txt`）、`REVISION` マニフェスト。
- `Cargo.toml`（root）: workspace members に `ffi-poc` を追記。
- `Cargo.lock`: `luau0-src 0.22.0+luau740` / `mlua 0.12.2` /
  `mlua-sys 0.13.0` / `find-msvc-tools` 等を再生成で追加。

### Decisions During Implementation

- **Luau revision は 0.740 に固定**: 当初 0.744 を vendored したが、
  crates.io index 確認で `luau0-src 0.22.1+luau744` が **yanked** で
  あることが判明。mlua 0.12.2 → mlua-sys 0.13.0 → `luau0-src ^0.22.0`
  の解決は非 yanked の `0.22.0+luau740`（Luau 0.740）に確定するため、
  「VM と Analysis を同一 revision で固定する」要件に従い Analysis 側も
  upstream tag `0.740` へ再 vendor した。0.740/0.744 間で shim が使う
  API（Frontend/Autocomplete/TypeError/ConfigResolver 等）は同一である
  ことを header 照合で確認済み。
- **VM シンボルは luau0-src の静的ライブラリに依存**: Analysis の
  非 Analysis 翻訳単位（Ast/Compiler/VM 等）は mlua-sys 経由の
  luau0-src（同一 0.740・同一 define）が供給し、重複コンパイルによる
  ODR 違反を回避する設計。headers は vendored 0.740 で一致。
- **結果は JSON + status code**: C++ 型・Luau 型・例外・ポインタを
  公開境界へ出さず、shim 内で JSON 化 → Rust で契約型へ parse。
  失敗は全て明示 status code → `AnalyzerError` 写像。
- **Completion カーソルは UTF-8 バイトオフセット**: shim が byte offset
  → `Luau::Position`（0 始まり line/行内バイト column）へ変換。
  replace_range は識別子プレフィックス簡易実装（PoC 範囲）。
- **Analyzer/Runtime 独立**: `AnalyzerRegistry` は Runtime Backend を
  参照せず、解析成功は実行 Grant を意味しない（契約 C5）。
- **unsafe 隔離**: `unsafe` は `ffi` モジュールと cfg(test) の
  `ffi_boundary_tests` に `#[expect(unsafe_code)]` + SAFETY コメントで
  局所許可。公開 API は安全な型のみ。

## Related Experiments

### Adopted

- `docs/impl/exp/rf-script-02-luau-bridge-poc.md` — 検証計画と実測経緯
  （revision 切替・MSVC `/utf-8`・link 順序・env 二系統問題の記録）

### Rejected

<!-- 同上 -->

## Validation

Windows（MSVC Build Tools 2022, CP932 ロケール）で実測:

- `mise run check ruviform-ffi-poc`（`cargo check --locked --all-targets`）: Pass
- `mise run lint ruviform-ffi-poc`（`cargo clippy --locked --all-targets -D warnings`）: Pass
- `mise run test ruviform-ffi-poc`（`cargo test --locked`）: 22/22 Pass
  （lib unit 8 + bridge integration 13 + faults 1）
- `cargo fmt --all -- --check`: Pass
- cargo 直接実行は補助証拠。canonical は上記 mise タスク。
- 詳細は `evidence/rf560/build-link-run.md`・`platform-matrix.md`。
  macOS/Linux は NotRun。

## Risks

- luau0-src 0.22.1+luau744 が yanked のため、mlua 側 VM を 0.744 以降へ
  上げるには mlua/mlua-sys の新版待ち。0.740 での API 差異は shim 使用範囲
  では確認されなかったが、将来の Luau 更新時は revision 再照合が必要。
- Analysis オブジェクトと luau0-src オブジェクトの ODR/ABI 一致は
  同一 revision + 同一 define の設計で担保するが、実行時の網羅的検証は
  fixture 範囲に限る。
- timeout fixture（`module_time_limit_sec`）は決定的な再現が難しく
  NotRun とする可能性がある。
- macOS/Linux は本環境に toolchain がなく NotRun（headless 証拠のみ）。

## Follow-ups

- `evidence/rf560/` の gap report・license ledger・版整合照合記録・
  Build/Link/Run 記録を完成させる。
- `docs/impl/exp/rf-script-02-luau-bridge-poc.md`（実施した探索がある
  場合）と docs index の doc-maintenance 再生成。
- review-protocol レビュー、Conventional Commits、push、PR（Refs #560）。

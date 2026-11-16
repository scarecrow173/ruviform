---
id: "PLAN-mJPkr5kJdau9iCE3oIHTKf"
type: "plan"
status: "approved"
title: "Issue #692 s2 Luau analyzer backend — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-692-luau-backend.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-692-luau-backend.md"
      - type: "file"
        path: "docs/tasks/issue-692-luau-backend.md"
      - type: "file"
        path: "docs/test-specs/issue-692-luau-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-692-luau-backend.md"
  implemented-by:
    - "docs/tasks/issue-692-luau-backend.md"
  depends-on:
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-692-luau-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-692-luau-backend.md"
    - "docs/test-specs/issue-692-luau-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-692-luau-backend.md"
  verified-by:
    - "docs/test-specs/issue-692-luau-backend.md"
  references:
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-692-luau-backend.md"
  defers: []
  deferred-by: []
---

# Issue #692 s2 Luau analyzer backend — 実装計画

## Scope split

単一 slice（s2）— 新設クレート `crates/ruviform-script-luau` のみ。
依存方向: `ruviform-script-luau` → `ruviform-script`（path）。
`ffi-poc`・consumer・`ruviform-script` 本体は無変更。

## File map

| パス | 操作 | 内容 |
| --- | --- | --- |
| `crates/ruviform-script-luau/Cargo.toml` | 新規 | manifest（deps: ruviform-script path, serde_json; build-deps: cc, find-msvc-tools; dev-deps: mlua luau+vendored） |
| `crates/ruviform-script-luau/build.rs` | 新規 | vendored Luau.Analysis + shim C++ コンパイル（ffi-poc 踏襲） |
| `crates/ruviform-script-luau/native/rf_luau_bridge.h` | 新規 | C ABI header（ffi-poc 移植） |
| `crates/ruviform-script-luau/native/rf_luau_bridge.cpp` | 新規 | shim 実装（env 名改名 + impl_version 更新のみ差分） |
| `crates/ruviform-script-luau/vendor/luau/**` | 新規 | Luau 0.740 vendored tree（ffi-poc から byte 同一コピー） |
| `crates/ruviform-script-luau/src/lib.rs` | 新規 | crate doc + module 宣言 + re-export + ffi_boundary_tests mod |
| `crates/ruviform-script-luau/src/contract.rs` | 新規 | shim status/JSON 定数 |
| `crates/ruviform-script-luau/src/ffi.rs` | 新規 | unsafe FFI 層（extern/BridgeHandle/ShimString/CreateOptions/versions） |
| `crates/ruviform-script-luau/src/analyzer.rs` | 新規 | LuauAnalyzer/LuauAnalyzerOptions/ScriptAnalyzer impl/JSON 写像 |
| `crates/ruviform-script-luau/src/ffi_boundary_tests.rs` | 新規 | FFI 生境界テスト（#[cfg(test)]） |
| `crates/ruviform-script-luau/tests/bridge.rs` | 新規 | 製品契約統合テスト |
| `crates/ruviform-script-luau/tests/faults.rs` | 新規 | fault injection env seam テスト |
| `crates/ruviform-script-luau/tests/registry.rs` | 新規 | AnalyzerRegistry end-to-end テスト |
| `crates/ruviform-script-luau/README.md` | 新規 | crate README（責務/依存境界/検証/非目標） |
| `Cargo.toml` (root) | 変更 | workspace members に `crates/ruviform-script-luau` 追加 + ruviform-script 行コメント更新 |
| `crates/README.md` | 変更 | crate 一覧 + 件数更新（生成 index ではない手書き一覧） |
| `Cargo.lock` | 変更 | 新規 package 追加のみ（依存 version 変更なし） |
| `docs/adr/issue-692-luau-dependency-selection.md` | 新規 | AC4 依存選定 ADR |
| `docs/impl/ir/issue-692-luau-backend.md` | 新規 | 実装記録（完了時） |
| `docs/impl/exp/issue-692-luau-backend.md` + `.jsonl` | 新規 | 実験ログ（完了時） |
| `evidence/rf692/s2-verification.md` | 新規 | s2 検証記録 |

## Dependency graph

```
analyzer.rs ──→ ffi.rs ──→ contract.rs (定数)
    │              │
    │              └──→ native shim (extern "C", build.rs で link)
    ├──→ ruviform-script (ScriptAnalyzer/AnalyzerDescriptor/…)
    └──→ serde_json (JSON → 中立型)

build.rs ──→ vendor/luau/** + native/*.cpp (cc)
tests/* ──→ analyzer.rs + ruviform-script::AnalyzerRegistry (+ mlua dev)
```

## Risk register

| リスク | 影響 | 対策 |
| --- | --- | --- |
| shim ABI/contract version 不一致 | connect 失敗 | fail-closed negotiation（PoC 実証済み機構を維持）+ ffi_boundary_tests で fault 検証 |
| `analyze(&self)` と shim owner-thread affinity の齟齬 | cross-thread panic/UB | `Mutex<Inner>` + owner ThreadId 検査 + NonNull で Send/Sync 非実装（safe code 到達不能、shim status 4 は defense-in-depth） |
| JSON→中立型写像の情報落ち（kind/deprecated/context） | completion 情報欠落 | spec の写像表で明示的に破棄 field を記録 — 中立契約に対応 field がないため仕様上の損失（design 記録） |
| line/col→offset 再計算の改行意味論ずれ | diagnostic range 誤り | shim `offsetToPosition` と Luau Lexer 規約（`\n` のみ計数）に一致させた規則を `position_offset` に実装 + unit test |
| vendor tree の line-ending/サイズ | prek hook / repo 肥大 | `.gitattributes eol=lf` で正規化（ffi-poc と同一）、byte 同一コピー |
| Luau revision と luau0-src の不一致 | ODR/ABI リスク（将来） | 両方 0.740 で固定 — ADR + build.rs コメント + REVISION で記録。mlua 更新時は再 vendor 必須を ADR に明記 |
| 初回 build 時間 | 開発遅延 | Luau.Analysis は大規模 C++ — 初回 check/test は長時間を想定 |
| `Mutex` poison | analyze 失敗 | `lock()` Err → `Internal` typed error（panic 連鎖を遮断） |

## Verification matrix

| 層 | 手段 | 合格基準 |
| --- | --- | --- |
| 契約適合 | `tests/bridge.rs` `tests/registry.rs` | ScriptAnalyzer impl・descriptor・全 error variant 写像・registry fail-closed |
| FFI 境界 | `src/ffi_boundary_tests.rs` `tests/faults.rs` | invalid UTF-8/範囲外/oversize/completion disabled/version fault が typed error |
| unit | `analyzer.rs` `#[cfg(test)]` | position_offset 改行意味論・JSON 写像 edge |
| lint/format | `mise run lint/check/fmt-check` + `prek --all-files` | `-D warnings` 通過・fmt 差分なし |
| 統合 | `mise run test ruviform-script-luau` | 全 test Pass（Windows x86_64） |

## Rollback

crate 削除 + workspace/README/lock 差分 revert。依存 crate・
consumer への影響なし（design Rollback 節と同一）。

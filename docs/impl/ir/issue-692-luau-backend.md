---
id: "IMPL-692LuauBackend0001"
type: "impl"
status: "completed"
title: "Issue #692 s2 ruviform-script-luau analyzer backend"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-script-luau/"
      - type: "directory"
        path: "docs/discovery/"
        note: "issue-692-luau-backend.md"
      - type: "file"
        path: "evidence/rf692/luau-backend-verification.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-692-luau-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-692-luau-backend.md"
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/adr/issue-692-luau-dependency-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-692-luau-backend.md"
    - "docs/impl/ir/issue-692-script-port.md"
    - "docs/impl/ir/rf-script-02-luau-bridge-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-692-luau-backend.md"
  verified-by: []
  references:
    - "docs/plans/issue-692-luau-backend.md"
  defers:
    - "実 runtime ScriptBackend 実装（eval/budget/artifact — 別 slice）"
    - "synvelis thin adapter 接続（AC3 — synvelis#271/#272/#273/#274）"
    - "macOS/Linux 実機検証・release-size/performance（Windows x86_64 のみ実測）"
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-692-luau-backend-exp.jsonl"
      - "docs/impl/exp/issue-692-luau-backend.md"
    rejected: []
---

# Issue #692 s2 — ruviform-script-luau Implementation Record

RF-SCRIPT-03: `ruviform-script`（RF-SCRIPT-00 中立契約）の C5
`ScriptAnalyzer` を、vendored Luau.Analysis 0.740 + 狭小 C ABI shim
で実装する実 analyzer backend crate の新設記録。

## 実装サマリ

- `crates/ruviform-script-luau/` 新設（workspace member 追加、
  `crates/README.md` 索引更新、計 41 クレート）。
- 公開 API: `LuauAnalyzer` / `LuauAnalyzerOptions` /
  `luau_language()` / `DEFAULT_ANALYZER_ID` のみ。公開面に
  Luau/vendor/C++/mlua 型・生ポインタ・shim status code を出さない。
- `ScriptAnalyzer::analyze(&self)` を `Mutex<AnalyzerState>` の
  内部可変性で実装。`NonNull` 由来で `Send`/`Sync` 非実装 —
  owner-thread affinity を型で表現。
- shim（`native/rf_luau_bridge.{h,cpp}`）は ffi-poc から製品化:
  env 名 `RF_LUAU_FAIL_*` 改名、impl_version 更新、ABI version・
  contract version・status code 体系は不変。
- `vendor/luau/` に Luau 0.740（tag `c0e346e`）を vendor —
  `REVISION` に mlua→mlua-sys→luau0-src 整合の根拠を記録。
- `build.rs` は Analysis .cpp を決定論的 sorted order でコンパイルし、
  `luauinliner`/`luauvm`/`luaubytecode`/`luaucompiler`/`luauast`/
  `luaucommon`/`luauconfig`/`luaucustom`/`luaurequire` の静的 lib を
  mlua-sys/luau0-src の build script 産物から再 emit（Windows link
  order 対策）。MSVC 環境は `find-msvc-tools` で補完。

## AC4 依存選定（ADR 要約）

`docs/adr/issue-692-luau-dependency-selection.md` に記録。
analyzer = native Luau.Analysis bridge（vendored + C ABI shim）。
`mlua` vendored Luau は shim がリンクする Luau VM 静的 lib の供給元
として**必須依存**（Rust API 未使用、coexistence test が兼用）。
runtime `ScriptBackend` は別 slice で再評価。却下案: mlua API のみ
（Analysis 非含有）、dynlink 共有 lib（配布・版整合リスク）。

## 検証（evidence/rf692/luau-backend-verification.md が正本）

- `mise run check|lint|test ruviform-script-luau`、`mise run fmt-check`、
  `prek run --all-files` — 全 pass（Windows 11 x64 / MSVC / rustc 1.99.1）。
- test 48 件: lib unit 16 + bridge 22 + faults 1 + registry 9。
- review-protocol 2 cycle 実施 — cycle 1 で P1×1/P2×2/P3 複数を検出・
  修正（cursor gating・JSON schema fail-visible 化・改行意味論を
  Luau Lexer の `\n` のみへ統一・thread 検査/version 検証強化等）。
  cycle 2 で `timeoutHits` の silent partial diagnostics（P2）を
  検出・fail-closed 化し、Reviewer A は BLOCKER COUNT: 0 を確認。
- fault fixture（invalid UTF-8/cursor range/source size/completion
  disabled/ABI・struct_size mismatch/thread mismatch/fault injection
  status 7）を製品契約型へ写像して実測。
- registry fail-closed 行列（UnknownLanguage/UnknownAnalyzer/
  NoCandidates 相当/AmbiguousAnalyzer/UnsupportedContractVersion/
  MissingCapability/StaleResult）を実 analyzer + stub で検証。

## 境界・非目標の遵守

- `ruviform-script` 中立契約 crate は無変更（`unsafe`/vendor 型の
  混入なし）。`Cargo.lock` 差分は新規 package エントリ 11 行のみ。
- runtime `ScriptBackend`/`ScriptSession` 実装・eval 経路は本 slice
  に含めない。Synvelis adapter（AC3）は consumer 側 gate。
- Issue #692 は open 継続 — AC checkbox・close 操作なし。
  PR は `Refs #692`。

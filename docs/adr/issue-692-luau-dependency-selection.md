---
id: "ADR-sEmeB5hwtcID0a4rBWSHTx"
type: "adr"
status: "accepted"
title: "Issue #692 s2 Luau analyzer 依存選定 — native Luau.Analysis bridge（AC4）"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "https://github.com/scarecrow173/ruviform/issues/560"
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/designs/issue-692-luau-backend.md"
  changes:
    added:
      - type: "doc"
        path: "docs/adr/issue-692-luau-dependency-selection.md"
      - type: "file"
        path: "crates/ruviform-script-luau"
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
    - "docs/adr/issue-558-script-backend-contract.md"
    - "docs/designs/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/692"
  derived-by:
    - "docs/designs/issue-692-luau-backend.md"
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-692-luau-backend.md"
    - "docs/discovery/issue-692-luau-backend.md"
    - "evidence/rf560/gap-report.md"
    - "evidence/rf560/revision-alignment.md"
    - "evidence/rf560/license-ledger.md"
    - "evidence/rf560/platform-matrix.md"
    - "evidence/rf560/build-link-run.md"
    - "ffi-poc/Cargo.toml"
    - "Cargo.lock"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Issue #692 s2 Luau analyzer 依存選定 — native Luau.Analysis bridge（AC4）

## Context and Problem Statement

Issue #692 AC4 は、Luau を Ruviform の Script/Analyzer 基盤へ統合する
際の依存選定を記録することを要求する。選択肢は (a) `mlua` vendored
Luau VM のみ、(b) `Luau.Analysis` を直接利用する native bridge、
(c) vendored Luau 全 tree の自前ビルド、(d) `ffi-poc` vendor の直接
参照である。

`synvelis-script-luau`（synvelis#273 interim）は既に `mlua` vendored
Luau を runtime eval 経路に使用しているが、C5 `ScriptAnalyzer` が要求
する `SyntaxAndType` diagnostics と completion は `mlua` の公開 API
からは提供されない — `luau0-src` は `Analysis/` を同梱せず、mlua は
解析面を露出しない。analyzer 側の依存選定を AC4 として独立に確定する
必要がある。

## Decision Drivers

* **Capability 充足**: C5 `AnalyzerCapabilities.diagnostics =
  SyntaxAndType` と completion を実 Luau.Analysis で提供する必要が
  ある（spec `issue-692-luau-backend.md`）。
* **版整合**: vendored `Luau.Analysis` の revision は `mlua`→
  `luau0-src` が解決する Luau revision と同一でなければならない
  （同一プロセス内で VM と Analysis が共存するため、シンボル衝突・
  ODR 違反を防ぐ）。
* **Fail-Closed**: ABI/contract/struct_size/version 不一致を型付き
  エラーで拒否する negotiation 機構が必要（RF-SCRIPT-02 で実証済み）。
* **境界の清潔さ**: vendor 型・C++ 型・unsafe・FFI を公開 API に
  出さず、中立契約型のみを露出する（`ruviform-script` C5）。
* **再現性**: `Cargo.lock` 既在依存のみで構成し、新規外部依存を
  増やさない。
* **正直な検証範囲**: PoC は Windows x86_64 で 22/22 tests Pass。
  macOS/Linux・release-size/performance・3-OS は NotRun であり、
  本 ADR はそれを Pass と偽装しない。

## Considered Options

### (a) `mlua` vendored Luau のみ

* `mlua 0.12.2` → `mlua-sys 0.13.0` → `luau0-src 0.22.0+luau740`
  （Cargo.lock 既在）を analyzer にも使用。
* **却下**: `luau0-src` は `Analysis/` を同梱せず、mlua の公開 API
  は VM eval のみで `SyntaxAndType` diagnostics / completion を
  提供しない。capability 充足不可。

### (b) `Luau.Analysis` native bridge（ffi-poc 方式を製品化）

* vendored `Luau.Analysis`（Luau 0.740）を `cc` で静的コンパイルし、
  狭小 C ABI（version negotiation + JSON 応答）で `LuauAnalyzer`
  から呼出す。
* **採用**: RF-SCRIPT-02（#560）で Windows 22/22 tests Pass を実証
  済み。version negotiation・fault injection・JSON 境界が確立済み。
  `mlua` VM との同一 revision 共存を PoC で確認済み。

### (c) vendored Luau 全 tree の自前ビルド

* `Analysis`/`Ast`/`Common`/`Compiler`/`Config`/`Require` を自前
  build script でフルビルド。
* **却下**: consumer が `mlua` backend と同一リンクした際に Luau
  シンボルの二重定義・ODR リスクがある。`luau0-src` が既に VM 側を
  提供しているため、Analysis のみに限定した (b) の方が衝突面積が
  小さい。

### (d) `ffi-poc` vendor の直接参照

* `ffi-poc/vendor/luau` を path 参照または symlink。
* **却下**: PoC（証拠・凍結）と製品 crate の分離原則に反する。
  製品 crate は自身の `vendor/luau` を持つ（byte 同一コピーで
  revision 整合を保つ）。

## Decision Outcome

**Option (b) — native Luau.Analysis bridge を採用** する。

`crates/ruviform-script-luau` は `ffi-poc` の shim（
`native/rf_luau_bridge.{h,cpp}`）を製品化し、vendored
`Luau.Analysis`（Luau 0.740）を `cc` で静的リンクする。
`mlua` vendored Luau は runtime eval 経路（将来の `ScriptBackend`
実装・synvelis interim adapter）で引き続き使用し、analyzer とは
独立した依存として維持する（hybrid: analyzer=native bridge、
runtime=mlua — runtime backend 実装は別 slice で依存選定を再評価
する）。

### Consequences

* **Positive**: C5 `SyntaxAndType` + completion を実 Luau.Analysis
  で提供可能。version negotiation で ABI/contract/struct_size の
  不一致を fail-closed。vendor/unsafe/FFI は crate 内部に閉じる。
* **Negative**: C++ toolchain（MSVC/clang）と `cc` build 依存が
  必要。Luau.Analysis の vendored tree は大きい（~76 TU）。
  Windows 以外の platform は未検証（NotRun）。
* **Neutral**: `mlua` は引き続き runtime 側の依存として残る。
  両者は同一 Luau revision（0.740）で整合するが、crate 間で
  直接の型共有はない（中立契約型のみで接続）。

### Confirmation

* `crates/ruviform-script-luau` の `tests/bridge.rs`・
  `tests/registry.rs`・`src/ffi_boundary_tests.rs` が Windows
  x86_64 で全 Pass すること（`mise run test ruviform-script-luau`）。
* `evidence/rf692/luau-backend-verification.md` に実行コマンド・OS・
  toolchain・exit code を記録すること。
* macOS/Linux・release-size/performance・3-OS 実機は NotRun として
  記録し、Issue #692 の AC4 checkbox 更新は consumer gate
  （synvelis#271/#272/#273/#274）および 3-OS 受入で別途行う。

## Pros and Cons of the Options

### (a) mlua vendored Luau のみ

* Good: 依存が単純、Cargo.lock 既在、build script 不要。
* Bad: `Analysis/` 非含有・mlua API に解析面なし → capability
  充足不可。却下。

### (b) native Luau.Analysis bridge

* Good: 実証済み（#560 Windows 22/22）、capability 充足、version
  negotiation 確立、unsafe/FFI を局所化可能。
* Bad: C++ toolchain 依存、vendored tree サイズ、Windows 以外未検証。

### (c) vendored Luau 全 tree 自前ビルド

* Good: Analysis + VM を単一 build で統一可能。
* Bad: mlua backend とのシンボル二重定義/ODR リスク。依存複雑化。
  却下。

### (d) ffi-poc vendor 直接参照

* Good: vendor 重複なし。
* Bad: PoC/製品分離違反、PoC 変更が製品に波及。却下。

## More Information

* RF-SCRIPT-02 証拠: `evidence/rf560/{gap-report,platform-matrix,
  revision-alignment,license-ledger,build-link-run}.md`
* PoC 実装: `ffi-poc/{Cargo.toml,build.rs,src/{analyzer,ffi,
  contract}.rs,native/rf_luau_bridge.{h,cpp},tests/{bridge,faults}.rs}`
* 版整合: Luau 0.740 = `luau0-src 0.22.0+luau740`（`0.22.1+luau744`
  は yanked — `revision-alignment.md`）。mlua/luau0-src 更新時は
  vendored `Analysis/` の再 vendor + `build.rs` の lib 名再 emit が
  必須。
* ライセンス: Luau は MIT（`vendor/luau/LICENSE` 等保持）。
  `license-ledger.md` に vendored source size・C++17/MSVC 要件・
  link-order・CRT/allocator/ODR 制約を記録。
* 3-OS: `platform-matrix.md` — Windows x86_64 headless PoC Pass、
  macOS/Linux NotRun。本 ADR は 3-OS 対応を主張しない。

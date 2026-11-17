---
id: "ADR-GUIvRTewPsVAJEv3wqlUBg"
type: "adr"
status: "accepted"
title: "Issue #716 runtime 依存選定 — mlua Rust API（#692 AC4 再評価）"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/716"
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
  changes:
    added:
      - type: "doc"
        path: "docs/adr/issue-716-runtime-backend.md"
      - type: "file"
        path: "crates/ruviform-script-luau/src/runtime.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-script-luau/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-script-luau/README.md"
      - type: "file"
        path: "crates/README.md"
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
    - "docs/adr/issue-692-luau-dependency-selection.md"
    - "docs/adr/issue-558-script-backend-contract.md"
    - "docs/designs/issue-692-luau-backend.md"
  refines:
    - "docs/adr/issue-692-luau-dependency-selection.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/716"
  derived-by:
    - "docs/designs/issue-716-runtime-backend.md"
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/discovery/issue-716-runtime-backend.md"
    - "Cargo.toml"
    - "Cargo.lock"
    - "evidence/rf692/luau-backend-verification.md"
  defers:
    - "実 OS/実 VM 経路の受入（consumer gate — synvelis#516・synvelis#267 AUTX-08）"
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Issue #716 runtime 依存選定 — mlua Rust API（#692 AC4 再評価）

## Context and Problem Statement

#692 AC4 は analyzer 側の依存選定を `issue-692-luau-dependency-
selection.md` で確定したが、runtime `ScriptBackend`（eval/
budget/artifact）の依存選定は「別 slice で再評価」と明示的に
保留されていた。本 Issue #716 がその再評価を行う。

runtime backend が必要とする機能は: chunk eval（text mode）、
safepoint interrupt（wall-time/cancel 観測）、memory limit、
Lua value → 中立 `ScriptValue` の持ち上げ。これらは全て mlua
0.12.2（`luau` feature → vendored Luau 0.740）の公開 safe API
で提供される。対抗案は analyzer と同じ native C ABI bridge を
runtime eval にも拡張する方式である。

## Decision Drivers

* **依存追加ゼロ**: `mlua 0.12.2 (luau)` は既依存で、shim が
  リンクする Luau VM 静的 lib の供給元として必須。Rust API を
  使っても `Cargo.toml`/`Cargo.lock` の変更は不要。
* **safe Rust 境界**: mlua API は safe。native bridge 追加は
  新たな `unsafe` FFI 面（eval/budget/value 写像を C++ で再実装）
  を生む — crate の unsafe 局所許可ポリシーに反する面積増。
* **実証済み意味論**: synvelis `synvelis-script-luau` interim
  adapter が同一 mlua version で eval/budget/artifact を製品
  稼働させており、写像規則（sandbox strip・interrupt・memory
  limit・lift 上限）が実証済み。
* **版整合**: mlua→luau0-src の Luau revision（0.740）は
  `vendor/luau`（Analysis）と同一 — 既存の版整合不変条件を
  そのまま継承できる。
* **責務分離**: analyzer（native bridge、解析面のみ必要）と
  runtime（VM eval）は既に独立 port。runtime を native bridge
  に寄せると 2 つの VM アクセス経路が併存し、境界が濁る。

## Considered Options

### (a) mlua Rust API（vendored Luau VM）

* 既存 `mlua` 依存の safe API で `Lua` state を session 毎に生成し
  eval/budget/artifact を実装。
* **採用**: 新規依存ゼロ・safe・interim で実証済み・analyzer の
  native bridge と独立した責務境界を維持。

### (b) native bridge 拡張（eval/budget を C ABI shim に追加）

* `native/rf_luau_bridge` に eval 系 export を追加し、VM 実行・
  interrupt・value 直列化を C++ 側で実装。
* **却下**: mlua が提供する機能を unsafe FFI で再実装するだけで
  capability 向上なし。`unsafe` 面積・SAFETY 監査対象・C++ shim
  の保守対象が増大。JSON 越境による値写像の二重実装（lift を
  C++ と Rust の両側で維持）が必要になり、canonical 整合の
  リスクを持ち込む。

### (c) 新規 crate 分離（runtime 専用 crate 新設）

* `ruviform-script-luau-runtime` 等を新設。
* **却下**: backend_id `"luau-mlua"` は language=luau の runtime
  実装として analyzer と同一 crate の責務（Luau 製品 backend
  統合）。crate を分けると mlua 依存と shim link 供給の配線が
  二重化し、version 整合不変条件の管理面が増える。Issue の
  編集範囲も `crates/ruviform-script-luau` 内を指定。

## Decision Outcome

**Option (a) — mlua Rust API を採用** する。

`crates/ruviform-script-luau/src/runtime.rs` に `LuauBackend`/
`LuauSession` を実装し、mlua safe API のみで eval/budget/
artifact を構成する。analyzer の native bridge は変更しない
（hybrid 構成の確定: analyzer=native bridge、runtime=mlua —
#692 AC4 ADR の「runtime=mlua」予告を本 ADR で確定）。

### Consequences

* **Positive**: 新規外部依存・unsafe・C++ shim 変更がゼロ。
  interim 実装の意味論を safe Rust へ写像でき、deterministic
  OS-L1 test をそのまま製品契約へ移植可能。
* **Negative**: `max_instructions` は mlua API に存在せず
  `UnsupportedBudget`（fail-closed で正直に拒否）。host binding
  実行は契約に seam がなく本 backend では非実行（consumer gate
  へ委譲）。
* **Neutral**: `mlua` の `send` feature は有効化しない —
  `Lua`/`ScriptSession` は `!Send` で owner-thread-only を
  型で表現する（`BackendCapabilities` と一致）。

### Confirmation

* `crates/ruviform-script-luau` の `tests/runtime.rs` が
  open/evaluate/cancel/budget/artifact/fail-closed 行列と
  `ScriptBackendRegistry` resolve→eval 疎通を Windows x86_64 で
  Pass すること（`mise run test ruviform-script-luau`）。
* `Cargo.toml`/`Cargo.lock` に差分がないこと（依存追加なし）。
* 実 OS/実 VM 経路の受入は consumer gate（synvelis#516・
  synvelis#267 AUTX-08）へ委譲 — 本 ADR は OS-L1 のみを対象とし、
  実機受入を主張しない。

## Pros and Cons of the Options

### (a) mlua Rust API

* Good: 依存ゼロ追加・safe・実証済み・責務境界明確。
* Bad: 命令数 budget 非対応（fail-closed で処理）、mlua の
  safe API が将来 version で変わり得る（`Cargo.lock` pin +
  版整合不変条件で管理）。

### (b) native bridge 拡張

* Good: VM 実行を C++ 側に統一できる（analyzer と同一経路）。
* Bad: unsafe/JSON 写像/保守面積の増大に対し capability 利得なし。
  mlua が既に同一 VM を safe に公開している。

### (c) runtime 専用 crate 新設

* Good: 責務の物理分離。
* Bad: 依存・link 供給・version 整合の配線重複。Issue の編集
  範囲外でもある。却下。

## More Information

* #692 AC4 ADR: `docs/adr/issue-692-luau-dependency-selection.md`
  — analyzer=native bridge 採用、runtime=mlua は「別 slice で
  再評価」と明記。本 ADR がその再評価の確定記録。
* interim 参照: `synvelis/crates/synvelis-script-luau/src/lib.rs`
  （READ-ONLY）— session 毎 fresh `Lua`・`ALL_SAFE` stdlib・
  `require`/`loadstring`/`print`/`debug` 除去・`ChunkMode::Text`・
  `set_interrupt`/`set_memory_limit`・lift 上限の実証済み写像。
* 版整合: mlua 0.12.2→mlua-sys 0.13.0→luau0-src 0.22.0+luau740
  = Luau 0.740。`vendor/luau`（Analysis）と同一 revision —
  `build.rs`/`Cargo.toml` の不変条件コメントどおり。
* mlua/luau0-src 更新時は vendored `Analysis/` の再 vendor と
  `build.rs` link lib 名再照合が必須（#692 AC4 ADR と同一条件）。

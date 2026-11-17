---
id: "DISC-uyGbmYQxvwiFeiFkGWSPO7"
type: "discovery"
status: "resolved"
title: "Issue #716 runtime ScriptBackend — 調査記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/716"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
    - "docs/designs/issue-692-luau-backend.md"
  changes:
    added: []
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
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-luau-backend.md"
    - "docs/adr/issue-692-luau-dependency-selection.md"
    - "evidence/rf692/luau-backend-verification.md"
    - "evidence/rf692/verification.md"
  defers: []
  deferred-by: []
---

# Issue #716 runtime ScriptBackend — 調査記録

## Summary

Issue #716（RF-SCRIPT-03 follow-up）の実装前調査。#692 s1（中立契約
`crates/ruviform-script`、PR #698）と s2（Luau analyzer backend
`crates/ruviform-script-luau`、PR #702）が main に着地済みで、
runtime `ScriptBackend`/`ScriptSession`（eval/budget/artifact）の
port owner が未実装のまま残っていた。本 slice がその port owner。
中立契約 trait・既存 Luau crate 構造・synvelis interim adapter
（eval/budget/artifact の consumer 側重複実装）・#692 AC4 ADR を
実読し、runtime 依存選定（mlua Rust API vs native bridge）と
session 意味論の写像を確定した（confidence: high — 全て
merge 済み成果物と実コードの実読に基づく）。

## Exploration Goal

- `crates/ruviform-script/src/{session,registry,budget,host,
  capability,value,canonical}.rs` が公開する authoritative な
  C2 runtime 契約（`ScriptBackend`/`ScriptSession`/`SessionConfig`/
  `SessionError`/`ExecutionBudget`/`CancelToken`/`HostApiSurface`/
  `ScriptArtifact`/`ScriptResult`）と fail-closed 規則を確定する。
- `crates/ruviform-script-luau` の既存構造（analyzer/ffi/contract/
  build.rs/vendor/native）と `Cargo.toml` の mlua 既依存の実態を
  確認し、runtime 追加の置き場所と依存影響を確定する。
- synvelis `synvelis-script-luau`（interim mlua adapter）の
  eval/budget/artifact 意味論を読み、交換対象としての設計参照を
  確定する。
- #692 AC4 ADR（`issue-692-luau-dependency-selection.md`）が
  runtime 依存を「別 slice で再評価」と明記していることを確認し、
  本 slice の ADR 作業範囲を確定する。

## Findings

- **中立契約は完備済み・変更不要**: `ScriptBackend::open_session
  (SessionConfig) -> Result<Box<dyn ScriptSession>, SessionError>`、
  `ScriptSession::{session_id, backend_id, evaluate(Option<&str>),
  cancel, cancel_token, close}`。`SessionError` は
  `UnsupportedBudget{kind}`/`InvalidHostSurface`/`SourceDigest
  Mismatch`/`SourceTooLarge`/`LanguageMismatch`/`IsolationViolation`/
  `ResourceExhausted`/`Backend` の typed variant。`ScriptErrorKind`
  は `Parse/Compile/Type/Runtime/BudgetExceeded/Cancelled/
  Unsupported/Host/Backend/Internal`（unit variant — budget kind
  payload なし）。`ScriptBackendRegistry::new()` は空出荷・fake
  プリ登録なし（`registry.rs`）。`ScriptArtifact::new` は payload
  `BTreeMap<String, ScriptValue>`（record）から `definition_digest`
  を計算する。
- **open_session の fail-closed 検証点**（contract `session.rs` +
  `tests/common/mod.rs` FakeBackend の語彙に整合）: language 一致、
  `host_bindings.validate_neutral()` + `HOST_SURFACE_SCHEMA_VERSION`、
  source size 上限、要求 budget 次元 ⊆ 広告可能集合
  （`UnsupportedBudget{kind}`）、`resume` digest vs
  `source.digest`（`SourceDigestMismatch`）。`placement_hint` は
  advisory（ignore または reject が許容）。
- **host binding に実行 seam がない**: `HostApiSurface` は記述子
  のみ（`host.rs` — "no handler execution seam"）。`SessionConfig`
  に handler を渡す経路が契約上存在しないため、本 backend は
  host call を実行できない。妥当な surface は検証して受理するが
  VM へは注入しない — script が binding 名を参照しても通常の
  Luau runtime error（nil index/call）で fail-closed する。
  `max_host_calls` は「host call 経路が存在しない」ため vacuous
  に充足される budget として受理する。
- **mlua は既依存で link 供給中**: `Cargo.toml:39` の
  `mlua = 0.12.2 (luau)` は analyzer shim がリンクする Luau VM
  静的 lib（mlua→mlua-sys→luau0-src 0.22.0+luau740 = Luau 0.740）
  の供給元として必須依存。runtime backend が mlua Rust API を
  使っても**新規外部依存・Cargo.lock 変更は不要**の見込み。
- **synvelis interim が runtime 意味論の実証済み参照**:
  `synvelis/crates/synvelis-script-luau/src/lib.rs` — session 毎に
  fresh `Lua`（`new_with(ALL_SAFE)`）、`require`/`loadstring`/
  `print`/`debug` 除去、`ChunkMode::Text`、`set_interrupt` で
  wall-clock/cancel、`set_memory_limit` で memory、canonical 相当
  の serialized size で output、lift で `ScriptValue` 化
  （depth/node 上限・record/array 判定・整数 i64 降格）、
  `max_instructions`/`max_host_calls` は `UnsupportedBudget`。
  ruviform 中立契約では `max_host_calls` の扱いと error variant
  が異なる（下記 design で確定）。
- **budget 次元の実現可能性**（mlua 0.12.2 API で実測可能な範囲）:
  `max_wall_time` = interrupt callback の deadline 検査、
  `max_memory_bytes` = `Lua::set_memory_limit`（0 は mlua で
  unlimited 意味のため consumer 指定 0 は `UnsupportedBudget` で
  reject）、`max_output_bytes` = eval 後の canonical 直列化サイズ
  検査、`max_host_calls` = host call 経路非存在ゆえ vacuous 受理、
  `max_instructions` = mlua に命令数 budget API なし →
  `UnsupportedBudget{Instructions}` で fail-closed。
- **cancel は contract `CancelToken` を interrupt closure へ clone
  注入**で実現可能（`CancelToken: Clone` = Arc<AtomicBool>）。
  session.cancel_token() が返す clone を consumer が cancel しても
  interrupt が `is_cancelled()` を観測する。deadline/cancel の
  区別は共有 `AtomicU8` hit flag で post-eval に写像
  （pcall 飲み込み対策の defense-in-depth — interim と同型）。
- **`ScriptErrorKind` に budget kind payload がない**:
  `BudgetExceeded` は unit variant のため、どの budget で落ちたかは
  `message` に記す（contract の意図どおり — kind 分類のみ契約）。
- **crate 内配置**: `src/runtime.rs`（新規 module）に
  `LuauBackend`/`LuauSession`/value lift を集約。`ffi.rs` は
  analyzer 専用のまま不変。`unsafe` は runtime では不要
  （mlua safe API のみ — `unsafe` 追加ゼロ）。
- **公開 enum 追加なし**: 新規公開型は `LuauBackend`（struct）のみ
  — `#[non_exhaustive]` enum ポリシーへの抵触なし。
- **`entry` / evaluate 回数**: contract は `evaluate(Option<&str>)`
  の entry override を「backend が named entry points を支持する
  場合」と条件付きで規定。本 backend は chunk return value のみ
  評価するため `Some(entry)` は `ScriptErrorKind::Unsupported` で
  fail-closed。evaluate は session あたり 1 回（2 回目は
  `Internal` — interim と同じ single-shot 意味論、VM 大域状態の
  再入曖昧性を排除）。

## Evidence

- `crates/ruviform-script/src/{session,registry,budget,host,
  capability,value,canonical,lib}.rs`（s1 merge — 契約
  authoritative 形）・`crates/ruviform-script/tests/{session,
  common/mod}.rs`（fail-closed 行列と FakeBackend 語彙）
- `crates/ruviform-script-luau/{Cargo.toml,build.rs,src/lib.rs,
  src/analyzer.rs,src/ffi.rs,src/contract.rs,README.md}`・
  `crates/ruviform-script-luau/tests/{bridge,registry,faults}.rs`
  （s2 merge — 既存構造と test 規約）
- `G:/repositry/audio-products/synvelis/crates/synvelis-script-luau/
  src/lib.rs`（cross-repo READ-ONLY — interim eval/budget/artifact
  実装、交換対象の設計参照）
- `docs/designs/issue-558-script-contract-design.md`（approved —
  C2/C3/C4/C6 正本）・`docs/adr/issue-558-script-backend-contract.md`
- `docs/adr/issue-692-luau-dependency-selection.md`（AC4 — runtime
  依存「別 slice で再評価」の明記）・`docs/{specs,designs,plans,
  tasks,test-specs,discovery,impl}/issue-692-*.md`（docs chain 先例）
- `evidence/rf692/{verification,luau-backend-verification}.md`
  （前 slice 検証実績と consumer gate 委譲の明記先例）
- `Cargo.toml`/`Cargo.lock`（mlua 0.12.2 既在・workspace lints
  `pedantic`+`exhaustive_enums` warn→`-D warnings`）
- `mise.toml` `[tasks.check]`/`[tasks.lint]`/`[tasks.test]`/
  `[tasks.fmt-check]`（package 必須引数 — 新規 tests/*.rs は
  自動収集）

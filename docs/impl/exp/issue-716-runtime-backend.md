---
id: "EXP-716RuntimeBackend0001"
type: "impl"
status: "completed"
title: "Issue #716 runtime ScriptBackend — 実装過程の観察と判断"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/716"
  changes:
    added:
      - type: "directory"
        path: "docs/impl/"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-716-runtime-backend.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-716-runtime-backend.md"
    - "docs/impl/exp/issue-692-luau-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #716 — experiment notes

実装過程の観察と判断の記録。正本 IR:
`docs/impl/ir/issue-716-runtime-backend.md`、証拠:
`evidence/rf716/verification.md`。companion JSONL:
`docs/impl/exp/issue-716-runtime-backend-exp.jsonl`。

## 採用した判断

- **mlua Rust API を runtime 依存として採用（#692 AC4 再評価）**:
  interim（synvelis `synvelis-script-luau`）が同一 mlua 0.12.2
  で eval/budget/artifact を実証済みで、必要な API
  （`new_with(ALL_SAFE)`/`set_interrupt`/`set_memory_limit`/
  `ChunkMode::Text`/`eval::<MultiValue>`/`pairs`）が全て safe
  API として存在。新規依存・unsafe・C++ shim 変更ゼロ。
  `docs/adr/issue-716-runtime-backend.md` に確定記録。
- **`send` feature は有効化しない**: mlua `send` feature を付けると
  `Lua: Send` になり `ScriptSession` の owner-thread-only 性を
  型で表現できなくなる。既定（非 send）で `!Send` を維持し、
  `BackendCapabilities{owner_thread_only: true, send: false,
  sync: false}` と一致させた。
- **contract `CancelToken` を interrupt closure に clone 注入**:
  interim は独自 `AtomicBool` を持っていたが、中立契約には
  `CancelToken`（`cancel_token()` で consumer へ clone を渡す
  仕組み）がある。session 保持の contract token を closure に
  clone して渡すことで、`session.cancel()`（既定実装 =
  `cancel_token().cancel()`）と consumer 側 cancel の両方が
  同一 flag を立て、interrupt が safepoint で観測する。
- **`source.digest` の再計算検証を追加**: FakeBackend は
  `resume` vs `source.digest` のみ検査するが、実 backend では
  `ArtifactSource.source_digest` が artifact provenance に
  なるため、宣言 digest が text と乖離する ScriptSource を
  境界で拒否する（sha256 ≤1MiB でコスト無視可能）。
- **`max_host_calls` を vacuous 受理**: `HostApiSurface` は
  記述子のみで handler 実行 seam が契約に存在しない。
  「host call ≤ limit」は構造的に常に真 — 受理は silent
  fail-open ではなく、capability_ids に `"host.call"` を
  広告しないことで非対応を正直に表現した。
- **evaluate single-shot + entry→Unsupported**: 契約は evaluate
  回数を規定しないが、VM 大域状態を持つ session での再 eval は
  大域汚染の曖昧性を生む。interim と同じ single-shot を採用し、
  2 回目は `Internal`、`entry` は `Unsupported` で fail-closed。
- **非有限 `Number` を許容**: interim は `ValueOutOfRange` で
  非有限を拒否したが、中立契約の `canonical_number` は NaN/±inf
  を決定的 tag record で表現するため、`Number(NaN)` をそのまま
  通す（余計な special-case を増やさない）。
- **`set_interrupt` は `Result` を返さない**: mlua 0.12.2 の
  `Lua::set_interrupt` は `()` 返却 — interim の
  `.map_err` 想定は本 API では不要（compile error で検出・修正）。

## 却下した代替案

- **native bridge 拡張（eval/budget を C ABI shim に追加）**:
  mlua safe API が必要機能を全て提供するため、unsafe FFI 面・
  JSON 値写像の二重実装・保守面積の増大に見合う利得なし（ADR）。
- **runtime 専用 crate 新設**: backend_id `luau-mlua` は
  language=luau の統合 backend であり analyzer と同一 crate の
  責務。依存・link 供給の配線重複を避ける（ADR）。
- **`evaluate` 複数回許容**: VM 大域状態の再入曖昧性を排除する
  ため single-shot を採用（consumer は再評価に新 session を
  開く）。
- **named entry points**: chunk return value のみ評価 —
  `entry=Some` は `Unsupported`（silent ignore しない）。

## 観察された問題と対処

- **`&dyn ScriptBackend` は Debug 非実装**: `resolve()` の
  `expect_err` が `T: Debug` bound で使えない — 契約 test の
  `expect_resolve_err` と同じ match helper を test 内に追加。
- **determinism test の script 設計ミス**: `{t.a+t.b, name="x"}`
  は int key + string key の mixed table で設計どおり Internal
  — test script を record-only `{sum=..., name=...}` に修正。
- **clippy pedantic on tests**: `unreadable_literal`/
  `doc_markdown`/`default_trait_access` を `--all-targets` で
  検出 — literal 区切り・doc backtick・`BTreeMap::default()`
  で修正（`#[allow]` 不使用）。

## 残課題（consumer gate 委譲）

- 実 OS/実 VM 経路の受入 — synvelis#516（port 交換・binding
  適合）・synvelis#267 AUTX-08（SCRIPT-RUN/OS E2E）。本 slice は
  deterministic OS-L1 のみで、実機・実 consumer 経路は未検証。
- `max_instructions` budget — mlua に命令数 API がなく
  `UnsupportedBudget`（fail-closed で正直に拒否）。必要なら
  将来 native 側 instruction hook の検討対象。
- host binding 実行 — 契約に handler seam が追加された時点で
  別 slice として実装（現状は記述子検証・受理のみ）。

---
id: "DESIGN-BL3vYGzJnsXP64pLXgyLln"
type: "design"
status: "approved"
title: "Issue #716 runtime ScriptBackend — 詳細設計"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/discovery/issue-716-runtime-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
    - "docs/designs/issue-558-script-contract-design.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/impl/ir/issue-716-runtime-backend.md"
  depends-on:
    - "docs/specs/issue-716-runtime-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-716-runtime-backend.md"
  derived-by:
    - "docs/plans/issue-716-runtime-backend.md"
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-716-runtime-backend.md"
    - "crates/ruviform-script/src/session.rs"
    - "crates/ruviform-script/src/budget.rs"
    - "crates/ruviform-script/src/value.rs"
    - "crates/ruviform-script/src/capability.rs"
    - "crates/ruviform-script/src/host.rs"
    - "crates/ruviform-script/src/canonical.rs"
    - "crates/ruviform-script-luau/src/analyzer.rs"
    - "crates/ruviform-script-luau/src/contract.rs"
  defers:
    - "実 OS/実 VM 経路の受入（consumer gate — synvelis#516・synvelis#267 AUTX-08）"
  deferred-by: []
---

# Issue #716 runtime ScriptBackend — 詳細設計

## Overview

`crates/ruviform-script-luau/src/runtime.rs` を新設し、
`LuauBackend`（`ScriptBackend` 実装）と private `LuauSession`
（`ScriptSession` 実装）を mlua 0.12.2 safe API のみで構成する。
既存 `analyzer.rs`/`ffi.rs`/`contract.rs`/`build.rs`/`native/`/
`vendor/` は不変。`lib.rs` に `mod runtime;` + `pub use
runtime::LuauBackend` を追加し、crate docs/README の
「runtime は非目標」記述を更新する。新規公開 enum なし。
unsafe 追加ゼロ。Cargo.toml/Cargo.lock 差分なし見込み。

## Architecture

```
SessionConfig (neutral)
   │ open_session — fail-closed 検証（spec 表）
   ▼
LuauSession { session_id, backend_id, token: CancelToken,
              deadline: Option<Instant>, interrupt_hit: Arc<AtomicU8>,
              lua: Lua, budget: ExecutionBudget, artifact_kind: Option<ArtifactKind>,
              evaluated: bool, closed: bool }
   │ evaluate(None)
   ▼
mlua Lua (fresh, ALL_SAFE, sandbox strip)
   │ lua.load(text).set_name("session.luau").set_mode(Text).eval::<MultiValue>()
   ▼
interrupt callback（deadline + token.is_cancelled → hit flag → Err）
   ▼
MultiValue → lift → canonical size check → artifact wrap
   ▼
ScriptResult / ScriptError (neutral)
```

`open_session` は検証のみで VM を生成するか、session 生成時に
generate するか — **session 生成時に fresh `Lua` を作る**
（検証通過後にのみ VM コストを払う。`open_session` の失敗経路で
VM を作らない）。

## Module layout

`src/runtime.rs` 内の構成（1 file — analyzer.rs と同程度の凝集）:

- `pub struct LuauBackend` — descriptor 保持、`new()`/`Default`。
- `impl ScriptBackend for LuauBackend` — `descriptor()`/
  `open_session()`。
- `struct LuauSession` — `impl ScriptSession`。
- `struct InterruptState { hit: AtomicU8 }` + 定数
  `HIT_CANCEL`/`HIT_DEADLINE`（interim と同型）。
- free fn: `validate_config`（open_session の fail-closed 表を
  順に実行）、`create_lua`（fresh VM + sandbox strip + budget
  適用）、`lift`/`lift_table`（値写像）、`map_eval_error`、
  `canonical_size`。
- 定数: `MAX_SOURCE_BYTES = 1 << 20`、`MAX_VALUE_DEPTH = 64`、
  `MAX_LIFT_NODES = 1_000_000`、`BACKEND_ID = "luau-mlua"`、
  `CHUNK_NAME = "session.luau"`。

## open_session 検証順序（決定的）

FakeBackend と同順序 + 本 backend 固有分を末尾へ:

1. `source.language != "luau"` → `LanguageMismatch`
2. `host_bindings.validate_neutral()` → `InvalidHostSurface`
3. `host_bindings.schema_version != HOST_SURFACE_SCHEMA_VERSION`
   → `InvalidHostSurface`
4. `source.text` byte 長 > `MAX_SOURCE_BYTES` → `SourceTooLarge`
5. `source.digest != source_digest(text)` → `SourceDigestMismatch`
   （expected=source.digest 宣言値、actual=再計算値 — provenance
   偽装を fail-closed）
6. `resume` 指定かつ `!= source.digest` → `SourceDigestMismatch`
   （expected=resume、actual=source.digest — FakeBackend と同じ向き）
7. `source.schema_version != SOURCE_SCHEMA_VERSION` → `Backend`
8. budget 次元走査: `max_instructions` → `UnsupportedBudget
   {Instructions}`；`max_memory_bytes == Some(0)` →
   `UnsupportedBudget{MemoryBytes}`；`usize::try_from` 失敗 →
   同；`max_wall_time` の `checked_add` overflow →
   `UnsupportedBudget{WallTime}`。対応次元 {WallTime,
   MemoryBytes, OutputBytes, HostCalls} は受理。
9. `artifact_kinds` ⊄ 広告集合 or len > 1 → `Backend`
10. `Lua::new_with` 失敗 → `ResourceExhausted`；sandbox strip・
    `set_memory_limit` 適用失敗 → `Backend`
11. `SessionId` を atomic counter から採番 → session 構築

## evaluate 詳細

```
if closed → Internal("session is closed")
if evaluated → Internal("session already evaluated")
evaluated = true
token.is_cancelled() → Cancelled
entry.is_some() → Unsupported
lua.load(text).set_name(CHUNK_NAME).set_mode(Text)
    .eval::<MultiValue>()
  Err(e) → map_eval_error(e, &hit_state)
interrupt_error() → hit flag 分類（cancel→Cancelled,
                    deadline→BudgetExceeded）
lowered = match values.len() { 0 → default, 1 → lift(v), _ → Internal }
if let Some(v) = lowered.return_value:
    canonical = canonical_value(&v) → Err → Internal（到達不能だが
    fail-closed に map）
    canonical.len() > max_output_bytes → BudgetExceeded
if artifact_kind:
    v must be Record → else Internal
    ScriptArtifact::new(kind, payload_map, ArtifactSource{
        source_digest, source_revision}, ProducedBy{backend_id,
        version: descriptor.version}) → Err → Internal
ScriptResult { return_value: lowered, artifact, diagnostics: [] }
```

`map_eval_error`: `hit==CANCEL` → Cancelled；`hit==DEADLINE` →
BudgetExceeded；`MemoryError` → BudgetExceeded；`SyntaxError` →
Parse；`RuntimeError` → Runtime；その他 → Backend。message は
`err.to_string()`（mlua/luau 文字列 — vendor 型は漏らさない）。

## sandbox / VM 構成

- `Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default())` —
  `StdLib::ALL_SAFE` は `debug`/`io`/`os`/`package` を含まない
  safe subset。さらに `globals().set("require", Nil)`・
  `loadstring`・`print`・`debug` を明示除去（interim と同じ
  4 点 — digest pin 外 code 実行・stdout 汚染・debug library
  公開の遮断。`type(require)=="nil"` 等を test で観測）。
- `ChunkMode::Text` — bytecode magic 入力を VM が拒否
  （`loadstring` 除去と合わせて bytecode 経路を閉じる）。
- `set_memory_limit(usize)` — `max_memory_bytes` 指定時のみ。
- `set_interrupt(closure)` — 常時設定。closure は
  `token.is_cancelled()` → hit=CANCEL + `Err(external
  "cancelled")`；deadline 到達 → hit=DEADLINE + `Err(external
  "wall-clock budget exceeded")`；それ以外 `Ok(VmState::Continue)`。
  closure は `token.clone()`（contract `CancelToken` — consumer
  側 cancel も観測）と `deadline: Option<Instant>`（Copy）を捕捉。

## artifact 意味論

`artifact_kinds` が空 → artifact 生成なし（`artifact: None`）。
1 件 `K` → lift 値が `Record` のときのみ `ScriptArtifact` 生成、
`return_value` は lift 値を `Some` で返す。`nil` 返却 →
`ScriptResult::default()`（artifact 要求があっても `None` —
条件付き artifact 生成を許容）。非 Record → `Internal`。

`ScriptArtifact::new` は payload `BTreeMap<String, ScriptValue>`
から `definition_digest` を canonical 経由で計算 — backend は
digest を自前計算しない（contract が authoritative）。

## 値 lift 詳細

`lift(value, depth, &mut nodes) -> Result<ScriptValue, ScriptError>`:

- depth > `MAX_VALUE_DEPTH` → `Internal`
- `*nodes == 0` → `Internal`（node 枯渇）; 各 node で decrement
- `Nil`→`Null`, `Boolean`→`Bool`, `Integer`→`Integer`,
  `Number`→(integral かつ `(-2^63..2^63)` → `Integer(f as i64)`
  — `cast_possible_truncation` は範囲保証済みとして扱う;
  非有限・範囲外・非整数 → `Number`)
- `String` → `to_str()` UTF-8 → `String`、失敗 → `Internal`
- `Table` → `lift_table`: `pairs()` 走査。空 → `Record({})`。
  全 key `String` → `Record`；全 key `Integer` かつ
  `1..=len` 連続 → `Array`；混在・疎・非文字列 key → `Internal`。
  `pairs()` の反復 error → `Runtime`。
- `Function`/`Thread`/`UserData`/`LightUserData`/`Buffer` 等 →
  `Unsupported`（値モデル非対応 — design C3 禁止越境の
  fail-closed）

`lift_table` の array 判定は interim と同じ 2-pass: まず
`(String,_)` 混在検査と integer key 収集、連続性
`1..=n` を検証してから lift。DAG（共有テーブル）は node 予算で
有界化 — 同一テーブルの再訪は再 lift（memoize しない —
interim と同じ。値は tree 展開される）。

## 非自明な判断

- **`max_host_calls` を受理する根拠**: host call 実行経路が
  契約・実装ともに存在しない（`HostApiSurface` は記述子のみ）。
  「host call 回数 ≤ limit」は構造的に常に真 — vacuous 受理は
  silent fail-open ではなく、consumer が宣言した上限を
  backend が破る経路が存在しないことの正直な反映。
  capability_ids に `"host.call"` は**広告しない**。
- **`source.digest` の再計算検証**: FakeBackend は検査しないが、
  artifact provenance（`ArtifactSource.source_digest`）が
  text と乖離し得るのは契約違反。sha256 1MiB 上限でコストは
  無視可能 — 境界での fail-closed 検証として採用。
- **evaluate single-shot**: contract は回数を規定しない。
  interim と同じ single-shot を採用 — VM 大域状態を持つ
  `Lua` での再 eval は大域汚染の曖昧性を生む。2 回目は
  `Internal` で fail-closed。`resume` は digest 検査のみで
  VM 状態復元ではない（contract の `resume` 意味論どおり）。
- **`entry` → `Unsupported`**: named entry points 非対応を
  typed error で返す（silent ignore しない）。
- **interrupt hit flag**: Luau interrupt は pcall 内で飲み込まれ
  得るため、eval 成功後も hit flag を再検査して Cancelled/
  BudgetExceeded を優先する（interim の defense-in-depth を継承）。
- **`owner_thread_only`/`!Send`**: mlua `send` feature 非有効で
  `Lua: !Send` — session を `Box<dyn ScriptSession>` で返すと
  `ScriptSession: Send` 要求は trait 定義にないため問題なし。
  `owner_thread_only: true` で正直に広告。

## 依存・link 影響

- `Cargo.toml`: 変更なし（mlua 既依存・`luau` feature 既存）。
- `Cargo.lock`: 変更なし見込み（新規依存なし）。
- `build.rs`: 変更なし（shim link 供給は analyzer 用に既存 —
  runtime が mlua API を使っても同一静的 lib を共有）。
- mlua `send` feature は有効化しない（`!Send` 保持のため）。

## test 戦略（`tests/runtime.rs` — 詳細は test-specs）

open_session fail-closed 行列（language/digest/size/budget/
artifact kind/host surface）→ evaluate 値写像（scalar/array/
record/nil/multi-return/非対応型）→ budget（wall/memory/
output/host_calls vacuous/instructions reject）→ cancel
（事前・consumer token・別 thread からの mid-eval）→ artifact
（kind/payload/digest/provenance）→ session lifecycle
（single-shot/close 冪等/closed eval）→ registry
resolve→eval 疎通と fail-closed。全て deterministic OS-L1。

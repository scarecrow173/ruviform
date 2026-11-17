---
id: "SPEC-ankzjfsDvEl62A6zPVHgnx"
type: "spec"
status: "approved"
title: "Issue #716 runtime ScriptBackend — 仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/716"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-716-runtime-backend.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-716-runtime-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-716-runtime-backend.md"
  depends-on:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-luau-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-716-runtime-backend.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
    - "docs/discovery/issue-716-runtime-backend.md"
    - "docs/adr/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-716-runtime-backend.md"
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/issue-692-luau-dependency-selection.md"
    - "docs/adr/issue-716-runtime-backend.md"
  defers: []
  deferred-by: []
---

# Issue #716 runtime ScriptBackend — 仕様

## Scope

Issue #716（RF-SCRIPT-03 follow-up）。`crates/ruviform-script-luau` に
中立契約 C2 `ScriptBackend`/`ScriptSession`（eval/budget/artifact
実行経路）の実 Luau runtime backend を実装し、vendored Luau VM
（mlua→mlua-sys→luau0-src、Luau 0.740）へ接続する。runtime 依存
選定（mlua Rust API vs native bridge — #692 AC4 再評価）を
`docs/adr/issue-716-runtime-backend.md` で確定する。

- **変更**: `crates/ruviform-script-luau/src/`（`runtime.rs` 新設 +
  `lib.rs` 公開面更新）、`tests/`（backend/session 系
  expected/observed test 追加）、`Cargo.toml`（依存選定で必要な
  場合のみ）、`README.md`。
- **変更**: `crates/README.md` の `ruviform-script-luau` 行
  （「runtime backend は別 slice」記述の更新のみ）。
- **非含有**: `crates/ruviform-script`（中立契約 — 原則無変更）、
  analyzer 側の変更、`mise.toml`/CI lane、docs index、consumer
  リポジトリ。

## Non-goals

- 実 OS/実 VM 経路の受入 — consumer gate（`scarecrow173/synvelis#516`
  port 交換・binding 適合、`scarecrow173/synvelis#267` AUTX-08
  SCRIPT-RUN/OS E2E）へ委譲。本 slice は deterministic OS-L1 範囲。
- host binding の実行 seam — 中立契約に handler 経路が存在しない
  ため本 backend は host call を実行しない（記述子の検証・受理
  のみ。実 binding 適合は consumer gate）。
- named entry points・evaluate 複数回実行・VM 状態永続化/
  Resume（`resume` は digest 検査のみで VM 状態復元ではない）。
- `max_instructions` budget — mlua に命令数 budget API がなく
  `UnsupportedBudget` で fail-closed。
- 3-OS（macOS/Linux）実機検証 — Windows x86_64 のみ実測
  （`evidence/rf560/platform-matrix.md` の NotRun 分類を継承）。

## Contract surface（`crates/ruviform-script-luau`）

### `LuauBackend`

- `LuauBackend::new() -> Self` / `Default` — descriptor を構築。
  FFI/shim 初期化は行わない（runtime は shim を使わない。
  `impl_version` の Luau revision 表記のみ `bridge_versions()` を
  参照）。
- `impl ScriptBackend for LuauBackend`:
  - `descriptor(&self) -> &BackendDescriptor`
  - `open_session(&self, config: SessionConfig)
    -> Result<Box<dyn ScriptSession>, SessionError>`

### descriptor 仕様

- `backend_id`: `ScriptBackendId::new("luau-mlua")`
- `language`: `ScriptLanguageId::new("luau")`（`luau_language()` 再利用）
- `version`: `ScriptBackendVersion::new(
  "<crate_ver>+<luau_version>", CONTRACT_VERSION)` —
  `env!("CARGO_PKG_VERSION")` + shim 報告 `luau_version` を合成
- `contract_version`: `ruviform_script::CONTRACT_VERSION`（=1）
- `capabilities`:
  - `artifact_kinds`: `{ActionRecipe, WorkflowDefinition}`
    （`ConsumerExtension{domain}` は domain 非有限のため非広告）
  - `capability_ids`: `{"session.evaluate"}`
  - `owner_thread_only: true`、`send: false`、`sync: false`
    （mlua `Lua` が `send` feature なしで `!Send`/`!Sync` —
    型で owner-thread affinity を表現）
  - `shared_vm_state: false`（session 毎に fresh `Lua` —
    F-16 isolation を構造的に保証）
  - `cancel_granularity: Cooperative`（Luau interrupt は safepoint
    観測 — statement boundary 相当。命令粒度は非対応）

### `open_session` fail-closed 検証（ shim/VM 到達前）

| 検証 | 失敗時 |
| --- | --- |
| `source.language != "luau"` | `LanguageMismatch{expected, actual}` |
| `source.schema_version != SOURCE_SCHEMA_VERSION` | `Backend{detail}`（envelope 版不一致 — 専用 variant なし） |
| `host_bindings.validate_neutral()` 失敗 / `schema_version != HOST_SURFACE_SCHEMA_VERSION` | `InvalidHostSurface{reason}` |
| `source.text.len() > 1 MiB` | `SourceTooLarge{bytes, max}` |
| `source.digest != source_digest(text)` | `SourceDigestMismatch{expected: source.digest, actual: 再計算}` |
| `resume` 指定かつ `!= source.digest` | `SourceDigestMismatch{expected: resume, actual: source.digest}` |
| 要求 budget 次元 ⊄ {WallTime, MemoryBytes, OutputBytes, HostCalls} | `UnsupportedBudget{kind}`（`Instructions` は常に非対応） |
| `max_memory_bytes == Some(0)` または `usize` 非変換可 | `UnsupportedBudget{MemoryBytes}`（0 は mlua で unlimited 意味 — silent fail-open 防止） |
| `max_wall_time` が `Instant + Duration` overflow | `UnsupportedBudget{WallTime}` |
| `artifact_kinds` が広告集合外を含む / 2 件以上 | `Backend{detail}`（専用 variant なし — 1 session 1 artifact） |
| VM 生成失敗 | `ResourceExhausted{detail}` または `Backend{detail}` |

`placement_hint` は advisory — 全 hint を受理し無視する
（owner-thread-only backend の実配置を広告するのみ）。

### budget 実施（eval 経路）

- `max_wall_time`: `Instant` deadline を interrupt closure が safepoint
  毎に検査 → `ScriptErrorKind::BudgetExceeded`。
- `max_memory_bytes`: `Lua::set_memory_limit`。`MemoryError` →
  `BudgetExceeded`。
- `max_output_bytes`: eval 後に持ち上げた値の canonical 直列化
  バイト長を検査 → 超過は `BudgetExceeded`。
- `max_host_calls`: host call 実行経路が存在しないため常に 0 ≤
  limit で vacuous 充足（受理するが enforcement 機構は不要 —
  設計判断として記録）。
- `max_instructions`: open_session で `UnsupportedBudget`。

### session 意味論

- session 毎に fresh `Lua`（`Lua::new_with(StdLib::ALL_SAFE,
  LuaOptions::default())`）。`require`/`loadstring`/`print`/`debug`
  global を `Nil` で除去（digest pin 外 code 実行・stdout 汚染・
  内部公開の遮断）。
- `evaluate(entry)`:
  - `closed` → `ScriptErrorKind::Internal`（lifecycle 契約違反）
  - 2 回目以降 → `Internal`（single-shot — VM 大域状態の再入
    曖昧性を排除）
  - `entry.is_some()` → `Unsupported`（named entry points 非対応）
  - cancel 済み → `Cancelled`
  - chunk は `ChunkMode::Text` のみ（bytecode magic を拒否）で
    1 回 eval。`MultiValue` は 0 件→`ScriptResult::default()`、
    1 件→lift、2 件以上→`Internal`。
  - `nil` 返却 → `ScriptResult::default()`（artifact/return_value
    とも `None`）。
  - lift 失敗・budget/cancel は typed `ScriptError`。
  - interrupt hit flag を post-eval に再検査（pcall 飲み込み対策）。
- artifact: `artifact_kinds` が 1 件 `K` のとき、lift 結果が
  `ScriptValue::Record` なら `ScriptArtifact::new(K, payload,
  ArtifactSource{source_digest, source_revision},
  ProducedBy{backend_id, version})` を生成。非 Record は
  `Internal`（artifact payload は record 契約）。`return_value` は
  lift 値をそのまま `Some` で返す（artifact payload と同一値）。
- `close()`: `Lua` state を drop。冪等。
- `cancel_token()`: session 保持の contract `CancelToken` の clone
  — consumer 側 cancel も interrupt が観測する。

### 値 lift（`LuaValue` → `ScriptValue`）

| mlua `Value` | `ScriptValue` |
| --- | --- |
| `Nil` | `Null` |
| `Boolean` | `Bool` |
| `Integer` | `Integer` |
| `Number(f)` | `f.fract()==0.0` かつ `|f| < 2^63` → `Integer(f as i64)`、それ以外（非有限・範囲外・非整数）→ `Number(f)` |
| `String` | UTF-8 検証後 `String`（非 UTF-8 → `Internal`） |
| `Table` | 全 string key → `Record`、連続整数 key `1..=n` のみ → `Array`、空 → `Record`。混在/疎配列/深さ >64/node >1M → `Internal` |
| その他（Function/Thread/UserData/Buffer 等） | `Unsupported`（値モデル非対応 — 禁止越境を fail-closed） |

`Absent` は Lua に対応物がないため lift は生成しない
（record field の `nil` は key 自体を消す — `Null` と `Absent`
の区別は top-level `nil`→`Null` 写像と record 省略で保存）。

### error 写像（mlua → `ScriptError`）

| mlua / 状態 | `ScriptErrorKind` |
| --- | --- |
| interrupt hit=cancel | `Cancelled` |
| interrupt hit=deadline / `MemoryError` / output 超過 | `BudgetExceeded` |
| `SyntaxError` | `Parse` |
| `RuntimeError` | `Runtime` |
| `Value` lift 非対応型 | `Unsupported` |
| lift 構造違反（深さ/node/混在 key/非 UTF-8） | `Internal` |
| その他 mlua error | `Backend` |

## ADR 参照

- `docs/adr/issue-716-runtime-backend.md` — runtime 依存選定
  （mlua Rust API 採用 — #692 AC4 再評価の確定記録）。
- `docs/adr/issue-692-luau-dependency-selection.md` — analyzer 側
  依存選定（native bridge）と runtime=mlua の hybrid 前提。
- `docs/adr/public-enum-non-exhaustive-policy.md` — 公開 enum は
  `#[non_exhaustive]`（本 slice の新規公開 enum なし）。
- `docs/adr/issue-558-script-backend-contract.md` — 中立契約正本。

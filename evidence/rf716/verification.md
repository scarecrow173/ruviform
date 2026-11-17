# rf716 verification — runtime ScriptBackend（eval/budget/artifact）

Issue #716（RF-SCRIPT-03 follow-up）— `ruviform-script-luau` への
runtime `ScriptBackend`/`ScriptSession` 実装と vendored Luau VM
（mlua 0.12.2 Rust API → luau0-src 0.22.0+luau740 = Luau 0.740）
への接続の検証記録。

- **検証レベル**: OS-L1（deterministic headless）— Windows
  `x86_64` 実機のみ。macOS/Linux は **NotRun**（3-OS は Daily/
  手動 Full CI 委譲 — `evidence/rf560/platform-matrix.md` の
  分類を継承）。
- **実施日**: 2026-11-17
- **ブランチ**: `feat/716-runtime-backend`
- **toolchain**: mise pin（`mise.toml` — rust 1.99.x 系、
  mlua 0.12.2 / luau0-src 0.22.0+luau740）

## 実行した検証（canonical = mise）

| コマンド | OS | toolchain | exit | 結果 |
| --- | --- | --- | --- | --- |
| `mise run check ruviform-script-luau` | Windows x86_64 | cargo/rustc (mise pin) | 0 | Pass |
| `mise run lint ruviform-script-luau` | Windows x86_64 | clippy `-D warnings` | 0 | Pass |
| `mise run test ruviform-script-luau` | Windows x86_64 | cargo test | 0 | Pass — 91 tests（unit 16 + bridge 22 + faults 1 + registry 9 + **runtime 43**） |
| `mise run test ruviform-script` | Windows x86_64 | cargo test | 0 | Pass — 中立契約 crate 非回帰 |
| `mise run fmt-check` | Windows x86_64 | cargo fmt --check | 0 | Pass |
| `mise exec -- prek run --all-files` | Windows x86_64 | prek hooks | 0 | Pass |

補助証拠（canonical ではない）: `cargo check`/`cargo test` 直接
実行は mise task 経由の確認に先立つ開発中の反復のみ — 最終結果は
上表の mise 経路。

## runtime backend 検証内訳（`tests/runtime.rs` — 43 tests）

- **descriptor/registry**: `luau-mlua`/`luau`/contract_version=1・
  artifact_kinds={ActionRecipe,WorkflowDefinition}・
  capability_ids={"session.evaluate"}・owner_thread_only・
  !send/!sync・shared_vm_state=false・Cooperative を広告。
  `ScriptBackendRegistry` resolve→open→evaluate 疎通 +
  空 registry/未広告 capability/ConsumerExtension artifact kind
  の fail-closed。
- **open_session fail-closed**: language mismatch・source digest
  再計算不一致・resume digest 不一致/一致・source >1MiB・
  `max_instructions`→UnsupportedBudget・`max_memory_bytes=0`→
  UnsupportedBudget・対応 4 次元受理・artifact kind 広告外/複数
  →Backend・host surface 不正/schema_version→InvalidHostSurface・
  妥当 surface 受理（実行 seam なし — script から binding 非到達
  を Runtime error で観測）・source schema_version→Backend。
- **evaluate/値写像**: scalar/array/record/nested/空 table・
  `return nil`/return なし→default・multi-return→Internal・
  function/coroutine/tostring→Unsupported・mixed/sparse/
  深さ>64/非 UTF-8→Internal・parse→Parse・runtime→Runtime・
  bytecode magic→err・entry→Unsupported・sandbox 4 global 除去
  （`type(require)=="nil"` 等）・determinism。
- **budget**: wall_time 超過→BudgetExceeded（`while true do
  end` + 1ms）・wall_time 内→成功・memory 100MiB vs 1MiB→
  BudgetExceeded・output canonical 102B vs 64B→BudgetExceeded・
  host_calls=0 vacuous 受理。
- **cancel**: 事前 cancel（consumer token clone）・`session.
  cancel()`・別 thread からの mid-eval cancel→全て Cancelled。
- **lifecycle**: single-shot（2 回目 Internal）・close 冪等・
  closed 後 evaluate→Internal・session_id 一意・placement_hint
  DedicatedThread 受理（advisory 無視）。
- **artifact**: ActionRecipe/WorkflowDefinition・payload record・
  `ArtifactSource{source_digest, source_revision}` provenance・
  `ProducedBy{backend=luau-mlua, version}`・digest=
  `definition_digest(payload)` 一致・return_value=Record・
  非 Record→Internal・nil→default・空 record→empty payload・
  artifact_kinds 空→artifact=None。

## Unsupported / NotRun（正直な分類）

- **`max_instructions`**: **Unsupported** — mlua に命令数 budget
  API がなく `open_session` で `UnsupportedBudget{Instructions}`
  （test で検証）。
- **host call 実行**: **Unsupported** — 中立契約に handler 実行
  seam が存在しない。記述子の検証・受理のみ（`max_host_calls`
  は vacuous 受理、capability `"host.call"` は非広告）。実 binding
  適合は consumer gate。
- **named entry points / evaluate 再実行 / Resume VM 復元**:
  **Unsupported** — `entry=Some`→`Unsupported`、2 回目 evaluate
  →`Internal`、`resume` は digest 検査のみ（VM 状態復元ではない）。
- **macOS/Linux**: **NotRun** — 3-OS は Daily/手動 Full CI 委譲。
- **`*`-full / `*-msrv` / Feature Matrix**: **NotRun** — タスク
  指定の必須検証に非含有（Daily/手動 Full CI 委譲）。

## consumer gate 委譲（AC4 — 文書化が達成条件）

実 OS/実 VM 経路の受入は **consumer gate へ委譲**する:

- `scarecrow173/synvelis#516` — port 交換・binding 適合
  （`synvelis-script-luau` interim adapter → `ruviform-script-luau`
  `LuauBackend` への置換え）
- `scarecrow173/synvelis#267` AUTX-08 — SCRIPT-RUN/OS E2E case
  （実 OS/実 VM 経路の E2E 受入）

先例: `evidence/rf692/verification.md` の consumer gate 委譲
明記と同型。本 slice は中立契約・backend 実装・deterministic
OS-L1 test までを担い、consumer 側の製品受入は行わない。

## 既知の制約・後続作業

- `max_instructions` budget は mlua API に存在しない — 必要なら
  将来 native 側 instruction hook の検討対象（現状は
  `UnsupportedBudget` で fail-closed）。
- host binding 実行は契約に handler seam が追加された時点で別
  slice（現状は記述子検証・受理のみ）。
- `docs/**/README.md`（docs index）は Coordinator が統合時に
  再生成 — 本 slice では未再生成（規約どおり）。

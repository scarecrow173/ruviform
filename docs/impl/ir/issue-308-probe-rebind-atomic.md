---
id: "IMPL-034UiuvjiiksJVjjkxtT5W"
type: "impl"
status: "completed"
title: "Issue #308 — Generation切替時のProbe再Binding全件整合・部分失敗契約"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/308"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/designs/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/test-specs/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/plans/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/tasks/issue-308-probe-rebind-atomic.md"
      - type: "file"
        path: "docs/impl/ir/issue-308-probe-rebind-atomic.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_probe.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/tasks/issue-308-probe-rebind-atomic.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-308-probe-rebind-atomic.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-308-probe-rebind-atomic.md"
    - "docs/designs/issue-308-probe-rebind-atomic.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-308-probe-rebind-atomic.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-308-probe-rebind-atomic.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #308 — Generation切替時のProbe再Binding全件整合・部分失敗契約

## Summary

Issue #185 の probe 実装では `on_publish` が `ClearAll{generation}` に続けて個別 `Bind` を bounded SPSC queue へ逐次投入しており、queue 飽和時に prefix のみ適用されて部分再 bind が残り得た（Err を返しても RT 側は半端な状態）。本実装は世代付き staged slots 方式を採用し、`RebindBegin`/`RebindStage`/`RebindCommit` の 3 相プロトコルで RT 側の live bindings を `mem::swap` による単一の不可分な置換に変更した。`on_publish` は `Result<RebindReceipt, ProbeError>` を返し、enqueue 失敗時は Commit を含まない inert prefix のみが queue に残り live bindings は不変 — drain 後の retry で全件 commit できる。未解決 point は receipt の `unbound` と `StatusKind::RebindDropped` で明示され、doc/impl 不一致（unresolved probe が静かに消える記述）も解消した。Capture は `CapturePolicy::StopAtBoundary`（既定・`StopReason::GenerationBoundary` で切断・残 budget 破棄）または `Restart`（新規 full budget の `CaptureStarted` で明示的な新 segment）を `set_capture_policy` で選択でき、旧世代の残 frame が新世代へ無断繰越しされることはない。`RecordMeta.generation` が segment 境界を区別し、RT 側は allocation/blocking/destructor を追加していない（`staged` は `bindings` と同じ `max_probes` capacity の事前確保 Vec、`mem::swap`/`clear`/`push` のみ）。

## Context

- 実施タスク: `docs/tasks/issue-308-probe-rebind-atomic.md`
- 上流: `docs/specs/issue-308-probe-rebind-atomic.md`、`docs/designs/issue-308-probe-rebind-atomic.md`（staged slots 方式の選定根拠: batch command variant より既存 SPSC/`ProbeCommand` 構造への変更が小さく、RT 側の epoch guard で fail-closed にできる）、`docs/plans/issue-308-probe-rebind-atomic.md`、`docs/test-specs/issue-308-probe-rebind-atomic.md`
- 原因: `ProbeBroker::on_publish` が `ClearAll` 適用後に逐次 `Bind` を push していたため、bounded queue の途中失敗で RT deck が部分適用状態になり得た。`capture_remaining` を新 binding へ無条件に引き継ぎ、unresolved probe を receipt 無しに脱落させていた点も Issue 指摘どおり。

## Implementation

### Changes

- `crates/ruviform-audio/src/typed/probe.rs`
  - `ProbeCommand` — `ClearAll` を廃し `RebindBegin{generation}`/`RebindStage{generation,binding}`/`RebindCommit{generation}` を追加（`#[non_exhaustive]` 維持）。
  - `ProbeDeck` — `staged: Vec<ProbeBinding>`（`max_probes` capacity 事前確保）と `staging: Option<u64>`（開いている attempt の epoch）を追加。
  - `drain_commands` — Rebind 3 相を `rebind_begin`/`rebind_stage`/`rebind_commit` に分離。generation 不一致または staging epoch 不一致の stage/commit は stale として drop + `stale_commands` 計上。staged 溢れ時は attempt を invalidate して commit も拒否。
  - `commit_rebind` — `mem::swap` で staged→live を不可分に適用し、新旧 set の diff から probe 毎に `Installed`/`RebindDropped`/`CaptureStarted`/`CaptureStopped{GenerationBoundary}` を発行。swap/clear のみで RT no-alloc 契約を維持。
  - `StopReason::GenerationBoundary`・`StatusKind::RebindDropped`・`CapturePolicy{StopAtBoundary,Restart}`・`RebindReceipt{generation,bound,unbound,capture_stopped,capture_restarted}` を追加。
  - `ProbeEntry.capture_policy` + `ProbeBroker::set_capture_policy` — policy は resolve 時に消費され RT へは command として送らない。
  - `on_publish` — 全 probe を resolve して staged binding を構築（未解決は `receipt.unbound` 行き・broker には残る）→ `StopAtBoundary` なら entry.mode を Monitor 化し `capture_stopped` 記録・`Restart` なら full budget で `capture_restarted` 記録 → Begin/Stage×N/Commit を `try_push`。途中失敗は `Err(CommandChannelFull)` で Commit 無し prefix のみ残る。
- `crates/ruviform-audio/tests/typed_probe.rs`
  - `republish`/`fill_commands`/`src_sink_spec`/`gen_counts`/`saturation_case`/`assert_retry_commits` helper を追加。
  - ex17 `rebind_queue_saturation_never_partially_applies` — capacity 8 に対し Begin 直前/直後/Stage 途中/Commit 直前/成功の 5 飽和点で部分適用ゼロ・`Unbound` 表面化・drain 後 retry で全件 commit を検証。
  - ex18 `rapid_republishes_commit_only_latest_generation` — gen2/gen3 の rebind を drain 前に連続 enqueue し、superseded 世代の record が 0・`stale_commands` 計上を検証。
  - ex19 `rebind_reports_and_drops_unresolved_points` — processor node を除いた spec への republish で `receipt.unbound`・`RebindDropped`・dropped probe の record ゼロを検証。
  - ex20 `capture_stops_at_generation_boundary_by_default` — 既定 policy で `CaptureStopped{GenerationBoundary}`・gen2 chunk ゼロ・明示 `capture()` での再開を検証。
  - ex21 `capture_restarts_with_fresh_budget_under_restart_policy` — `Restart` で full budget の `CaptureStarted`・旧 segment 残 frame 非継承を検証。
  - ex22 `rebind_preserve_output_golden` — republish+rebind 前後で sink 出力が bit 一致。
  - `ProbeChain` に `src: NodeId` を追加（`src_out` tap 用）。

### Contract amendment — observable behavior changes (migration note)

| Path | Before | After |
| --- | --- | --- |
| `ProbeBroker::on_publish` 戻り値 | `Result<(), ProbeError>` | `Result<RebindReceipt, ProbeError>`（bound/unbound/capture 遷移を型付きで返す） |
| rebind command 列 | `ClearAll` + 個別 `Bind`（部分適用し得た） | `RebindBegin`/`RebindStage`/`RebindCommit`（RT 側で不可分 commit） |
| unresolved point の扱い | 静かに脱落（doc と実装が不一致） | `receipt.unbound` + `StatusKind::RebindDropped`、broker には残り次回 publish で再 bind 可 |
| capture の世代跨ぎ | `capture_remaining` を新 binding へ黙って引継ぎ | `StopAtBoundary`（既定）で `GenerationBoundary` 停止・budget 破棄、または `Restart` で新規 segment |
| `StatusKind`/`StopReason` | — | `RebindDropped`/`GenerationBoundary` variant 追加（`#[non_exhaustive]` のため source-compatible） |

### Decisions During Implementation

- **staged slots 方式を採用**（棄却: `spsc` への batch push 追加、`ProbeCommand` への巨大 batch variant）— RT 側の ownership 境界（`ProbeDeck`）で staged/live を分離する方が、queue payload を不変に保ち `ruviform-rt` を無変更で済ませ、epoch guard による fail-closed を既存の generation guard と同じ構造で実現できる。`spsc.rs` は未変更。
- **commit は `drain_commands` 内・`run_site` 前に実行** — 同一 callback 内で observer が見るのは常に「全旧 or 全新」のみ。
- **staged 溢れ時は attempt 全体を invalidate** — 部分 staged が commit される経路を構造的に塞ぐ（broker は `max_probes` 上限で stage 数 ≤ `max_probes` のため通常到達しない防御経路）。
- **`StopAtBoundary` は broker 側で entry.mode を Monitor に更新** — enqueue 失敗時も RT 側 generation guard が既に capture を止めているため、broker 状態と観測実態が一致する。
- **policy は resolve 時消費・command 化しない** — RT 側は staged binding の mode/budget のみを見ればよく、policy 自体は非 RT の関心事。

## Verification

- OS: Windows 11 x64（Git Bash） / toolchain: `cargo 1.99.0`（mise pin）
- `git fetch origin main && git merge origin/main` — clean merge（競合なし）後に再検証
- `mise run fmt-check` — exit 0
- `mise run check` — exit 0（`cargo check --workspace --all-targets`）
- `mise run lint` — exit 0（`cargo clippy --workspace --all-targets -- -D warnings`）
- `mise run test` — exit 0（`cargo test --workspace`・179 test groups・0 failed。`tests/typed_probe.rs` 18/18 pass・新規 6 件含む）
- `mise run test-rt` — exit 0（RT フィルタ lane・135 groups・0 failed。`rt_probe_process_allocates_nothing` を含む RT no-alloc 非回帰を確認）

NotRun（未実測・明記）: 実デバイス/実 plugin 経路での probe 観測（#294 cross-consumer lane は coordinator 側の後続検証）、browser harness（#310 は本 Issue の前提ではない）。`on_publish` の呼出は ruviform 内テストのみで、consumer repo（synvelis/audio-lens）からの直接利用は現時点で存在しない。

## Consumer Impact Materials

- **synvelis**: **no direct impact（将来の受け皿）** — ruviform 内に `on_publish` の caller は存在せず、consumer 側の呼出経路は未配線。`RebindReceipt`/`CapturePolicy` は GUI-readable receipt として設計されており、synvelis#107/#109 の capture-stop/segment oracle が必要とする「requested/enqueued/installed/observed」の分離情報を提供する土台になる。
- **audio-lens**: **no impact** — probe typed runtime の利用は現時点で consumer 未接続。

## Related Experiments

### Adopted

- 世代付き staged slots（`RebindBegin`/`RebindStage`/`RebindCommit` + `mem::swap` commit）— design で選定した方式をそのまま実装。

### Rejected

- `ruviform-rt/src/spsc.rs` への batch push 追加 — batch command variant 方式より変更面が大きく、staged slots で同等の all-or-nothing を達成できるため。
- `ProbeCommand` への単一巨大 batch variant — command サイズが `max_probes` に比例して bounded でなくなり、SPSC slot の固定サイズ前提を崩すため。

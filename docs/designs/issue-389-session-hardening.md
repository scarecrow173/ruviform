---
id: "DESIGN-034UnxZxmELaLcIYblZFRI"
type: "design"
status: "approved"
title: "Issue #389 Plugin Editor Session hardening 設計（deadline/clock・明示 degrade・GraphicsLost・DComp 記録）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "file"
        path: "evidence/rf389-session-hardening/report.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-session-hardening.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-session-hardening.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "evidence/rf388-native-dock-windowing/report.md"
  derived-by:
    - "docs/plans/implement-issue-389-session-hardening.md"
    - "docs/test-specs/issue-389-session-hardening.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-session-hardening.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 Plugin Editor Session hardening 設計

## Context

PR #427 の `editor_session` 契約層は状態機械・request 世代照合・
terminal-once・L0/L1 ops 写像を確立したが、親 design 改訂 1 R6 が
残課題を明示している: wall-clock timeout 監視は Deferred のまま、
Floating degrade は未達、GPU host 固有 cause は未定義、そして
PR #501 の実測（GPUI 0.2.2 `CreateTargetForHwnd(topmost=true)` が
child HWND を覆い plugin pixel が合成されない）は #389 の
capability 記述へ未反映。本設計はこの 4 項目を**契約層の語彙として**
確定する。全て additive であり、既存 28 contract test と
`ruviform-poc-native-dock`（`open`/`close` sync API のみ使用）を
非回帰に保つ。

## Scope

### In Scope

- `SessionClock` port・`SessionDeadlinePolicy`・`MonotonicSessionClock`・
  request deadline の arm/evaluate・`tick`/`tick_all`・`OpenOutcome`・
  `EditorSessionEvent::Degraded`・`begin_degrade`・
  `SessionFailCause::GraphicsLost`・`EditorOpError::GraphicsLost`。
- capability matrix（本 design 末尾）への DComp `Unsupported` 行と
  pixel compositing `NotRun` の明記、`evidence/rf389-session-hardening/`
  の検証記録。

### Out of Scope

- 実 adapter の floating window / 実 pixel compositing（上流 GPUI
  制約・別 Issue）。
- `Plugin` trait・`HostCommand`/wire・`GuiParentToken` の契約変更。
- `ruviform-ui-gpui`・`ruviform-poc-native-dock` 他 crate の編集
  （additive API のため変更不要）。
- RT thread からの呼出し経路の存在自体（port は非 RT 専用で型が
 それを強制する — RT 側に clock/deadline API は生えない）。

## Components and Boundaries

すべて `crates/ruviform-plugin-host/src/editor_session.rs` 内の
additive 追加。`ruviform-plugin-core` は不変。

### D1: `SessionClock` port と `SessionDeadlinePolicy`

```rust
/// Monotonic clock port for editor-session request deadlines.
/// Values are only meaningful as differences; the epoch is the
/// implementation's own.
pub trait SessionClock {
    /// Current monotonic time.
    fn now(&self) -> Duration;
}

/// std clock: monotonic `Instant` elapsed since construction.
#[derive(Debug)]
pub struct MonotonicSessionClock { start: Instant }
impl MonotonicSessionClock { pub fn new() -> Self }
impl Default for MonotonicSessionClock { ... }
impl SessionClock for MonotonicSessionClock { ... }

/// Bounded-timeout policy armed on a session.
pub struct SessionDeadlinePolicy {
    /// Max time an in-flight request may remain incomplete.
    pub timeout: Duration,
    /// Monotonic clock the deadline is evaluated against.
    pub clock: Box<dyn SessionClock>,
}
```

- `Duration`（`std::time`）を時刻表現に使う — fake clock が任意の
  時刻を返せるよう `Instant` ではなく経過時間のドメイン値とする。
- `PluginEditorSession` は `deadline_policy: Option<SessionDeadlinePolicy>`
  と `deadline_at: Option<Duration>`（clock domain の絶対 deadline）を
  持つ。`deadline_at` は in-flight request（`Creating`/`Detaching`/
  `Closing`/`Recovering`）がある間だけ意味を持ち、読む側は必ず
  in-flight 判定を伴う（stale 値は残りうるが読まれない）。
- **default**: `PluginEditorSession::new` は
  `SessionDeadlinePolicy { timeout: DEFAULT_REQUEST_TIMEOUT,
  clock: MonotonicSessionClock::new() }` を arm する
  （`pub const DEFAULT_REQUEST_TIMEOUT: Duration =
  Duration::from_secs(10)`）。deadline は記録されるが、発火には
  driver の `tick`/`tick_all` 呼出しが必要 — 監視しない driver の
  挙動は現行と同一（非回帰）。`set_deadline_policy(None)` で明示的に
  disarm できる。
- `set_deadline_policy` は in-flight request があれば deadline を
  再 arm する（policy 差替えで期限が宙に浮かない）。

### D2: deadline の arm/evaluate と `Timeout` 終端

- `next_request()`（全 `begin_*` と `complete_detach` の re-attach
  request が通る唯一の採番点）で `deadline_at` を arm する:
  `clock.now().saturating_add(timeout)`。policy 不在なら `None`。
- `tick(&mut self) -> bool`: policy の `clock.now()` が in-flight
  request の `deadline_at` に到達していれば、その request を
  `EditorOpError::Timeout` 完了として終端化する:
  - `Creating`/`Recovering` → `complete_attach(request, Err(Timeout))`
    → `Failed{Timeout}` + `Failed` event。
  - `Detaching` → `complete_detach(request, Err(Timeout))` → 同上。
  - `Closing` → `complete_close` は error を運ばないため
    `mark_failed(Timeout)` 直接 → `Failed{Timeout}` + `Failed` event
    （`editor_live` は維持 — adapter が実際に close したか不明なため）。
  - いずれも後続の `complete_*` は state 不一致で `StaleRequest`
    になり、terminal event の重複は起きない。
- `request_deadline() -> Option<Duration>`: 現在の in-flight request
  の deadline（clock domain）— driver が wakeup 時刻を計算するための
  観測面。in-flight でなければ `None`。
- `PluginEditorSessions::tick_all() -> usize`: 全 session を 1 回ずつ
  tick し、timeout した request 数を返す。driver が registry 経由で
  一括監視するための面。
- **sync convenience（`open`/`close`/`recover`/`reattach`）は
  preempt 不能**: ops 呼出しは inline であり、deadline は
  `begin_*`→`complete_*` の間にのみ意味を持つ。doc に明記する
  （「呼出側は GUI thread で ops が速く返ることを前提とする」）。

### D3: 明示 degrade（`OpenOutcome`・`Degraded` event・`begin_degrade`）

```rust
/// How an open request concluded.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OpenOutcome {
    /// Attached in the requested mode — a clean attach.
    Attached(EditorCapabilities),
    /// The requested mode was refused (`Unsupported`) and the session
    /// explicitly degraded to `mode`. NOT a requested-mode attach —
    /// never a Native Dock PASS.
    Degraded { mode: AttachMode, capabilities: EditorCapabilities },
}
```

- `EditorSessionEvent::Degraded { mode: AttachMode }` を追加。
  degrade 成功時の**唯一の**終端 event — `Attached`/`Recovered` は
  発火しない（PASS 偽装の禁止）。
- `PluginEditorSession::open_or_degrade(ops, lease, size, preferred,
  fallback) -> Result<OpenOutcome, EditorSessionError>`:
  1. size 検証 → `begin_open(preferred)`。
  2. `ops.open(preferred)`:
     - `Ok(caps)` → 通常 attach → `OpenOutcome::Attached(caps)`。
     - `Err(Unsupported)` かつ `fallback != preferred` →
       `pending_mode = fallback` + `degrade_pending = true` にして
       `ops.open(fallback)` を **同一 request id で 1 回だけ** retry:
       - `Ok(caps)` → `complete_attach(request, Ok)` が
         `Attached{fallback}` に着地し `Degraded{fallback}` を発火
         → `OpenOutcome::Degraded{mode: fallback, capabilities: caps}`。
       - `Err(e)` → `Failed{fail_cause(e)}` → `Err(Ops(e))`。
     - その他の `Err(e)` → retry せず `Failed` → `Err(Ops(e))`。
  3. `preferred == fallback` は retry が無意味なので通常 `open` と
     同じ経路（`OpenOutcome::Attached` のみ返りうる）。
- degrade の retry 条件は **`Unsupported` のみ**: `LeaseRejected`/
  `HostLost`/`Plugin`/`Timeout` 等は mode 固有でないため degrade
  対象外（1 回の open 試行で終端）。
- `degrade_pending: bool`（`reattach_pending` と同型の private
  flag）: in-flight `Creating`/`Recovering` が degrade 経由である
  ことを `complete_attach` へ伝え、`Attached`/`Reattached`/
  `Recovered` の代わりに `Degraded{mode}` を発火させる。
  consume 時と `mark_failed` でクリア。
- async 経路: `begin_degrade(mode) -> Result<u64, EditorSessionError>`
  — `Failed` から `begin_recover` 相当の `Recovering{request}` に
  入り `degrade_pending` を立てる。driver が `ops.open(fallback)` を
  非同期に呼び `complete_attach` で着地させると `Degraded{mode}` が
  発火する。`begin_recover` 自体は不変（`Recovered` 経路を維持）。
- **degrade は Native Dock PASS の代替ではない**: `Degraded` event・
  `OpenOutcome::Degraded`・capability matrix の全てで通常 attach と
  区別される。L0 の `InProcessEditorOps` は `AttachMode::Floating` を
  依然 `Unsupported` で拒否するため、L0 での degrade retry は
  honest に `Failed{Unsupported}` へ終端する（floating surface が
  無いのは adapter の実態であり、session は偽らない）。

### D4: `GraphicsLost` cause

- `SessionFailCause::GraphicsLost` を追加（`HostLost` の次）。
  「graphics device/driver が喪失（device removed・driver reset・
  compositor failure）— editor の pixel が失われうるが DSP instance
  は生存しうる」。
- `EditorOpError::GraphicsLost` を追加し `fail_cause` で
  `GraphicsLost` へ写像。`Display` にも文言を追加。
- 到達経路は 2 つ: ops adapter が `Err(GraphicsLost)` を返す
  （→ `Failed{GraphicsLost}`）、driver が `notify_terminated(
  GraphicsLost)` / `PluginEditorSessions::notify_terminated` を呼ぶ
  （→ `Terminated{GraphicsLost}`）。`HostError` には GPU 概念が
  ないため `map_host_error` は変更しない。

### D5: capability matrix（本 slice 時点・正直記録）

凡例は親 design R5 と同じ: **Supported** = コード上成功経路あり
（実機未検証）、**Unsupported** = 定義済み拒否、**NotRun** =
実機・実 binary 未実施、**対象外** = 初期スコープ外。

| 経路 | Dock（Docked open） | Floating | Resize | Hide/Show | Reparent | Close | 根拠・状態 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| L0 / VST3（Windows） | Supported（`open_gui` 実装・`kPlatformTypeHWND`） | Unsupported | Unsupported（verb なし） | Unsupported | Unsupported | Supported（`close_gui` 実装） | `ruviform-plugin-vst3/src/host.rs`。実 attach 証拠は #388/#389 記録（本 slice では再実行せず NotRun） |
| L0 / VST3（非 Windows） | Unsupported（`EDITOR_PLATFORM_TYPE=None`） | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported | 非 Windows attach 未実装 |
| L0 / CLAP | Unsupported（`Plugin` 既定） | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported | GUI verb は既定 Unsupported のまま |
| L0 / AU（macOS） | 対象外 | 対象外 | 対象外 | 対象外 | 対象外 | 対象外 | ADR `plugin-architecture-clap-vst3`: AU は初期対象外 |
| L0 / Ruviform 製 GPUI Plugin | NotRun | NotRun | NotRun | NotRun | NotRun | NotRun | plugin 側 GPUI editor adapter・実装なし |
| L1 out-of-process（全規格） | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported | `L1EditorOps` 全 op 拒否・`l1/mod.rs` GUI verb 拒否 |
| Fake ops（contract） | Supported（fake） | 成功する（fake は mode/caps を強制しない） | 成功する（fake） | 成功する（fake） | 成功する（fake） | Supported（fake） | 状態機械契約検証のみ。**実 Dock PASS ではない** |
| **GPUI/DComp native surface pixel compositing（Windows）** | **Unsupported（実測）** | — | — | — | — | — | PR #501 実測: GPUI 0.2.2 `directx_renderer.rs` `CreateTargetForHwnd(hwnd, topmost=true)` が swapchain を child HWND の上に描画し、GPUI window 配下の全 child HWND（plugin editor・対照 probe HWND 双方）が画面に合成されない。HWND 機構（parentage/geometry/clip/hit-test/focus/lifetime/re-dock）は全て Pass だが plugin 画素は観測不能。根拠: `evidence/rf388-native-dock-windowing/report.md` |
| **native surface pixel compositing 実測（plugin 画素の目視確認）** | **NotRun** | — | — | — | — | — | DComp 制約により現行 GPUI 経路では観測不能。修復は上流 GPUI/adapter 設計の別 Issue |

## Data and Control Flow

```text
driver (GUI thread):
  session.set_deadline_policy(Some(SessionDeadlinePolicy{
      timeout, clock: Box<dyn SessionClock> }))   // 省略時は default arm
  session.open_or_degrade(ops, lease, size, Docked, Floating)
    -> ops.open(Docked) -> Ok: Attached + OpenOutcome::Attached
                     -> Err(Unsupported): ops.open(Floating)
                          -> Ok: Attached{Floating} + Degraded{Floating}
                                 + OpenOutcome::Degraded{Floating}
                          -> Err(e): Failed{cause(e)} + Failed event
  // async driver:
  session.begin_open(Docked) / begin_degrade(Floating) / begin_close ...
  ops.* を非同期 dispatch → complete_*(request, result)
  定期的に session.tick() / sessions.tick_all()
    -> deadline 超過 request を Failed{Timeout} へ終端
```

## Non-Functional Constraints

- 非 RT 専用・`unsafe` 不使用・外部依存追加なし（`std::time` のみ）。
- 全公開 enum `#[non_exhaustive]`（`OpenOutcome` を含む）。
- 既存 public API の signature は不変 — `open`/`close`/`recover`/
  `reattach`/`begin_*`/`complete_*`/`notify_terminated` は additive
  のみ。`ruviform-poc-native-dock` は変更なしで compile できる。
- `editor_session.rs` の surface-hygiene test（GPUI/OS handle 語彙の
  禁止）を維持 — 新規語彙に toolkit/OS handle 名を含めない。
- degrade・timeout は「成功の別形」ではなく独立した終端として
  観測可能であること（PASS 偽装の禁止を型と event で強制）。

## Risks and Trade-offs

- **default arm による surprise**: default policy が arm されると
  driver が `tick` を呼び始めた時点で timeout が発火しうる。
  緩和: 既定値は `DEFAULT_REQUEST_TIMEOUT`（10s）と明示 const、
  `set_deadline_policy(None)` で disarm 可能、sync path は
  preempt されないため既存利用に影響なし。
- **degrade retry の二重副作用**: `ops.open` を 2 回呼ぶため、
  adapter が preferred 試行で部分的に editor を作ってから
  `Unsupported` を返す実装だと orphan しうる。契約として
  「`Unsupported` を返す open は editor を生成していない」
  （既存の refused-open 規律と同じ前提）を doc に明記する。
- **`Closing` timeout の `Failed` 着地**: close が timeout すると
  `Failed{Timeout}`（`editor_live` 維持）— adapter が後で close
  を完了しても `complete_close` は `StaleRequest` で無視され、
  session は `close` 再試行か `recover` で収束する。二重終端なし。
- **clock の時計合わせ**: `Duration` domain は monotonic 経過時間
  のみを意味し、wall-clock・RT clock・media clock とは無関係。
  doc で明記。

## Deferred Design Concerns

- GUI runtime の timer/cancel 基盤が `tick`/`tick_all` を呼ぶ周期を
  所有する — composition root（`ruviform-ui-gpui`/engine 側）の
  責務であり本 slice では接続しない。
- 実 floating window adapter・pixel compositing 修復（上流 GPUI
  `topmost` 制約または adapter 設計変更）— 別 Issue。
- L1 wire での timeout/degrade 通知 — issue-115 解除条件充足後。

## Verification Notes

- [ ] `crates/ruviform-plugin-host/tests/editor_session.rs` に
  fake-clock timeout・degrade・GraphicsLost の contract test を追加し
  green（test-spec `docs/test-specs/issue-389-session-hardening.md`
  の SH-xx case）。
- [ ] `mise run fmt-check`/`check`/`lint`/`test` green。
- [ ] 既存 contract test・poc-native-dock 非回帰。

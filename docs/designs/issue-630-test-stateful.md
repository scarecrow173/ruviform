---
id: "DESIGN-034Y4dCo7zV1IwCz9gEIi0"
type: "design"
status: "approved"
title: "Issue #630 test_stateful slice — 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-test-stateful.md"
  implements:
    - "docs/specs/issue-630-test-stateful.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  blocks:
    - "docs/plans/issue-630-test-stateful.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/plans/issue-630-test-stateful.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_stateful slice — 設計

## Context

`issue-630-adapter-state-bridge` が production host bridge の
`save_state`/`restore_state` を実装済み（VST3: component
`getState`/`setState` + controller `setComponentState`/`setState`/
`getState` の `IBStream` 往復、CLAP: `clap.ext.state` の
plugin-defined byte stream）。しかし fixture 側は全 class が state
verb を正直に decline しており（VST3 `kNotImplemented`、CLAP
extension 未宣言 → `HostError::Unsupported`）、実 ABI の state 経路に
独立 oracle が存在しなかった。本 slice は state を持つ第 3 の
plugin class `test_stateful` を両 fixture cdylib に追加し、
save → mutate → restore → save round-trip を実 binary + L1
out-of-process runner で検証する。

## State blob law（両 format 共通 codec）

永続 state は gain parameter の値のみ。16 byte の決定的 blob:

```text
[u32 LE magic "RFST"][u32 LE version = 1][f64 LE value]
```

- **VST3**: component payload は **plain** gain、controller
  payload は **normalized** gain を保持（VST3 の component/
  controller 二分割に対応）。`setComponentState` は component
  blob（plain）を decode し normalized controller state へ変換。
  production bridge の restore sequence
  （`component.setState → component.getState →
  controller.setComponentState → controller.setState`）がそのまま
  動くことを前提に、両 payload は同一 codec で書かれる。
- **CLAP**: `clap.ext.state` は adapter framing を持たない単一
  stream — plain gain の 16 byte blob をそのまま write/read する。
- 検証: 長さ不一致・magic/version 不一致・非有限値は defined
  rejection。有限の range 外値は `ParamInfo` plain range に clamp
  （`text_to_value` と同じ law）。serialization は byte-exact
  deterministic（`f64::to_le_bits` 経由、NaN payload は decode 側で
  reject）。

## VST3 側構造

- `stateful.rs`（新規）: `SharedGain`（`Arc<AtomicU64>` — plain
  gain の f64 bits を保持する lock-free cell。process thread と
  state verb の双方から atomic load/store のみで共有）、blob
  codec、`IBStream` read/write helper、`TestStatefulProcessor`
  （`gain.rs` の `apply_gain` を `pub(crate)` 化して再利用 — DSP
  law の単一正本を維持）、`TestStatefulComponent`。
- `component.rs`: `ComponentStateSurface` trait（`set_state`/
  `get_state` の `IBStream` 面）を導入し `TestComponent` に
  `Option<Arc<dyn ComponentStateSurface>>` を追加。
  `with_state(...)` constructor で stateful class のみ surface を
  持つ。stateless class は `state_surface=None` → 従来どおり
  `kNotImplemented`（既存 `state_stream_reports_not_implemented`
  test がそのまま通る）。
- `controller.rs`: `ParamController` に stateful variant
  （`new_stateful`）。`setComponentState` は component blob（plain
  domain）を decode → normalized へ変換して mirror 更新、
  `setState`/`getState` は normalized blob を codec 経由で往復。
- `factory.rs`: `countClasses` 6→8、index 6/7 = stateful
  component/controller（fresh `uid`、既存 CID と非衝突）。
- `gain.rs`: `apply_gain` を `pub(crate)` に（DSP law 再利用）。

## CLAP 側構造

- `stateful.rs`（新規）: `TestStatefulShared`（`AtomicLifecycle` +
  `ParamMirror` — mirror が persisted state cell を兼ねる）、
  `TestStatefulMainThread`（ports + params + **`PluginStateImpl`**
  — `save`/`load` は mirror の plain gain を codec blob で
  `OutputStream`/`InputStream` に read/write）、
  `TestStatefulAudioProcessor`（`TestGainProcessor` 再利用、
  process は `plugin.rs` と同一の mirror sync-in/events/sync-out
  構造）、`TestStatefulClapPlugin`（descriptor
  `org.ruviform-test.clap-test-stateful`、
  `declare_extensions` に `PluginState` 追加登録）。
- `lib.rs`: `TestSuiteFactory` に 3 つ目の descriptor 追加、
  `plugin_count()=3`、`create_plugin` dispatch 追加。
- `Cargo.toml`: `clack-extensions` features に `"state"` 追加
  （plugin-side extension のみ、host 側 feature 不要）。

## Persistent-state law（activation 非 reset）

`test_gain` は activation で mirror を default に reset する
「deterministic per-activation law」を持つ。`test_stateful` は
これを**意図的に外す**: `activate_inner` は mirror を reset せず、
init default（1.0）または `load`/restore で書き込まれた値を保持
する。restore → activate → process の順で restored gain が DSP を
駆動することが round-trip の意味を持つための必須差分。

## RT boundary

state verb は VST3 では component/controller（non-RT 面）、CLAP
では main-thread `PluginStateImpl`（non-RT 面）にのみ存在し、
`process`/`IAudioProcessor::process`/`clap_process` の RT 経路は
一切変更しない。`SharedGain`/`ParamMirror` は atomic のみ、blob
codec は stack 上の固定長配列、state I/O は `IBStream`/
`InputStream`/`OutputStream` 経由の non-RT bounded I/O。`rt_`
`assert_no_alloc` test が process 経路の無 allocation を継続証明
する。

## L1 E2E oracle 設計上の確定事項

- **ordering gate = delivered audio**: L1 data plane は lookahead
  （2 block）+ bounded drain を持ち、`SaveState`/`RestoreState`
  は FIFO stdin loop の同期 command — しかし `process` submit は
  fire-and-forget で、state verb が mutation event block を
 追い越し得る。fixture の 3 gain（1.0/0.75/1.75）は constant
  input `0.5/-0.25` に対し pairwise-distinct な output
  （0.5/-0.25, 0.375/-0.1875, 0.875/-0.4375）を生むため、期待 gain
  の delivered output 観測 = それ以前の全 block（event block 含む）
  が child で処理済みの証明になる。各 state verb はこの gate の
  後にのみ送出する。
- **VST3 blob framing**: production bridge は
  `[u32 comp_len][comp][u32 ctrl_len][ctrl]` で frame する —
  test は 40 byte（8 overhead + 16 + 16）を assert し、component
  payload の plain gain を offset 12 の f64 で decode する。
- **CLAP blob**: framing なしの raw 16 byte stream — test は
  offset 8 の f64 を decode する。
- **malformed case**: truncated / trailing-bytes / corrupted magic
  / non-finite payload は全て `HostError::PluginFailed`（defined
  rejection）で、instance は生存し `SaveState` が有効 blob を
  返し続ける。

## Risks and Trade-offs

- **state verb と audio thread の race（CLAP）**: `load` が
  main-thread で mirror を書き換える間に進行中 block の sync-out
  が旧値を store し得る — param flush と同じ既知の CLAP thread
  model 性質で、fixture の test 経路（L1 は単一 FIFO、in-process
  test は逐次駆動）では決定的。VST3 側も同じ atomic cell 設計で
  整合。
- **VST3 controller domain 変換**: `setComponentState` の
  plain→normalized 変換は `ParamInfo::normalize` に委譲 —
  production bridge が送る順序（component payload を controller
  に渡す）と fixture の decode domain が一致することを in-process
  test で検証済み。

## Deferred Design Concerns

- `test_instrument` / CLAP `test_editor` class — 後続 slice
  （Issue #630 の残 acceptance）。
- 外部 SDK validator（VST3 SDK validator / CLAP validator）—
  Deferred 継続。
- preset/bank・GUI state・foreign format migration — 本 slice
  Non-Goal。

## Verification Notes

- [x] `mise run fmt-check` / `check` / `lint` / `test`（両 crate）
- [x] `mise run check-l1-real-plugin` / `lint-l1-real-plugin`
- [x] `mise run test-l1-real-plugin` — stateful e2e 2+2 case 実走
- [x] `mise run test-rt` — `rt_` filter 全 pass
- [x] `prek run --all-files` / forbidden-path diff check

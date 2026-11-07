---
id: "DISCOVERY-034T6RWRk7Pj3Qm9vN0xF4"
type: "discovery"
status: "proposed"
title: "Issue #387 Driver-neutral Component/View Specs + Headless Behavior + GPUI Renderer 棚卸"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/387"
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
    - "../specs/issue-384-driver-neutral-gui-architecture.md"
    - "../specs/issue-385-dtcg-resolved-theme.md"
    - "../specs/issue-386-driver-neutral-gui-host.md"
    - "../specs/issue-390-driver-contract-lab.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/issue-387-component-view-specs.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #387 棚卸: 既存資産と所有境界

Issue #387 (Driver-neutral Component/View Specs・Headless Behavior・
GPUI Controls/Viz Renderer) 着手前の棚卸。既存実装を未実装扱いに戻さず、
再利用可能な資産と本 Issue が所有する差分を切り分ける。

## 既存資産(変更しない基盤)

| 資産 | 所在 | 現状 | #387 での扱い |
| --- | --- | --- | --- |
| `WindowDriver`/Host 契約 | `crates/ruviform-ui/src/driver.rs` | RF386 所有。`WindowSpec`/`WindowLifecycle`/`DriverEvent`/`Capability`/`WindowDriver` + typed `UiError` | 消費のみ。Window 内に Component/View を mount するのは本 Issue の責務 |
| `DockWorkspace`/`ViewInstanceId` | `crates/ruviform-ui/src/dock.rs` | `ViewInstanceId(NonZeroU64)`、dock tree / move / layout restore | `ViewInstanceId` を View 同一性の正本として消費。dock ロジックは変更しない |
| `FakeDriver` | `crates/ruviform-ui-test/src/fake_driver.rs` | deterministic、`MAX_EVENT_BACKLOG`、close 流儀、Unsupported 型 | そのまま利用 |
| Driver Lab manifest | `crates/ruviform-ui-test/fixtures/driver-lab/{cases,stories}.json` | `UI-DVR-02`/`03` と `LAB-SV-FADER-DOCK`/`LAB-AL-WAVEFORM-SEEK` の fake/gpui binding が `blocked(#387)` | manifest は RF390 所有だが binding 更新は #387 の成果物として行う(validation schema には触れない) |
| `ResolvedTheme` | `crates/ruviform-ui-theme` | RF385 所有。DTCG 解釈→不変 snapshot。`Color`/`Length`/`ResolvedValue` | GPUI 側が一方向消費。第二 Theme 正本を作らない |
| `ruviform-viz` | `crates/ruviform-viz` | `LodBucket`/`ViewBin`/`WaveformSeries`/`WaveformSource`、bounded query、NaN/gap、f64 | 型と意味論を再利用。DSP/query/cancellation は consumer 側に残す |
| GPUI adapter | `crates/ruviform-ui-gpui` | `GpuiDriver` + 最小 `RvfRoot` fixture(title + 配信イベント数)。`#[cfg(feature = "gpui")]` 内に GPUI 型を封じ込め、`MAX_INBOX` bounded | ここに renderer を追加。`IntentSink` を再利用 |

## TypeScript 正本(移植元)

| ファイル | 所有する意味論 |
| --- | --- |
| `ui/core/src/interaction.ts` | `InteractionControl` gesture 機械: `Begin → Preview* → Commit|Cancel`、begin-while-active は `rejected`、terminal 後入力の `rejected(stale)`/unknown `noop`、detach/focus-lost→safe cancel、availability 遷移、seq dedup、atomic `set`/`adjust`/`reset` |
| `ui/core/src/value.ts` | `ValueDescriptor`/`ValueMapper`: normalized↔value、clamp、step quantize、out-of-range policy、text parse、`DisplaySpecial` |
| `ui/binding/src/contract.ts` | `CommandIntent`/`CommandReceipt`/`StateProjection`/`CommandPhase`、`IntentType(begin/preview/commit/cancel/action)`、preview coalescible |
| `ui/binding/src/binding.ts` | binding state 機械: minted command id、pending/ack、receipt rank(in-flight<accepted|queued<applied<terminal)、projection revision guard、detach→stale+generation bump |
| `ui/guitest/src/oracle.ts` | semantic oracle: 同値条件(同一 controlId で preview 不可分、terminal は別 partition、detach policy、stale/already-terminated) |

## 所有境界の確認

- #387 が所有: `ViewSpec`/`ComponentSpec` schema、`ComponentKey`、
  最小 Props/Variant/Visual State/Accessibility semantics、
  mount/reconcile/unmount の projection、`updated/removed/unmounted`/
  stale generation の定義、headless behavior projection、
  `ruviform-ui-gpui` 内の Controls/Viz renderer。
- 変更禁止(消費のみ): `driver.rs`(RF386)、`dock.rs`(RF386)、
  `ruviform-ui-theme`(RF385)、`driver_lab.rs` harness 本体(RF390)、
  NativeSlot/Session(RF388/389)。
- `cases.json`/`stories.json` の binding 行は case 成果物の所有物
  (RF390 契約に沿った編集。`rerun`/`owner`/`mechanism` 語彙を守る)。

## 観測されたギャップ(#387 で埋める差分)

1. Component/View の中立 schema が無い — `RvfRoot` は固定 fixture。
2. Gesture/value 意味論が Rust に無い — TS canonical の port が必要。
3. `UI-DVR-02`/`03` 両 story の fake binding が `blocked`。
4. Theme → renderer style の写像器が無い — canonical semantic token
   名を #387 が定義する必要がある(Synvelis DTCG `color.*` と
   AudioLens TS `surface.*` は語彙が異なり、共通名を中立層で定める)。
5. Viz の描画経路が無い — resolved bins (`ViewBin`/`LodBucket`) を
   spec の data payload として受け取り、query/LOD は consumer 側に
   残す設計とする(契約層に port trait を置かない)。

## 製品テーマ観測(検証 fixture 用)

- Synvelis DTCG dark: `color.bg #1b1d21`/`color.fg #e8e8ea`/
  `color.accent #6aa9e8`/`color.err #e86a6a`/`color.border #33373f`。
- AudioLens TS `ThemeInput`: `surface #161b20`/`interactive #45b8b4`/
  `fader.track #39424d`/`viz.series.1 #4fd1a5`/`viz.waveform.fill`/
  `seekbar.track`。語彙が異なるため、renderer 消費名は
  中立 semantic token 名(`surface`/`interactive`/`fader.track`/
  `viz.*` 系、AudioLens 語彙を底本に採用)で fixture DTCG を作る。

## 未確定事項と判断

| 論点 | 判断 |
| --- | --- |
| 契約配置 | `ruviform-ui` 内 `view`/`behavior` module(新 crate は作らない。#384 設計表の「共通契約層」配置) |
| `ruviform-ui` → `ruviform-viz` 依存 | 追加しない。`ruviform-ui` は「Primitives のみ依存」宣言を持つため、resolved payload は `WaveformBin`(`ViewBin`/`LodBucket` 同一 field 意味)として契約層で規定。viz→contract 変換は adapter/consumer 責務 |
| `ViewSpec` mount 経路 | `post_event`/`WindowDriver` には触れず、driver crate の公開 API(`GpuiDriver`/`RvfView` 構成)で consumer が渡す。composition root の責務 |
| GPUI headless | gpui 0.2.2 は `TestAppContext::add_window_view` を持つ。gpui layer semantic 試験を headless で試行、だめなら `not_run` + 実機 lane |

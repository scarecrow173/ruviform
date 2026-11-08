---
id: "SPEC-034Uc80WBRvRmOb7dt2yeU"
type: "spec"
status: "approved"
title: "Issue #464 — Consumer-neutral GUI Scenario Contract"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/429"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/fake.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
    - "docs/specs/issue-429-consumer-observation-seam.md"
    - "docs/designs/issue-429-consumer-observation-seam.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
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

# Issue #464 — Consumer-neutral GUI Scenario Contract

## What Should Be Built

`ruviform-ui-test` に、Driver-neutral な薄い Scenario Contract を追加する。
通常の Rust 関数として記述する Scenario/Case ID・Target・Capability 要件・
操作/観測/Result の最小契約であり、JSON DSL・独自 Test Executor・
新しい GUI フレームワーク・二重 Runner は作らない。

契約の中核は `SemanticGuiTest` 相当の薄い trait で、既存の
`ControlInput`・`ViewHost`・`ComponentSnapshot`・`CommandIntent`/
`CommandReceipt`/`CommandPhase`・`WindowId`/`ViewInstanceId`/
`ComponentKey`/`AttachEpoch`/`generation` をそのまま再利用する。
`begin`/`preview`/`commit` 等のヘルパーは既存 enum のヘルパーに留め、
第二の Gesture/state 正本を作らない。

Fake 側（`ViewHost` を window に束ねた adapter）と GPUI 側
（`GpuiObserver` を包む adapter）が同一契約を実装し、consumer の
`#[test]` composition root が同じ Scenario 関数を両 adapter に対して
実行できる。共通 Scenario API は `ruviform-ui` の契約型のみで構成し、
GPUI 型・`gpui::TestAppContext` 等を露出しない。
`ruviform-ui-test -> ruviform-ui-gpui` の依存逆転は禁止。

## Why Now

#390 が Driver Contract/Fake・driver-lab registry・boundary guard を
完了し、#429/#436/#442 が `GpuiObserver` の観測 seam を公開した。
Synvelis(#114)・AudioLens(#166) の両 Consumer が製品 GUI テストを
共通の方法で記述・実行するには、driver 別の呼び出しを吸収する
薄い契約層が必要である。下流の #426（Case/Story binding）・
#313（実 OS 入力）・#314（pixel Golden）は本契約の能力分離を前提に
独立して進む。

## Users and Value

- Synvelis / AudioLens のテスト作者: 製品シナリオを 1 度書けば
  Fake と GPUI simulated の両方で実行でき、証拠レベルを混同しない。
- Ruviform メンテナ: scenario 契約の変更点が 1 箇所に集約され、
  driver ごとの追従が adapter 実装に限定される。

## Scope

### In Scope

- `ruviform-ui-test` への Scenario/Case ID・`TargetQuery`/
  `ScenarioTarget`・`ScenarioCapability` 集合・`GuiAction`・
  `MintedIntent`・`ScenarioError`・`ScenarioVerdict`/`ScenarioReport`・
  `SemanticGuiTest` trait・Fake adapter（`ViewHost` map）の追加。
- `ruviform-ui-gpui` への `GpuiObserver` adapter（`gpui-test` feature
  限定）の追加と、target 解決に必要な最小 seam
  （`ViewObservation` への view identity 追加）。
- `ruviform-ui` への不可避な最小追加（`ViewHost::attach_epoch()`
  getter・`ViewHost::deliver_event(&Event)` による wire→view decode の
  一本化・`AttachEpoch` の serde derive）— PR 本文に根拠を明記する。
- Fake/GPUI で同一 Scenario 関数が実行できることを示す共有
  parity scenario と、それを検証する両側テスト、mise lane、
  docs chain（spec/design/plan/task/test-spec）。

### Out of Scope

- 新しい GUI フレームワーク・DSL・二重 Runner・JSON Scenario 定義・
  独自 Test Executor。
- `crates/ruviform-ui-webview/**`、`ui/**`、`poc/**`。
- 実 OS 入力/Focus/IME/window/DPI/a11y（#313/#243）、pixel
  screenshot/Golden（#314）、実 Plugin Native Dock（#388/#389 +
  Synvelis #176）、driver-lab Case/Story 拡張（#426）、cross-repo
  evidence/CI（#294/#219）、consumer 側 fixture/oracle の Ruviform への
  持ち込み。
- GPUI 型の共通 API への露出、`Send` 化や `!Send` context の thread
  越境。

## Requirements

- Scenario/Case ID・run ID・layer/driver/capability・tested SHAs・
  環境情報・target/generation・expected/observed/receipt・
  timeout/cleanup・evidence refs・verdict を記録する report 型を提供
  する。verdict 語彙は `pass`/`fail`/`not_run`/`unsupported`/`blocked`
  を維持し、`blocked` は consumer-verify lane では既存どおり
  `not_run` + 理由へ写像する。
- Target は `WindowId + ViewInstanceId + ComponentKey` の論理 triple
  で表し、解決時に `AttachEpoch`・`generation`・component mounted を
  pin する。操作・観測のたびに照合し、stale key/epoch/generation は
  明示的な `ScenarioError` とする。表示文言・絶対 pixel 座標を
  stable identity に使わない。
- Semantic port は `ControlInput` dispatch・`select_row`/
  `select_tab`/`invoke` 相当・`observe`（`ComponentSnapshot`
  read-only）・`settle`（bounded queue/settle）・intent drain・
  receipt/projection の `post_event` 投入を再利用する。観測は
  read-only で、製品 service への書き込み・認可迂回を行わない。
- Capability は `SemanticAction`/`SimulatedPointerOrKeyboard`/
  `RealOsPointerOrKeyboard`/`Ime`/`VisualCapture`/`MultiWindow`/
  `NativePluginDock` の区分で宣言し、実行前に照合する。Fake の pass
  を simulated-input・real-OS・draw-plan・pixel-Golden の証拠へ昇格
  しない。GPUI `dispatch_input` の直接注入は hit test・実 pointer
  入力の証拠ではない。
- 待機は bounded queue/settle と receipt/revision 条件で行い、
  一律 sleep・永続的 test-only 制御口・OS 時計/GPU の完全決定論の
  主張をしない。`TestDriver` の process/timeout/cleanup を再利用する。

## Acceptance Criteria

- [ ] `ruviform-ui-test` に Scenario/Case ID・Target・Capability・
  Result の最小契約と `SemanticGuiTest` 相当 trait が存在し、
  JSON DSL/独自 executor を含まない。
- [ ] Fake adapter と GPUI adapter が同一契約を実装し、共通 Scenario
  関数が両者で実行できることを示すテストがある。
- [ ] stale key/epoch/generation が明示 Error になることを検証する
  テストがある。
- [ ] 共通 API の public signature に GPUI 型が現れず、boundary
  guard（`check-ui-driver-boundary`）が pass する。
- [ ] `SemanticAction` 以外の capability を要求する scenario が
  pre-flight で `unsupported` 判定され、Fake pass が他証拠レベルへ
  昇格しない。
- [ ] `pass`/`fail`/`not_run`/`unsupported`/`blocked` の verdict
  語彙と report フィールドが Issue #464 の要求を満たす。

## Deferred Design Concerns

- keyboard/IME・pointer 座標・pixel capture を含む拡張 action:
  #313/#314 の Owner 領域。本契約の `GuiAction` は `#[non_exhaustive]`
  のため後続で variant を追加できる。
- consumer 側 scenario/oracle の採用証拠（Step C）: Synvelis #114・
  AudioLens #166 の各 Issue で実施。本 Issue では Ruviform 側の
  契約と adapter のみを提供する。

## Sources and Relations

- Issue #464 本文（受入条件 1–7・非対象 Owner 表・擬似 API）。
- `docs/specs/issue-390-driver-contract-lab.md`・
  `docs/designs/issue-390-driver-contract-lab.md`: verdict 語彙・
  layer 語彙・manifest/evidence 規約の正本。
- `docs/specs/issue-429-consumer-observation-seam.md`・
  `docs/designs/issue-429-consumer-observation-seam.md`:
  `GpuiObserver` seam の境界（GPUI 型を出さない・`gpui-test` feature）。
- `docs/test-specs/issue-390-driver-boundary-lab.md`: boundary
  guard の検査契約。

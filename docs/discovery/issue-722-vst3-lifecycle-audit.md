---
id: "DSC-722vst3LifecycleAudit01"
type: "discovery"
status: "final"
title: "Issue #722 production VST3 adapter lifecycle 監査 — test fixture 同型欠陥の診断"
created: "2026-11-19"
updated: "2026-11-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/722"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "evidence/rf630/abi-validator-verification.md"
    - "docs/specs/issue-630-abi-validator.md"
  related:
    - "../specs/issue-722-vst3-lifecycle-audit.md"
---

# Issue #722 production VST3 adapter lifecycle 監査

Issue #630 の外部 ABI validator（Steinberg VST3 SDK 3.8.1 validator）が
test fixture `ruviform-plugin-test-vst3` で検出した lifecycle 契約違反と
同型の欠陥が、production adapter `ruviform-plugin-vst3` に存在するかを
監査した記録。fixture 側は PR #723（commit `f89c5ad`）で修正済みであり、
本 slice はその修正パターンを正本として production 側を照合する。

## 監査対象と一次情報

- 対象: `crates/ruviform-plugin-vst3/src/component.rs`（`IPluginBase` /
  `IComponent` / `IAudioProcessor` 実装）・`src/controller.rs`
  （`IEditController` 実装・監査のみ）
- 正本パターン: `crates/ruviform-plugin-test-vst3/src/component.rs`
  （read-only 参照 — `terminate` 非終端化・`setupProcessing` 受理拡大済み）
- neutral lifecycle: `crates/ruviform-plugin-core/src/lifecycle.rs`
  （read-only — `Destroy` → `Destroyed` は終端。CLAP `destroy` 用の契約
  であり本 slice では変更しない）
- VST3 SDK 契約の一次情報: VST3 SDK 3.8.1 同梱ソース
  （commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`）
  - `IComponent::setupProcessing` のドキュメントは
    `[UI-thread & (Initialized | Connected)]` — つまり無効化状態
    （`Initialized`、実効的に `Connected` も含む disabled 側）での
    呼び出しが合法
  - `TerminateInitializeTest` は同一 COM オブジェクト上で
    `terminate()` → `initialize()` を再実行する — VST3 の `terminate`
    は COM 破棄ではなく非終端
  - `SuspendResumeTest` は `setActive(true)` の前に
    `setupProcessing` を呼ぶ — 実ホストが `Initialized` 状態で
    `setupProcessing` を呼び得ることの SDK 内実例

## 欠陥 1: `terminate` の終端遷移 — 同型欠陥あり（修正対象）

### production 現行コード（修正前）

`component.rs` の `terminate` は `LifecycleEvent::Destroy` を遷移する
だけで、成功時に `kResultOk` を返していた。neutral lifecycle の
`Destroy` は `Created`/`Initialized` → `Destroyed`（終端）であり、
`Destroyed` は以後の全イベントを拒否する。

結果として VST3 `terminate` 成功後に同一 COM オブジェクトで
`initialize` を呼ぶと `kResultFalse` となり、SDK の terminf suite
（`terminate` → 再 `initialize`）に違反する。fixture が #630 で
検出・修正した欠陥と同型。

### fixture 側の正本パターン（`ruviform-plugin-test-vst3`）

`transition(Destroy)` 成功後に adapter 層で
`state.lifecycle = Lifecycle::new()` + `state.config = None` とし、
lifecycle を `Created` に戻す。core の `Destroy` 終端契約は維持したまま
adapter 層でリセットする方針 — production も同じ層・同じ形が整合。

## 欠陥 2: `setupProcessing` の受理 state 過狭 — 同型欠陥あり（修正対象）

### production 現行コード（修正前）

`IAudioProcessor::setupProcessing` は
`state.lifecycle.state() != LifecycleState::Activated` を `kResultFalse`
で拒否していた。SDK 文書の受理 state は `(Initialized | Connected)`
（disabled 側）であり、`Initialized` での呼び出しを誤って拒否する。
`Activated` のみ受理は SDK の `SuspendResumeTest` が行う
`setActive(true)` 前の `setupProcessing` を拒否し、かつ処理中
（`Processing`）の renegotiation も SDK 契約上は処理停止後に許容される
範囲を狭めていた。

### fixture 側の正本パターン

`Initialized | Activated` を受理し、`Created`・`Processing`・終端状態を
拒否する。production も同じ受理集合に合わせる。

## 欠陥 3: bus info 不一致 — 同型欠陥なし（結論）

fixture 側で検出された bus info 不一致は「報告する bus 構成と実
component の不一致」型の欠陥。production adapter は stereo 固定で
自己整合している:

- `setBusArrangements` は `kStereo` のみ受理（`component.rs`）
- `getBusInfo` は `channelCount = 2` を報告
- `getBusArrangement` は `kStereo` を報告

報告値と実処理経路（stereo interleaved buffer 処理）が一致しており、
fixture 型の「報告 ≠ 実装」の不一致は確認されなかった。なお
`evidence/rf630/abi-validator-verification.md` が記録する
「controller.rs:72 の terminate が `Destroy` を遷移」という記述は
現行コードと食い違う — 実測では `GainController::terminate` は
無条件 `kResultOk` を返し neutral lifecycle を持たない
（test fixture 側 `controller.rs` と同じ trivial 形）。本監査での
実測を正とする。

## `controller.rs` の監査結論

`GainController::terminate` は状態機械を持たず無条件 `kResultOk`。
VST3 `IEditController::terminate` も非終端だが、controller は
`initialize`/`setActive`/`setProcessing` 相当の状態を保持しないため
reset 対象がなく、現行の trivial 実装で契約を満たす。変更不要。

## `host.rs` の監査結論（変更なし）

host bridge の teardown 順序は `setProcessing(false)` →
`setActive(false)` → `terminate` であり、terminate が非終端化されても
`config = None` により `setProcessing(true)` は再 setup なしに
再開できない。既存順序は新契約下でも有効で機能変更は不要
（doc comment の記述は fixture 側の旧契約を参照しており既に陳腐化
しているが、挙動に影響しないため本 slice では触らない）。

## 結論

| 欠陥 | production での有無 | 対応 |
| --- | --- | --- |
| terminate 終端遷移 | あり | adapter 層で lifecycle/config をリセット |
| setupProcessing 受理 state 過狭 | あり | `Initialized \| Activated` に拡大 |
| bus info 不一致 | なし | 変更不要（根拠は上記） |

修正は fixture と同じ adapter 層リセット方針に揃え、core の
`Destroy` 終端契約は維持する。

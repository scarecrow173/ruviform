# Issue #722 — production VST3 adapter lifecycle 監査報告

- 日付: 2026-11-19
- ブランチ: `fix/722-vst3-lifecycle-audit`
- 対象: `crates/ruviform-plugin-vst3`（production adapter）
- 正本パターン: `crates/ruviform-plugin-test-vst3`（fixture、#630 で修正済み）
- SDK 一次情報: VST3 SDK 3.8.1（source commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`）

## 監査結果サマリ

| # | fixture で検出された欠陥 | production での有無 | 対応 |
| --- | --- | --- | --- |
| 1 | `terminate` の終端遷移（`Destroy` → `Destroyed` で再 initialize 不可） | **あり** | adapter 層で lifecycle/config をリセット（`component.rs:513-525`） |
| 2 | `setupProcessing` の受理 state 過狭（`Activated` のみ） | **あり** | `Initialized \| Activated` に拡大（`component.rs:731-736`） |
| 3 | bus info 不一致（報告 ≠ 実 component） | **なし** | 変更不要（根拠は下記） |

## 欠陥 1: terminate 終端遷移 — あり・修正済み

### 修正前の実コード

`component.rs` の `terminate`（旧 :507-515）は
`state.lifecycle.transition(LifecycleEvent::Destroy)` の成功のみで
`kResultOk` を返していた。`ruviform-plugin-core` の neutral lifecycle
では `Destroy` は `Created`/`Initialized` → `Destroyed`（終端）で、
`Destroyed` は以後の全イベントを拒否する。したがって terminate 成功後
の `initialize` は `kResultFalse` となり、SDK `TerminateInitializeTest`
（同一 COM オブジェクトで `terminate()` → `initialize()`）に違反する。

TDD red で実測確認: `terminate_allows_reinitialize` が
2 回目の `initialize` で `kResultFalse`（期待 `kResultOk`）を返して
失敗（exit 101）。

### 修正内容

fixture（`ruviform-plugin-test-vst3/src/component.rs`:477-489 付近）と
同一形に揃えた:

```rust
match state.lifecycle.transition(LifecycleEvent::Destroy) {
    Ok(_) => {
        state.lifecycle = Lifecycle::new();
        state.config = None;
        kResultOk
    }
    Err(_) => kResultFalse,
}
```

- `Destroy` 遷移はバリデーションとして残す（`Activated`/`Processing`
  からの terminate は neutral 遷移表で illegal → `kResultFalse` を維持）
- 成功時は `Lifecycle::new()` で `Created` に戻し、`config = None` で
  negotiated processing config を破棄 → terminate 後の
  `setProcessing(true)` は再 `setupProcessing` まで拒否される
- core の `Destroy` 終端契約は変更していない（CLAP `destroy` 用に維持）

## 欠陥 2: setupProcessing 受理 state — あり・修正済み

### 修正前の実コード

`IAudioProcessor::setupProcessing`（旧 :716 付近）は
`state.lifecycle.state() != LifecycleState::Activated` を
`kResultFalse` で拒否していた。

SDK 契約との照合:

- `IComponent::setupProcessing` の SDK ドキュメントは
  `[UI-thread & (Initialized | Connected)]` — disabled 側 state での
  呼び出しが合法
- `SuspendResumeTest` は `setActive(true)` の前に
  `setupProcessing` を呼ぶ（SDK suite 内の実例）
- `Processing` 中の renegotiation は SDK 契約上も不適切
  （処理中の再 `prepare` は不可）

TDD red で実測確認: `setup_processing_accepted_while_initialized` が
`Initialized` での `setupProcessing` に `kResultFalse`（期待
`kResultOk`）を返して失敗。

### 修正内容

```rust
if !matches!(
    state.lifecycle.state(),
    LifecycleState::Initialized | LifecycleState::Activated
) {
    return kResultFalse;
}
```

- `Initialized`（`setActive(true)` 前の SDK 合法呼び出し）と
  `Activated`（処理停止後の renegotiation）の両方で受理
- `Created`・`Processing`・その他は `kResultFalse` を維持
- `kSample64`・`maxSamplesPerBlock` 範囲（≤0 または
  `MAX_NEGOTIATED_FRAMES = 1<<20` 超過）・sample rate・`prepare`
  失敗の拒否と、成功時の scratch 確保・`ProcessConfig` 保存は不変

## 欠陥 3: bus info 不一致 — なし

fixture 型の「報告 bus 構成 ≠ 実 component」不一致は production では
確認されなかった。stereo 固定で自己整合:

- `setBusArrangements` は `kStereo` のみ受理
- `getBusInfo` は `channelCount = 2` を報告
- `getBusArrangement` は `kStereo` を報告
- `process` は stereo interleaved buffer を処理 — 報告と実装が一致

## `controller.rs` の監査（変更不要）

`GainController::terminate`（`controller.rs:72-74`）は状態機械を持たず
無条件 `kResultOk` を返す。`IEditController` は `initialize`/
`setActive`/`setProcessing` 相当の状態を保持しないため reset 対象がなく、
VST3 `IEditController::terminate` の非終端性に対し現行の trivial 実装で
契約を満たす。test fixture 側 `controller.rs` と同じ形。

補足: `evidence/rf630/abi-validator-verification.md` が記録する
「controller.rs:72 の terminate が `Destroy` を遷移」という記述は
現行コード（無条件 `kResultOk`・`Destroy` 遷移なし）と食い違う。
本監査の実測を正とする — おそらく fixture 側 controller を指す記述の
混同と思われる。

## `host.rs` の監査（変更不要）

teardown 順序は `setProcessing(false)` → `setActive(false)` →
`terminate`。terminate 非終端化後も `config = None` により
`setProcessing(true)` は再 `setupProcessing` なしに拒否されるため、
bridge が terminate 後に processing を再開する経路は存在せず、
既存順序は新契約下でも有効。doc comment が fixture の旧契約
（setupProcessing が Activated 必須）を参照している点は陳腐化して
いるが挙動に影響しないため本 slice では未修正（編集範囲外）。

## 修正ファイル

- `crates/ruviform-plugin-vst3/src/component.rs`
  - `terminate`（:513-525）非終端化
  - `setupProcessing`（:721-736 付近）受理 state 拡大
  - lifecycle mapping doc table（:10-17）と doc comment 同期
  - unit test 2 件追加（`terminate_allows_reinitialize`・
    `setup_processing_accepted_while_initialized`）
- `ci/plugin-abi-validator/run.mjs`
  - `--vst3-package` オプション追加・`--vst3-dll` stem からの
    package/bundle 名導出・leg ラベルと artifacts 記録
  - 既定動作（fixture bundle）は不変

## 検証結果（詳細は docs/impl/ir/issue-722-vst3-lifecycle-audit.md）

環境: Windows (win32/x64)、rustc 1.99.1 (58cf8f9c3 2026-10-13)、
Node v26.12.1、VST3 SDK 3.8.1 validator
（`%LOCALAPPDATA%/ruviform/plugin-abi-validator/vst3sdk-build/bin/Release/validator.exe`）、
clap-validator 0.4.1。HEAD: `7124f0b24915ce18a139d71267582c54da7d903a`

- TDD red: 修正前コードで新規 2 テストが期待どおり失敗（exit 101）
- `mise run test ruviform-plugin-vst3`: 44 tests pass（exit 0）
- `mise run fmt-check` / `check` / `lint`: exit 0
- `mise run test-l1-real-plugin`: exit 0（1 回目は scope 外の
  CLAP fixture flaky で exit 101、再実行で pass — 詳細は impl/ir）
- `node --test ci/plugin-abi-validator/tests/`: 18 tests pass（exit 0）
- `mise run test-plugin-abi-validator`（引数なし）: verdict NotRun・
  exit 1 — validator パスを環境変数/引数で渡さない既定起動は
  fail-closed で NotRun を返す設計どおりの挙動（rf630 と同じ起動
  方法で実走は下記のとおり実施）
- ABI validator fixture 回帰 lane（既定対象・明示 validator パス指定）:
  verdict **Passed**・exit 0 — CLAP 5 legs Passed + VST3 fixture
  235/235。証拠: `evidence/rf722/abi-validator-fixture/`
- ABI validator **production** lane:
  `node ci/plugin-abi-validator/run.mjs --vst3-dll
  target/debug/ruviform_plugin_vst3.dll` → bundle
  `ruviform-plugin-vst3.vst3`（package 名は dll stem から導出）を
  stage し SDK validator を実走。**47 passed, 0 failed・verdict
  Passed・exit 0**。`[Terminate/Initialize]`・`[Suspend/Resume]`・
  `[Valid State Transition]`・`[Bus Consistency]` 等の lifecycle 系
  suite が全て `[Succeeded]`。証拠: `evidence/rf722/abi-validator/`
  （`run.json`・`vst3-validator.txt`・clap legs 生出力）

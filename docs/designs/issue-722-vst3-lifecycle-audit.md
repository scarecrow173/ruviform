---
id: "DESIGN-722vst3LifecycleAudit03"
type: "design"
status: "approved"
title: "Issue #722 production VST3 adapter lifecycle 契約監査 — 設計"
created: "2026-11-19"
updated: "2026-11-19"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/discovery/issue-722-vst3-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-722-vst3-lifecycle-audit.md"
  implements:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/discovery/issue-722-vst3-lifecycle-audit.md"
  blocks:
    - "docs/plans/issue-722-vst3-lifecycle-audit.md"
  supersedes: []
---

# Issue #722 設計 — production VST3 adapter lifecycle 契約監査

## 設計方針

fixture（`ruviform-plugin-test-vst3`）の #630 修正を正本パターンとし、
production adapter に同じ adapter 層リセット方針を適用する。
neutral lifecycle（`ruviform-plugin-core`）は CLAP `destroy` の終端
契約を持つため変更せず、VST3 `terminate` の非終端性は adapter 層で
吸収する — 両 adapter で同一の責務分担になる。

## 変更点

### 1. `component.rs` — `terminate`（IPluginBase）

現行: `LifecycleEvent::Destroy` 遷移成功で `kResultOk`（→ `Destroyed`
終端、再 `initialize` 不可）。

修正後: `transition(Destroy)` をバリデーションとして実行し、成功時は
`state.lifecycle = Lifecycle::new()`（`Created` へ）と
`state.config = None`（negotiated processing config 破棄 — これにより
`setProcessing(true)` は再 `setupProcessing` まで拒否される）を行い
`kResultOk` を返す。`Destroyed` には留まらない。fixture の
`terminate` と同一形。

拒否条件は不変: `try_borrow_mut` 失敗（reentrancy）・`Activated`/
`Processing` からの `Destroy`（neutral 遷移表で illegal）は
`kResultFalse`。

### 2. `component.rs` — `setupProcessing`（IAudioProcessor）

現行: `LifecycleState::Activated` のみ受理。

修正後: `matches!(state, Initialized | Activated)` で受理。
`Created`・`Processing`・その他の状態は `kResultFalse`。
`kSample64`・`maxSamplesPerBlock` 範囲（≤0 または
`MAX_NEGOTIATED_FRAMES = 1<<20` 超過）・sample rate・`prepare` 失敗の
拒否ロジックは不変。成功時の scratch 再確保と `ProcessConfig` 保存も
不変 — `Initialized` で setup しても `setActive(true)` →
`setProcessing(true)` の既存経路でそのまま処理に入れる
（`setProcessing` の state ガードは `Activated` のみで、`config`
有無も要求するため矛盾しない）。

### 3. doc table / doc comment の同期

component.rs 冒頭の lifecycle mapping table と `terminate`/
`setupProcessing` の doc comment を新契約に合わせて更新する
（VST3 `terminate` 非終端・`setupProcessing` の受理 state）。

### 4. `controller.rs` — 変更なし

`GainController` は neutral lifecycle を持たず `terminate` は無条件
`kResultOk`。VST3 `IEditController::terminate` の非終端性に対し
reset 対象の状態が存在しないため現行実装で契約を満たす
（discovery の監査結論どおり）。

### 5. `host.rs` — 変更なし

teardown 順序 `setProcessing(false)` → `setActive(false)` →
`terminate` は新契約下でも合法かつ安全。`config = None` により
terminate 後の `setProcessing(true)` は拒否されるため、bridge が
terminate 後に processing を再開する経路はない。doc comment の
陳腐化は挙動に影響しないため本 slice では修正しない
（範囲は component.rs/controller.rs/unit test に限定）。

### 6. `ci/plugin-abi-validator/run.mjs` — production bundle 対象化

最小汎用化:

- `--vst3-package <name>` オプション追加。未指定時は `--vst3-dll`
  のファイル stem から導出（`ruviform_plugin_vst3.dll` →
  `ruviform-plugin-vst3`）。既定の fixture dll からは従来どおり
  `ruviform-plugin-test-vst3` が導出され、既定動作は不変。
- `stageVst3Bundle` に bundle 名引数を追加（既定は従来の fixture 名で
  後方互換）。VST3 bundle layout は inner binary 名 = bundle 名を
  要求するため、package 名 + `.vst3` を bundle 名とする。
- `buildFixtureArtifacts` は CLAP fixture と導出された VST3 package
  を build 対象にする（`--vst3-dll` が fixture なら従来と同一の
  build 対象）。
- leg の plugin ラベル・run.json の `artifacts` に導出 package/
  bundle を記録し、証拠の一意性を保つ。
- fail-closed 契約・NotRun 判定・verdict 評価は不変。
  `node --test ci/plugin-abi-validator/tests/` の契約テストは
  既存シグネチャを維持して pass すること。

## 却下した代替案

- **core の `Destroy` を非終端化**: CLAP `destroy` の契約を壊し
  CLAP adapter に影響する。fixture と同じく adapter 層リセットが正しい
  責務分担。
- **`setupProcessing` を全状態で受理**: `Processing` 中の renegotiation
  は SDK 契約上も不適切（処理中の再 prepare は不可）であり、
  `Created` では `ProcessSetup` の前提（`initialize` 済みの
  bus/sample-rate 構成）が成立しない。`Initialized | Activated` が
  SDK 文書と一致する最小の受理集合。
- **host 側で terminate 後の再 initialize を肩代わり**: VST3 COM
  オブジェクトの lifecycle は adapter が所有すべき責務であり、
  host bridge に契約補正を持ち込まない。
- **run.mjs に production leg を別建て**: 単一 VST3 leg の対象を
  引数で切り替える方が小さく、fixture 既定動作を変えない。
  leg 追加は CLAP legs との重複実行を生むだけで利益がない。

## リスク

- `Initialized` での `setupProcessing` 成功後、host が `setActive`
  せず `terminate` した場合: `config` は terminate で破棄されるため
  安全。`Activated` で setup → `setProcessing(true)` の既存経路も
  `config` ガードで保護される。
- `setupProcessing` を `Initialized` で呼ぶ host が `ProcessSetup`
  に無効な bus 構成を渡す場合: 既存の sample size/frame/sample rate
  検証がそのまま適用される。
- validator が production bundle を拒否する可能性: bundle layout・
  `GetPluginFactory`/`IPluginFactory` の class 登録は fixture と同一
  機構（`ruviform-plugin-export`）のため差異はない見込み。実走で
  確認する。

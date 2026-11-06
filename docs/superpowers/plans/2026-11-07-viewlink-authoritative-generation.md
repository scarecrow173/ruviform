# ViewLink authoritative generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** backendのattach応答generationをViewLinkへ採用し、Synvelisのremount後の操作不能を解消する。

**Architecture:** epochの正本はbackend。Ruviformはoptionalなauthoritative generationをlink-managed binding/query/subscriptionへ伝え、Synvelisはtoken検証後のattach応答値を渡す。SDK修復を先に統合しconsumerへ取り込む。

**Tech Stack:** TypeScript / Vitest、Rust UiRuntime、mise、Orca。

**Spec:** ../specs/2026-11-07-viewlink-authoritative-generation-design.md （2026-11-07ユーザー承認済み）

## Global Constraints

- 新規依存、backend stale guard緩和、consumer counterの複製を行わない。
- 検証はmise runの正規タスク。native実機laneは実行証拠がなければNotRun。
- RuviformとSynvelisは別repoの変更。各AGENTSとworkspace worker-guideに従う。
- spec→design→plan→taskのdoc graphを整備し、内部承認gateは独立review-protocolで判定する。

## Review Focus

- 同window・同epochのattachが購読を増やさない。
- 同window・異epochが旧gesture/query/subscriptionを失効させる。
- 不正epochでlink状態を部分変更しない。
- 古いattach応答が有効なepochを巻き戻さない。
- legacy attachとlate-created bindingの既存契約を保つ。

### Task 1: Ruviform SDK epoch採用

**Files:** ui/binding/src/link.ts、ui/binding/test/link.test.ts、必要な契約文書・doc graph。
**Interface:** ViewLink.attach(windowId: string, generation?: number): void。

- [ ] 既存ViewLink、CommandBinding、QueryBinding、subscription lifecycleとdoc graphを確認する。
- [ ] link.test.tsに、fresh linkへgeneration=7を採用→既存/後発bindingのwire generationとsubscription contextが7、detach/reattachでgeneration=11を採用する恒久テストを書き、mise run test-ui-bindingで期待するREDを確認する。
- [ ] 同window同epochの冪等性、異epoch時の旧subscription/gesture/query失効、無効epoch時の原子性、local fallbackを実契約でテストする。
- [ ] optional generationを採用し、同window異epochでは必要なlifecycle遷移を行う。既存callerのlocal incrementは維持する。
- [ ] mise run check-ui-binding / test-ui-binding / build-ui-binding、および影響するguitest等の既存gateを実行する。意味のある影響範囲に限定して結果を保存する。
- [ ] 最新mainを取り込み、生成物/doc graphを整合、論理単位のcommitを作る。
- [ ] 別担当の独立Reviewer A/B→Validator→Judgeで確認し、必要な指摘を修正する。実装者自身の自査を代用しない。
- [ ] 自branchへpush、PR作成、候補SHAのmanual Full CIをdispatchして実結果を報告する。merge/Issue closeはcoordinatorが担当する。

### Task 2: Synvelis consumer取り込み（Task 1統合後、別dispatch）

**Files:** ui/src/viewmodel/console-view-model.ts、ui/test/console.test.tsx、ui/test/binding.test.ts、Cargo.lock、既存SDK生成物/IR/task。
**Interface:** token検証後にlink.attach(windowId, res.generation)を呼ぶ。

- [ ] 旧SDKで同viewId remountのattach reply=2に対するwire ctx.generation不一致を恒久テストRED化する。
- [ ] Ruviform統合mainを既存branch-following依存で採用し、lock/vendor/manifestを同期する。
- [ ] attach応答のgenerationを渡す。pending attach破棄後の新attach、古い応答の逆順到着、remount後のdispatchとreceiptを確認する。
- [ ] SDK値を反映するだけのfake期待値を避け、backend live epochとdispatchの不一致をdropする契約も検証する。
- [ ] mise gate、候補head manual Full CI、独立review-protocolを通し、PR171を更新する。native NotRunは維持する。

### Task 3: coordinator統合

- [ ] RF PR/head/review/CIを照合して統合し、Task 2をdispatchする。
- [ ] SV候補headの結果でP1が解消したことを判定する。native正式受入は別gateとして扱う。
- [ ] 証跡保全、worker release、Issue/PR状態確認、統合済み資源のcloseout、state/archive同期を行う。

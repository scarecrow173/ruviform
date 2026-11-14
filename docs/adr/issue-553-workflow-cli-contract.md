---
id: "ADR-JwwKl26JnYjSWNJAuhGJs5"
type: "adr"
status: "accepted"
title: "Consumer-neutral Workflow/CLI orchestration契約の採否（Issue #553 RF-AUTX-01）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  changes:
    added:
      - type: "doc"
        path: "docs/adr/issue-553-workflow-cli-contract.md"
      - type: "doc"
        path: "docs/specs/issue-553-workflow-orchestration.md"
      - type: "doc"
        path: "docs/designs/issue-553-workflow-orchestration-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-553-workflow-orchestration.md"
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
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/designs/issue-558-script-contract-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  derived-by:
    - "docs/designs/issue-553-workflow-orchestration-design.md"
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-553-workflow-orchestration.md"
    - "evidence/rf553/contract-trace.md"
    - "evidence/rf553/consumer-applicability.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Consumer-neutral Workflow/CLI orchestration契約の採否（Issue #553 RF-AUTX-01）

## Context and Problem Statement

Ruviform は Synvelis・AudioLens を支える共通基盤であり、両 Consumer は「複数 Step を束ねて実行・取消・再開する」orchestration の形を持つ。Synvelis は AUTX-00〜08（synvelis#259〜#267）で Action/Atomic Macro/Workflow/Headless CLI を設計中、AudioLens は #127 で Stage Resume/Cancel/CLI/Export を既に持つ。Issue #553 は、既存 Ruviform Headless CLI 基盤（#155）と Synvelis AUTX-00〜08 を比較し、**AudioLens にも独立適用できる Workflow 計画・有界 Step 実行・Cancel/Resume・CLI Port の汎用部分だけ**を抽出して、共通化の可否を決裁する DESIGN-ONLY スライスである。

決定が必要なのは次の点である:

- Ruviform が共通の orchestration 契約（Plan/Step Port/Run State/Cancel/Resume/Budget/CLI Port の語彙）を文書として確定するか。
- Ruviform が共通 Workflow Engine / CLI crate を**今**実装するか、それとも Deferred とするか。
- 各候補を Reuse/Extend/New/Consumer-only/Deferred のどれに分類し、最小実装責務・後続 Issue・再評価 Trigger をどう記録するか。

## Decision Drivers

* **Consumer 中立性**: Synvelis Gateway 型（`CapabilityRef`/`Grant`/`Approval`/`Effect`/`CommitGate`/`JobPort`/`Composite`/`CAS`/`Undo`）・GPUI・AudioLens 製品型・独自 Grant Policy を generic 公開契約へ流入させない。
* **第二正本の禁止**: Capability Registry/Policy/Project Writer/Commit engine/Job truth/Approval store/Transaction engine を generic orchestration が複製しない。
* **非重複**: 既存 `ruviform-exec`/`ruviform-cli-sample`/#155/#558〜560 と責務を重複させない。
* **精度/性能の保護**: AudioLens f64/NaN/gap/PTS/長尺/Export・Synvelis RT/DAW semantics を共通化で犠牲にしない。
* **YAGNI/片側成功の禁止**: 片方の Consumer の成功だけで共通化を採用しない。AudioLens は現要件で Workflow/Script を要求しない。
* **正直な未実測**: 未実行の共存・実 OS・実 Job/Resume/Cancel を Pass に偽装しない。

## Considered Options

### Option A: 共通 Engine/CLI crate を今実装する（Go full）

Ruviform に `ruviform-orchestration`（仮）と共通 CLI envelope crate を新設し、Synvelis が最初の consumer として移行する。

- Pros: 契約と実装が一致し、Synvelis が generic を直接使える。
- Cons: AudioLens が現要件で必要としない依存・複雑性を強制する。Synvelis AUTX-05/06 の具象設計が未確定な段階で generic を固定すると、Consumer 側の発見（Grant Scope/Effect Bound/Job truth の境界）を取り込めず手戻りが大きい。#155 が「CLI framework を昇格しない」と確定した方針と衝突する。

### Option B: 契約語彙のみ文書化し、実装は Deferred（採用）

Consumer-neutral な orchestration の「形」（Versioned Bounded Plan・Step Port・Run/Step State・Cancel Requested/Confirmed・Indeterminate・Idempotency/Digest・Budget・CLI Result 境界）を契約として固定するが、Ruviform 共通 Engine/CLI crate は作らない。Synvelis は `synvelis-automation`/`synvelis-cli` が具象 Owner のまま進め、AudioLens は既存 stage-job モデルを維持する。共通実装は再評価 Trigger 成立時に別 Issue で判断する。

- Pros: generic の「形」と Consumer の「意味」を分離し、第二正本・Consumer 型流入・精度犠牲を設計段階で防ぐ。Synvelis が具象を先行して安定させた後に generic を固定できるため手戻りが小さい。#155 の「CLI framework 非昇格」方針と整合する。
- Cons: 契約が文書に留まり、実装がないため「共通化が本当に機能するか」は未実証。Synvelis/AudioLens が契約から乖離するリスクは残る（test-spec と consumer-applicability で監視）。

### Option C: 共通化しない（No-go）

orchestration は完全に Consumer-owned とし、Ruviform は `ruviform-exec` の Job/Cancel primitive のみ提供する。

- Pros: 最も単純。Consumer の自由度が最大。
- Cons: Synvelis/AudioLens/将来 Consumer が同じ「versioned bounded plan・fail-closed validation・cancel-requested≠confirmed・indeterminate reconcile」の形を個別に再発明する重複を許す。Issue が求める「汎用部分の抽出」という成果を出せない。

## Decision Outcome

**Option B を採用する。** Consumer-neutral orchestration 契約を文書として確定し（design §C2）、Ruviform 共通 Engine/CLI crate の実装は Deferred とする。

### Candidate 分類（最小実装責務）

| 候補 | 判定 | 最小実装責務 / Owner |
| --- | --- | --- |
| `ruviform-exec` `CancellationToken`/`Progress`/`OfflineJob`/`JobState`/`JobOutcome`/`JobEvent`/`JobError` | **Reuse** | 既存。変更不要。Consumer orchestration の下位 substrate |
| `ruviform-cli-sample` | **Consumer-only（参照例）** | #155 で「例・API 非昇格」確定。CLI shell/envelope/exit code は Consumer |
| #155 headless 分離方針 | **Reuse（方針）** | headless/GUI 依存分離の検証手法を継承 |
| `ruviform-media-core` TimeBase/MediaTime/Fact | **Reuse（条件付き）** | Step I/O の時間・観測値が必要な Consumer のみ |
| Versioned Bounded Plan の**形** | **New（契約文書化）/ 実装 Deferred** | design §C2.1。具象は Synvelis AUTX-05 |
| `PlanValidator`（Fail-Closed 規則の形） | **New（契約）/ Consumer-only 実装** | design §C2.2。規則内容は Consumer adapter |
| `StepExecutionPort` | **New（契約）/ 実装 Deferred** | design §C2.3。transport-neutral orchestration Port |
| `RunState`/`Checkpoint`/`RecoveryCursor` | **New（契約）/ Consumer-only 実装** | design §C2.4。journal authority は Consumer |
| Cancel Requested/Confirmed・Indeterminate | **Reuse概念 → 契約固定** | design §C2.4/C4。`ruviform-exec` JobState.Cancelling と整合 |
| `IdempotencyRef`/`Outcome`/`Digest` | **New（契約）** | design §C2.5 |
| Resource Budget・Non-RT・Grant/Trust/Job ownership 境界 | **New（契約）** | design §C2.6/C4 |
| CLI Entry/Result Encoding Adapter 境界 | **Consumer-only** | #155 確定。Synvelis `CliEnvelope`/Exit 0,2-10・AudioLens `al-cli` は各 Consumer |
| Synvelis CapabilityRef/Grant/Approval/Effect/CommitGate/JobPort/Composite/CAS/Undo | **Consumer-only** | `synvelis-gateway`/`synvelis-project` が唯一正本 |
| AudioLens f64/NaN/gap/PTS/Dataset/Stage/Export semantics | **Consumer-only** | `al-analysis`/`al-job`/`al-dataset`/`al-cli` |
| Script VM/Analyzer（Luau/Rhai） | **分離（非重複）** | rf#558〜#560 の領域。本契約の Plan は Script lowering の出力先 |
| GPUI/GUI | **対象外** | Driver-neutral ViewSpec は別 lane |

### 後続 Issue 候補（決裁。起票は Coordinator 判断）

- **RF-AUTX-02 候補（Deferred）**: Ruviform 共通 orchestration 実装。再評価 Trigger 成立後に起票。
- **Synvelis 側追従**: synvelis#28（Ruviform 依存追従）・synvelis#259〜#267（AUTX lane）が具象 Owner。generic 契約の adapter 適合は各 AUTX Issue の受入で確認。
- **AudioLens 側追従**: audio-lens#127（継続追従）・#189（旧 WebView 撤去）。orchestration 適用は現要件で consumer-only 継続。

### 再評価 Trigger（共通実装を再検討する条件）

1. Synvelis AUTX-05/06/03 の具象実装が完了し、generic 契約の「形」が安定した（Breaking change がない）ことが確認できた。
2. AudioLens または第三 Consumer が、既存 stage-job モデルでは不足する**独立した**有界 Step orchestration（分岐・非同期 Job 待機・外部 I/O 混在・Cancel/Resume）の必要性を具体的 Scenario で示した。
3. 上記 1・2 が揃った時点で、`ruviform-orchestration` 仮称 crate の新設可否を別 Issue で判断する。現時点では起票しない。

### Positive / Negative consequences

- Positive: generic の「形」と Consumer の「意味」を分離し、第二正本・Consumer 型流入・精度犠牲を設計段階で防ぐ。Synvelis が具象を先行して安定させた後に generic を固定できる。#155 の CLI framework 非昇格方針と整合。
- Negative: 契約が文書に留まり実装がないため「共通化が本当に機能するか」は未実証。Synvelis/AudioLens が契約から乖離するリスクは test-spec と consumer-applicability で監視する。

### Confirmation

- `evidence/rf553/contract-trace.md` で受入条件→契約条項→根拠の対応を固定。
- `evidence/rf553/consumer-applicability.md` で 2 Consumer の正例・反例と Reuse/Extend/New/Consumer-only/Deferred 判定を記録。
- `docs/test-specs/issue-553-workflow-orchestration.md` で各契約条項の検証 Rule/Example を固定。
- `evidence/rf553/verification.md` で実行した検証と NotRun/Unsupported を記録。
- 再評価 Trigger 成立時に本 ADR を supersede または新 ADR で更新する。

## Implementation Plan

* Affected paths: `docs/specs/issue-553-workflow-orchestration.md`、`docs/designs/issue-553-workflow-orchestration-design.md`、`docs/test-specs/issue-553-workflow-orchestration.md`、`docs/plans/issue-553-workflow-orchestration-plan.md`、`docs/tasks/issue-553-workflow-orchestration-task.md`、`docs/impl/ir/issue-553-workflow-orchestration.md`、`docs/impl/exp/issue-553-workflow-orchestration.md`、`evidence/rf553/`。
* Patterns to follow: 既存 docs チェーンの front matter・relation・管理 index 規約（`spec-doc`/`design-doc`/`adr-doc`/`test-spec-doc`/`plan-doc`/`task-doc`/`impl-doc`/`doc-maintenance`）。`docs/designs/ruviform-architecture-design-baseline.md` のレイヤー/RT 分離、`docs/adr/public-enum-non-exhaustive-policy.md` の公開 enum 方針、`docs/designs/issue-155-headless-cli-foundation.md` の headless/CLI 非昇格方針。
* Patterns to avoid: `crates/**` の新設、`Cargo.toml`/`Cargo.lock`/`mise.toml`/CI の変更、Synvelis Gateway/Project/Grant/Job 型・GPUI・AudioLens 製品型・独自 Grant Policy の契約への混入、実装前の公開 API/Engine 昇格、片側 Consumer の成功だけでの共通化採用。
* Dependencies or configuration: なし（文書のみ。実装 crate・依存追加は再評価 Trigger 成立後の別 Issue の範囲）。

## Verification

* [x] `docs/specs/issue-553-workflow-orchestration.md` が本 ADR の決定を要件・受入条件として反映している。
* [x] `docs/designs/issue-553-workflow-orchestration-design.md` が棚卸し対応表・契約語彙（C2.1〜C2.6）・2 Consumer Scenario・Fault Matrix・Deferred を定義している。
* [x] `docs/test-specs/issue-553-workflow-orchestration.md` が契約条項を検証可能な Rule/Example へ写像している。
* [x] `evidence/rf553/` が contract-trace・consumer-applicability・verification を保持している。
* [ ] Synvelis AUTX-05/06/03 の具象実装で generic 契約の「形」が安定する（後続ゲート、現時点 NotRun）。
* [ ] AudioLens または第三 Consumer が独立した有界 Step orchestration の必要性を具体的 Scenario で示す（後続ゲート、現時点 NotRun）。
* [ ] 再評価 Trigger 成立後に `ruviform-orchestration` 仮称 crate の新設可否を別 Issue で判断する（後続ゲート、現時点 Deferred）。

## Pros and Cons of the Options

### Option A: 共通 Engine/CLI crate を今実装する（Go full）

* Good, because 契約と実装が一致し、Synvelis が generic を直接使える。
* Bad, because AudioLens が現要件で必要としない依存・複雑性を強制する。Synvelis AUTX-05/06 の具象設計が未確定な段階で generic を固定すると、Consumer 側の発見を取り込めず手戻りが大きい。#155 の「CLI framework 非昇格」方針と衝突する。

### Option B: 契約語彙のみ文書化し、実装は Deferred（採用）

* Good, because generic の「形」と Consumer の「意味」を分離し、第二正本・Consumer 型流入・精度犠牲を設計段階で防ぐ。Synvelis が具象を先行して安定させた後に generic を固定できる。#155 の CLI framework 非昇格方針と整合する。
* Bad, because 契約が文書に留まり実装がないため「共通化が本当に機能するか」は未実証。Synvelis/AudioLens が契約から乖離するリスクは残る（test-spec と consumer-applicability で監視）。

### Option C: 共通化しない（No-go）

* Good, because 最も単純で Consumer の自由度が最大。
* Bad, because Synvelis/AudioLens/将来 Consumer が同じ「versioned bounded plan・fail-closed validation・cancel-requested≠confirmed・indeterminate reconcile」の形を個別に再発明する重複を許す。Issue が求める「汎用部分の抽出」という成果を出せない。

## Consequences

* Positive: generic の「形」と Consumer の「意味」を分離し、第二正本・Consumer 型流入・精度犠牲を設計段階で防ぐ。Synvelis が具象を先行して安定させた後に generic を固定できる。#155 の CLI framework 非昇格方針と整合する。
* Negative: 契約が文書に留まり実装がないため「共通化が本当に機能するか」は未実証。Synvelis/AudioLens が契約から乖離するリスクは test-spec と consumer-applicability で監視する。
* Neutral: Synvelis AUTX-05/06/03 と AudioLens 既存 stage-job は具象 Owner のまま進めるため、即時の実装ブロッカーはない。orchestration 契約語彙は文書として確定し、将来の共通実装 Issue の参照点となる。

## More Information

* Issue #553 本文（RF-AUTX-01、DESIGN-ONLY の契約レビューが正本）。
* 上位/関連: Issue #219（両 Consumer 平等）・#294（共通変更時の独立回帰）・#558〜#560（Script/Analyzer 契約・PoC）・#155（Headless CLI 基盤）。
* Consumer: Synvelis #259（AUTX-00）・#260〜#267（AUTX-01〜08）・#271〜#274（SV-SCRIPT-01〜04）・#28（Ruviform 依存追従）、AudioLens #127（継続追従・Script 非要求）・#189（旧 WebView 撤去）。
* 証拠: `evidence/rf553/contract-trace.md`（受入条件→契約条項→根拠）、`evidence/rf553/consumer-applicability.md`（2 Consumer Reuse/New/Consumer-only/Deferred 判定）。
* 関連 ADR: `docs/adr/public-enum-non-exhaustive-policy.md`（公開 enum 方針）、`docs/adr/issue-558-script-backend-contract.md`（Script 契約・非重複）。

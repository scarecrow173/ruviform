---
id: "ADR-034Xe9xZEqz5ftcu5yCqXB"
type: "adr"
status: "accepted"
title: "言語非依存 Script Backend/Analyzer 契約（Issue #558 RF-SCRIPT-00）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  changes:
    added:
      - type: "doc"
        path: "docs/adr/issue-558-script-backend-contract.md"
      - type: "doc"
        path: "docs/specs/issue-558-script-contract.md"
      - type: "doc"
        path: "docs/designs/issue-558-script-contract-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-558-script-contract.md"
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
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  derived-by:
    - "docs/designs/issue-558-script-contract-design.md"
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-558-script-contract.md"
    - "evidence/rf558/contract-trace.md"
    - "evidence/rf558/consumer-applicability.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 言語非依存 Script Backend/Analyzer 契約（Issue #558 RF-SCRIPT-00）

## Context and Problem Statement

Ruviform は Synvelis・AudioLens を支える共通基盤であり、両 Consumer は Script 機能を要求し得る（Synvelis は SV-SCRIPT-01〜04 で GPUI Editor・Action Script・Workflow lowering を計画、AudioLens は現時点では Script 非要求だが将来の昇格可能性を残す）。Issue #558 は、Luau を初期標準としつつ Rhai との同一プロセス共存を PoC 検証対象とする **言語非依存 Script Runtime/Analyzer 契約** を確定する DESIGN-ONLY スライスである。

決定が必要なのは、次の境界の形である:

- `ScriptLanguageId` と `ScriptBackendId` を分離し、Version/Capabilities を持つ `ScriptBackendRegistry` が同一プロセス内の複数 Backend を実行時に選択する契約。
- `ScriptValue`/`ScriptError`/`ScriptArtifact` の VM 非依存値モデル（Absent/Null 区別・i64 精度・TimeDomain/Resource ID 保持）。
- Runtime から独立した `ScriptAnalyzer`/Capabilities/Source Revision/Diagnostics/Completion の Port。
- Fail-Closed 判定表と Fault Matrix を機械検査可能な形で固定し、実 Runtime 共存・3OS・native FFI は #559/#560 の PoC 受入へ委譲する。

## Decision Drivers

* **共存性**: 同一実行ファイル内で Luau/Rhai を同時登録し実行時に選択できる必要がある（Issue #558 ベースライン。ビルド時排他 feature 切替では代替不可）。
* **言語非依存**: Vendor 型（mlua/Rhai/Luau.Analysis の値・エラー・ハンドル）を公開面に出さず、2 Consumer が同一契約を消費できる必要がある。
* **Fail Closed**: 未知・曖昧・Version/Capability 不一致を型付きエラーで拒否し、暗黙フォールバックを許さない必要がある。
* **値の完全性**: Absent≠Null、i64 精度、TimeDomain/Resource ID を維持し、VM 固有型・Grant/Secret・製品型の越境を禁止する必要がある。
* **独立性**: Runtime と Analyzer は別 Registry/Port とし、Analyzer 成功は実行 Grant を意味しない。
* **正直な未実測**: 未実行の Runtime 共存・3OS・native FFI を Pass に偽装せず #559/#560 の受入へ明示的に委譲する必要がある。
* **非重複**: 既存の `ruviform-exec`/`ruviform-cli-sample`/Consumer 側 Coordinator/Gateway/Job と責務を重複させない（#553 の材料）。

## Considered Options

* **Option A**: Vendor 型を直接公開する API（mlua/Rhai の型を Ruviform 公開面に出す）
* **Option B**: 単一言語固定（Luau のみを対象とし抽象化しない）
* **Option C**: 言語非依存 Port 契約 + 実行時 Registry/Dispatch（選択）
* **Option D**: ビルド時 feature 排他で Backend を切替える

## Decision Outcome

Chosen option: "**Option C — 言語非依存 Port 契約 + 実行時 Registry/Dispatch**", because 同一プロセス複数 Backend の同時登録・実行時選択・Fail Closed を契約として確定でき、#559/#560 の PoC が実装前に一意の型・意味論を参照できるからである。Option A は Vendor 型が公開面へ漏れ 2 Consumer の製品型非依存を壊し、Option B は Rhai 共存 PoC と将来言語追加を不可能にし、Option D は Issue が明示的に禁止するビルド時排他切替である。

### Consequences

* Good, because Luau/Rhai の同一意味 Versioned Action/Workflow Definition を 1 契約で表現でき、#553 の Workflow 共通化判定へ中立材料を供給できる。
* Good, because Analyzer を Runtime から分離することで、未実装能力を Unsupported と正直に返せ、型検査完了の偽装を防げる。
* Good, because Fail-Closed 判定表と Fault Matrix が機械検査可能な試験マトリクスへ直接対応し、PoC の受入基準が先に固定される。
* Bad, because 契約の網羅性を文書で担保する必要があり、実装前に型・エラー分類・Fixture を詳細化しなければ #559/#560 で設計齟齬が出るリスクがある。
* Bad, because `ScriptSession` の Send/Sync を要求しない設計は、Consumer 側の配置・所有権設計を各自に委ねるため、Backend Capabilities の広告規則を #559 PoC で実測検証する必要がある。

### Confirmation

* `docs/designs/issue-558-script-contract-design.md` の公開型・Fail-Closed 表・Fault Matrix・Fixture 割当が本 ADR の決定を具体化していることを `review-protocol` で確認する。
* `docs/test-specs/issue-558-script-contract.md` の Rule/Example が各契約条項を検証可能な形に写像していることを監査する。
* `evidence/rf558/contract-trace.md` が受入条件→契約条項→根拠参照を対応付け、未実行項目が NotRun/Deferred と明記されていることを確認する。
* #559/#560 の PoC 完了時に、本契約の型・意味論・Fixture が実測で確認されることを後続ゲートとして残す。

## Implementation Plan

* Affected paths: `docs/specs/issue-558-script-contract.md`、`docs/designs/issue-558-script-contract-design.md`、`docs/test-specs/issue-558-script-contract.md`、`docs/plans/issue-558-script-contract-plan.md`、`docs/tasks/issue-558-script-contract-task.md`、`docs/impl/ir/issue-558-script-contract.md`、`docs/impl/exp/issue-558-script-contract.md`、`evidence/rf558/`。
* Patterns to follow: 既存 docs チェーンの front matter・relation・管理 index 規約（`spec-doc`/`design-doc`/`adr-doc`/`test-spec-doc`/`plan-doc`/`task-doc`/`impl-doc`/`doc-maintenance`）。`docs/designs/ruviform-architecture-design-baseline.md` のレイヤー/RT 分離、`docs/adr/public-enum-non-exhaustive-policy.md` の公開 enum 方針。
* Patterns to avoid: `crates/**` の新設、`Cargo.toml`/`Cargo.lock`/`mise.toml`/CI の変更、VM vendor 型の公開、製品型（Synvelis/AudioLens/GPUI）の契約への混入、実装前の公開 API 昇格。
* Dependencies or configuration: なし（文書のみ。実装 crate・依存追加は #559/#560 の範囲）。

## Verification

* [x] `docs/specs/issue-558-script-contract.md` が本 ADR の決定を要件・受入条件として反映している。
* [x] `docs/designs/issue-558-script-contract-design.md` が公開型・Registry/Dispatch・値モデル・Analyzer Port・Fail-Closed 表・Fault Matrix・Fixture を定義している。
* [x] `docs/test-specs/issue-558-script-contract.md` が契約条項を検証可能な Rule/Example へ写像している。
* [x] `evidence/rf558/` が contract-trace・2-consumer 適用性・検証記録を保持している。
* [ ] #559 PoC で Luau/Rhai 同一バイナリ共存・実行時選択・Fail Closed が実測 Pass する（後続ゲート、現時点 NotRun）。
* [ ] #560 PoC で Luau.Analysis native FFI・3OS 配布が実測 Pass する（後続ゲート、現時点 NotRun）。

## Pros and Cons of the Options

### Option A: Vendor 型を直接公開する API

* Good, because 実装が最も単純で、mlua/Rhai の機能をそのまま露出できる。
* Bad, because Vendor 型（`mlua::Value`・`rhai::Dynamic`・ネイティブハンドル）が公開面へ漏れ、Synvelis/AudioLens の製品型非依存を壊す。Backend 差し替え・言語追加のたびに公開 API が変わる。

### Option B: 単一言語固定（Luau のみ）

* Good, because 抽象化コストが最小で、初期標準である Luau に最適化できる。
* Bad, because Issue が要求する Rhai 同一プロセス共存 PoC（#559）と将来言語追加の余地を失う。Analyzer/Runtime の言語非依存境界も定義できない。

### Option C: 言語非依存 Port 契約 + 実行時 Registry/Dispatch（採用）

* Good, because 同一プロセス複数 Backend の同時登録・実行時選択・Fail Closed を契約で保証でき、#559/#560 の PoC が実装前に一意の参照を持てる。
* Bad, because 契約の詳細化（値モデル・エラー分類・Fixture・Fault Matrix）を文書で先に固定する必要があり、設計負荷が高い。

### Option D: ビルド時 feature 排他で Backend を切替える

* Good, because バイナリサイズを最小化でき、feature flag で簡易に切替えられる。
* Bad, because Issue が明示的に禁止する「ビルド時排他切替」であり、同一実行ファイル内での Luau/Rhai 共存を実現できない。実行時 Dispatch の契約も定義できない。

## More Information

* Issue #558 本文（2026-11-11 設計ベースライン・2026-11-13 ゲート整理が正本）。
* 上位/関連: Issue #553（Workflow/CLI 共通化判断）、#559（Registry/Runtime PoC）、#560（Luau.Analysis FFI/3OS PoC）、#219（両 Consumer 平等）、#294（共通変更時の独立回帰）。
* Consumer: Synvelis #259（AUTX-00）・#271〜274（SV-SCRIPT-01〜04）、AudioLens #127（Script 非要求の明記）。
* 証拠: `evidence/rf558/contract-trace.md`（受入条件→契約条項→根拠）、`evidence/rf558/consumer-applicability.md`（2 Consumer Reuse/New/Deferred 判定）。
* 関連 ADR: `docs/adr/public-enum-non-exhaustive-policy.md`（将来の公開 enum/型拡張方針）。

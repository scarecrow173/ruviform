---
id: "SPEC-034XBX7XEtiZrtlQez9DxN"
type: "spec"
status: "proposed"
title: "Issue #294 first slice — consumer-verify 証拠の virtual-first Case schema v2 拡張"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/discovery/issue-294-virtual-first-evidence-schema.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-294-virtual-first-evidence-schema.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-294-virtual-first-evidence-schema.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
    - "POLICIES.md"
  refines:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  refined-by: []
  derives-from:
    - "docs/discovery/issue-294-virtual-first-evidence-schema.md"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
    - "ci/consumer-verify/evidence.mjs"
    - ".github/workflows/consumer-verify.yml"
    - "ci/daily/decide.mjs"
  defers: []
  deferred-by: []
---

# Issue #294 first slice — consumer-verify 証拠の virtual-first Case schema v2 拡張

## What Should Be Built

`ci/consumer-verify/` の lane/case record と evidence JSON を、着地済み virtual-first 検証契約（`scarecrow173/ruviform#627` Closed、POLICIES Effective 2026-11-15）の Case schema（`docs/designs/issue-627-virtual-first-verification-contract-design.md` §2）へ additive に拡張する first slice。

1. **lane record の拡張**: `evidence.mjs` の `makeLaneRecord` が生成する各 lane/case record に、Case schema の必須軸を追加する — `case_id`（lane 内一意の識別子）、`level`（`OS-L1`/`GUI-L2`/`EXT-L3`）、`environment_kind`（`deterministic-backend`/`virtual-os-endpoint`/`test-plugin`/`real-window`/`physical-device`）、`requirement`（`Required`/`Deferred`/`NotApplicable`）。結果軸は既存 `outcome`（`pass`/`fail`/`not_run`/`unsupported`/`skipped`）を維持し、`requirement` とは直交させる（同一項目へ畳み込まない）。optional 項目として `oracle`/`fixture_version`/`resolved_ruviform_sha`/`expected`/`observed`/`device`/`endpoint`/`plugin`/`abi`/`sdk`/`arch`/`os_ver` を受理し、提示時は fail-closed で検証する。
2. **`schemaVersion` 1→2**: evidence JSON の `schemaVersion` を 2 に bump する。`contract` は `"consumer-verify-tuple/v1"` のまま固定 — `ci/daily/decide.mjs` の tuple 回復経路（contract 文字列照合）を壊さない。旧 artifact の意味は変えない。
3. **workflow の静的 lane 行**: `.github/workflows/consumer-verify.yml` の全 lane 記録経路（`rec()` helper・extra task 直接 `lane` 呼出し・finalize 失敗 fallback `printf`）が新項目を正直に宣言する。既存 outcome の意味・語彙は不変で、新規 Pass の付与なし。finalize 失敗 fallback の `"schemaVersion":1` hardcode も 2 に合わせる。
4. **fail-closed 検証**: `readLanes` が `lane` コマンド経由でない行も同じ語彙（case_id/level/environment_kind/requirement の必須性・enum 値集合・optional 項目の型）を満たすことを検証し、偽造/未知値が status に寄与しないことを保証する。
5. **doc chain**: slug `issue-294-virtual-first-evidence-schema` の discovery/spec/design/plan/task/test-spec/impl( ir/exp ) 文書チェーンを作成し、Issue レベル AC のうち本 slice 対象外の項目（3repo SHA tuple binding・consumer oracles・#628-#631 fixture version）を deferred として記録する。

## Why Now

- POLICIES.md は 2026-11-15 に仮想優先受入へ改訂済み（§2.1・§3 証拠項目）で、Case schema の必須記録項目（case_id・level・environment_kind・requirement・独立 Oracle・resolved Ruviform SHA・fixture version）を要求するが、consumer-verify の証拠 JSON はまだ schemaVersion 1 の6項目 lane record のままである。
- `issue-627-policies-revision-proposal.md` のゲート移管台帳は「#294 の tuple Gate 構造は維持 / 証拠項目を schema に合わせて拡張。旧証拠の意味は変えない」と本 slice の位置づけを固定済み。
- #628（Test Runtime）・#629（仮想 OS Endpoint）・#630（Test Plugin）・#631（GUI Runner）の実装が進む前に、証拠契約側が Case schema を保持できる器を用意しないと、後続 slice で証拠形式の再設計が必要になる。

## Users and Value

- **Issue #294 / #627 の受入 Owner・Coordinator**: lane/case 単位で level・環境種別・要求状態を機械可読に判定でき、`Required` の `NotRun`/`Fail` を Pass に見せない fail-closed 構造が schema 層で保証される。
- **`ci/daily` tuple scheduler**: `contract` 不変により既存の証拠回復・照合経路をそのまま使い続けられる。
- **後続 slice（#628-#631・Issue レベル AC）**: `oracle`/`fixture_version`/`resolved_ruviform_sha`/`expected`/`observed` 等の optional 項目が schema に予約済みとなり、実装時に additive 拡張ではなく記録で済む。

## Scope

### In Scope

- `ci/consumer-verify/evidence.mjs`: `SCHEMA_VERSION` 1→2、lane record への新規必須4項目 + optional 項目の追加、`lane` CLI flag、`readLanes` の fail-closed 語彙検証。
- `.github/workflows/consumer-verify.yml`: 両 verify job の `rec()` helper・全 lane 行・extra task lane 記録・finalize 失敗 fallback の schemaVersion/新項目対応。
- `ci/consumer-verify/tests/`: `evidence.test.mjs`・`consumer-verify-yml-contract.test.mjs` の拡張（新項目の必須性・語彙・直交性・旧 artifact 意味不変・yml 構造契約）。
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-294-virtual-first-evidence-schema.md` doc chain と管理 index 再生成、`evidence/rf294/` の検証記録。

### Out of Scope

- `contract` 文字列の bump（`consumer-verify-tuple/v1` → v2）— `ci/daily`・`daily-ci.yml`・`docs/development-environment.md` への波及を伴うため後続 slice。
- `outcome` → `result` の改名、`OUTCOMES` 5値語彙の変更（`ci/tracker` contract test が pin）。
- Issue レベル AC: 3repo SHA tuple binding の拡張、consumer 独立 Oracle の配線、#628-#631 fixture/plugin version の記録、Windows 実 GUI/OS 仮想 Endpoint 専用 lane、post-merge dispatch lane の実走証拠。
- `mise.toml`・`docs/development-environment.md`・`ci/daily/`・`POLICIES.md`・`AGENTS.md`・他 workflow・`crates/` の変更。
- 新規 Pass の付与・既存 outcome 語彙の意味変更・skipped の pass 化。

## Requirements

- **Case schema 適合**: 各 lane/case record が `case_id`/`level`/`environment_kind`/`requirement`/結果軸（`outcome`）を保持し、`requirement` と結果軸が直交する（`Required`+`NotRun`/`Fail`/`Unsupported` を許容し、いずれかへ畳み込まない）。
- **語彙の閉包**: `level` ∈ {`OS-L1`,`GUI-L2`,`EXT-L3`}、`environment_kind` ∈ {`deterministic-backend`,`virtual-os-endpoint`,`test-plugin`,`real-window`,`physical-device`}、`requirement` ∈ {`Required`,`Deferred`,`NotApplicable`}。未知値・欠落は fail-closed で拒否。
- **contract 不変**: evidence JSON の `contract` は `"consumer-verify-tuple/v1"`、`kind`/`status`/`tuple`/`run`/`consumer`/`coverage` の構造と意味は不変。`schemaVersion` のみ 2。
- **正直な宣言**: workflow の静的 lane 行は実際の検証内容に対応する level/environment_kind/requirement を宣言し、実行しない lane に Pass を付与しない。capability-absent の `unsupported`・owner 付き `not_run` の outcome は維持。
- **fail-closed 検証**: `readLanes` は JSONL の各行が lane record schema（必須項目・enum・optional 項目の型）を満たすことを検証し、違反行で finalize を失敗させる。`resolved_ruviform_sha` 提示時は 40-hex SHA を要求。
- **検証入口**: `mise run test-ci-consumer-verify`（fixture + yml contract test）が green、`mise exec -- actionlint .github/workflows/consumer-verify.yml`・`mise exec -- prek run --all-files` が pass。

## Acceptance Criteria

- [ ] lane/case record が `case_id`/`level`/`environment_kind`/`requirement`/`outcome`（結果軸・`requirement` と直交）+ optional `oracle`/`fixture_version`/`resolved_ruviform_sha`/`expected`/`observed`/`device`/`endpoint`/`plugin`/`abi`/`sdk`/`arch`/`os_ver` を保持する（`evidence.mjs` の `makeLaneRecord`/`readLanes` で強制）。
- [ ] `SCHEMA_VERSION === 2`、`CONTRACT === "consumer-verify-tuple/v1"`、evidence JSON の既存 top-level 構造（`kind`/`run`/`candidate`/`consumer`/`tuple`/`status`/`coverage`）の意味が不変。
- [ ] `mise run test-ci-consumer-verify` の fixture + yml contract test が green（新項目の必須性・語彙 enum・直交性・旧 artifact 意味不変・workflow の case 宣言構造を含む）。
- [ ] workflow の静的 lane 行が level/environment_kind/requirement を正直に宣言 — 新規 Pass の付与なし、既存 outcome 語彙の意味は不変、skipped = derived record のみ。
- [ ] tuple/evidence JSON は `contract` = `consumer-verify-tuple/v1` のまま（daily 回復経路を壊さない）、finalize 失敗 fallback の `schemaVersion` も 2。
- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml` + `mise exec -- prek run --all-files` が pass。実走証拠（post-merge dispatch lane）は NotRun のまま。
- [ ] doc chain（discovery/spec/design/plan/task/test-spec/impl ir/exp）と管理 index が整合。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `contract` の v2 bump（schema 版との一致） | `ci/daily`・`daily-ci.yml` env・`docs/development-environment.md` の同期を要し本 slice scope 外 | Issue #294 後続 slice / `docs/development-environment.md` 同期 | contract と schemaVersion の版ずれが残る（語彙上は分離済みで実害なし） |
| `oracle`/`fixture_version`/`resolved_ruviform_sha` の実値記録 | 独立 Oracle・fixture version は #628-#631 の実装産物。schema は enabler のみ | `scarecrow173/ruviform#628`-`#631`・Issue #294 後続 slice | Case 記録が optional 項目を持たないまま進む |
| 3repo SHA tuple binding・consumer 独立 Oracle 判定・Windows 実 GUI/OS 仮想 Endpoint 専用 lane | Issue レベル AC であり本 slice は schema の enabler のみ | Issue #294 本体・#628-#631 | schema があっても証拠が結合されない |
| `docs/development-environment.md` の `consumer-verify-tuple/v1` 証拠契約記述の同期 | 別 Worker リース中のため本 slice では編集しない。schemaVersion 2 と lane record 拡張を記述へ反映する必要がある可能性 | Issue #294 後続 slice / 当該ファイルのリース Worker | 文書と実装の一時的な乖離 |

## Sources and Relations

- Issue #294（first slice: virtual-first Case schema への証拠契約拡張、schemaVersion 1→2）
- `docs/discovery/issue-294-virtual-first-evidence-schema.md`（現状監査・downstream 制約の実測）
- `docs/specs/issue-294-cross-consumer-candidate-gate.md`（tuple Gate・証拠契約の先行 spec — `refines`）
- `docs/specs/issue-627-virtual-first-verification-contract.md`・`docs/designs/issue-627-virtual-first-verification-contract-design.md`（Case schema 正本）
- `docs/designs/issue-627-policies-revision-proposal.md`（ゲート移管台帳 RF #294/#281 行）
- `POLICIES.md`（Effective: 2026-11-15 仮想優先受入 — Required/Deferred/NotApplicable × Pass/Fail/NotRun/Unsupported）

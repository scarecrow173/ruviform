---
id: "TSPEC-034XBZwsg35LDBA7RJdmOL"
type: "test-spec"
status: "approved"
title: "Issue #294 first slice — consumer-verify Case schema v2 のテスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-virtual-first-evidence-schema.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-virtual-first-evidence-schema.md"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-294-virtual-first-evidence-schema.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-294-virtual-first-evidence-schema.md"
  verified-by: []
  references:
    - "ci/consumer-verify/evidence.mjs"
    - "ci/consumer-verify/tests/evidence.test.mjs"
    - "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
  defers: []
  deferred-by: []
---

# Issue #294 first slice — consumer-verify Case schema v2 のテスト仕様

## Purpose

consumer-verify の lane/case record が virtual-first Case schema（`docs/designs/issue-627-virtual-first-verification-contract-design.md` §2）の記録契約を schemaVersion 2 で保持し、`requirement` と結果軸の直交・fail-closed 語彙・`contract` v1 固定・旧 artifact 意味不変を、`mise run test-ci-consumer-verify` の fixture test と yml contract test が offline で保証する。契約が supersede されるか schemaVersion が再 bump されるまで有効。

## Feature

consumer-verify 証拠 schema v2 — lane/case record が `case_id`/`level`/`environment_kind`/`requirement`/`outcome`（結果軸）+ optional Case 項目を保持し、evidence JSON が `schemaVersion: 2`・`contract: "consumer-verify-tuple/v1"` を出力する。

## Rules

- **R1 必須項目**: `makeLaneRecord` は `case_id`（非空 string）・`level`・`environment_kind`・`requirement` を必須とし、欠落・空値・enum 外で throw する。`name`/`outcome` の既存検証は維持。
- **R2 語彙閉包**: `level` ∈ {`OS-L1`,`GUI-L2`,`EXT-L3`}、`environment_kind` ∈ {`deterministic-backend`,`virtual-os-endpoint`,`test-plugin`,`real-window`,`physical-device`}、`requirement` ∈ {`Required`,`Deferred`,`NotApplicable`}、`outcome` ∈ `OUTCOMES` 5値（変更なし）。
- **R3 直交**: `requirement` と `outcome` は別 field であり、どちらの値域にも他方の値を入れない。`Required`+`not_run`/`unsupported`/`fail` の組合せは有効な record として受理される（status 導出は `outcome` のみを見る — 既存行列不変）。
- **R4 optional 項目**: `oracle`/`fixture_version`/`expected`/`observed`/`device`/`endpoint`/`plugin`/`abi`/`sdk`/`arch`/`os_ver` は string or null。`resolved_ruviform_sha` は null または 40-hex SHA（`SHA_RE`）。非適合値は `lane`/`readLanes` で拒否。
- **R5 readLanes fail-closed**: JSONL の各行は `lane` コマンド経由かどうかに関わらず R1-R4 を満たさなければ finalize が失敗する（schemaVersion 1 形式の行・enum 外・必須欠落は拒否）。
- **R6 版と契約**: `SCHEMA_VERSION === 2`、`CONTRACT === "consumer-verify-tuple/v1"`。evidence JSON の `kind`/`run`/`candidate`/`consumer`/`accessMode`/`tuple`/`status`/`statusReason`/`coverage`/`generatedAt` の構造・意味は schemaVersion 1 と不変。
- **R7 status 導出不変**: `deriveStatus`/`coverage` の戻り値・優先順位・`shaMatch` 三値・secret 非転写・summary 形状は v1 と同一（`ci/tracker` が import する `deriveStatus`/`OUTCOMES` の signature・語彙を壊さない）。
- **R8 workflow 宣言**: `consumer-verify.yml` の全 `rec` 呼出しが `"<case_id>|<level>|<environment_kind>|<requirement>"` spec を第1引数に持ち、直接 `lane` 呼出し（extra task）が `--case-id`/`--level`/`--environment-kind`/`--requirement` を持つ。finalize 失敗 fallback は `"schemaVersion":2`・`"contract":"consumer-verify-tuple/v1"`。非既定分類行（GUI visual/OS=`GUI-L2|real-window|Required`、PLUGIN-NATIVE-DOCK=`EXT-L3|test-plugin|Required`）が明示 spec を持つ。

## Examples

- `makeLaneRecord({name:"x", outcome:"pass", caseId:"al-check", level:"OS-L1", environmentKind:"deterministic-backend", requirement:"Required"})` → record が `case_id:"al-check"`・`level:"OS-L1"`・`environment_kind:"deterministic-backend"`・`requirement:"Required"`・optional 項目 null を持つ。
- `requirement:"Required"` + `outcome:"not_run"` の record は受理され、`deriveStatus` は `not_run` を返す（直交の実例）。
- lanes JSONL に `{name:"x", outcome:"pass"}`（v1 形式・case 項目なし）の行 → finalize が schema error で失敗。
- lanes JSONL に `level:"L9"` や `requirement:"required"`（小文字）→ finalize が失敗。
- `resolved_ruviform_sha:"abc"`（非40-hex）→ `lane`/`readLanes` が失敗。`"a".repeat(40)` → 受理。
- evidence JSON: `schemaVersion===2`、`contract==="consumer-verify-tuple/v1"`、`status`/`coverage`/`tuple` が v1 と同じ導出結果。
- yml: `rec "GUI visual/OS + 実 window lanes"` 行の spec が `GUI-L2|real-window|Required`、synvelis の `PLUGIN-NATIVE-DOCK` 行が `EXT-L3|test-plugin|Required`、fallback printf が `"schemaVersion":2`。

## Guarantees

- v2 record の必須4項目と enum 語彙が `node --test` で全分岐 pin され、未知値・欠落が status に寄与しない。
- `contract` v1 固定により `ci/daily` の tuple 回復経路と `ci/tracker` の `deriveStatus`/`OUTCOMES` consumer が非破壊。
- workflow の静的 lane 行が level/environment_kind/requirement を宣言し、outcome・status 導出・summary は不変 — 新規 Pass なし。

## Non-goals

- `oracle`/`fixture_version`/`resolved_ruviform_sha`/`expected`/`observed` の実値供給（#628-#631 以降の slice）。
- `contract` の v2 bump、`outcome`→`result` 改名、status 語彙・summary 列の変更。
- 実走 dispatch lane の証拠（post-merge のみ）。

## Risk

- `readLanes` 強化と workflow 更新が不整合だと全 run が finalize 失敗 → R5/R8 と preflight 自己検証（`test-ci-consumer-verify`）で検出。
- 分類値の恣意的付与 → R8 で非既定行の spec を pin し、既定 `OS-L1|deterministic-backend|Required` の妥当性を review で確認。

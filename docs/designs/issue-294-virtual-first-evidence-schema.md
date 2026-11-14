---
id: "DESIGN-034XBY8vBLs0BGGIYKZ9DB"
type: "design"
status: "approved"
title: "Issue #294 first slice — consumer-verify Case schema v2 証拠契約設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/discovery/issue-294-virtual-first-evidence-schema.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-294-virtual-first-evidence-schema.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
  blocks:
    - "docs/plans/issue-294-virtual-first-evidence-schema.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "POLICIES.md"
  refines:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  derived-by:
    - "docs/plans/issue-294-virtual-first-evidence-schema.md"
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
  references:
    - "ci/consumer-verify/evidence.mjs"
    - ".github/workflows/consumer-verify.yml"
    - "ci/daily/decide.mjs"
    - "ci/tracker/tests/issue-tracker-workflows-contract.test.mjs"
  defers: []
  deferred-by: []
---

# Issue #294 first slice — consumer-verify Case schema v2 証拠契約設計

## Context

`ci/consumer-verify/evidence.mjs` は consumer-verify workflow の証拠器で、`classify-ref`・`lane`（JSONL 追記）・`finalize`（status 導出 + evidence JSON/summary 出力）の3コマンドを持つ。lane record は `{name, command, feature, os, outcome, reason}` のみで、着地済み virtual-first Case schema（#627 design §2）が要求する `case_id`/`level`/`environment_kind`/`requirement`/optional Oracle・fixture・resolved SHA を保持しない。

設計上の硬い制約（discovery で実測済み）:

- `ci/daily/decide.mjs` の `classifyRunEvidence` は `kind`/`contract`/`run.runId`/`consumer.*`/`status`/`tuple.*` を検証するが `schemaVersion` と `lanes` を読まない。`contract` 文字列を変えると `DAILY_TUPLE_CONTRACT` pin・`daily-ci.yml` env・`docs/development-environment.md` へ波及する。
- `ci/tracker/tests/issue-tracker-workflows-contract.test.mjs`（編集範囲外）が `deriveStatus`/`OUTCOMES` を import し、lane object の `outcome` キーと5値語彙を pin する。
- workflow の全 lane 記録経路は `rec()` helper（各 job の Record evidence step）・extra task ループの直接 `node "$EV" lane` 呼出し・finalize 失敗時の最小証拠 `printf` の3系統。

## Scope

### In Scope

- `evidence.mjs` の lane record schema v2（必須4項目 + optional 項目）と `lane` CLI flag、`readLanes` の fail-closed 語彙検証、`SCHEMA_VERSION` 2。
- `consumer-verify.yml` の全 lane 記録行への case 宣言付与と fallback `schemaVersion` 更新。
- contract test の拡張。

### Out of Scope

- `contract` bump、`outcome` 改名・語彙変更、status 導出・coverage・summary の意味変更、Issue レベル AC（tuple binding・Oracle・fixture version・実走証拠）。

## Components and Boundaries

### 1. lane record schema v2（`evidence.mjs`）

lane/case record の確定 shape（JSONL 1行 = evidence JSON `lanes[]` の1要素）:

| 項目 | 必須 | 語彙/制約 |
| --- | --- | --- |
| `name` | yes | 既存。非空 string（表示名） |
| `command` | yes | 既存。既定 `"-"` |
| `feature` | yes | 既存。既定 `"-"` |
| `os` | yes | 既存。既定 `"-"` |
| `outcome` | yes | 既存。`OUTCOMES` 5値（結果軸。`requirement` と直交） |
| `reason` | yes | 既存。null 可 |
| `case_id` | yes（新規） | 非空 string。repo 内一意の Case 識別子 |
| `level` | yes（新規） | `OS-L1` / `GUI-L2` / `EXT-L3` |
| `environment_kind` | yes（新規） | `deterministic-backend` / `virtual-os-endpoint` / `test-plugin` / `real-window` / `physical-device` |
| `requirement` | yes（新規） | `Required` / `Deferred` / `NotApplicable` |
| `oracle` | optional | string or null（独立 Oracle 識別子） |
| `fixture_version` | optional | string or null |
| `resolved_ruviform_sha` | optional | null または 40-hex SHA（`SHA_RE`） |
| `expected` / `observed` | optional | string or null（独立 Oracle の期待値/観測値） |
| `device` / `endpoint` / `plugin` / `abi` / `sdk` / `arch` / `os_ver` | optional | string or null |

- optional 項目は常に key を出力し未提示時は `null`（`reason: null` の既存先例に倣い shape を固定する）。`resolved_ruviform_sha` のみ値がある場合 `SHA_RE` を要求。
- `outcome` は結果軸のまま維持する。Case schema の `result`（Pass/Fail/NotRun/Unsupported）に対応するが、lane 語彙は lowercase で `skipped`（derived record 専用）を含む既存契約を変えない — `contract` v1 の意味不変と `ci/tracker` pin の両立のため。`requirement` × `outcome` の直交は schema 層で強制（両者を別 field に保持し、どちらかの値域に他方を混入させない）。

新規 export: `LEVELS`・`ENVIRONMENT_KINDS`・`REQUIREMENTS`（enum 配列。`OUTCOMES` と同じ公開定数パターン）。

`makeLaneRecord` は新規必須項目の欠落・enum 外を throw し、optional 項目を検証付きで record へ載せる。`readLanes` は parse 後の各行へ同一 schema の fail-closed 検証を適用する（`lane` コマンド経由でない行も同じ vocabulary を満たす構造を維持）。

`lane` CLI は `--case-id`・`--level`・`--environment-kind`・`--requirement` を必須 flag とし、optional 各項目に対応する flag（`--oracle` `--fixture-version` `--resolved-ruviform-sha` `--expected` `--observed` `--device` `--endpoint` `--plugin` `--abi` `--sdk` `--arch` `--os-ver`）を追加する。

### 2. evidence JSON（`buildEvidence`）

- `schemaVersion: SCHEMA_VERSION` → 2。`contract`/`kind`/`run`/`candidate`/`consumer`/`accessMode`/`tuple`/`status`/`statusReason`/`coverage`/`generatedAt` の構造・意味は不変。`lanes[]` が v2 record を保持するだけで top-level schema の変更は `schemaVersion` のみ。
- `deriveStatus`/`coverage`/`markdownSummary` は `outcome` のみを読み、変更しない（summary 表の列追加はしない — JSON artifact が機械可読の正本）。

### 3. workflow（`consumer-verify.yml`）

両 verify job の `rec()` helper を case spec 引数へ拡張する。`rec()` の第1引数を `"<case_id>|<level>|<environment_kind>|<requirement>"` の spec 文字列とし、`IFS='|'` で分解して `lane` CLI へ渡す。これにより各静的 lane 行が4項目を1箇所で正直に宣言でき、行ごとの引数爆発を避ける。

```bash
rec() {
  # $1 = "<case_id>|<level>|<environment_kind>|<requirement>"
  # $2 = name, $3 = command, $4 = feature, $5 = outcome, [$6 = reason]
  local spec="$1"; shift
  local case_id level env_kind requirement
  IFS='|' read -r case_id level env_kind requirement <<< "$spec"
  ...node "$EV" lane ... --case-id "$case_id" --level "$level" \
      --environment-kind "$env_kind" --requirement "$requirement" [--reason ...]
}
```

- 既定分類は `OS-L1|deterministic-backend|Required` — ubuntu-latest 上の mise/cargo/UI headless lane・dep/pin/fetch/setup 等の infra lane・GUI-CONTRACT/SEMANTIC の headless contract lane・extra task が該当。各行は固有の `case_id` を宣言する（例: `al-check-full`、`sv-test-ui`、`al-extra-<task>`）。
- 非既定分類は `rec` の spec に明示値を書く: `GUI visual/OS + 実 window lanes` → `GUI-L2|real-window|Required`（audiolens/synvelis 共通）、`PLUGIN-NATIVE-DOCK (実 plugin binary)` → `EXT-L3|test-plugin|Required`（synvelis のみ）、`macOS/実 OS lanes` → `OS-L1|deterministic-backend|Required`（outcome=unsupported 不変）。
- GUI-CONTRACT boundary guard / GUI-SEMANTIC fake driver contract の lane（候補 tree 実行系）は `resolved_ruviform_sha` に `$RUVIFORM_REV` を記録する — `gui_candidate` step が `rev-parse HEAD == RUVIFORM_REV` を検証済みの実解決値であり、宣言値ではない。not_run/else 分岐の行も同一 case を指すため同値を記録する。これらの行のみ `rec` の後に `--resolved-ruviform-sha` を付与する第2 helper（`rec_cand`）または spec 拡張で対応する（実装時に簡潔な方を選択 — いずれも additive）。
- `requirement` の値根拠: POLICIES §2 の OS別表 — Windows で提供・利用する GUI 機能の GUI-L2 と EXT-L3（plugin/native dock）は Required、macOS/Linux の OS-L1 も Required。`unsupported`/`not_run` outcome と `Required` requirement の併記は「必須だが本 run では実行不能/未実行」の正直な直交記録であり、Pass 化ではない。
- extra task ループの直接 `lane` 呼出しにも `--case-id "<consumer>-extra-$t"`・`--level OS-L1`・`--environment-kind deterministic-backend`・`--requirement Required` を付与（task 名は preflight で `^[A-Za-z0-9][A-Za-z0-9_-]*$` 検証済みで case_id に安全）。
- finalize 失敗 fallback の `printf` は `"schemaVersion":2` に更新（`contract` は不変）。

### 4. fail-closed と downstream 非破壊

- `readLanes` の検証強化により、v2 必須項目を欠く行・enum 外の値・型不正の optional 項目・非 SHA の `resolved_ruviform_sha` は finalize を失敗させる。これは「`lane` コマンド経由でない行も同じ vocabulary を満たす構造」の維持であり、workflow 全経路が v2 を供給するため実 run では発火しない。
- `ci/tracker` contract test が import する `deriveStatus`/`OUTCOMES` の signature・戻り値・語彙は不変。`buildEvidence` の env 検証・status 導出・shaMatch 三値・secret 非転写も不変。
- `decide.mjs` が読む evidence JSON の field は全て不変（`schemaVersion` は未検証、`lanes` 未読）。

## Data and Control Flow

1. workflow の各 lane step が `rec()`/直接 `lane` で case spec 付き record を JSONL へ追記。
2. `finalize` が `readLanes` で全行を schema 検証 → `buildEvidence` が status 導出（`outcome` のみ参照・不変）→ `schemaVersion: 2` の evidence JSON・summary・`status=` を出力。
3. `decide.mjs --mode tuple` が artifact から `kind`/`contract`/`tuple`/`status` を回復 — v2 でも同一経路。

## Non-Functional Constraints

- additive のみ: 既存 field の改名・削除・意味変更なし。旧 schemaVersion 1 artifact の解釈は変えない。
- fail-closed: 未知 enum・必須欠落・型不正は記録時（`lane`）と読込時（`readLanes`）の両方で拒否。
- 決定的: `node --test` fixture test が全分岐を offline で pin。

## Risks and Trade-offs

| リスク | 影響 | 緩和 |
| --- | --- | --- |
| workflow の記録経路更新漏れで finalize が常時失敗 | 全 run が最小証拠 fail | yml contract test で全 `rec` 呼出しの spec 形状・直接 `lane` 呼出しの flag を pin。`test-ci-consumer-verify` が preflight 自己検証として走る |
| `requirement` 分類の恣意的付与 | Required/Deferred の誤記が受入判定を歪める | 分類規則を本設計に固定し、非既定行のみ spec に明示。outcome は不変のため status への影響なし（schema 層の宣言のみ） |
| optional 項目の形骸化 | oracle/fixture_version が永遠に null | 本 slice は enabler のみ — #628-#631 実装 slice が値を供給する契約として deferred 記録 |
| `schemaVersion` 2 と `contract` v1 の版ずれ | consumer が版対応を誤読 | `docs/development-environment.md` が「`schemaVersion` と別物の `contract`」と既に区別。PR 本文に後続同期要否を記録 |

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `contract` v2 bump + `ci/daily`・`daily-ci.yml`・`docs/development-environment.md` 同期 | daily 回復経路への波及は別 slice の責務 | Issue #294 後続 slice | 版ずれの恒久化（実害は小さいが混乱の種） |
| `oracle`/`fixture_version`/`resolved_ruviform_sha` の全 lane 実値記録 | 独立 Oracle・fixture version は #628-#631 の産物 | `scarecrow173/ruviform#628`-`#631`・Issue #294 | optional 項目が null のまま残る |
| Job Summary markdown への level/requirement 列追加 | 人間可読 summary の拡張は必須でない（JSON が正本） | 必要時の後続 slice | summary の情報量が JSON より少ないまま |

## Verification Notes

- [ ] `mise run test-ci-consumer-verify` が pass（新規・既存 fixture test を含む）
- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml` が pass
- [ ] `mise exec -- prek run --all-files` が pass
- [ ] `ci/tracker` の contract test（`deriveStatus`/`OUTCOMES` consumer）を含む `node --test ci/` 全体が pass — 範囲外ファイルの非破壊確認として補助実行
- [ ] `review-protocol` で blocker なし

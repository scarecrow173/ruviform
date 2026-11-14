---
id: "IMPL-034Sdv2CaseSchemaQ7m"
type: "impl"
status: "completed"
title: "Issue #294 first slice — consumer-verify Case schema v2 証拠契約拡張（schemaVersion 1→2・contract v1 維持）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/specs/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/designs/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/plans/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/tasks/issue-294-virtual-first-evidence-schema.md"
      - type: "doc"
        path: "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
    modified:
      - type: "file"
        path: "ci/consumer-verify/evidence.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/evidence.test.mjs"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "workflow"
        path: ".github/workflows/consumer-verify.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/issue-294-virtual-first-evidence-schema.md"
  implemented-by: []
  depends-on:
    - "../../designs/issue-294-virtual-first-evidence-schema.md"
    - "../../plans/issue-294-virtual-first-evidence-schema.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-294-virtual-first-evidence-schema.md"
    - "../../discovery/issue-294-virtual-first-evidence-schema.md"
    - "../../test-specs/issue-294-virtual-first-evidence-schema.md"
    - "../../designs/issue-627-virtual-first-verification-contract-design.md"
    - "issue-294-consumer-verify-tuple-evidence.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/issue-294-virtual-first-evidence-schema.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-294-virtual-first-evidence-schema.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #294 first slice — consumer-verify Case schema v2 証拠契約拡張

## Summary

`ci/consumer-verify/evidence.mjs` の lane/case record を virtual-first Case schema（`scarecrow173/ruviform#627` contract §2 — 着地済み `POLICIES.md` Effective 2026-11-15）へ additive に拡張した。`SCHEMA_VERSION` を 1→2 に bump し、`CONTRACT = "consumer-verify-tuple/v1"`・`outcome` 語彙・`deriveStatus`/`coverage`/`markdownSummary`・top-level tuple/run/consumer 構造は不変。各 lane record は必須 `case_id`/`level`（`OS-L1`/`GUI-L2`/`EXT-L3`）/`environment_kind`（5 値閉集合）/`requirement`（`Required`/`Deferred`/`NotApplicable` — 結果軸と直交）を保持し、optional `oracle`/`fixture_version`/`resolved_ruviform_sha`/`expected`/`observed`/`device`/`endpoint`/`plugin`/`abi`/`sdk`/`arch`/`os_ver` を string-or-null で受理する。`readLanes` は `lane` コマンド経由でない行にも同一語彙の fail-closed 検証を課し、schema-v1 行・未知 enum・型不正・非 40-hex `resolved_ruviform_sha` を拒否する。`.github/workflows/consumer-verify.yml` の全記録経路（`rec()` helper・extra-task 直接 lane・resolution tuple 行・finalize fallback `printf`）を更新し、fallback は `schemaVersion: 2` + `contract: consumer-verify-tuple/v1` を出力する。

## Context

- 実施タスク: `docs/tasks/issue-294-virtual-first-evidence-schema.md`（`docs/plans/issue-294-virtual-first-evidence-schema.md` 配下の単一 stream）
- 上流: `docs/specs/`・`docs/designs/`・`docs/test-specs/` の `issue-294-virtual-first-evidence-schema.md`、正本契約 `docs/designs/issue-627-virtual-first-verification-contract-design.md` §2 Case schema
- downstream 制約（変更禁止の実測確認済み）: `ci/daily/decide.mjs` は `kind`/`contract`/run/consumer/tuple/status のみ検証し `schemaVersion`・`lanes` を読まない → `contract` v1 固定で回復経路は不変。`ci/tracker` の contract test は `deriveStatus`/`OUTCOMES` を import し `outcome` 名と 5 値語彙を pin → 結果軸の rename・語彙変更は不可（`requirement` は別軸フィールドとして追加）。
- Issue レベル AC（3repo SHA tuple binding・consumer oracles・`scarecrow173/ruviform#628`-`#631` fixture version・実走検証）は本 slice 対象外 — schema は enabler のみ（spec/design の deferred 表に記録）。

## Implementation

### Changes

- `ci/consumer-verify/evidence.mjs` — `LEVELS`/`ENVIRONMENT_KINDS`/`REQUIREMENTS`/`OPTIONAL_LANE_FIELDS` を export 追加、`SCHEMA_VERSION` 2。`makeLaneRecord` に `caseId`/`level`/`environmentKind`/`requirement` 必須引数と optional 12 項目（camelCase→snake_case 変換、`""`→`null` 正規化、未知 option 拒否、`resolved_ruviform_sha` は 40-hex または null）を追加。`lane` CLI に `--case-id`/`--level`/`--environment-kind`/`--requirement`/`--oracle`/`--fixture-version`/`--resolved-ruviform-sha`/`--expected`/`--observed`/`--device`/`--endpoint`/`--plugin`/`--abi`/`--sdk`/`--arch`/`--os-ver` flag を追加。`assertLaneRecord` を新設し `readLanes` が全行へ同一 fail-closed 検証（name/outcome/case 4 項目 enum・optional 型・sha 形式）を適用。
- `ci/consumer-verify/tests/evidence.test.mjs` — 新規 11 test 追加（Case schema 群 9 + CLI 2）。schema v2 pin、必須 4 項目の snake_case 着地・欠落拒否、閉集合 enum 拒否（語彙 export の deepEqual 含む）、`requirement`×`outcome` 直交、optional 型検証・snake_case・null 正規化、`resolved_ruviform_sha` sha 形式、未知 option 拒否、CLI lane/finalize の v1 行拒否・flag 検証。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` — Case spec 形状 pin（全 `rec()` 第 1 引数が `"${CONSUMER_NAME}-<slug>|<分類>[|$RUVIFORM_REV]"`）、`DEF` 既定分類、candidate 束縛行（gui-boundary/gui-semantic ×3 分岐 = 6 行/ job）への `resolved_ruviform_sha`、直接 lane 呼出しの必須 flag、fallback `schemaVersion: 2`/`contract` v1、非既定分類（GUI-L2 real-window・EXT-L3 test-plugin・OS-L1 physical-device）が `unsupported` のまま pass を得ないこと。
- `.github/workflows/consumer-verify.yml` — 両 verify job の `rec()` を `"<case_id>|<level>|<environment_kind>|<requirement>[|<resolved_ruviform_sha>]"` spec 方式へ変更（`local`+`IFS='|' read` で分解、`$DEF="OS-L1|deterministic-backend|Required"` を job 内既定分類として定義）。全 53 `rec` 呼出しに `${CONSUMER_NAME}-<slug>` case_id と正直な分類を付与。resolution tuple 行・extra-task 直接 lane 呼出しに必須 flag を付与。finalize fallback `printf` を `schemaVersion: 2` に更新（`contract` は v1 のまま）。

### Case 分類の確定（workflow 全 lane）

- 既定 `OS-L1|deterministic-backend|Required`: preflight・dep access・manifest pin/fetch・lock/rev check・check/test・RT・headless deps・CLI smoke・UI headless（setup/check/test/build）・ui vendor・extra tasks（aggregate+個別）・workspace presence・tuple clone/guard・head drift・GUI-CONTRACT boundary/semantic（candidate tree 上の headless 契約検査 — fake driver/guard であり実 window ではない）。
- `GUI-L2|real-window|Required`（outcome `unsupported` 固定）: `GUI visual/OS + 実 window lanes` 両 job。
- `EXT-L3|test-plugin|Required`（outcome `unsupported` 固定）: synvelis `PLUGIN-NATIVE-DOCK (実 plugin binary)` のみ（audiolens に同 lane なし）。
- `OS-L1|physical-device|Required`（outcome `unsupported` 固定）: `macOS/実 OS lanes` 両 job。
- `not_run` 固定行（gui-consumer-side・task 不在 gui boundary/semantic・workspace presence）は `OS-L1|deterministic-backend|Required` — headless fixture 系であり実装未接続を正直記録。
- GUI-CONTRACT/GUI-SEMANTIC の 6 行/ job は spec 第 5 要素に `$RUVIFORM_REV` を渡し `resolved_ruviform_sha` を記録（preflight が 40-hex を検証済み・candidate step が `rev-parse HEAD` 照合済みの値）。

### Decisions During Implementation

- **`rec` spec 文字列方式**: design の第一候補どおり `"<case_id>|<level>|<environment_kind>|<requirement>[|<resolved_sha>]"` を第 1 引数化。53 呼出し全てに case_id+分類を literal/`$DEF` で明示する方が、位置引数追加（後方互換の曖昧さ・reason 省略時の引数ずれ）より fail-closed で読みやすい。`resolved_ruviform_sha` は GUI candidate 行のみに限定（run 全体は top-level tuple が既に束縛しており、candidate tree 評価の区別が意味を持つのはこの行のみ）。
- **case_id は `${CONSUMER_NAME}-<slug>`**: 両 job の job env `CONSUMER_NAME`（pinned env・GITHUB_ENV 偽装不可）から導出し、同一行を `replace_all` で対称更新。slug は lane の意味を表す lowercase-hyphen（`preflight`/`tuple-guard`/`gui-boundary`/`gui-real-window`/`plugin-native-dock`/`macos-os` 等）。
- **optional field の `""`→`null` 正規化**: `EXPECTED_SHA ?? "" || null` の既存慣行に合わせ、CLI が明示空文字を渡しても null に落とす。`readLanes` 側は `""` を malformed として拒否（producer は生成しない形を consumer が弾く非対称は fail-closed として許容）。
- **`OPTIONAL_LANE_FIELDS` は camelCase**: record キーは snake_case だが option 名は camelCase（`fixtureVersion`/`resolvedRuviformSha`/`osVer`）。初回 snake_case で定義したため `lane` CLI が全 optional flag を unknown option で拒否する不具合が `mise run test-ci-consumer-verify` で即検出され修正 — test 駆動で契約を pin できた。
- **exp 文書なし**: 探索は設計 doc + 本 IR の Decisions に記録する水準（PoC 計測・代替実装の実測比較を要する性質ではない）と判断し `docs/impl/exp/` は作成しない。

### Known Limitations（正直記録）

- **実 runner 検証は未実施（NotRun）**: workflow_dispatch lane の実走は post-merge の Coordinator 管轄。本 slice は schema/記録層のみで lane 実行内容・回数は不変。
- **`requirement` は現行 lane 全て `Required`**: `Deferred`/`NotApplicable` の実使用例はまだない（POLICIES 語彙の enabler であり、未使用機能の明示的 NotRequired 記録は後続 slice の consumer 側判断）。
- **optional 項目の大半は未 populate**: `oracle`/`fixture_version`/`expected`/`observed`/`device`/`endpoint`/`plugin`/`abi`/`sdk`/`arch`/`os_ver` は schema が受理するが workflow は `resolved_ruviform_sha`（GUI candidate 行）以外を未記録 — Issue レベル AC の tuple/Oracle/fixture binding は deferred。
- **`docs/development-environment.md` の `consumer-verify-tuple/v1` 証拠契約記述は未同期**: 別 Worker リースのため本 PR では編集しない — `schemaVersion` 2 と lane Case 項目の追記が後続同期を要する（PR 本文に記録）。

## Related Experiments

### Adopted

なし。

### Rejected

なし（exp 文書は作成せず — 上記 Decisions 参照）。

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| 証拠器 fixture + yml 契約 | `mise run test-ci-consumer-verify` | **82 pass / 0 fail**（node --test ci/consumer-verify/ — evidence.test.mjs + consumer-verify-yml-contract.test.mjs） |
| workflow 構文 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` | exit 0・エラー 0 |
| hygiene | `mise exec -- prek run --all-files` | 全 hook pass（trim/eof/yaml/toml/line-ending/bom/private-key/actionlint） |
| downstream consumer 非破壊（補助・canonical=mise） | `node --test ci/` | **333 pass / 0 fail**（tracker `deriveStatus`/`OUTCOMES` import・daily decide 系を含む） |
| 実 runner dispatch | — | **NotRun**（post-merge・Coordinator 管轄。schema 層のみの変更で lane 実行内容は不変） |

## Risks

- **`rec` spec の shell 分解は fail-open ではなく fail-missing**: spec が malformed（field 不足・未知 enum）だと `evidence.mjs lane` が exit 1 → `set -uo pipefail`（`-e` なし）で当該行のみ欠落 → finalize は残行で status 導出。行欠落は pass 水増しにならない（lane 減少→not_run/unsupported 化方向）が、記録網羅性は yml contract test の `rec "` spec 形状 pin で担保。
- **`"${rf[@]}"` 空配列展開**: bash 4.4+ では `set -u` 下でも安全（ubuntu-latest = bash 5）。spec 第 5 要素なしの行は flag 自体を渡さない。
- **v1 artifact との混在**: `readLanes` は v2 schema のみ受理するため、v1 JSONL を finalize に食わせると fail — これは意図（旧 artifact の意味は変えず、新 schema で再記録された run のみが v2 証拠を出す）。top-level `schemaVersion: 2` は additive bump で `ci/daily` は未検査のまま通過。

## Follow-ups

- `docs/development-environment.md` の証拠契約記述の後続同期（別 Worker リース — `schemaVersion` 2・lane Case 項目・`rec` spec 方式の記録）。
- Issue レベル AC: 3repo SHA tuple binding・consumer Oracle・`scarecrow173/ruviform#628`-`#631` fixture version・Windows L2/L3・macOS/Linux L1 の実測接続（本 slice は enabler のみ）。
- post-merge の consumer-verify dispatch による v2 証拠 artifact の実走確認（Coordinator 管轄）。
- `requirement` の `Deferred`/`NotApplicable` 実使用は consumer 側 fixture 実装（AL #166/#127・SV #175-177）と連動。

## Links

- Issue: <https://github.com/scarecrow173/ruviform/issues/294>
- 契約正本: `docs/designs/issue-627-virtual-first-verification-contract-design.md`（`scarecrow173/ruviform#627` Closed）
- 関連: `docs/impl/ir/issue-294-consumer-verify-tuple-evidence.md`（本 slice の拡張元 tuple 証拠契約）

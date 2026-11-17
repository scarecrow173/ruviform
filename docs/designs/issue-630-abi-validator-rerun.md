---
id: "DESIGN-630abiValidatorRerun03"
type: "design"
status: "approved"
title: "Issue #630 外部 ABI validator 再実走 slice — 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/discovery/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-abi-validator-rerun.md"
  implements:
    - "docs/specs/issue-630-abi-validator-rerun.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator.md"
  blocks:
    - "docs/plans/issue-630-abi-validator-rerun.md"
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator 再実走 slice — 設計

## 方針

既存 runner（`ci/plugin-abi-validator/run.mjs`、schema /1）の
leg モデルと fail-closed verdict 集約を維持し、「意図的に作れ
ない surface」を宣言的データとして追加する。判定ロジックは
runner 内の純粋関数（`normalizeClapLeg` /
`applyVst3DeclaredUncreatable` / `clapLegExclusions`）に集約し、
contract test で固定する。

## CLAP 側: `expectCreateDecline`

`CLAP_PLUGINS` の plugin 定義に optional な
`expectCreateDecline: { pattern, reason }` を追加する
（`test_spawn_fail` のみ）。`normalizeClapLeg` は JSON report が
parse できた場合に限り以下を評価する:

- `leg.failedCases` の各 case について、`code === "failed"` かつ
  `details` が `pattern`（`Could not create the plugin instance|
  returned a null pointer`、case-insensitive）に一致するものを
  `declined`、それ以外を `unexpected` に分類する。
- `counts.crashed === 0 && declined.length > 0 &&
  unexpected.length === 0` のときのみ leg を Passed とし、
  `leg.expectation = { kind: "create-declined", reason,
  declinedCases, unexpectedCases(最大10), crashedCases }` を記録。
- 条件を満たさない場合は Failed。`proc.error`（spawn 自体の失敗）
  は expectation より先に Failed。JSON が無い場合は expectation
  を適用せず generic verdict 経路へ落ちる（tool 側異常を
  expectation で隠さない）。

## CLAP 側: library-level exclusion

`CLAP_FIXTURE_LIBRARY_EXCLUDES` を新設し、clap target の
`excludes` として保持する。`clapLegExclusions(target, plugin,
{ includePlugin })` が target-level と plugin-level の除外を
`{pattern, reason}` の配列に合成する。

- `preset-discovery-(crawl|load)` は fixture target のみに適用
  （production CLAP target の `excludes` は空 — production adapter
  に preset-discovery は無く、仮に生えても tool 側 deadlock は
  別問題として顕在化させる）。
- 各 leg の `exclusions` フィールドに適用した全除外と理由を記録
  （旧 `leg.exclusion` 単数形を複数形配列に置き換え）。
- informational unfiltered fault leg は `includePlugin: false`
  で fault plugin 固有の param 除外のみ解除し、library 除外は
  維持する（deadlock する case を informational leg で再発火
  させない）。

## VST3 側: `VST3_DECLARED_UNCREATABLE`

`{ "<vst3Package>": [{ className, reason }] }` の宣言 table を
追加し、`ruviform-plugin-test-vst3` のみに `test_spawn_fail` を
登録する。`applyVst3DeclaredUncreatable(leg)` は leg の
`rawOutput`（evidence 書き出し前に呼ぶ）から
`Failed to create component instance of <class>!` marker の
出現回数を数え、`leg.declaredUncreatable` に記録する。

- occurrences > 0 が期待値。declared class の occurrences が
  0 で leg.status が Passed の場合は Failed に倒し理由を
  `leg.details` に追記する — 「拒否されるはず」を観測なしに
  Pass にしない。
- `leg.package` が宣言 table に無い場合（production adapter を
  `--vst3-dll`/`--vst3-package` で渡した場合）は no-op。
- suite validator の `Result:` 集計自体は authoritative のまま
  （declaredUncreatable は証拠記録であり verdict を緩めない）。

## evidence schema /2

`run.json` の `schema` を `rf630-abi-validator-run/2` に bump。
leg の追加フィールド:

- `exclusions: [{pattern, reason}]`（旧 `exclusion` を置換）
- `expectation: {kind:"create-declined", reason, declinedCases,
  unexpectedCases[], crashedCases}`（CLAP create-decline leg のみ）
- `declaredUncreatable: [{class, occurrences, reason}]`（VST3
  fixture leg のみ）

consumer は schema version で判別する。rawOutput の txt 分離・
tools/artifacts の SHA-256・build 記録・evaluation 集約の規約は
/1 から不変。

## 失敗時の倒れ方（fail-closed 維持）

| 状況 | 結果 |
| --- | --- |
| validator binary/artifact 不在・build 失敗 | NotRun・exit 非 0（/1 と同じ） |
| spawn error / JSON 無し / 非 0 exit で report 無し | Failed/Unsupported（expectation 非適用） |
| spawn_fail leg で unexpected failed・crashed > 0・decline 0 | Failed |
| preset-discovery 以外の新規 failed case | Failed（exclusion は宣言 pattern のみ） |
| VST3 `Result:` に failed > 0 | Failed（declaredUncreatable は緩和しない） |
| VST3 declared class の decline 証拠 0 | Passed→Failed |

## リスクと対応

- **exclusion の広がりすぎ**: pattern は `preset-discovery-(crawl|
  load)` に限定し、`preset-discovery-descriptor-consistency` は
  実行される。pin 更新時に除外を再評価する旨を reason に明記。
- **expectation が真の defect を隠す**: pattern は create_plugin-
  null signature に限定。process/state/param 系の failed は
  unexpected として Failed。decline 0 件も Failed。
- **VST3 marker 文字列の変更**: SDK validator の出力 format が
  変われば occurrences=0 → Failed（fail-closed）。
- **informational leg の再 deadlock**: unfiltered leg でも
  library exclusion を維持する設計で回避済み。

---
id: "DSC-630abiValidatorRerun01"
type: "discovery"
status: "final"
title: "Issue #630 外部 ABI validator 再実走 slice — post-#735 surface の調査と declared-expectation 設計判断"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-abi-validator.md"
    - "docs/discovery/issue-630-preset-spawn-fault.md"
    - "evidence/rf630/test-preset-spawn-fault-verification.md"
  related:
    - "../specs/issue-630-abi-validator-rerun.md"
    - "../specs/issue-630-abi-validator.md"
    - "../specs/issue-630-preset-spawn-fault.md"
---

# Issue #630 外部 ABI validator 再実走 slice — 調査記録

PR#735（`5b2dbe00` — test_stateful preset surface + spawn/pre-Hello
fault fixtures）merge 後の ABI surface に対して外部 validator を
再実走するにあたり、旧 runner（schema
`rf630-abi-validator-run/1`）の前提が stale になった点と、実走
プローブで確認した validator 側の挙動を記録する。

## post-#735 で変化した ABI surface

| surface | CLAP | VST3 |
| --- | --- | --- |
| preset | `clap.factory.preset-discovery` + `clap.ext.preset-load`（catalog 4 preset: Default/Half/Boost/Mute、`RFST` state codec 共有） | `IUnitInfo`/`IUnitHandler`/`IUnitHandler2`（controller）+ `IProgramListData`/`IUnitData`（component） |
| spawn fault | `test_spawn_fail` descriptor（`org.ruviform-test.clap-test-spawn-fail`）— factory `plugin_count()==6` に列挙されるが `create_plugin` が意図的に `None` を返す | `test_spawn_fail` class — `CLASS_ROWS` 11 class に列挙されるが `createInstance` が `kInvalidArgument` で拒否 |
| pre-Hello fault | L1 `--ruviform-test-fault=MODE`（exit/abort/hang/stderr/bad-fixture） | 同左 |

preset-spawn-fault slice の spec/design/verification では「外部 ABI
validator の preset surface 検証は別 slice・NotRun」と明示的に
deferral されており、本 slice がその残件を回収する。

## 実走プローブで確認した validator 挙動

### CLAP（clap-validator 0.4.1、pin binary）

- `test_spawn_fail` を含む全 leg で、plugin-instance case が
  `Could not create the plugin instance ... returned a null pointer`
  signature の failed として大量に報告される（test_spawn_fail leg
  では 35 件）。これは fixture の意図仕様であり defect ではないが、
  旧 runner は全 failed case を一律 Failed として扱うため leg が
  fail する。
- library-level の `preset-discovery-crawl` / `preset-discovery-load`
  case が fixture の preset-discovery factory 登録を検出して実行
  されるが、clap-validator 0.4.1 内部で 45 秒 timeout + JSON report
  無しの異常終了を起こす。upstream 既知問題
  （free-audio/clap-validator issue #41 — MetadataReceiver の
  self-deadlock、0.4.1 pin 以降の master で修正済み）と判断。
  timeout した sandboxed child が DLL をロックしたまま残存する
  副作用も確認し、プロセスを手動で終了させた。
- `preset-discovery-descriptor-consistency` は正常終了し catalog を
  検証するため、crawl/load のみ除外しても preset surface の外部
  検証が完全に空になるわけではない。

### VST3（Steinberg SDK validator、VST 3.8.1 / SDK commit `3cdf9ca`）

- suite validator は bundle 内全 class を scan→`createInstance` する。
  `test_spawn_fail` に対し `Failed to create component instance of
  test_spawn_fail!` / `Failed to setup test!` を反復出力するが、
  これらの setup 失敗は `Result: N tests passed, M tests failed`
  の集計に含まれない。最終結果は `235 passed, 0 failed`（exit 0）。
- 意図的 create 拒否が run.json 上で不可視になるため、declared
  記録として出現回数を別途集計する必要がある。

## 設計判断（declared-expectation 機構）

fail-closed 原則を維持したまま「意図的に作れない surface」を扱う
ため、test_fault の `param-(set|fuzz)` 除外と同型の宣言的機構を
導入する:

1. **CLAP `expectCreateDecline`**: plugin 定義に「create 拒否が
   期待される」宣言を持たせ、leg 正規化時に failed/crashed case
   が全て宣言 pattern（create_plugin-null signature）に一致し、
   crash 0・decline 1 件以上の場合のみ Passed とする。それ以外は
   Failed — 予期しない拒否や crash は従来どおり defect signal。
2. **CLAP library-level exclusion**: `preset-discovery-(crawl|load)`
   を target 単位の宣言的除外とし、理由（upstream issue #41・pin
   version 条件）を leg.exclusions に記録。plugin 固有除外
   （test_fault の param）と合成する `clapLegExclusions` を追加。
   informational unfiltered fault leg では plugin 固有除外のみ
   解除し library 除外は維持する。
3. **VST3 `VST3_DECLARED_UNCREATABLE`**: bundle package 名を
   key に「列挙されるが作れない class」を宣言し、raw output 中の
   decline marker 出現回数を `leg.declaredUncreatable` に記録。
   宣言 class の decline 証拠が 0 件なら Passed→Failed に倒す
   （oracle が拒否を観測したことを要求 — 「拒否されるはず」を
   仮定で Pass にしない）。production adapter（`--vst3-dll`/
   `--vst3-package` 経路）には適用しない。
4. **schema bump**: leg に `expectation`/`declaredUncreatable`/
   `exclusions`（複数形）フィールドが増えるため run.json schema を
   `rf630-abi-validator-run/2` に更新。

## スコープ境界の確認

- `crates/` 配下は本 slice では一切編集しない（fixture・production
  adapter とも zero-diff）。validator の指摘が fixture 契約違反を
  示した場合は `ask` で占有拡張を求める方針だったが、実走では
  契約違反は検出されなかった。
- preset-discovery crawl/load の外部検証は「実施していない」ので
  あり Passed ではない — 除外理由・upstream 参照・fixture 側
  in-process ABI test による代替証拠を明記し、validator pin 更新時
  に再評価する。
- consumer verdict（AC7）・実 Window lane（AC4）は本 slice 対象外。

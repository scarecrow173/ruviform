---
id: "DESIGN-726clapValidatorPanic03"
type: "design"
status: "approved"
title: "Issue #726 clap-validator param-fuzz panic — 設計と判断記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-726-clap-validator-panic.md"
    - "docs/discovery/issue-726-clap-validator-panic.md"
    - "evidence/rf726/investigation.md"
    - "evidence/rf726/upstream-report-draft.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-726-clap-validator-panic.md"
  implements:
    - "docs/specs/issue-726-clap-validator-panic.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-726-clap-validator-panic.md"
    - "docs/discovery/issue-726-clap-validator-panic.md"
  blocks:
    - "docs/plans/issue-726-clap-validator-panic.md"
  supersedes: []
---

# Issue #726 設計 — clap-validator param-fuzz panic の判断記録

## 設計方針

本 slice は調査・判断系であり、コード変更は「runner 契約の違反が
実測で確認された場合のみ」に限定する。まず `--include`/`--exclude`
filter で panic を単独再現まで絞り込み、upstream source（master
`5bc2ef3`）と照合して根因の所在を確定し、その結果から leg 扱い・
upstream 報告・再開条件を決める。実測と source 照合が揃うまで
runner には触れない。

**追記（lane 実走後）**: 契約違反が実測で確認されたため、
`run.mjs`/`run.test.mjs` に限定した runner 修正を実施した
（下記「runner 契約の照合結果」参照）。leg 定義・exclude 条件・
verdict 語彙は変更していない。

## 調査設計（実施済み・結果は investigation.md）

1. **絞り込み行列**: `clap-validator validate --json --plugin-id
   org.ruviform-test.clap-test-fault --include <regex> <dll>` を
   `param-fuzz-basic` / `param-fuzz-modulation` / `param-fuzz-sample-accurate` /
   `param-fuzz-bounds` / `param-set` / `preset-discovery` に適用し、
   対照として test_gain の `param-fuzz-basic` と、killer 2 件を除く
   unfiltered 相当 run を実行。各 raw stdout/stderr を
   `evidence/rf726/filter-runs/` に保存。
2. **機構の確定**: panic するのは「sandboxed child が exit 0 で結果
   未出力」になる経路のみ。fixture の `exit` mode が plain `2.0` で
   exit code 0 を返し、validator fuzzer の stepped 値生成
   （`rng.rs` の `.round()`）が `2.0` を必ず射程に含める。
   `param-fuzz-sample-accurate` は abort（非 0 終了）を踏むため
   `SandboxError::Crashed` として正常処理される — Issue タイトルの
   帰属は誤り。
3. **upstream 照合**: master HEAD の `sandbox.rs`:98 `.expect()` と
   `cli.rs` `parallel_fork_join` の scoped-thread 伝播が未修正である
   こと、issue list に同件がないことを確認。

## 判断記録（AC2-AC4 への回答）

### AC2 upstream 報告要否 → **報告する（ドラフト作成済み）**

根拠:

- bug は validator 自身の堅牢性欠陥であり、任意の plugin が
  sandboxed child を status 0 で終了させた場合に再現する
  （fixture は reproducer に過ぎない）。validator は "This is a bug
  in the validator" と自ら panic を認めている。
- master HEAD に未修正で残存、既存 Issue/PR になし。
- 報告文ドラフト: `evidence/rf726/upstream-report-draft.md`。
  **実起票済み**: https://github.com/free-audio/clap-validator/issues/44
  （Coordinator がドラフト内容を確認のうえ投稿）。

### AC3 leg の扱い → **現行維持（leg 定義は無変更）**

- canonical `test_fault` leg の `exclude: "param-(set|fuzz)"` を維持。
  param-mutating case が fixture の意図的 fatal mode を踏んで
  sandboxed child を殺すのは設計どおりであり、panic は validator
  bug であって fixture/adapter の欠陥ではない。fatal surface の
  実検証は L1 out-of-process fault lane が担う。
- informational unfiltered leg を維持: `Unsupported` + details で
  「intentional crash surface × tool-side termination」を正直記録し、
  raw 出力を `clap-test_fault-unfiltered.txt` に保存。
- `param-fuzz-(basic|modulation)` のみを外す細工は不採用 — 引き金は
  「exit(0) を踏む任意の param-mutating case」であり、特定 test 名に
  依存した filter は脆い。param 系全除外が安定した契約。
- pin 変更は upstream fix 入り release 待ち（能動的に pin を動かす
  release が存在しない）。

### AC4 無フィルタ run 再開条件

informational unfiltered leg（実質 test_fault のみが対象 — 他 plugin
は canonical leg が既に無フィルタ）が再開される条件:

1. clap-validator の panic 修正（success exit + 無出力を crashed/failed
   として扱い、worker panic を per-test に隔離して JSON を出力）を
   含む release が公開される。
2. runner の pin/provision 対象をその release に更新する
   （`docs/specs/issue-630-abi-validator.md` の pinned release 手順）。
3. unfiltered leg を再実行し、JSON 出力に per-test の
   crashed/failed 記録が含まれることを確認する。

それまでは informational leg が `Unsupported` を記録し続けるのが
正直な状態であり、canonical legs の verdict には影響しない。

## runner 契約の照合結果 → **契約違反を実測で確認し修正**

`normalizeClapLeg`/`evaluate` の verdict ロジック自体は要求契約を
満たす（下表）。しかし lane 実走で **stdio 捕捉層の契約違反**を
確認した:

| 契約 | 実装 | test |
| --- | --- | --- |
| validator が JSON 無しで異常終了 → fail-closed | `normalizeClapLeg`: `status!==0 && !counts` → `Unsupported`（tool-side termination） | `run.test.mjs`:161-172（status 101 + panic stderr の signature を pin） |
| run.json は必ず出力 | `run()` が legs 評価後に必ず `run.json` を write | `run.test.mjs`:267-285 |
| informational leg は verdict に不介入 | `canonicalLegs()` が `informational` を除外 | `run.test.mjs`:258-263 |
| Unsupported を Pass にしない | `evaluate()`: canonical Unsupported → verdict Unsupported・exit 1 | `run.test.mjs`:252-256 |

**違反**: `runClapLeg` が `spawnSync` の pipe stdio を使用。validator
の sandboxed child は stdio を継承し、preset-discovery deadlock
（0.4.1・upstream issue 41 未収録）で wedge した orphan が pipe を
保持し続けるため、validator 完走・JSON 出力済みにもかかわらず
spawnSync が 300s timeout の `ETIMEDOUT` を返し、`proc.error` 優先
評価で **完全な JSON report を捨てて Failed を記録**していた。
informational unfiltered leg も本来の `Unsupported — validator
exited 101 without JSON`（panic 契約）ではなく
`Failed — spawn error ETIMEDOUT` と誤記録されていた。

**修正**: `spawnCapture`（file-backed stdio の spawnSync wrapper）を
追加し `runClapLeg` に適用。orphan 孫 process が stdout/stderr の
file handle を保持しても、spawnSync は direct child の終了で復帰
する。`runVst3Leg` は対象外（Steinberg validator は sandbox
grandchild を spawn しない。同型パターンは follow-up 候補）。
contract test 2 件追加: stdio 継承 grandchild がいても child exit
で復帰すること、および spawn 失敗・timeout が引き続き fail-closed
であること。

leg 定義・exclude 条件・verdict 語彙は**無変更**。filter を runner
経路に組み込む案も検討したが、panic の引き金が validator 版依存で
変わり得るため、現行の param 系全除外 + informational 記録が最も
安定で、追加の leg filter 機構は YAGNI。

## 副次観測の扱い

- **preset-discovery crawl/load の 45s timeout が全 fixture leg を
  Failed にする回帰を確認**: rf630/rf732 evidence では同 test は
  skipped だったが、現在は test_stateful の preset surface 実装後に
  実走し、0.4.1 の upstream deadlock（issue 41・master 修正済み・
  未リリース）で crashed ×2/leg となる。panic とは別件の upstream
  bug であり、fixture の意図的 crash surface ではないため
  **exclude せず Failed を正直に記録**する（param-fuzz との違い:
  あちらは fixture 設計どおりの fatal surface、こちらは validator
  側 bug による未検証状態）。upstream release での pin 更新が
  解消条件。`descriptor-consistency` は success で preset discovery
  の一部検証は維持される。
- sandbox `wait_timeout` が timed-out child を kill しない:
  orphan child が DLL を lock して `cargo build` を失敗させる事象を
  実測（本調査で orphan `clap-validator.exe` 14+ 個を kill）。
  upstream 報告ドラフトに併記。runner 側の対処は「build 失敗時に
  orphan を確認・kill」という運用注意に留め、runner コードへの
  kill ロジック追加は行わない（外部プロセスの選択的 kill は危険で、
  timeout は validator 内部の責務）。

## 検証メモ

- `node --test ci/plugin-abi-validator/tests/`: 23 tests pass
  （baseline 21 + spawnCapture 2 件）。
- opt-in lane 実走（`--clap-validator … --clap-production
  --evidence-dir evidence/rf726/abi-validator-fixed`）の結果は
  `evidence/rf726/abi-validator-fixed/run.json` と impl 記録に記録。
  修正前 runner の実走（ETIMEDOUT による誤記録の再現証拠）は
  `evidence/rf726/abi-validator/run.json` に保存。

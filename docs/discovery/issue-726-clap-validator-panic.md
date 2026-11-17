---
id: "DSC-726clapValidatorPanic01"
type: "discovery"
status: "final"
title: "Issue #726 clap-validator 0.4.1 param-fuzz panic — 根因所在と upstream 報告要否の調査"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/726"
    - "evidence/rf630/abi-validator/clap-test_fault-unfiltered.txt"
    - "evidence/rf732/abi-validator/clap-test_fault-unfiltered.txt"
    - "evidence/rf726/investigation.md"
    - "docs/specs/issue-630-abi-validator.md"
  related:
    - "../specs/issue-726-clap-validator-panic.md"
---

# Issue #726 clap-validator 0.4.1 param-fuzz panic 調査

rf#630/#732 の ABI-validator lane で観測された clap-validator 0.4.1 の
panic（plugin child exit `0xc0000409` → `a scoped thread panicked` →
validator exit 101・JSON 未出力）の根因所在・最小再現・upstream 報告
要否・leg 扱いを確定するための調査記録。生データと判定表は
`evidence/rf726/investigation.md` と `filter-runs/` にある。

## 監査対象と一次情報

- validator: clap-validator **0.4.1**
  （`%LOCALAPPDATA%/ruviform/plugin-abi-validator/clap-validator.exe`、
  sha256 `825439ad…53e38a`）。upstream source は
  `free-audio/clap-validator` @ master `5bc2ef3`（浅い clone）を照合。
- fixture: `crates/ruviform-plugin-test-clap`（`test_fault`、
  plugin id `org.ruviform-test.clap-test-fault`）— read-only。
  `Fault` param は stepped `[0.0, 4.0]` で plain `2.0` が
  `std::process::exit(0)` に相当する exit mode。
- runner: `ci/plugin-abi-validator/run.mjs` — canonical leg は
  test_fault に `--exclude param-(set|fuzz)`、unfiltered は
  informational leg として `Unsupported` 記録。

## 絞り込みで確定した事実

`--include`/`--exclude` regex filter による単独・対照実行
（`evidence/rf726/filter-runs/`）:

1. **`param-fuzz-basic` / `param-fuzz-modulation` 単独で panic 再現**
   （~15ms・exit 101・stdout 0 byte）。`param-fuzz-sample-accurate`
   単独は `crashed: 0xc0000409` を正常記録して exit 1 で完走 —
   **Issue タイトルの帰属は誤り**で、実際の killer は stepped 値
   `2.0`（exit code 0）を生成する param-fuzz 系テスト。
2. **対照の test_gain は同じ `param-fuzz-basic` で success** —
   panic は test_fault の fault surface 固有の interaction。
3. **panic 連鎖の正確な形**: fixture が `exit(0)` で sandboxed child を
   「成功終了・結果未出力」にする → `run_sandboxed`
   （upstream `src/cli/sandbox.rs`:91-98）が `status.success()` を
   `Crashed` と誤認せず結果ファイルを `.expect()` deserialize →
   panic → `parallel_fork_join`（`src/cli.rs`:55）の scoped worker
   panic が main に伝播 → validator 全体が JSON 無しで死亡。
   `-j1` では worker 無しで main が直接 panic（切り分け証拠）。
4. **非 0 終了は正しく処理される**: abort（`0xc0000409`）は
   `SandboxError::Crashed` → per-test `crashed` 記録・JSON 出力・
   exit 0。bug は「success exit + 無出力」の経路に限定される。

## 代替案の比較

| 案 | 判定 | 根拠 |
| --- | --- | --- |
| fixture (`test_fault`) の bug | 棄却 | fault param が fatal mode を持つのは rf#630 の設計どおり（L1 fault lane の oracle）。param fuzzer が stepped 値を生成するのも validator の正常動作 |
| validator bug（panic 伝播） | **採用** | upstream source に `.expect()` panic と scoped-thread 伝播が master HEAD に残存。非 0 終了は処理できるのに success+無出力で全結果を失うのは明確な tool bug |
| runner (`run.mjs`) の bug | 棄却 | panic signature（exit 101 + JSON 無し）は `normalizeClapLeg` が Unsupported に fail-closed 記録済み。contract test `run.test.mjs`:161 が pin 済み |

## upstream 照合結果

- 既存 Issue/PR に同件なし（open は #43 のみ）。#41（preset deadlock
  fix）は master source に存在するが 0.4.1 binary では preset-discovery
  crawl/load が 45s timeout → binary は #41 前の build と整合。
  #42（param-set-* test bug）も master 修正済み・0.4.1 未収録で、
  param-set-* failed の症状と一致 — panic とは切り分ける。
- master HEAD `5bc2ef3` の `sandbox.rs`:98 `.expect()` と
  `cli.rs` scoped-thread 伝播は**未修正** → upstream 報告が必要。
  報告文ドラフト: `evidence/rf726/upstream-report-draft.md`。
  実起票は worker 権限外 → Coordinator へ hand-off。

## 暫定結論（spec へ promotion）

- leg の扱い: **現行維持**（canonical `param-(set|fuzz)` exclude +
  informational unfiltered leg の Unsupported 正直記録）。runner zero-diff。
- 無フィルタ run 再開条件: panic 修正入り release への pin 更新後、
  informational leg で JSON 出力（per-test crashed/failed）を確認。
- 残る未解決事項: upstream Issue の実起票（Coordinator step）と
  その URL の docs 追記（残 gate）。

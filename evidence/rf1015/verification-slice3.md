# rf#1015 slice 3 検証記録 — Structured Documents legacy exception 解消

## 対象と SHA の役割

- issue: `scarecrow173/ruviform#1015` slice 3。Issue は close しない。
- branch: `test/1015-structured-proof-redesign`。
- base SHA: `527ddd7c1eb97377b12aee44772dfeafe6cc40c9`（最終検証前に取り込んだ `origin/main`）。
- verified Sync Commit: `f6517d639a66b3f2301f92473a70ba7d84e4abf0`（最新 main を取り込んだ merge commit。製品/proof/mise の最終変更 commit `01b369a452c69521149de762c5278b5ac331e8ea` を含み、以降は docs/skill/evidence-only とする）。
- 初期 evidence sync commit: 本ファイルと ledger/spec/skill 同期を含む後続 commit で確定する。evidence は自分自身の commit hashを固定点として書けないため、PR本文で最終 headを追跡する。

製品/proof/mise の変更 commit:

1. `ec3bf6ac` — CSV limit decision core抽出と in-src proof。
2. `2b4824ba` — `sd-csv-limits` / `sd-byte-budget` selective entry追加と structured invocationの `--no-unwinding-checks` 除去。
3. `01b369a4` — `BoundedRead` の内部 error構築簡素化と proof loop/unwind bound の再設定。

## 実施した再設計

CSV の `max_records` / `max_fields` / `max_field_bytes` / `max_record_bytes` 判定を `csv.rs` 内の crate-private な pure decision coreへ分離した。coreは allocation、I/O、`csv_core`/DFA、診断構築を持たない。`CsvReader` は parserが確定した record番号、field span、累積 fill countと現在 capacityを coreへ渡す。limit enforcementの公開API、default値、error kind、limit name、発生位置、record/field境界、cursor整合は変えていない。

旧 parser-facing Kani harness `csv_max_records_gate` / `csv_max_field_bytes_gate` は退役した。これは失敗隠しではなく Property起点の redesignである。旧保証は parser経由 end-to-endだったが Unwinding Assertions無効と concrete skeletonに依存し、`build_dfa` を proof boundaryに含んでいた。新保証は production decision coreの symbolic exactnessを Unwinding Assertions有効で示すが、parserが coreを毎回呼ぶ接続条件は Kani未証明であり、既存 proptest/integration coverageに依存する。任意 record内容、複合 limit交互作用、`csv_core` correctnessも未証明領域として残る。

`BoundedRead` では公開挙動を変えず、limit超過時の内部 `io::Error` を payloadなしの `ErrorKind::Other` へ簡素化した。`was_limit_exceeded` が authoritativeであり、各 readerは limit超過を `DocumentErrorKind::LimitExceeded` へ再分類する。proof loopは 6 byte入力・2 byte/readから static最大4回へ絞り、`#[kani::unwind(8)]` に余裕を持たせた。`--no-unwinding-checks` は使っていない。

## 環境

- canonical lane runner OS: Windows 11 x86_64。
- Kani execution environment: WSL2 Ubuntu。
- Kani Rust Verifier 0.68.0、CBMC 6.11.0。
- canonical verdictは mise lane。直接 `cargo kani` と `/usr/bin/time` は補助証拠（canonical = mise）である。

## 実行した正規検証

最終検証対象は verified Sync Commit `f6517d639a66b3f2301f92473a70ba7d84e4abf0` とする。以下の表は最終確認時の記録であり、docs/evidence-only commitは verified Sync Commitを進めない。

| lane | command | exit | verdict | 記録 |
| --- | --- | ---: | --- | --- |
| selective Kani CSV | `mise run test-formal-kani sd-csv-limits` | 0 | pass | 2026-11-26T11:55:50Z–11:55:57Z。3/3 harness成功。record count 0/3 failed・cover 3/3・0.074 s。buffer growth 0/58 failed・cover 4/4・0.124 s。field bytes 0/5 failed・cover 3/3・0.098 s。Unwinding Assertions有効 |
| selective Kani byte budget | `mise run test-formal-kani sd-byte-budget` | 0 | pass | 2026-11-26T11:55:57Z–11:56:16Z。1/1 harness成功、0/357 failed（6 unreachable）、verification time 11.452 s。Unwinding Assertions有効 |
| selector negative smoke | `mise run test-formal-kani invalid-selector` | 2 | pass | 2026-11-26T11:56:16Z。Kani実行前に usage error。期待どおり exit 2 |
| fmt | `mise run fmt-check` | 0 | pass | 最終検証で実行 |
| check | `mise run check ruviform-io-structured --all-features` | 0 | pass | 最終検証で実行 |
| lint | `mise run lint ruviform-io-structured --all-features` | 0 | pass | 最終検証で実行 |
| crate regression | `mise run test ruviform-io-structured --all-features` | 0 | pass | 最終検証で実行 |
| property test | `mise run test-formal` | 0 | pass | 最終検証で実行。`formal_structured` 35件を含む73件 |
| prek | `mise exec -- prek run --all-files` | 0 | pass | 2026-11-26T11:56:54Z–11:56:57Z。全 configured hooks pass、auto-fix差分なし |

補助 resource観測（canonicalではない）:

| 対象 | user CPU time | wall clock | max RSS |
| --- | ---: | ---: | ---: |
| `cargo kani ... --harness csv_limit_decision` | 0.92 s | 4.09 s | 155,172 KiB |
| `cargo kani ... --harness bounded_read_budget_gate` | 14.35 s | 18.92 s | 724,640 KiB |

事前予算との関係: CSV decision core 8 CPU-min/2 GiB、BoundedRead 5 CPU-min/1 GiB。どちらも予算内である。Kaniは unsupported construct warning（`caller_location`、foreign function）を出したが、verificationは成功し、到達した unsupported constructによる failureではなかった。warningを成功条件として使わず、`SUCCESSFUL` verdictと failure/cover数を証拠にした。

## Mutation-fail と revert

実施した mutantは decision coreの field bytes比較を `end - start > max_field_bytes` から `end - start >= max_field_bytes` へ変える1箇所だけである。transient mutantは commitしていない。

- mutation run: `mise run test-formal-kani sd-csv-limits`、2026-11-26T11:38:01Z–11:38:07Z、exit 1。
- 検出: `csv_limit_decision_field_bytes_exactness` が `1 of 5 failed`。失敗は `assert!(actual == (len > max))` の semantic violationであり、unwinding assertion failureではない。他2 proofは成功。
- revert: 該当行を `>` へ戻し、`git diff` が空であることを確認した。
- revert後再検証: 2026-11-26T11:55:50Z–11:55:57Zの canonical runが exit 0 / pass。`test-formal` も最終検証で再実行した。

`BoundedRead` probe比較の既存観察（backend read粒度で mutationがマスクされ得る）は ledgerの履歴として残す。slice 3の新規 mutation証拠は selective Kaniで取得した CSV decision core mutantである。

## crate全体 `--no-unwinding-checks` の撤廃

`mise.toml` の structured invocationについて、native pathとWSL2 pathの両方から `--no-unwinding-checks` を除去した。selective `sd-csv-limits` / `sd-byte-budget` の両経路にも同 flagはない。selectorなし aggregate挙動は維持し、wire/archive/WAV invocationと `w-ipc-op` selector semanticsは変更していない。旧 crate全体例外と `build_dfa` c >= 128 rowの議論は経緯として ledger/spec/skillに残し、例外が消えた扱いにはしない。旧3 harnessの successfulは legacy-exception resultであり、通常の Proof Passに計上しない。

## consumer impact assessment

audio-lens/synvelisの `crates/`、`Cargo.toml`、`Cargo.lock` を対象に `ruviform-io-structured`、`ruviform_io_structured`、`CsvReader`、`CsvReadOptions` を検索し、直接利用は見つからなかった。本変更は crate-internalな decision core抽出と `BoundedRead` 内部 error payload簡素化であり、公開 API・limit契約・error kind/limit name/position/encoding/dialectの observable behaviorを変えない。したがって consumer独立回帰は NotApplicableと判断する。この判断は Issue全体の consumer回帰 ACを完了扱いにしない。

## 手動確認

- `mise.toml` の structured native/WSL2 invocationに `--no-unwinding-checks` が残っていないことを確認した。
- selective filterが `csv_limit_decision` を含む3 proofだけ、`bounded_read_budget_gate` の1 proofだけを選ぶことを lane出力で確認した。
- `w-ipc-op` selectorの command/filterは変更していない。
- docs indexは編集していない。統合時に Coordinatorが整備する。
- `Cargo.lock` に差分はない。
- `formal_structured.proptest-regressions` は mutation副産物を残さず変更していない。

## 未実施 lane / 後続

- selectorなし `mise run test-formal-kani`: NotRun。wire/archive harnessが verdictを返せない既知状態であり、本 sliceの Required evidenceにしない。
- Workspace full lane、Feature Matrix、MSRV、3OS横断、GUI-L2/EXT-L3、手動 CI dispatch: NotApplicable/NotRun。本 sliceの指定外であり、merge後 main SHAの統合検証は Daily CIが担う。
- consumer repo test run: NotApplicable。直接利用なし、public behavior unchanged。
- 残件: WAV diagnostic boundary、wire/archive代表 core、read_frame EOF、SampleFormat inventory、他 structured group、consumer回帰の統合判断。

## 残る未証明領域とリスク

- parserが CSV decision coreを毎回呼ぶ接続条件は Kani未証明。既存 proptest/integration coverageに依存する。
- 任意 record内容、複合 limit交互作用、`csv_core` correctnessは証明していない。
- `BoundedRead` proofは 6 byte concrete input・2 byte/readの境界に限定され、任意長・任意 backend readerは未証明。
- 上記を Passに含めず、ledger/specへ同期した。

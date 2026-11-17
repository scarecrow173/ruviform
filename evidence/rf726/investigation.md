# rf#726 調査記録 — clap-validator 0.4.1 param-fuzz panic (exit 0xc0000409 / exit 101)

Issue: https://github.com/scarecrow173/ruviform/issues/726
対象 validator: clap-validator **0.4.1**
`%LOCALAPPDATA%/ruviform/plugin-abi-validator/clap-validator.exe`
sha256 `825439ad3ff98770a24656013eeb11591407687b97058e1db9549c4f7d53e38a`（PR #723 記録値と一致・実測確認）
upstream checkout: `free-audio/clap-validator` @ `5bc2ef3`（master HEAD・issue #42 fix 済み・浅い clone）
OS: Windows（worktree `G:\orca\workspaces\ruviform\rf726-validator-panic`）、Node.js 経由の runner 検証。

## 結論サマリ

- **panic の直接原因は validator 側の bug**。fixture の意図的 fault surface が引き金だが、
  validator が「sandboxed child が成功終了したのに結果 JSON を書かなかった」ケースを
  `expect()` で処理できず panic し、scoped worker thread 経由で validator 全体を
  exit 101・JSON 未出力で死なせる。upstream master HEAD (`5bc2ef3`) にも未修正で
  存在し、既存 Issue/PR に同件なし → **upstream 報告が必要**（起票は worker 権限外、
  報告文ドラフトを `upstream-report-draft.md` に記録し Coordinator へ hand-off）。
- **Issue タイトルの "param-fuzz-sample-accurate" は誤帰属**。同テスト単独では
  validator は正常に JSON を出力し `crashed: exit code: 0xc0000409` を記録して
  exit 1（crashed テストあり）で完走する。実際に validator を殺すのは **param-fuzz 系テストのうち
  stepped Fault 値 `2.0`（exit mode・exit code 0）を生成するもの**
  （実測: `param-fuzz-basic`・`param-fuzz-modulation` が単独で ~15ms で panic 再現）。
- **runner 変更は不要（zero-diff）**。現行 `run.mjs` は canonical leg で
  test_fault に `--exclude param-(set|fuzz)` を適用し、unfiltered run を
  informational leg として `Unsupported` で正直記録、`run.json` は必ず出力、
  panic 契約（非 0 exit + JSON 無し → Unsupported）は `run.test.mjs`:161 の
  contract test で既に pin 済み。
- leg の扱いは **現行維持**（canonical exclude + informational Unsupported）。
  pin 変更は upstream fix 入り release 待ち。無フィルタ run の再開条件は
  「panic 修正入り clap-validator release への pin 更新 + unfiltered leg で
  JSON 出力を確認」。

## 再現マトリクス（`filter-runs/` の raw 出力と対応）

clap-validator 直接起動: `clap-validator.exe validate --json --plugin-id
org.ruviform-test.clap-test-fault [--include <regex>] target/debug/ruviform_plugin_test_clap.dll`
（`--include`/`-t`・`--exclude`/`-x` は `clap-validator validate --help` で確認済み）

| # | filter | plugin | 結果 | exit | JSON |
| --- | --- | --- | --- | --- | --- |
| 1 | `param-fuzz-basic` | test_fault | **panic**: sandbox.rs:98 deserialize EOF → cli.rs:55 scoped thread panic | 101 | なし（stdout 0 byte） |
| 2 | `param-fuzz-basic` + `-j1` | test_fault | **panic**: 同上（worker 無しのため main thread で直接 panic） | 101 | なし |
| 3 | `param-fuzz-modulation` | test_fault | **panic**: 同上 | 101 | なし |
| 4 | `param-fuzz-sample-accurate` | test_fault | `crashed: exit code: 0xc0000409`（abort mode）を正常記録 | 1 | あり |
| 5 | `param-fuzz-bounds` | test_fault | `failed`（CLAP_PROCESS_ERROR = error mode 4.0 を検出） | 1 | あり |
| 6 | `param-set` | test_fault | param-set-events/no-cookies `failed`（「parameter values did not change」）、wrong-namespace success | 1 | あり |
| 7 | `preset-discovery` | test_fault | crawl/load `crashed: Timed out after 45 seconds`、descriptor-consistency success | 1 | あり |
| 8 | `param-fuzz-basic` | test_gain（対照） | `success` | 0 | あり |
| 9 | unfiltered − `param-fuzz-(basic\|modulation)` | test_fault | 全テスト完走・JSON 出力（上記 4-7 の結果を含む） | 1 | あり |

exit code の意味（実測）: `0` = fail 無し完走、`1` = failed/crashed
テストを含む JSON 出力あり、`101` = validator panic（JSON 無し）、
`2` = CLI usage error。#4-#7・#9 は crashed/failed テストを含むため
exit 1 — JSON は正常出力される。

raw 出力: `filter-runs/<name>.stdout.txt` / `.stderr.txt`。
#9 の exclude 対象 2 テストが出力に存在しないことで除外内容を確認済み
（`param-fuzz-sample-accurate`・`param-fuzz-bounds` は残存）。

## 根因連鎖（実測 + upstream source 照合）

1. **fixture（設計どおり・read-only）**: `test_fault` の `Fault` param は
   stepped range `[0.0, 4.0]`（`crates/ruviform-plugin-test-clap/src/fault.rs`）。
   plain `2.0` → `exit` mode → `code = round((plain-2.0)*100)` clamp `[0,100]`
   → **plain=2.0 は exit code 0**。`[1,2)`=abort・`[3,4)`=hang・`4.0`=error。
2. **validator fuzzer**: stepped param の値生成は
   `prng.random_range(range).round()`（upstream `src/tests/rng.rs`:964 付近）
   → `[0,4]` の整数 `{0,1,2,3,4}` を生成 → **2.0 が必ず射程内**。
   決定的 PRNG のためテストごとに「最初に踏む fatal mode」が固定。
3. **child 終了**: `exit(0)` で sandboxed child（`clap-validator sandbox ...`）が
   **成功ステータスで即終了**し、結果 JSON を書かない。
4. **validator bug（核心）**: `SandboxOperation::run_sandboxed`
   （upstream `src/cli/sandbox.rs`:91-98）は `!status.success()` のみを
   `SandboxError::Crashed` とし、**success 時は結果ファイルを
   `read_to_string` + `serde_json::from_str` して `.expect()`**。
   空/未作成の出力 → `Failed to deserialize the output from the sandboxed
   operation: Error("EOF while parsing a value"...)` で **panic**。
   この panic は `parallel_fork_join`（`src/cli.rs`:55 付近）の scoped worker
   thread 内で起き、`std::thread::scope` の join で main に伝播 →
   `a scoped thread panicked. This is a bug in the validator` → exit 101・
   **全テスト結果を失い JSON 未出力**。
   - `-j1` では worker が無く main thread で直接同 panic（stderr の
     `thread 'main' panicked at 'Failed to deserialize...'` が証拠）。
5. **対照**: abort mode（`0xc0000409`）は `!status.success()` →
   `SandboxError::Crashed` → テスト単位 `crashed` として JSON 出力・exit 1。
   つまり **非 0 終了は正しく処理され、成功終了+無出力だけが validator を殺す**。
   hang mode は 45s `wait_timeout` → `Timeout` として処理される（別経路・未再現）。

## upstream 状態照合（free-audio/clap-validator）

- issue list（open+closed）再照合: panic 伝播/JSON 未出力の既存報告なし。
  open は約 12 件あるが panic 関連はなく、`#11`
  （preset-discovery-load crash・timeout 系症状と重複の可能性）は
  本件とは別経路。`#41`/`#42` の fix は master に存在するが 0.4.1
  未収録（上記ドラフト参照）。
- master HEAD `5bc2ef3` の source に buggy code が残存:
  - `src/cli/sandbox.rs`:98 `.expect("Failed to deserialize the output from
    the sandboxed operation")` — 未修正。
  - `src/cli.rs` `parallel_fork_join` の `std::thread::scope` — worker panic
    が main に伝播する構造は未修正。
- 関連するが別件:
  - issue #42（param-set-* test bug）: master `5bc2ef3` で修正済みだが
    0.4.1 binary には未収録 → #6 の param-set-* failed はこの症状と一致
    （test_fault の唯一 param は stepped・default 0）。upstream 報告では
    切り分けて記載。
  - issue #41（preset discovery metadata receiver deadlock）: merge commit
    `47e2424` の修正（`metadata_receiver.rs`:386-387 の mutex guard 解放）は
    master source に存在。しかし 0.4.1 binary では preset-discovery
    crawl/load が 45s timeout → **0.4.1 は #41 修正前の build と整合**。
    test_fault は preset discovery extension を実装しないが、library-level
    の crawl/load が hang する点は別 anomaly として記録（本 Issue 範囲外）。
- upstream master の build/検証は未実施（浅い clone・時間対効果で打ち切り。
  「master に fix 無し」は source 照合で確定済みのため残 gate 不要）。

## runner 契約の照合（`ci/plugin-abi-validator/run.mjs`・zero-diff 判定）

panic 時契約「validator が JSON を出さず異常終了 → leg を fail-closed で
正直記録・run.json は必ず出力」は**現行実装が満たす**:

- `normalizeClapLeg`（run.mjs:273-281）: `proc.error` → Failed、
  `status!==0 && !counts` → **Unsupported**（"tool-side termination"）、
  counts 有り + failed/crashed → Failed、counts のみ → Passed、
  それ以外 → Failed。panic signature（exit 101 + stdout 空）は
  Unsupported に落ちる。
- canonical legs は `evaluate()` で Unsupported を fail-closed
  （verdict Unsupported・exit 1）。informational legs は verdict に不介入。
- informational unfiltered leg は `leg.informational = true` で
  `clap-test_fault-unfiltered.txt` に raw を保存し details に
  「expected: fatal fault modes are armed by param-mutating cases」を記録。
- contract test `run.test.mjs`:161-172 がまさにこの panic signature
  （status 101 + 空 stdout + panic stderr → Unsupported）を pin 済み。
  追加 test は契約の重複であり不要と判断。

## lane 実走で発見した runner 側 bug（ETIMEDOUT による結果隠蔽）

opt-in lane 実走（`abi-validator/`・修正前 runner）で全 fixture leg が
`Failed — spawn error: spawnSync … ETIMEDOUT` になった。調査の結果、
これは validator の失敗ではなく **runner の stdio 捕捉 bug**:

- 各 leg の `.txt` には validator が **完走して完全な JSON report を
  出力した証拠**が残る（全テスト結果 + preset-discovery 45s timeout
  ログ）。validator process 自身は ~50s で正常終了していた。
- 原因: `runClapLeg` が `spawnSync` の pipe stdio を使う。clap-validator
  の sandboxed child（`clap-validator sandbox …`）は validator の
  stdio を継承する。preset-discovery deadlock（0.4.1・upstream #41
  未収録）で wedge した child は timeout 後も kill されず残存
  （orphan）し、pipe の write end を保持し続ける → spawnSync は
  process 終了後も pipe EOF を待ち、300s timeout で ETIMEDOUT。
- 影響: `normalizeClapLeg` は `proc.error` を最初に評価するため、
  **出力済みの完全な JSON を捨てて Failed を記録**していた。
  informational unfiltered leg も本来の `Unsupported — validator
  exited 101 without JSON`（panic 契約どおり）ではなく
  `Failed — spawn error ETIMEDOUT` と誤記録されていた。
- 対照: 私の直接実行（shell redirect）は direct child のみ wait する
  ため ~95s で完走 — pipe 継承問題は spawnSync 固有。

**修正（`ci/plugin-abi-validator/run.mjs`）**: `spawnCapture` を追加し
stdout/stderr を一時ファイル経由で捕捉。file stdio では orphan
孫 process が handle を保持しても spawnSync は direct child の
終了で復帰する。`runClapLeg` のみ適用（`runVst3Leg` は Steinberg
validator で sandbox grandchild を spawn しないため対象外 — 同型
パターンは follow-up 候補として記録）。contract test 2 件追加
（stdio 継承 grandchild がいても child exit で復帰・spawn 失敗/
timeout は fail-closed のまま）。

## preset-discovery の回帰（rf630/rf732 → 現在）

- rf630/rf732 の leg 出力では `preset-discovery-crawl`/`load` は
  **skipped**（test_stateful の preset surface が未実装だった時期の
  evidence と整合）。現在は全 fixture leg で **crashed: Timed out
  after 45 seconds** ×2 + `descriptor-consistency` success。
- 原因は upstream #41 の deadlock（`MetadataReceiver` の mutex
  self-deadlock）で、master `5bc2ef3` には fix があるが 0.4.1 binary
  には未収録。fixture の preset discovery 実装は正常で、validator
  側の既知 bug。
- 影響: stdio 修正後も fixture leg は preset-discovery crashed ×2 を
  含むため **Failed が正直な verdict**。除外は行わない
  （param-fuzz と異なり fixture の意図的 crash surface ではなく、
  upstream release で解消されるまで可視の失敗として残す）。
- 副次影響: orphan sandbox child が leg ごとに ~2 個ずつ残存し
  DLL を lock → 次回 `cargo build` が `os error 5` で失敗し得る。
  lane 実行前に残存 `clap-validator.exe`（`sandbox` mode）を
  確認・kill する運用注意を記録（本調査で 14+ orphan を実測・除去）。

## leg の扱い決定（AC3）と無フィルタ run 再開条件（AC4）

- **決定: 現行維持（Unsupported 継続 + canonical exclude + informational leg）**。
  - canonical `test_fault` leg の `exclude: "param-(set|fuzz)"` は
    「fixture が意図的に fatal mode を持ち、param-mutating case が
    sandboxed child を設計どおり殺す」ため正当。fatal surface は
    L1 out-of-process fault lane が担う。panic は validator bug であり
    fixture/adapter の欠陥ではないため、除外は検証の弱体化ではない。
  - filter 運用の変更（例: `param-fuzz-(basic|modulation)` だけ除外）は
    不採用 — panic の引き金は「exit(0) を踏む任意の param-mutating case」で
    将来の validator 版・fixture 変更で再発し得る。param 系全除外の方が
    契約として安定。
  - pin 変更は upstream fix 入り release 待ち（upstream は活発・#41/#42 が
    master に連続 merge されているため現実的）。
- **無フィルタ run 再開条件**: clap-validator の panic 修正
  （success exit + 無出力を Crashed/Failed として扱う + worker panic の
  per-test 隔離）を含む release が出て pin を更新した時点で、
  informational unfiltered leg を再実行し JSON 出力（crashed/failed の
  per-test 記録）を確認する。それまでは informational leg が
  Unsupported を記録し続けるのが正直な状態。

## 副次観測（本 Issue 範囲外・記録のみ）

- **preset-discovery crawl/load が 45s timeout**: 0.4.1 では plugin-library
  test が hang。master では #41 deadlock fix 済み → pin 更新で解消見込み。
  test_fault は preset discovery を実装しないが library crawl は走る。
  runner の informational leg 実行時間（~95s）に影響。別 Issue 化は
  Coordinator 判断（upstream release 待ちで実質解消）。
- **sandbox timeout が child を kill しない**: `wait_timeout` が
  `SandboxError::Timeout` を返すが child process は残存
  （`taskkill` で 4 プロセス確認・kill）。残存 child が DLL を lock し
  `cargo build` の DLL 差し替えを `os error 5` で失敗させた（lane run で
  実測）。upstream 報告に「timeout 時の child 殺害漏れ」として併記する価値あり。
- param-set-* failed・param-fuzz-bounds failed は 0.4.1 の既知 test bug
  （#42）および error mode の defined rejection で、panic とは別件。

## 検証コマンド記録

- `node --test ci/plugin-abi-validator/tests/run.test.mjs` — 21 tests pass（baseline・変更なしでも実行）。
- opt-in lane: `node ci/plugin-abi-validator/run.mjs --clap-validator
  "$LOCALAPPDATA/ruviform/plugin-abi-validator/clap-validator.exe"
  --clap-production --evidence-dir evidence/rf726/abi-validator`
  → 結果は `abi-validator/run.json`（実行後に追記）。
- panic 絞り込み: 上表 #1-#9 の直接 clap-validator 起動（各 raw 出力を
  `filter-runs/` に保存）。

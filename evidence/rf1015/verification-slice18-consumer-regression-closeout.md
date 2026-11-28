# rf#1015 slice 18 verification — Consumer regression evidence / closeout assessment

## 対象と SHA の役割

- branch: `test/1015-consumer-regression`。
- base SHA / 検証対象 SHA: `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`。作業開始時の `origin/main` であり、rf#1064 / PR #1065 の consumer-verify workspace-aware manifest pinning を含む。
- 本 slice は `evidence/rf1015/` のみを変更する evidence-only sliceであり、製品コード、proof harness、workflow、helper、mise task、consumer repoを変更していない。
- consumer-verify candidate tuple:
  - ruviform: `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`
  - audio-lens: `0f1a1aac86336bff98df8bab1162e23bf5c36672`
  - synvelis: `a5e29228e6a08940e7396325daab138295a5e894`
  - contract: `consumer-verify-tuple/v2`
- 検証はローカル証拠と workflow_dispatch run の読み取りである。merge 後 main SHA の統合検証や Issue close の判断は Coordinator が行う。

## consumer-verify run 46144010070

`gh run watch 46144010070 --repo scarecrow173/ruviform --exit-status` を foreground で実行した。run は開始前に完了済みで、watch は 2026-11-28T12:32:28Z に開始し、2026-11-28T12:32:29Z に `exit 1` で終わった。出力は `Run consumer-verify (46144010070) has already completed with 'failure'` だった。待機コマンドの exit 1は watch 自体の異常ではなく、run conclusion `failure` を表す。

run URL: <https://github.com/scarecrow173/ruviform/actions/runs/46144010070>

| job | conclusion | 観測 |
| --- | --- | --- |
| `preflight` | success | 入力 validation、`mise run test-ci-consumer-verify`、anonymous access probe が success |
| `verify-audiolens` | success | audio-lens lane は artifact status `pass` |
| `verify-synvelis` | failure | synvelis lane は artifact status `fail`、最初の失敗は `mise run test-rt (RT no-alloc)` |
| `summary` | failure | 片 consumer の fail を run failureとして集約 |

artifact は read-onlyで取得し、改変せず `consumer-evidence-audiolens-run-46144010070.json` と `consumer-evidence-synvelis-run-46144010070.json` として固定した。artifact元名はそれぞれ `consumer-evidence-audiolens-attempt-1`、`consumer-evidence-synvelis-attempt-1` である。

## audio-lens lane

artifact statusは `pass`、coverageは pass 16 / fail 0 / not_run 1 / unsupported 2 / skipped 1。resolved SHAは期待値 `0f1a1aac86336bff98df8bab1162e23bf5c36672` と一致し、`shaMatch=true` だった。access modeは `anonymous`。

主要 laneの結果:

- dependency access probe: pass
- workspace-aware manifest pin: pass
- manifest pin + cargo fetch / single-SHA lockfile guard: pass
- `mise run check-ruviform-lock`: pass
- `mise run check-full` fallback lane: pass
- `cargo build -p al-cli --profile ci` smoke: pass
- `mise run check-cli-headless` smoke: pass
- `mise run test-full` fallback lane: pass
- rf628 oracle `mise run test al-analysis --test rf628_oracle`: pass。fixture versionは `ruviform-audio-test-0.1.0@eea9e2090a23e24dd5cd7a0799b25175c91cad9c`
- UI vendor sync、setup-ui、check-ui、test-ui: pass
- candidate tree上の GUI-CONTRACT boundary guardと GUI-SEMANTIC fake driver contract: pass

非Pass行は失敗ではない。extra task未指定の aggregate行は skipped、consumer-side通常 View/Domain headless fixtureは AL #127 owner付き not_run、実window laneと macOS/実 OS laneは ubuntu headless capability不足の unsupportedである。これらを Passには計上しない。

## synvelis lane

artifact statusは `fail`、status reasonは `lane_fail: mise run test-rt (RT no-alloc)`、coverageは pass 6 / fail 1 / not_run 1 / unsupported 3 / skipped 12。resolved SHAは期待値 `a5e29228e6a08940e7396325daab138295a5e894` と一致し、`shaMatch=true` だった。access modeは `anonymous`。

失敗前に通過した lane:

1. dependency access probe: pass
2. workspace-aware manifest pin: pass
3. manifest pin + cargo fetch / single-SHA lockfile guard: pass
4. `mise run check-ruviform-rev`: pass
5. `mise run check-full` fallback lane: pass
6. `mise run test-full` fallback lane: pass

最初の失敗地点は `verify-synvelis` jobの `test-rt lane (RT no-alloc)` stepである。job logでは 2026-11-28T12:31:40.735Zに最初の `error: linking with 'cc' failed: exit status: 1` が出て、最初の出力 binaryは `target/debug/deps/tim03_loop_boundary-ebfe1e93b8116686` だった。12:31:40.895Zに `collect2: fatal error: ld terminated with signal 7 [Bus error], core dumped` が記録された。compile summaryでは `synvelis-player` の test `ctl_02`、`pc_c6_oracle_pl_rt`、`tim03_loop_boundary`、`sf04_probe_adapter` が link errorで compileできず、stepは exit code 101で終わった。`running` / `test result:` markerはこの stepになく、RT test本体の assertion failureには到達していない。

この failで後続の `check-headless-deps`、rf628 oracle 2 lane、UI vendor/setup/check/test/build、vendor整合、candidate tree GUI-CONTRACT/GUI-SEMANTICは skippedになった。したがって synvelisには rf628 oracle verdictも GUI candidate lane verdictもない。static not_runは consumer-side通常 View/Domain headless fixture、static unsupportedは実window、PLUGIN-NATIVE-DOCK、macOS/実 OS laneである。これらを Passには計上しない。

## 失敗分類

分類は **インフラ/toolchain failure（恒久 root causeは unknown）** とする。rf#1015の共通実装変更に起因する製品回帰とは断定しない。consumer側の test binary構成が linker負荷を高めている可能性は残るが、この sliceの証拠だけでは consumer側既存問題と infrastructure flakyを完全には分けられない。

根拠:

- 失敗は RT testの実行や oracle mismatchではなく、linker `ld` の signal 7 Bus errorである。
- `mise run check-full` と `mise run test-full` が同じ consumer checkoutと candidate tupleで先に passしている。
- 直前 run `46132073358` でも、別の ruviform candidate `b1745816100b85e229b49a5e29ca8fbc963096e4` と別の synvelis resolved SHA `88ec448e16b5cf868f73c8af4f50902c73b079da` で、同じ `test-rt` laneが同じ `ld terminated with signal 7 [Bus error]` を出していた。今回の tuple固有の初発症状ではない。
- rf#1015が触れた wire/archive/structured documents/WAVの semantic failure、precision failure、RT allocation assertionを示すログはない。

したがって、この runは synvelisの独立回帰 Passを示さない。同時に、run failureを rf#1015製品回帰の証拠としても使わない。恒久原因の切り分けと `test-rt` laneの再実行・修復は残件である。この sliceでは workflow/helper/testを修正せず、追加 rerunも行わない。

## 代表 important Propertyの closeout assessment

slice 1で代表 important Propertyとして選ばれたのは `W-IPC-OP`、`W-FRM-READCLASS`、`ARC-IDX-LIMIT`、`ARC-PAYLOAD-LIMIT` である。slice 1-17の ledger、spec、slice 14-17証跡、mutation証跡を読み直した assessmentは次のとおり。

| Property | 再設計 proofの verdict | 事前予算との関係 | 検出能力 | assessment |
| --- | --- | --- | --- | --- |
| `W-IPC-OP` | Required / Kani Pass。4/4 harness、Unwinding Assertions有効 | 5 CPU-min / 1 GiBに対し verification time計 70.762 s、wall約96 s、観測 peak 231,740 KiB | `MAX_OP_LEN`を+1へ緩める mutationを boundary proofが検出し、revert後 Pass | 定義した decision-core境界では事前予算内に証明完了 |
| `W-FRM-READCLASS` | Required / Kani Pass。2/2 harness、cover 9/9と8/8、Unwinding Assertions有効 | 12 CPU-min / 3 GiBに対し verification time計 0.83051482 s、wall約19 s、auxiliary max RSS 105,620 KiB | partial prefixを `Eof`へ戻す mutationを両 harnessが検出し、完全revert後 Pass | 定義した read-classification境界では事前予算内に証明完了 |
| `ARC-IDX-LIMIT` | Required / Kani Pass。ZIP index、TAR拡張 payload確保前判定、TAR固定 block計上の3 proofが Pass | 各5 CPU-min / 1 GiBに対し verification time 0.23537865 s / 1.8670735 s / 0.21418972 s、wall約7 s / 16.617 s / 5.539 s | entries gateとTAR拡張・固定blockの等号 mutationを検出し、revert後 Pass | 定義した index/metadata decision-core境界では事前予算内に証明完了 |
| `ARC-PAYLOAD-LIMIT` | Required / Kani Pass。TAR declared、ZIP declared entry-size、ZIP output cap、ZIP measured total、TAR measured total、TAR entry progressの6 proofが Pass | 各5 CPU-min / 1 GiBに対し verification time 0.08520902 s / 0.08727401 s / 0.25182262 s / 0.35422927 s / 0.88929915 s / 2.0711884 s。wallは約6–9.5 s | declared-size equality、output-cap classification、ZIP/TAR accumulation identity、entry progress無視 mutationを検出し、revert後 Pass | 定義した payload budget decision-core境界では事前予算内に証明完了 |

closeout judgment: 代表 important Propertyについて、slice 1で定義した再設計境界内の Required proofは事前予算内で Passへ到達した。これは parser shell、shell-core接続、複数 entry composition、実 I/O、UTF-8 validation本体、decompression、CRC、error priority、consumer互換性までを bounded exhaustiveに証明した意味ではない。ledgerが残す未証明領域は Passに含めない。`W-FRM-READCLASS`の `stream_len <= 32` は入力域分割であり、32 byte超のwindowと実transportは未証明領域に残る。`ARC-PAYLOAD-LIMIT`の6 proofは unionであって、TAR streaming composition全体や archive composition全体の証明ではない。

## この branchで実行した正規検証

runner OSは Windows 11 x86_64（10.0.26300.9974、MSYS2/MinGW shell）、toolchainは `rustc 1.99.1 (58cf8f9c3 2026-10-13)`、`cargo 1.99.1 (5f94df478 2026-08-27)`、mise 2026.11.2 windows-x64、node v26.12.1。実行時 branch headは evidence commit前の `eea9e2090a23e24dd5cd7a0799b25175c91cad9c` だった。

| lane | UTC | exit | verdict | 結果 |
| --- | --- | ---: | --- | --- |
| `mise run fmt-check` | 2026-11-28T12:27:08Z–12:27:20Z | 0 | Pass | rustfmt差分なし。Windows command line長の os error 206は発生しなかった |
| `mise run test-ci-consumer-verify` | 2026-11-28T12:27:25Z–12:27:27Z | 0 | Pass | node contract tests 95/95 pass。workflow contractと evidence evaluatorの非回帰 |
| `mise run test-formal` | 2026-11-28T12:27:36Z–12:28:43Z | 0 | Pass | IPC 8/8、transport 6/6、Archive 26/26、Structured Documents 35/35、WAV 9/9。計84/84 pass |

これらは current main `eea9e209` に対するローカル非回帰である。`test-formal` は Kaniを起動しない Property Test laneであり、Required Kani proofの再実行ではない。

## branch CI

`gh workflow run ci.yml --repo scarecrow173/ruviform --ref test/1015-consumer-regression` で manual dispatchし、`gh run watch 46145831482 --repo scarecrow173/ruviform --exit-status --interval 60` を実行した。watch exit codeは 0、run statusは `completed`、conclusionは `success` だった。run URLは <https://github.com/scarecrow173/ruviform/actions/runs/46145831482>、createdAtは 2026-11-28T12:40:26Z、最終更新は 2026-11-28T12:50:10Zである。

CI target headは evidence commit `955a20a8b7198cd07434c072716651f21db00ed9`。23/23 jobが successで、`ci-summary` も successだった。manual dispatchの `expected_sha` inputは空であり、ci-summaryは checkout SHA artifactに基づく実 checkout記録を行う通常経路で評価された。

この CI結果を evidenceへ追記する commitは CI targetの子孫になる。ただし差分は `evidence/rf1015/` のみで、製品コード・workflow・build設定・lockfileの code headは変わらない。したがって CI target `955a20a8` と最終 evidence headの関係は「evidence-only追記による派生」であり、コード検証対象が変わったわけではない。

## 実行しなかった検証・non-claims

| 項目 | state / verdict | 理由 |
| --- | --- | --- |
| selectorなし `mise run test-formal-kani` | NotRun | verdictを返せない legacy aggregateを含む。closeoutは slice 1-17の selective proof証跡を assessmentした |
| 代表Propertyの selective Kani再実行 | NotRun | 本 sliceは製品/proof/mise差分を持たず、canonical指定は `fmt-check` / `test-ci-consumer-verify` / `test-formal`。既存証跡を current mainの Property Test非回帰と分離して記録する |
| Workspace full、Feature Matrix、MSRV、3OS横断 | NotRun | evidence-onlyのローカル検証範囲外。branch CIを別途 dispatchする |
| GUI-L2 / EXT-L3、macOS/実OS、実window、実plugin | NotRun / Unsupported | consumer-verify artifactでも capability不足または未実装として記録された。Passにしない |
| synvelis rf628 oracle / RT挙動 / UI candidate lanes | NotRun | `test-rt` link failureにより後続stepがskipped。oracle verdictは存在しない |
| consumer製品精度・RT・互換性の維持 | NotRun | audio-lensは passしたが、synvelis独立回帰が failのままなので両Consumer条件は成立しない |
| Issue close | NotApplicable | 本 sliceは closeout evidenceを固定するだけで、Issueは closeしない |

## 残件

1. synvelis `test-rt` linker Bus errorの恒久原因切り分け。linker/toolchain環境、test binary構成、runner resourceのどれかを consumer側または infra側で特定する必要がある。
2. synvelisの独立回帰は `test-rt`通過後に、rf628 oracle、RT-alloc oracle、UI、candidate GUI contract laneまで到達させる必要がある。
3. consumer-side通常 View/Domain headless fixture、実window、PLUGIN-NATIVE-DOCK、macOS/実 OS laneは owner付き NotRun/Unsupportedのままである。
4. 恒久受入には merge後 main SHAの統合検証が必要である。branch CIは `955a20a8b7198cd07434c072716651f21db00ed9` で success済みだが、merge後 mainの証拠ではない。
5. docs indexは Coordinatorが統合時に整備する。本 sliceでは編集しない。

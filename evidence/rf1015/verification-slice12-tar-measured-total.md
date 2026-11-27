# rf#1015 slice 12 verification — Archive `ARC-PAYLOAD-LIMIT` TAR measured total read-window / accumulation

## 目的と範囲

slice 12 は `ARC-PAYLOAD-LIMIT` のうち、TAR interactive `Read` pathと drain pathが共有する scalar decision subboundaryを閉じる。対象は remaining total budget、nonzero demandに対する budget exhaustion classification、1 回の read/drainで許す最大 byte window、measured bytes accumulationである。

非対象は TAR streaming composition、複数 entry composition、pending/discard/finish state machine全体、shell-core接続の Kani証明、TAR header parse、PAX/GNU extension folding、任意 malformed stream、decompression、CRC、TAR metadata gate、wire/WAV残部、consumer regressionである。これらは後続 sliceまたは Issue 全体の残作業として残す。

## SHA の役割

| 役割 | SHA / 参照 |
| --- | --- |
| base SHA | `9aab551cb4d258896035a7db5a44cbc7222ae3ef` |
| 検証対象 SHA / verified Sync Commit | `486b52963757fc7dd807fae287eedc078440600c` |
| 初期 evidence sync commit | `a73201aa114c3d80cc7421490639a054151b270d` |
| 最終 head SHA | PR 本文で追跡する |

`486b52963757fc7dd807fae287eedc078440600c` は TAR measured total decision core、in-src proof、deterministic regression、selective laneを含む最後の製品/proof/mise変更 commitである。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。

## 実装

`crates/ruviform-io-archive/src/tar/read.rs` に crate-private pure decision coreを追加した。

- `tar_measured_total_remaining(max_total_uncompressed_bytes, accumulated_before)` は saturating subtractionで remaining total budgetを返す。
- `tar_measured_total_window_decision(remaining_budget, demand, capacity)` は 1 回の read/drain windowと budget exhaustionを返す。`demand == 0` は budget残量に関係なく no-read/non-exhaustion、positive demandで remaining budgetが0なら exhaustion、それ以外は `demand`・`remaining_budget`・`capacity` の最小値である。
- `tar_measured_total_accumulate(accumulated_before, measured_bytes)` は saturating additionで次の accumulated totalを返す。

production接続は `TarEntryReader::read` の per-call window/budget判定と successful read後の accumulation、`TarArchiveReader::drain_bytes` の per-chunk window/budget判定と successful chunk後の accumulationに限定した。public API、limit/error契約、inclusive budget equality、zero-length entry acceptance、padding/pending/receipt semantics、error kind/message、Truncated/Io/InvalidState分類は変更していない。

## Proof boundary

新しい in-src proof `arc_tar_measured_total_decision_exactness` は production implementationを対象にする。model、test double、`cfg(kani)` による製品実装置換、`--no-unwinding-checks`、入力域を縮める `assume` は使っていない。`#[kani::unwind(2)]` であり、loopはない。

proof inputは次の全 `u64` domainである。

- `max_total_uncompressed_bytes`
- `accumulated_before`
- `demand`
- `capacity`
- `measured_bytes`

production invariant `accumulated_before <= max_total_uncompressed_bytes` は assumeしない。全 domain exactnessは production reachable subsetを含む stronger proofである。`capacity == 0` も proof input domainに含める。scalar semanticsでは window 0・non-exhaustionだが、production接続では entry read shellの empty-buffer early returnまたは drain固定 nonzero capacityにより到達しない。

oracleは production実装の写しではなく、remaining boundary、zero-demand、budget exhaustion、demand/remaining/capacity minimum、equality tie、zero-capacity scalar case、zero-measured identity、non-overflow sum、`u64::MAX` saturationを独立した関係式で区別する。cover propertyはこれらの代表到達を確認する。

## 環境

| 項目 | 値 |
| --- | --- |
| runner OS | Windows 11 x86_64 |
| Windows rustc/cargo | rustc 1.99.1 / cargo 1.99.1 |
| Kani実行環境 | WSL2 Ubuntu 24.04.4 LTS |
| WSL kernel | `6.6.87.2-microsoft-standard-WSL2` |
| WSL rustc/cargo | rustc 1.99.0 / cargo 1.99.0 |
| Kani | Kani Rust Verifier 0.68.0 |
| CBMC | 6.11.0 |
| mise | 2026.11.2 windows-x64 |

## Canonical selective proof

canonical verdictは mise laneである。direct `cargo kani` と `/usr/bin/time` は補助証拠（canonical = mise）としてだけ使う。

| run | command | 時刻（UTC） | exit | result |
| --- | --- | --- | --- | --- |
| pre-mutation | `mise run test-formal-kani arc-tar-measured-total` | 2026-11-27T13:52:32Z–13:52:41Z | 0 | Pass。1/1 harness `proofs::arc_tar_measured_total_decision_exactness`、0/58 failed、cover 14/14 satisfied、verification time 1.3129115 s、lane wall 8.465 s |
| mutation | 同上 | 2026-11-27T13:53:05Z–13:53:12Z | 1 | mutantを検出。2/58 failed（1 unreachable）、cover 13/14 satisfied、verification time 0.75833935 s、lane wall 5.882 s |
| post-revert | 同上 | 2026-11-27T13:53:34Z–13:53:41Z | 0 | Pass。1/1 harness、0/58 failed、cover 14/14 satisfied、verification time 0.88929915 s、lane wall 6.897 s |

mutationは `tar_measured_total_accumulate` を measured bytesを加えず accumulated-beforeをそのまま返す identityへ 1 箇所だけ変えた。失敗した assertionは non-overflow sumと exact `u64::MAX` boundaryであり、unwinding assertion failureではなく semantic violationである。mutantは完全に revertし、revert後の `git diff` は製品実装に差分がないことを確認した。

## Shell-core connection regression

`crates/ruviform-io-archive/tests/formal_archive.rs` に deterministic regression `tar_measured_total_connection_regression` を追加した。対象は次の 4 条件である。

1. entries `[3,5]`、budget 8で exact totalを acceptし、receipt payload bytesが measured sumの8になる。
2. entries `[3,6]`、budget 8で second entryが残り budgetの5 byteだけを yieldし、次の nonzero demandで `LimitExceeded` になる。
3. entries `[3,5,0]`、budget 8で exact budget使用後も zero-length entryと archive finishが成功し、payload totalが変わらない。
4. drain pathで exact budgetを acceptし、1 byte超過を `LimitExceeded` で rejectする。

fixture builderが対象ケースを表現できたため、lease外の `tests/common/` は変更していない。

## 正規検証

| command | exit | result |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-io-archive` | 0 | Pass |
| `mise run lint ruviform-io-archive` | 0 | Pass |
| `mise run test ruviform-io-archive` | 0 | Pass。unit 22、archive integration 23、TAR reader 44、TAR writer 15、ZIP reader 34、ZIP writer 10、doctest 10 |
| `mise run test-formal` | 0 | Pass。`formal_ipc` 8/8、`formal_transport` 5/5、`formal_archive` 23/23、`formal_structured` 35/35、`formal_wav` 9/9 |
| `mise run test-formal-kani arc-tar-measured-total` | 0 | Pass。post-revert canonical resultは上表 |
| `mise run test-formal-kani arc-payload-limit` | 0 | Pass。0/16 failed、cover 5/5、verification time 0.08018735 s |
| `mise run test-formal-kani arc-tar-struct` | 0 | Pass。0/83 failed、cover 12/12、verification time 0.20096105 s |
| `mise run test-formal-kani arc-zip-measured-total` | 0 | Pass。0/10 failed、cover 7/7、verification time 0.29076508 s。baseが PR #1051を含むため closest same-group regressionとして実行 |
| `mise run test-formal-kani invalid-selector` | 2 | Pass（期待どおり usage validation）。Kani/environment execution前に停止 |
| `mise exec -- prek run --all-files` | 0 | Pass。2026-11-27T14:21:03Z–14:21:11Z、wall 8 s。trim trailing whitespace、fix end of files、check yaml、check toml、mixed line ending、fix utf-8 byte order marker、detect private key、actionlintが Pass。check illegal windows namesは対象ファイルなしで skipped |

## コストと予算

事前予算は 5 CPU-min / 1 GiBである。canonical post-revert selective laneは lane wall 6.897 s、verification time 0.88929915 sだった。補助計測として direct WSL `cargo kani` を `/usr/bin/time` で実行した結果、elapsed wall 5.26 s、maximum resident set size 104,784 KiB、user time 1.48 s、system time 0.50 s、verification time 0.98589045 sだった。この計測は compileを含む補助証拠であり、canonical verdictではない。実測は事前予算内である。

## Consumer impact assessment

対象 SHAで audio-lensと synvelisを再検索した。検索対象は各 repositoryの `crates/`、`Cargo.toml`、`Cargo.lock`である。

| repository | SHA | crate reference | exact symbol |
| --- | --- | --- | --- |
| audio-lens | `0e15ee44916f6b45918e072a19a04c449169ce7d` | `ruviform-io-archive` / `ruviform_io_archive`: 0件 | `TarArchiveReader` / `ArchiveReadLimits`: 0件 |
| synvelis | `0954a19f74711dfe4b4b54dcdcf87f2ba992a940` | `ruviform-io-archive` / `ruviform_io_archive`: 0件 | `TarArchiveReader` / `ArchiveReadLimits`: 0件 |

実行した検索は `rg -n 'ruviform-io-archive|ruviform_io_archive' <scope>` と `rg -n 'TarArchiveReader|ArchiveReadLimits' <scope>` で、両 repositoryの3 scopeとも 0件だった。crate referenceも exact-symbol reachabilityもないため、本 sliceの consumer independent regressionは NotApplicableと判断する。この判断は Issue 全体の consumer acceptance criterionを完了にしない。

## 実行しなかった lane

| lane | judgment | reason |
| --- | --- | --- |
| selectorなし `mise run test-formal-kani` | NotRun | verdictを返せない既知の wire/archive harnessを含む default aggregateを本 sliceの Required evidenceにしない。aggregateの既知状態は解消していない |
| Workspace full lane | NotRun | 本 sliceの Required検証ではない。Daily Full CIへ委譲する |
| Feature Matrix / MSRV / 3OS横断 | NotRun | 本 sliceの Required検証ではない。対象は internal scalar refactorであり、feature/platform依存を追加していない |
| GUI-L2 / EXT-L3 | NotApplicable | UI・外部 consumer 変更を含まない |
| consumer independent regression | NotApplicable | audio-lens/synvelisの対象 scopeに crate referenceと exact-symbol reachabilityがない |
| 手動 CI dispatch | NotRun | 通常 sliceは CI非ブロッキングであり、受入条件が手動 dispatchを要求していない |
| TAR streaming composition / parser-core接続 proof | Deferred | scalar subboundaryの proof範囲外。後続 sliceへ残す |

## 残る未証明領域

- TAR measured total streaming composition、複数 entry composition、read/discard/pending/finishをまたぐ state machine、shell-core接続条件。
- TAR parser-core接続、TAR metadata/path/link gate、TAR header parse、PAX/GNU extension folding、任意 malformed stream。
- ZIP/TAR decompression correctness、CRC correctness、archive composition proof。
- wire残部と WAV残部。
- Issue 全体の consumer independent regression判断。

## 受入条件との対応

- Required proofは Unwinding Assertions有効の selective laneで Passし、事前予算内だった。
- Able-to-Failは accumulation identity mutantを semantic assertion failureで検出し、revert後に selective proofと `test-formal` を再実行した。
- 正規 Property Testと `ruviform-io-archive` regressionは Passした。
- `arc-payload-limit`、`arc-tar-struct`、baseに存在する `arc-zip-measured-total` の direct regressionは Passした。
- unknown selectorは exit 2で停止した。
- consumer impact assessmentは search-onlyで実施し、NotApplicable判断の根拠を記録した。
- docs indexは編集していない。Coordinatorが統合時に整備する。

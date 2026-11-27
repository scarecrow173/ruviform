# rf#1015 slice 13 verification — Archive `ARC-PAYLOAD-LIMIT` TAR entry progress / read-demand composition

## 目的と範囲

slice 13 は `ARC-PAYLOAD-LIMIT` の TAR streaming compositionのうち、interactive `Read` pathの entry progressと 1 回の read-demand compositionだけを閉じる。対象は declared/producedからの remaining entry payload、entry残量・total budget残量・caller buffer capacityからの read window/budget exhaustion/entry exhaustion、produced accumulation、payload exhaustion classificationである。

非対象は TAR streaming composition全体、複数 entry composition、read/discard/pending/finishをまたぐ state machine、shell-core接続の Kani証明、TAR header parse、PAX/GNU extension folding、任意 malformed stream、TAR metadata/path/link gate、decompression、CRC、wire/WAV残部、consumer regressionである。これらは後続 sliceまたは Issue 全体の残作業として残す。

## SHA の役割

| 役割 | SHA / 参照 |
| --- | --- |
| base SHA | `f52f53cb5318f5ed8eaebfb58845e83162e41964` |
| 検証対象 SHA / verified Sync Commit | `deca7e9b4752f1994df736d7d70f1bcd1b14d026` |
| 初期 evidence sync commit | 本ファイルを含む後続 commit |
| 最終 head SHA | PR 本文で追跡する |

`deca7e9b` は TAR entry progress decision core、in-src proof、deterministic regression、selective laneを含む最後の製品/proof/mise変更 commitである。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。

## 実装

`crates/ruviform-io-archive/src/tar/read.rs` に crate-private pure decision coreを追加した。

- `tar_entry_remaining_payload(declared, produced)` は saturating subtractionで remaining entry payloadを返す。
- `tar_entry_read_window_decision(entry_remaining, remaining_budget, capacity)` は entry残量を demandとして slice 12の `tar_measured_total_window_decision` へ渡し、window、budget exhaustion、entry exhaustionを返す。window計算そのものは再実装していない。
- `tar_entry_accumulate_produced(produced_before, read_bytes)` は saturating additionで次の producedを返す。
- `tar_entry_payload_exhausted(declared, produced)` は declared/producedだけから entry exhaustionを返す。

production接続は `TarEntryReader::read` の demand compositionと post-read accumulation/exhaustion、`discard_remaining` の drain量、`Drop` の pending skip量、`finish` の payload未消費 `InvalidState` 判定に限定した。public API、limit/error契約、inclusive budget equality、zero-length entry acceptance、empty-buffer early return、padding/pending/receipt semantics、error kind/message、Truncated/Io/InvalidState/LimitExceeded分類は変更していない。

## Proof boundary

新しい in-src proof `arc_tar_entry_progress_decision_exactness` は production implementationを対象にする。model、test double、`cfg(kani)` による製品実装置換、`--no-unwinding-checks`、入力域を縮める `assume` は使っていない。`#[kani::unwind(2)]` であり、loopはない。

proof inputは `declared`、`produced`、`budget_remaining`、`capacity`、`read_bytes` の全 `u64` domainである。production invariant `produced <= declared` は assumeしない。全 domain exactnessは production reachable subsetを含む stronger proofである。`capacity == 0` も proof input domainに含める。scalar semanticsでは window 0・non-exhaustionだが、production接続では entry read shellの empty-buffer early returnにより到達しない。

oracleは production実装の写しではなく、remaining boundary-1/equality/boundary+1 saturation、exhaustion equality、exhausted entryの zero-budget no-read、positive demandの budget exhaustion、entry残量/budget/capacityの strict minimumと equality tie、zero-capacity scalar case、zero-read identity、non-overflow sum、`u64::MAX` saturationを独立した関係式で区別する。cover propertyはこれらの代表到達を確認する。

## 環境

| 項目 | 値 |
| --- | --- |
| runner OS | Windows 11 x86_64 |
| Windows rustc/cargo | rustc 1.99.1 / cargo 1.99.1 |
| Kani実行環境 | WSL2 Ubuntu 24.04.4 LTS |
| WSL kernel | `6.6.87.2-microsoft-standard-WSL2` |
| WSL rustc/cargo | rustc 1.99.0 / cargo 1.99.0 |
| Kani | Kani Rust Verifier 0.68.0 |
| CBMC | 6.11.0（`cargo kani --version`出力） |
| mise | 2026.11.2 windows-x64 |

## Canonical selective proof

canonical verdictは mise laneである。direct `cargo kani` と `/usr/bin/time` は補助証拠（canonical = mise）としてだけ使う。

| run | command | 時刻（UTC） | exit | result |
| --- | --- | --- | --- | --- |
| pre-mutation | `mise run test-formal-kani arc-tar-entry-progress` | 2026-11-27T15:15Z頃 | 0 | Pass。1/1 harness `proofs::arc_tar_entry_progress_decision_exactness`、0/43 failed、cover 14/14 satisfied、verification time 1.1905043 s |
| mutation | 同上 | 2026-11-27T15:20Z頃 | 1 | mutantを検出。2/43 failed、cover 14/14 satisfied、verification time 1.014198 s |
| post-revert | 同上 | 2026-11-27T15:23:47Z–15:23:56Z | 0 | Pass。1/1 harness、0/43 failed、cover 14/14 satisfied、verification time 2.0711884 s、lane wall 9.485 s |

mutationは `tar_entry_remaining_payload` が producedを無視して declaredをそのまま返す 1 箇所だけの変更である。失敗した assertionは remaining equality `declared - remaining == produced` と exhaustion後の `remaining == 0` であり、unwinding assertion failureではなく semantic violationである。mutantは完全に revertし、revert後の `git diff` は製品実装に残存 mutantがないことを確認した。

## Shell-core connection regression

`crates/ruviform-io-archive/tests/formal_archive.rs` に deterministic regression `tar_entry_progress_connection_regression` を追加した。対象は次の 5 条件である。

1. caller buffer capacityと budgetが entry残量より大きい場合、1 回の readが declaredちょうどまでで終わる。
2. 小さな bufferでの chunked readで producedが 2+2+1 と正確に積み上がり、exhaustionが declared boundaryで起きて finishが成功する。
3. 部分読取後の dropが pendingを作り、`discard_pending` が残り payloadと paddingを drainして次 entryへ復帰する。
4. 部分読取後の `discard_remaining` が declared bytesの `Discarded` receiptを返し、archiveが続行できる。
5. payload未消費での `finish` が `InvalidState` になり、archiveが `Pending` へ残り、`discard_pending` で回復する。

fixture builderが対象ケースを表現できたため、lease外の `tests/common/` は変更していない。

## 正規検証

| command | exit | result |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-io-archive` | 0 | Pass |
| `mise run lint ruviform-io-archive` | 0 | Pass |
| `mise run test ruviform-io-archive` | 0 | Pass。unit 22、archive integration 24、TAR reader 44、TAR writer 15、ZIP reader 34、ZIP writer 10、doctest 10 |
| `mise run test-formal` | 0 | Pass。`formal_ipc` 8/8、`formal_transport` 5/5、`formal_archive` 24/24、`formal_structured` 35/35、`formal_wav` 9/9 |
| `mise run test-formal-kani arc-tar-entry-progress` | 0 | Pass。post-revert canonical resultは上表 |
| `mise run test-formal-kani arc-tar-measured-total` | 0 | Pass。0/58 failed、cover 14/14、verification time 1.0578437 s |
| `mise run test-formal-kani arc-payload-limit` | 0 | Pass。0/16 failed、cover 5/5、verification time 0.0978676 s |
| `mise run test-formal-kani arc-tar-struct` | 0 | Pass。0/83 failed、cover 12/12、verification time 0.24979648 s |
| `mise run test-formal-kani arc-zip-measured-total` | 0 | Pass。0/10 failed、cover 7/7、verification time 0.34427017 s。baseが PR #1051を含むため closest same-group regressionとして実行 |
| `mise run test-formal-kani invalid-selector` | 2 | Pass（期待どおり usage validation）。Kani/environment execution前に停止 |
| `mise exec -- prek run --all-files` | 0 | Pass。evidence-sync working treeで実行 |

## コストと予算

事前予算は 5 CPU-min / 1 GiBである。canonical post-revert selective laneは lane wall 9.485 s、verification time 2.0711884 sだった。補助計測として direct WSL `cargo kani` を `/usr/bin/time` で実行した結果、elapsed wall 5.03 s、maximum resident set size 105,476 KiB、user time 1.99 s、system time 0.53 s、verification time 1.3952447 sだった。この計測は compileを含む補助証拠であり、canonical verdictではない。実測は事前予算内である。

## Consumer impact assessment

対象 SHAで audio-lensと synvelisを再検索した。検索対象は各 repositoryの `crates/`、`Cargo.toml`、`Cargo.lock`である。

| repository | SHA | crate reference | exact symbol |
| --- | --- | --- | --- |
| audio-lens | `ad277403c1ae7609eee29e1bfd361a9c5715fc33` | `ruviform-io-archive` / `ruviform_io_archive`: 0件 | `TarArchiveReader` / `ArchiveReadLimits`: 0件 |
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
| TAR streaming composition残部 / parser-core接続 proof | Deferred | scalar subboundaryの proof範囲外。後続 sliceへ残す |

## 残る未証明領域

- TAR streaming compositionの残部、複数 entry composition、read/discard/pending/finishをまたぐ state machine、shell-core接続条件。
- TAR parser-core接続、TAR metadata/path/link gate、TAR header parse、PAX/GNU extension folding、任意 malformed stream。
- ZIP/TAR decompression correctness、CRC correctness、archive composition proof。
- wire残部と WAV残部。
- Issue 全体の consumer independent regression判断。

## 受入条件との対応

- Required proofは Unwinding Assertions有効の selective laneで Passし、事前予算内だった。
- Able-to-Failは remaining payload mutantを semantic assertion failureで検出し、revert後に selective proofと `test-formal` を再実行した。
- 正規 Property Testと `ruviform-io-archive` regressionは Passした。
- `arc-tar-measured-total`、`arc-payload-limit`、`arc-tar-struct`、baseに存在する `arc-zip-measured-total` の direct regressionは Passした。
- unknown selectorは exit 2で停止した。
- consumer impact assessmentは search-onlyで実施し、NotApplicable判断の根拠を記録した。
- docs indexは編集していない。Coordinatorが統合時に整備する。

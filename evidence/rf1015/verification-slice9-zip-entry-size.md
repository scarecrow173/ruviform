# rf#1015 slice 9 verification — Archive `ARC-PAYLOAD-LIMIT` ZIP declared entry-size gate

## 対象と SHA の役割

- base SHA: `092e18e30110b889bcc6cec3720136c7463a840c`（branch 開始時および最終検証前の `origin/main`）。
- 検証対象 SHA / verified Sync Commit: `e04db55b508364002f55cf501b7b91401d8499cf`。
  ZIP declared entry-size decision core、in-src proof、deterministic regression、
  `arc-zip-entry-size` selective lane、proof placement documentation を含む最後の
  製品/proof/mise 変更 commit である。実装 commit `f582f3cb` に対し、`e04db55b` は
  test module documentationだけを更新する comment-only descendantである。
- evidence sync commit: `59498772b55db2ed9321db11f6d662a612963649`。本書・ledger・spec・skill・`runs.jsonl`
  を含む初期 docs/evidence-only commitである。この SHA 自体を evidence 内へ記録する
  correction commitも docs/evidence-onlyであり、最終 head SHA は PR 本文で追跡する。

## 実装した再設計

`ZipArchiveReader::open_entry_inner` は central directory record の declared
uncompressed size と `max_entry_uncompressed_bytes` だけを受け取る crate-private な
`zip_declared_entry_size_gate_decision` を local header decode の前に呼ぶ。core は
allocation、I/O、`Read`/`Seek`、EOCD/CD/local-header parser、Deflate/zlib、Vec buffering、
CRC、environment access、診断/error message 構築を持たない。判定は従来どおり declared
size が limit より大きい場合だけ reject、等しい場合は accept である。shell は従来どおり
`ArchiveErrorKind::LimitExceeded`、limit name、actual/max を構築し、standalone open、
verify_all total budget、entry output hard cap、cap overflow classification、CRC/size
検証順序、ZIP64/encryption/unsupported/malformed/overlap の observable behavior を変えていない。

新しい proof は production core と同じ `src/zip/read.rs` の
`#[cfg(kani)] mod proofs` にある `arc_zip_entry_size_declared_gate_exactness` である。
declared size と limit を symbolic `u64` にし、assume なし、`#[kani::unwind(2)]`、
Unwinding Assertions 有効で exactness を検証する。cover property は declared size の
boundary-1/boundary/boundary+1、in-cap acceptance、over-cap rejection を覆う。
`cfg(kani)` は proof placement のためだけであり、製品実装を置き換えていない。

production の declared size は ZIP classic record の `u32` field から `u64` へ widening
される。proof が core input domain 全体の `u64` を対象にするのは、この production subset
を含む stronger proof であり、保証差はない。u64 のうち `u32::MAX` を超える入力は
production では到達しないが、core の契約を広い整数範囲で示すだけであり、公開挙動を
変更しない。

本 slice が退役した旧 ZIP declared entry-size shell harness はない。`ARC-PAYLOAD-LIMIT`
全体を proof 済みと二重計上せず、新保証は ZIP declared entry-size gate decision
subboundary に限定する。既存 proptest `zip_entry_size_limit_gate`、
`zip_total_measured_gate`、`zip_writer_roundtrip` は parser shell・integration/proptest
保証として残し、Kani 証明と保証種別を混同しない。

## 環境

- canonical lane runner OS: Windows 11 x86_64。
- Kani execution environment: WSL2 Ubuntu 24.04.4 LTS、kernel
  `6.6.87.2-microsoft-standard-WSL2`。
- Kani Rust Verifier 0.68.0、CBMC 6.11.0。
- canonical verdict は mise lane。直接 `cargo kani` と `/usr/bin/time` は補助証拠
  （canonical = mise）である。

## 実行した正規検証

| lane | command | 時刻（UTC） | exit | verdict | 記録 |
| --- | --- | --- | ---: | --- | --- |
| selective Kani pre-mutation | `mise run test-formal-kani arc-zip-entry-size` | 09:21:57–09:22:04 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.090303086 s |
| selective Kani mutation | `mise run test-formal-kani arc-zip-entry-size` | 09:22:24–09:22:30 | 1 | Fail（検出成功） | mutant で 1/16 failed、cover 4/5、verification time 0.07266181 s |
| selective Kani post-revert | `mise run test-formal-kani arc-zip-entry-size` | 09:22:50–09:22:56 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.08727401 s |
| selective Kani final sync commit | `mise run test-formal-kani arc-zip-entry-size` | 09:34:39–09:34:45 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.08582513 s |
| fmt | `mise run fmt-check` | 09:34:04–09:34:15 | 0 | Pass | rustfmt diff なし |
| check | `mise run check ruviform-io-archive` | 09:34:15–09:34:18 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-io-archive` | 09:34:18–09:34:20 | 0 | Pass | clippy `-D warnings` pass |
| crate regression | `mise run test ruviform-io-archive` | 09:34:20–09:34:27 | 0 | Pass | unit/integration/formal archive/doctest pass |
| proptest post-revert / final sync commit | `mise run test-formal` | 09:34:51–09:34:57 | 0 | Pass | `formal_ipc` 8/8、`formal_transport` 5/5、`formal_archive` 20/20、`formal_structured` 35/35、`formal_wav` 9/9 |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 09:24:05–09:24:05 | 2 | Pass（期待どおり usage error） | unknown selector は environment detection/Kani 実行前に停止 |

`mise run test ruviform-io-archive` では shell-core 接続を確認する deterministic regression
`zip_declared_entry_size_connection_regression` を含む `formal_archive` 20/20 が pass した。
mutation 副産物や不要な regression file は残していない。

## selective Kani とコスト

canonical command は `mise run test-formal-kani arc-zip-entry-size` であり、WSL2 内では
`cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_zip_entry_size`
を実行した。出力では `zip::read::proofs::arc_zip_entry_size_declared_gate_exactness`
だけが checking 対象になり、他 proof を巻き込まなかった。post-revert run は
`SUCCESSFUL`、0/16 failed、cover 5/5 satisfied であり、Unwinding Assertions を無効化
していない。

canonical lane は pre-mutation run が verification time 0.090303086 s / wall 約7 s、
post-revert run が verification time 0.08727401 s / wall 約6 sだった。補助計測
（canonical = mise）では、post-revert後の direct `cargo kani` に対する
`/usr/bin/time -v` が exit 0、wall clock 3.77 s、user CPU 0.44 s + system CPU
0.39 s、max RSS 106,008 KiBだった。compile を含む補助 wall clock と Kani の
verification time は混同しない。支配要因は compile/loader 側であり、proof 自体は
scalar comparison である。事前予算 5 CPU-min / 1 GiB に対し、verification time、CPU
時間、観測メモリのいずれも十分以内である。完了時間や exit 0 を Proof Pass の根拠には
せず、`SUCCESSFUL` verdict、失敗数、cover 到達を証拠にした。

## Mutation-fail と revert

実施した mutant は decision core の declared-size gate 比較
`declared > max_entry_uncompressed_bytes` を `>=` へ変える 1 箇所だけである。
transient mutant は commitしていない。

- mutation run: `mise run test-formal-kani arc-zip-entry-size`、
  2026-11-27T09:22:24Z–09:22:30Z、exit 1。
- 検出: `arc_zip_entry_size_declared_gate_exactness` が `1 of 16 failed`。失敗は boundary
  equality で `Accept` を期待する semantic assertion であり、unwinding assertion
  failure ではない。verification time 0.07266181 s。
- revert: 該当行を `>` へ戻し、`git diff --exit-code -- crates/ruviform-io-archive/src/zip/read.rs`
  が clean であることを確認した。mutant を示す未 revert diff はない。
- revert後再検証: selective Kani が 09:22:50Z–09:22:56Z に exit 0 / pass、
  `test-formal` が 09:23:16Z–09:23:57Z に exit 0 / pass。comment-only descendant
  `e04db55b` でも selective Kani が 09:34:39Z–09:34:45Z に exit 0 / pass、
  `test-formal` が 09:34:51Z–09:34:57Z に exit 0 / passだった。

## consumer impact assessment

2026-11-27T09:26:06Z に audio-lens/synvelis の `crates/`、`Cargo.toml`、
`Cargo.lock` を対象に `ZipArchiveReader`、`ArchiveReadLimits`、
`ruviform-io-archive`、`ruviform_io_archive` を検索し、両 consumer で `rg` exit 1
（no matches）だった。本変更は crate-internal な decision core 抽出であり、公開 API、
`ArchiveReadLimits` default 値、limit 契約、inclusive budget 境界、error kind/name/message、
observable behavior を変えないため、consumer 独立回帰は NotApplicable と判断する。
この判断は Issue 全体の consumer 回帰 AC を完了扱いにしない。

## 手動確認

- selective filter が `arc_zip_entry_size_declared_gate_exactness` だけを選ぶことを lane
  出力で確認した。
- native/WSL2 の両分岐に同じ selector semantics を追加したことを diff で確認した。
- 既存 selector、selector なし aggregate invocation、wire/archive/WAV/structured invocation、
  WSL target dir 強制、WAV `-j1` は変更していない。
- equality case が accept のままであることを proof の boundary assertion と deterministic
  regressionで確認した。
- deterministic regressionでは declared size 15/16/17 に対し accept/accept/reject を確認し、
  over-limit reject は local data byte decode 前の `max_entry_uncompressed_bytes` に対応する
  `LimitExceeded` であることを検査した。regressionでは local header の先頭 byte を壊しても
  over-limit caseだけが size gate で先に拒否されることを確認し、接続が data byte decode 前に
  あることを pin した。
- `Cargo.lock` に差分はない。
- docs index（`docs/**/README.md`）は編集していない。統合時に Coordinator が整備する。
- mutation 実験の副産物や不要な regression file は残していない。

## 未実施 lane / 後続

- selector なし `mise run test-formal-kani`: NotRun。verdict を返せない既知の
  wire/archive harness を含む default aggregate を本 slice の Required evidence にしない。
- 既存 `mise run test-formal-kani arc-payload-limit`: NotApplicable。`src/tar/read.rs`
  と `arc_payload_limit_*` proof に差分がないため再実行していない。
- Workspace full lane、Feature Matrix、MSRV、3OS 横断、GUI-L2/EXT-L3、手動 CI
  dispatch: NotApplicable/NotRun。本 slice の指定外であり、merge 後 main SHA の統合
  検証は Daily CI が担う。
- consumer repo test run: NotApplicable。直接利用なし、public behavior unchanged。
- 残件: `W-FRM-READCLASS`、`W-FRM-OVERSIZE`、`W-IPC-SERDE`、WAV diagnostic boundary、
  read_frame EOF、SampleFormat inventory、consumer 回帰の統合判断。`ARC-PAYLOAD-LIMIT`
  のうち TAR measured total、ZIP measured total/output cap、decompression、CRC、
  parser-core 接続の Kani proof も後続である。

## 残る未証明領域とリスク

- shell が core を呼ぶ接続条件は Kani 未証明。deterministic regression と既存
  proptest/integration coverage に依存する。
- ZIP EOCD/CD/local-header parse、measured total budget、entry output hard cap、cap overflow
  classification、decompression correctness、CRC32 correctness、local/central consistency、
  overlap audit、ZIP64、multi-disk、任意 malformed stream、TAR/ZIP roundtrip は proof boundary
  に入れていない。
- `ARC-PAYLOAD-LIMIT` は TAR declared proofと ZIP declared entry-size proofの union であって、
  TAR measured total・ZIP measured total/output cap・decompression・parser-core接続を含む
  全体の bounded exhaustive proofではない。上記を Pass に含めず、ledger/spec/skill へ同期した。

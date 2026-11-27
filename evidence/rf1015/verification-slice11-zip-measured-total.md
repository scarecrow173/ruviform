# rf#1015 slice 11 verification — Archive `ARC-PAYLOAD-LIMIT` ZIP measured total budget accumulation

## 対象と SHA の役割

- base SHA: `e3281c62043ef36616f6bf2fde643ba0df877458`（branch 開始時および最終検証前の `origin/main`）。
- 検証対象 SHA / verified Sync Commit: `1ffa93a90c11849778bd22c391c33d986ecaabd1`。
  ZIP measured total budget accumulation decision core、in-src proof、deterministic regression、
  `arc-zip-measured-total` selective lane を含む最後の製品/proof/mise 変更 commit である。
- evidence sync commit は本 branch の後続 commit で確定する。evidence は自分自身を含む
  commit hash を固定点として書けないため、最終 head SHA は PR 本文で追跡する。

## 実装した再設計

`ZipArchiveReader::verify_all` は entry を開く前に、`max_total_uncompressed_bytes` と
archive 内で既に measured/accumulated 済みの uncompressed byte total だけを受け取る
crate-private な `zip_measured_total_remaining` を呼び、次 entry へ渡す remaining total
budget を計算する。entry verification 成功後には、accumulated total と entry receipt の
measured uncompressed bytes だけを受け取る crate-private な `zip_measured_total_accumulate`
を呼び、次の accumulated total を計算する。remaining は zero saturation、accumulation は
`u64::MAX` saturation を保持する。

core は allocation、I/O、`Read`/`Seek`、EOCD/CD/local-header parser、Deflate/zlib、
Vec buffering、CRC、environment access、診断/error message 構築を持たない。shell は
従来どおり `open_entry_inner` への remaining budget 受け渡し、entry output cap decision、
entry finish、overlap audit、receipt 構築の順序と error kind/message を維持する。公開 API、
`ArchiveReadLimits` default 値、limit 契約、inclusive budget 境界、error kind/name/message、
CRC/size mismatch、decompression、ZIP64/encryption/unsupported/malformed/overlap の
observable behavior は変更していない。

新しい proof は production core と同じ `src/zip/read.rs` の
`#[cfg(kani)] mod proofs` にある `arc_zip_measured_total_decision_exactness` である。
`max_total_uncompressed_bytes`、`accumulated_before`、`measured_entry_bytes` を全 `u64`
domain で symbolic にし、assume なし、`#[kani::unwind(2)]`、Unwinding Assertions 有効で
remaining budget と accumulation の exactness を検証する。production で期待される
`accumulated_before <= max_total_uncompressed_bytes` invariant は assume せず、全 domain
exactness は production reachable subset を含む stronger proof として記録する。cover
property は remaining boundary-1/boundary equality/boundary+1 saturation、zero-measured
identity、non-overflow sum、`u64::MAX` equality、overflow saturation の 7 経路を覆う。
`cfg(kani)` は proof placement のためだけであり、製品実装を置き換えていない。

本 slice が退役した旧 ZIP measured total shell harness はない。`ARC-PAYLOAD-LIMIT`
全体を proof 済みと二重計上せず、新保証は ZIP measured total budget accumulation
decision subboundary に限定する。既存 proptest `zip_total_measured_gate`、
`zip_writer_roundtrip` は parser shell・integration/proptest 保証として残し、Kani 証明と
保証種別を混同しない。

## 環境

- canonical lane runner OS: Windows 11 x86_64。
- Kani execution environment: WSL2 Ubuntu 24.04.4 LTS、kernel
  `6.6.87.2-microsoft-standard-WSL2`。
- Kani Rust Verifier 0.68.0、CBMC 6.11.0。WSL2 側 rustc 1.99.0、Windows 側 rustc 1.99.1。
- canonical verdict は mise lane。直接 `cargo kani` と `/usr/bin/time` は補助証拠
  （canonical = mise）である。

## 実行した正規検証

| lane | command | 時刻（UTC） | exit | verdict | 記録 |
| --- | --- | --- | ---: | --- | --- |
| selective Kani pre-mutation | `mise run test-formal-kani arc-zip-measured-total` | 12:51:21–12:51:29 | 0 | Pass | 1/1 harness、0/10 failed、cover 7/7、verification time 0.2987511 s |
| selective Kani mutation | `mise run test-formal-kani arc-zip-measured-total` | 12:51:57–12:52:03 | 1 | Fail（検出成功） | mutant で 2/10 failed、cover 6/7、verification time 0.31026772 s |
| selective Kani post-revert | `mise run test-formal-kani arc-zip-measured-total` | 12:52:15–12:52:21 | 0 | Pass | 1/1 harness、0/10 failed、cover 7/7、verification time 0.35422927 s |
| direct regression: ZIP output cap | `mise run test-formal-kani arc-zip-output-cap` | 12:53:17–12:53:24 | 0 | Pass | 1/1 harness、0/38 failed、cover 6/6、verification time 0.26061136 s |
| direct regression: ZIP entry-size | `mise run test-formal-kani arc-zip-entry-size` | 12:53:24–12:53:30 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.08630775 s |
| direct regression: ZIP index | `mise run test-formal-kani arc-idx-limit` | 12:53:30–12:53:37 | 0 | Pass | 1/1 harness、0/19 failed、cover 8/8、verification time 0.15108013 s |
| group regression: TAR declared | `mise run test-formal-kani arc-payload-limit` | 12:53:37–12:53:43 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.084754534 s |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 12:53:43–12:53:44 | 2 | Pass（期待どおり usage error） | unknown selector は environment detection/Kani 実行前に停止 |
| proptest post-revert | `mise run test-formal` | 12:53:57–12:54:40 | 0 | Pass | `formal_ipc` 8/8、`formal_transport` 5/5、`formal_archive` 22/22、`formal_structured` 35/35、`formal_wav` 9/9 |
| fmt | `mise run fmt-check` | 12:57:22–12:57:34 | 0 | Pass | rustfmt diff なし |
| check | `mise run check ruviform-io-archive` | 12:57:34–12:57:36 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-io-archive` | 12:57:36–12:57:38 | 0 | Pass | clippy `-D warnings` pass |
| crate regression | `mise run test ruviform-io-archive` | 12:57:38–12:57:42 | 0 | Pass | unit/integration/formal archive/doctest pass（`formal_archive` 22/22、doctest 10/10） |

`mise run test ruviform-io-archive` では shell-core 接続を確認する deterministic regression
`zip_measured_total_connection_regression` を含む `formal_archive` 22/22 が pass した。
mutation 副産物や不要な regression file は残していない。

## selective Kani とコスト

canonical command は `mise run test-formal-kani arc-zip-measured-total` であり、WSL2 内では
`cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_zip_measured_total`
を実行した。出力では `zip::read::proofs::arc_zip_measured_total_decision_exactness`
だけが checking 対象になり、他 proof を巻き込まなかった。post-revert run は
`SUCCESSFUL`、0/10 failed、cover 7/7 satisfied であり、Unwinding Assertions を無効化
していない。

canonical lane は pre-mutation run が verification time 0.2987511 s / wall 約8 s、
post-revert run が verification time 0.35422927 s / wall 約6 sだった。補助計測
（canonical = mise）では、post-revert後の direct `cargo kani` に対する
`/usr/bin/time -v` が exit 0、wall clock 3.78 s、user CPU 0.72 s + system CPU
0.37 s、max RSS 106,684 KiBだった。compile を含む補助 wall clock と Kani の
verification time は混同しない。支配要因は compile/loader 側であり、proof 自体は
scalar saturation arithmetic である。事前予算 5 CPU-min / 1 GiB に対し、verification
time、CPU 時間、観測メモリのいずれも十分以内である。完了時間や exit 0 を Proof Pass
の根拠にはせず、`SUCCESSFUL` verdict、失敗数、cover 到達を証拠にした。

## Mutation-fail と revert

実施した mutant は accumulation semantics を identity へ変え、
`zip_measured_total_accumulate` が `measured_entry_bytes` を加えず `accumulated_before`
をそのまま返す 1 箇所だけである。transient mutant は commitしていない。

- mutation run: `mise run test-formal-kani arc-zip-measured-total`、
  2026-11-27T12:51:57Z–12:52:03Z、exit 1。
- 検出: `arc_zip_measured_total_decision_exactness` が `2 of 10 failed`。失敗は
  non-overflow sum と exact `u64::MAX` boundary を期待する semantic assertion であり、
  unwinding assertion failure ではない。verification time 0.31026772 s。
- revert: 該当行を saturating accumulation へ戻し、`git diff --exit-code --
  crates/ruviform-io-archive/src/zip/read.rs` が clean であることを確認した。mutant を
  示す未 revert diff はない。
- revert後再検証: selective Kani が 12:52:15Z–12:52:21Z に exit 0 / pass、
  `test-formal` が 12:53:57Z–12:54:40Z に exit 0 / pass。

## shell-core接続 regression

deterministic regression は既存 `tests/common/` fixture builderだけを使う。2 entry
3 byte + 5 byte の measured total が budget 8 の equality で `verify_all` に成功し、receipt
の payload bytes が measured sum 8 になることを確認する。3 byte + 6 byte で measured total が
budget 8 を 1 byte 超える場合は、前 entry の measured bytes が remaining budget を減らし
`LimitExceeded` になることを確認する。3 byte + 5 byte + 0 byte では、budgetをちょうど使い切った
後の zero-length entryでも `verify_all` に成功し、payload total 8 が変わらないことを確認する。
この regression は複数 entry composition、error priority、parser correctness の Kani proof や
oracle 拡張ではない。shell が core を呼ぶ接続条件は Kani 未証明であり、proptest/deterministic
regression coverageに依存する保証差として残す。

## consumer impact assessment

2026-11-27T12:55Z 頃に audio-lens `af20701ba3ed64efc4964c9c1faaea18139eb6cb` と
synvelis `61573b66f1be4aba923118db290c403483d4485f` の `crates/`、`Cargo.toml`、
`Cargo.lock` を対象に、crate reference として `ruviform-io-archive` / `ruviform_io_archive`、
exact symbol として `ZipArchiveReader` / `ArchiveReadLimits` を `rg` で検索した。両 consumer、
両検索で対象 scope 内の件数は 0 だった。本変更は crate-internal な decision core 抽出であり、
公開 API、`ArchiveReadLimits` default 値、limit 契約、inclusive budget 境界、error
kind/name/message、observable behavior を変えないため、consumer 独立回帰は NotApplicable と
判断する。この判断は Issue 全体の consumer 回帰 AC を完了扱いにしない。

## 手動確認

- selective filter が `arc_zip_measured_total_decision_exactness` だけを選ぶことを lane
  出力で確認した。
- native/WSL2 の両分岐に同じ selector semantics を追加したことを diff で確認した。
- 既存 selector、selector なし aggregate invocation、wire/archive/WAV/structured invocation、
  WSL target dir 強制、WAV `-j1` は変更していない。
- remaining budget の equality は inclusive のままであり、zero-length entry acceptance、
  receipt accumulation、cap/overflow classification を変えていないことを proof、deterministic
  regression、既存 proptestで確認した。
- measured total decision だけを coreへ移し、decompression、CRC、parser、overlap audit、
  entry output cap decision を coreへ移していないことを diffで確認した。
- `Cargo.lock` に差分はない。
- docs index（`docs/**/README.md`）は編集していない。統合時に Coordinator が整備する。
- mutation 実験の副産物や不要な regression file は残していない。

## 未実施 lane / 後続

- selector なし `mise run test-formal-kani`: NotRun。verdict を返せない既知の
  wire/archive harness を含む default aggregate を本 slice の Required evidence にしない。
- `mise run test-formal-kani arc-tar-struct`: NotApplicable。`src/tar/read.rs` と
  `arc_tar_struct_*` proof に差分がないため再実行していない。
- Workspace full lane、Feature Matrix、MSRV、3OS 横断、GUI-L2/EXT-L3、手動 CI
  dispatch: NotApplicable/NotRun。本 slice の指定外であり、merge 後 main SHA の統合
  検証は Daily CI が担う。
- consumer repo test run: NotApplicable。crate reference も exact-symbol reachability もなく、
  public behavior unchanged。
- 残件: `W-FRM-READCLASS`、`W-FRM-OVERSIZE`、`W-IPC-SERDE`、WAV diagnostic boundary、
  read_frame EOF、SampleFormat inventory、consumer 回帰の統合判断。`ARC-PAYLOAD-LIMIT`
  のうち TAR measured total、decompression、CRC、parser-core 接続の Kani proof も後続である。

## 残る未証明領域とリスク

- shell が core を呼ぶ接続条件は Kani 未証明。deterministic regression と既存
  proptest/integration coverage に依存する。
- ZIP EOCD/CD/local-header parse、decompression correctness、CRC32 correctness、
  local/central consistency、overlap audit、ZIP64、multi-disk、任意 malformed stream、
  TAR/ZIP roundtrip、TAR measured total、複数 entry composition、error priority は proof
  boundary に入れていない。
- `ARC-PAYLOAD-LIMIT` は TAR declared proof、ZIP declared entry-size proof、ZIP output
  hard-cap/cap-overflow classification proof、ZIP measured total budget accumulation proofの
  union であって、TAR measured total・decompression・CRC・parser-core接続・archive composition
  を含む全体の bounded exhaustive proofではない。上記を Pass に含めず、ledger/spec/skill へ
  同期した。

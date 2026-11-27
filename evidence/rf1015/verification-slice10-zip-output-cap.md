# rf#1015 slice 10 verification — Archive `ARC-PAYLOAD-LIMIT` ZIP entry output hard-cap / cap-overflow classification

## 対象と SHA の役割

- base SHA: `5d54434f45a1cc0741d241f60747705a1bdeb0f1`（branch 開始時および最終検証前の `origin/main`）。
- 検証対象 SHA / verified Sync Commit: `93df62a7b49a6f57e4fb2a27d68cb64b9079e416`。
  ZIP entry output cap decision core、in-src proof、deterministic regression、
  `arc-zip-output-cap` selective lane を含む最後の製品/proof/mise 変更 commit である。
- evidence sync commit: `1836626c34c03aba0b97e06008e7895d1dd3a2b7`（`docs(formal): sync zip output cap proof evidence`）。
  evidence は自分自身を含む commit hash を固定点として書けないため、最終 head SHA は
  PR 本文で追跡する。

## 実装した再設計

`ZipArchiveReader::open_entry_inner` は central directory record の declared
uncompressed size と `verify_all` から渡される remaining total budget input だけを受け取る
crate-private な `zip_entry_output_cap_decision` を呼ぶ。standalone open では input が
`None`、hard cap は declared size である。budget input がある場合、hard cap は
`min(declared, remaining)` である。`ZipEntryReader::probe` は同じ decision 値を使い、
cap を超える output がある場合に、cap が remaining budget 由来で declared を下回るときだけ
`LimitExceeded`、それ以外を `SizeMismatch` へ写像する。`remaining == declared` の equality は
budget-capped ではないので `SizeMismatch` である。

core は allocation、I/O、`Read`/`Seek`、EOCD/CD/local-header parser、Deflate/zlib、
Vec buffering、CRC、environment access、診断/error message 構築を持たない。shell は
従来どおり error kind、message、CRC/size 検証順序を構築し、declared entry-size gate、
standalone open、verify_all total accumulation、decompression、CRC、ZIP64/encryption/
unsupported/malformed/overlap の observable behavior を変えていない。公開 API、
`ArchiveReadLimits` default 値、limit 契約、error kind/name/message は変更していない。

新しい proof は production core と同じ `src/zip/read.rs` の
`#[cfg(kani)] mod proofs` にある `arc_zip_output_cap_decision_exactness` である。
declared size、remaining budget、`None`/`Some` 選択を symbolic にし、assume なし、
`#[kani::unwind(2)]`、Unwinding Assertions 有効で hard cap と classification の exactnessを
検証する。`None`/`Some` は symbolic bool と symbolic `u64` から構成する full-domain encoding
であり、input narrowing ではない。cover property は remaining boundary-1/boundary/boundary+1、
budget-capped classification、declared-capped classification、no-budget classification を覆う。
`cfg(kani)` は proof placement のためだけであり、製品実装を置き換えていない。

production の declared size は ZIP classic record の `u32` field から `u64` へ widening
される。proof が core input domain 全体の `u64` を対象にするのは、この production subset
を含む stronger proof であり、保証差はない。remaining budget は production と同じ `u64`
scalar input である。

本 slice が退役した旧 ZIP output cap shell harness はない。`ARC-PAYLOAD-LIMIT`
全体を proof 済みと二重計上せず、新保証は ZIP entry output hard-cap / cap-overflow
classification decision subboundary に限定する。既存 proptest `zip_entry_size_limit_gate`、
`zip_total_measured_gate`、`zip_writer_roundtrip` は parser shell・integration/proptest
保証として残し、Kani 証明と保証種別を混同しない。

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
| selective Kani pre-mutation | `mise run test-formal-kani arc-zip-output-cap` | 11:38:41–11:38:46 | 0 | Pass | 1/1 harness、0/38 failed、cover 6/6、verification time 0.22961685 s |
| selective Kani mutation | `mise run test-formal-kani arc-zip-output-cap` | 11:39:12–11:39:18 | 1 | Fail（検出成功） | mutant で 2/38 failed、cover 2/6、verification time 0.25769493 s |
| selective Kani post-revert | `mise run test-formal-kani arc-zip-output-cap` | 11:39:38–11:39:44 | 0 | Pass | 1/1 harness、0/38 failed、cover 6/6、verification time 0.25182262 s |
| direct regression: ZIP entry-size | `mise run test-formal-kani arc-zip-entry-size` | 11:40:24–11:40:30 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.085352175 s |
| direct regression: ZIP index | `mise run test-formal-kani arc-idx-limit` | 11:40:30–11:40:37 | 0 | Pass | 1/1 harness、0/19 failed、cover 8/8、verification time 0.15658887 s |
| group regression: TAR declared | `mise run test-formal-kani arc-payload-limit` | 11:40:37–11:40:43 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.083519146 s |
| fmt | `mise run fmt-check` | 11:41:05–11:41:16 | 0 | Pass | rustfmt diff なし |
| check | `mise run check ruviform-io-archive` | 11:41:16–11:41:18 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-io-archive` | 11:41:18–11:41:21 | 0 | Pass | clippy `-D warnings` pass |
| crate regression | `mise run test ruviform-io-archive` | 11:41:21–11:41:28 | 0 | Pass | unit/integration/formal archive/doctest pass |
| proptest post-revert | `mise run test-formal` | 11:41:28–11:42:12 | 0 | Pass | `formal_ipc` 8/8、`formal_transport` 5/5、`formal_archive` 21/21、`formal_structured` 35/35、`formal_wav` 9/9 |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 11:40:54–11:40:54 | 2 | Pass（期待どおり usage error） | unknown selector は environment detection/Kani 実行前に停止 |

`mise run test ruviform-io-archive` では shell-core 接続を確認する deterministic regression
`zip_output_cap_connection_regression` を含む `formal_archive` 21/21 が pass した。
mutation 副産物や不要な regression file は残していない。

## selective Kani とコスト

canonical command は `mise run test-formal-kani arc-zip-output-cap` であり、WSL2 内では
`cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_zip_output_cap`
を実行した。出力では `zip::read::proofs::arc_zip_output_cap_decision_exactness`
だけが checking 対象になり、他 proof を巻き込まなかった。post-revert run は
`SUCCESSFUL`、0/38 failed、cover 6/6 satisfied であり、Unwinding Assertions を無効化
していない。

canonical lane は pre-mutation run が verification time 0.22961685 s / wall 約5 s、
post-revert run が verification time 0.25182262 s / wall 約6 sだった。補助計測
（canonical = mise）では、post-revert後の direct `cargo kani` に対する
`/usr/bin/time -v` が exit 0、wall clock 3.74 s、user CPU 0.66 s + system CPU
0.36 s、max RSS 105,228 KiBだった。compile を含む補助 wall clock と Kani の
verification time は混同しない。支配要因は compile/loader 側であり、proof 自体は
scalar comparison である。事前予算 5 CPU-min / 1 GiB に対し、verification time、CPU
時間、観測メモリのいずれも十分以内である。完了時間や exit 0 を Proof Pass の根拠には
せず、`SUCCESSFUL` verdict、失敗数、cover 到達を証拠にした。

## Mutation-fail と revert

実施した mutant は decision core の overflow classification 比較
`hard_cap < declared` を `<=` へ変える 1 箇所だけである。transient mutant は commitしていない。

- mutation run: `mise run test-formal-kani arc-zip-output-cap`、
  2026-11-27T11:39:12Z–11:39:18Z、exit 1。
- 検出: `arc_zip_output_cap_decision_exactness` が `2 of 38 failed`。失敗は remaining
  equalityと budgetなしで declared-overproductionを期待する semantic assertion であり、
  unwinding assertion failure ではない。verification time 0.25769493 s。
- revert: 該当行を `<` へ戻し、`git diff --exit-code -- crates/ruviform-io-archive/src/zip/read.rs`
  が clean であることを確認した。mutant を示す未 revert diff はない。
- revert後再検証: selective Kani が 11:39:38Z–11:39:44Z に exit 0 / pass、
  `test-formal` が 11:41:28Z–11:42:12Z に exit 0 / pass。

## shell-core接続 regression

deterministic regression は既存 `tests/common/` fixture builderだけを使う。
standalone openでは declared size 8に対し物理 output 9 byteの entryを作り、8 byteだけが
yieldされ、超過 probeが `SizeMismatch` になることを確認する。`verify_all` では単一 entryに
対し remaining budget 7/8/9、declared size 8を使い、boundary-1では `LimitExceeded`、
equalityでは `SizeMismatch`、boundary+1でも capが declared側に留まり `SizeMismatch` になることを
確認する。この regression は measured total budget accumulation の proof や oracle 拡張ではない。
shell が core を呼ぶ接続条件は Kani 未証明であり、proptest/deterministic regression coverageに
依存する保証差として残す。

## consumer impact assessment

2026-11-27T11:41:25Z–11:41:28Z に audio-lens `614c0de439be4e1554668cf2a70f5910ac1c49e6` と
synvelis `61573b66f1be4aba923118db290c403483d4485f` の `crates/`、`Cargo.toml`、
`Cargo.lock` を対象に `ZipArchiveReader`、`ArchiveReadLimits`、`ruviform-io-archive`、
`ruviform_io_archive` を検索し、両 consumer で `rg` exit 1（no matches）だった。
repo 全体では synvelis docs/evidence に設計参照があるだけで、audio-lens には一致がなかった。
本変更は crate-internal な decision core 抽出であり、公開 API、`ArchiveReadLimits` default 値、
limit 契約、inclusive budget 境界、error kind/name/message、observable behavior を変えないため、
consumer 独立回帰は NotApplicable と判断する。この判断は Issue 全体の consumer 回帰 AC を
完了扱いにしない。

## 手動確認

- selective filter が `arc_zip_output_cap_decision_exactness` だけを選ぶことを lane
  出力で確認した。
- native/WSL2 の両分岐に同じ selector semantics を追加したことを diff で確認した。
- 既存 selector、selector なし aggregate invocation、wire/archive/WAV/structured invocation、
  WSL target dir 強制、WAV `-j1` は変更していない。
- equality case が `SizeMismatch` のままであることを proof の boundary assertion と deterministic
  regressionで確認した。
- hard cap計算と overflow classificationだけを coreへ移し、measured total accumulation、
  decompression、CRC、parserを coreへ移していないことを diffで確認した。
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
- consumer repo test run: NotApplicable。直接利用なし、public behavior unchanged。
- 残件: `W-FRM-READCLASS`、`W-FRM-OVERSIZE`、`W-IPC-SERDE`、WAV diagnostic boundary、
  read_frame EOF、SampleFormat inventory、consumer 回帰の統合判断。`ARC-PAYLOAD-LIMIT`
  のうち TAR measured total、ZIP measured total budget accumulation、decompression、CRC、
  parser-core 接続の Kani proof も後続である。

## 残る未証明領域とリスク

- shell が core を呼ぶ接続条件は Kani 未証明。deterministic regression と既存
  proptest/integration coverage に依存する。
- ZIP EOCD/CD/local-header parse、measured total budget accumulation、decompression correctness、
  CRC32 correctness、local/central consistency、overlap audit、ZIP64、multi-disk、任意 malformed
  stream、TAR/ZIP roundtrip、TAR measured total は proof boundary に入れていない。
- `ARC-PAYLOAD-LIMIT` は TAR declared proof、ZIP declared entry-size proof、ZIP output
  hard-cap/cap-overflow classification proofの union であって、TAR measured total・ZIP measured
  total budget accumulation・decompression・CRC・parser-core接続を含む全体の bounded exhaustive
  proofではない。上記を Pass に含めず、ledger/spec/skill へ同期した。

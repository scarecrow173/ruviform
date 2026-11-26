# rf#1015 slice 4 verification — Archive `ARC-IDX-LIMIT`

## 対象と SHA の役割

- base SHA: `62da651850b45833b9d1add7e76225a545594ba5`（branch 開始時の `origin/main`）。
- 検証対象 SHA / verified Sync Commit: `acb773db88ef8e40bfb43a491336b00fad7cbb47`。
  ZIP index decision core、in-src proof、旧 parser-facing harness 退役、`arc-idx-limit` selective lane を含む最後の製品/proof/mise 変更 commit である。
- evidence sync commit: 本書・ledger・spec・skill・`runs.jsonl` を含む後続の docs/evidence-only commit。evidence は自分自身を含む commit hash を固定点として書けないため、最終 head SHA は PR 本文で追跡する。
- 2026-11-26T12:41Z 時点で `origin/main` は `62da651850b45833b9d1add7e76225a545594ba5` のままだったため、main merge commit はない。

## 実装した再設計

`ZipArchiveReader::new` は EOCD の declared central-directory count/size と `ArchiveReadLimits` だけを受け取る crate-private な `zip_index_gate_decision` を central directory buffering の前に呼ぶ。core は allocation、I/O、`Read`/`Seek`、EOCD/CD parser、Vec buffering、診断構築を持たない。判定順は従来どおり entries budget、metadata budget、accept であり、`ZipIndexGateDecision::{EntriesExceeded, MetadataExceeded, Accept}` へ写像する。shell は従来どおり同じ `ArchiveErrorKind::LimitExceeded`、limit name、actual/max を構築する。公開 API、default 値、EOCD/CD/ZIP64/multi-disk/unsupported/malformed/overlap の observable behavior は変更していない。

新しい proof は production core と同じ `src/zip/read.rs` の `#[cfg(kani)] mod proofs` にある `arc_idx_limit_decision_exactness` である。declared count/size と limit 値を全て symbolic にし、assume なし、`#[kani::unwind(2)]`、Unwinding Assertions 有効で exactness を検証する。cover property は entry count/metadata size の boundary-1/boundary/boundary+1、両方 over-limit の entries-first priority、in-cap acceptance を覆う。`cfg(kani)` は proof placement のためだけであり、製品実装を置き換えていない。

旧 parser-facing `zip_index_gates` harness は残さず退役した。これは失敗隠しではなく、concrete ZIP skeleton と EOCD/CD parser shell を含む verdict 未発行 proof を、pure decision core exactness proof へ redesign したものである。初期の proof 草案では oracle が entries-first priority を正しく写せず selective lane が fail したが、これは製品不具合や mutation ではなく harness/model 誤りであり、assertion を修正してから最終検証した。

## 環境

- canonical lane runner OS: Windows 11 x86_64。
- Kani execution environment: WSL2 Ubuntu。
- Kani Rust Verifier 0.68.0、CBMC 6.11.0。
- canonical verdict は mise lane。直接 `cargo kani` と `/usr/bin/time` は補助証拠（canonical = mise）である。

## 実行した正規検証

| lane | command | 時刻（UTC） | exit | verdict | 記録 |
| --- | --- | --- | ---: | --- | --- |
| fmt | `mise run fmt-check` | 12:41:51–12:42:02 | 0 | Pass | rustfmt diff なし |
| check | `mise run check ruviform-io-archive` | 12:42:02–12:42:04 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-io-archive` | 12:42:04–12:42:07 | 0 | Pass | clippy `-D warnings` pass |
| crate regression | `mise run test ruviform-io-archive` | 12:42:07–12:42:20 | 0 | Pass | unit/integration/formal archive/doctest pass |
| proptest | `mise run test-formal` | 12:42:20–12:43:02 | 0 | Pass | `formal_ipc` 7/7、`formal_transport` 4/4、`formal_archive` 18/18、`formal_structured` 35/35、`formal_wav` 9/9 |
| selective Kani | `mise run test-formal-kani arc-idx-limit` | 12:43:18–12:43:24 | 0 | Pass | 1/1 harness、0/19 failed、cover 8/8、verification time 0.17472197 s |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 12:48:41–12:48:42 | 2 | Pass（期待どおり usage error） | unknown selector は environment detection/Kani 実行前に停止 |
| selective Kani post-revert | `mise run test-formal-kani arc-idx-limit` | 12:50:06–12:50:13 | 0 | Pass | 1/1 harness、0/19 failed、cover 8/8、verification time 0.23537865 s |
| proptest post-revert | `mise run test-formal` | 12:50:13–12:50:23 | 0 | Pass | 5 群全て pass（7/4/18/35/9） |
| prek | `mise exec -- prek run --all-files` | 12:52:51–12:52:54 | 0 | Pass | 全 configured hooks pass、auto-fix diff なし |

`mise run test ruviform-io-archive` では parser-core 接続を担保する既存 proptest `zip_index_limit_gates` / `zip_metadata_limit_gate` を含む `formal_archive` 18/18 が pass した。coverage 不足は確認されず、新しい regression file や mutation 副産物は残していない。

## selective Kani とコスト

canonical command は `mise run test-formal-kani arc-idx-limit` であり、WSL2 内では `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_idx_limit` を実行した。出力では `zip::read::proofs::arc_idx_limit_decision_exactness` だけが checking 対象になり、他 proof を巻き込まなかった。post-revert run は `SUCCESSFUL`、0/19 failed、cover 8/8 satisfied であり、Unwinding Assertions を無効化していない。

canonical lane は pre-mutation run が verification time 0.17472197 s / wall 約6 s、post-revert run が verification time 0.23537865 s / wall 約7 sだった。補助計測（canonical = mise）では、mutation前の同じ filter の direct `cargo kani` が verification time 0.15054883 s、wall clock 3.63 s、user CPU 0.52 s + system CPU 0.39 s、`/usr/bin/time` max RSS 105,228 KiBだった。post-revert後の補助 `/usr/bin/time` run（2026-11-26T13:15:21Z–13:15:37Z、compileを含む）は exit 0、wall clock 16.20 s、user CPU 3.21 s + system CPU 2.45 s、max RSS 152,908 KiBだった。compileを含む補助 wall clock と Kaniの verification timeは混同しない。事前予算 5 CPU-min / 1 GiB に対し、verification time・CPU 時間・観測メモリのいずれも十分以内である。完了時間や exit 0 を Proof Pass の根拠にはせず、`SUCCESSFUL` verdict、失敗数、cover 到達を証拠にした。

## Mutation-fail と revert

実施した mutant は decision core の entries gate 比較 `u64::from(cd_count) > u64::from(max_entries)` を `>=` へ変える 1 箇所だけである。transient mutant は commitしていない。

- mutation run: `mise run test-formal-kani arc-idx-limit`、2026-11-26T12:49:49Z–12:49:55Z、exit 1。
- 検出: `arc_idx_limit_decision_exactness` が `2 of 19 failed`。失敗は boundary equality で `MetadataExceeded` / `Accept` を期待する semantic assertion であり、unwinding assertion failure ではない。verification time 0.22488578 s。
- revert: 該当行を `>` へ戻し、`rg` で `if u64::from(cd_count) > u64::from(max_entries)` を確認した。製品/proof/mise の 3 ファイルに未 revert diff はない。
- revert後再検証: selective Kani が 12:50:06Z–12:50:13Z に exit 0 / pass、`test-formal` が 12:50:13Z–12:50:23Z に exit 0 / pass。

## consumer impact assessment

2026-11-26T12:50:40Z–12:50:41Z に audio-lens/synvelis の `crates/`、`Cargo.toml`、`Cargo.lock` を対象に `ZipArchiveReader`、`ArchiveReadLimits`、`ruviform-io-archive`、`ruviform_io_archive` を検索し、`rg` exit 1（no matches）だった。synvelis リポジトリ全体には設計/docs/evidence 上の参照があるが、code/manifest/lockfile に直接利用はない。本変更は crate-internal な decision core 抽出であり、公開 API・limit 契約・error kind/limit name/actual/max・observable behavior を変えないため、consumer 独立回帰は NotApplicable と判断する。この判断は Issue 全体の consumer 回帰 AC を完了扱いにしない。

## 手動確認

- selective filter が `arc_idx_limit_decision_exactness` だけを選ぶことを lane 出力で確認した。
- `w-ipc-op` / `sd-csv-limits` / `sd-byte-budget` selector の command/filter は変更していない。
- selector なし aggregate invocation、wire/archive/WAV/structured invocation、target dir 方針は変更していない。
- `Cargo.lock` に差分はない。
- docs index（`docs/**/README.md`）は編集していない。統合時に Coordinator が整備する。
- mutation 実験の副産物や不要な regression file は残していない。

## 未実施 lane / 後続

- selector なし `mise run test-formal-kani`: NotRun。verdict を返せない既知の wire/archive harness を含む default aggregate を本 slice の Required evidence にしない。
- Workspace full lane、Feature Matrix、MSRV、3OS 横断、GUI-L2/EXT-L3、手動 CI dispatch: NotApplicable/NotRun。本 slice の指定外であり、merge 後 main SHA の統合検証は Daily CI が担う。
- consumer repo test run: NotApplicable。直接利用なし、public behavior unchanged。
- 残件: `ARC-PAYLOAD-LIMIT`、`W-FRM-*`、他 wire/archive Property、WAV diagnostic boundary、read_frame EOF、SampleFormat inventory、consumer 回帰の統合判断。

## 残る未証明領域とリスク

- parser が core を呼ぶ接続条件は Kani 未証明。既存 proptest/integration coverage に依存する。
- EOCD scan、実 CD parse、Vec allocation、decompression、parser skeleton は proof boundary に入れていない。
- 複数 entry、ZIP64、deflate payload、local/central 不整合、overlap audit はこの proof の保証に含めない。
- 上記を Pass に含めず、ledger/spec/skill へ同期した。

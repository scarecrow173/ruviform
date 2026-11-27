# rf#1015 slice 15 verification — TAR 固定 metadata block の計上

## 検証対象

`ARC-IDX-LIMIT` のうち、`TarArchiveReader::advance` が読み取った 512 バイトの metadata
block 1 個の計上を検証した。旧検証は `tar_metadata_gate` による proptest であり、
対応する旧 Kani harness はない。slice 14 の拡張 payload の確保前判定と既存 proof は維持した。

| 役割 | SHA / 参照 |
| --- | --- |
| base SHA | `ddd27b255862e6ec1659c620ac49526159796b0b` |
| 検証対象 SHA / verified Sync Commit | `fca426594c7294547d19e3af65f44f6726d59ef0` |
| 変異・初回検証の対象 SHA | `d7c0d10c47f5f254279c05ee4a9f524e4152b24f` |
| evidence sync commit | 本ファイルを含む後続 commit |
| 最終 head SHA | PR 本文で追跡 |

検証対象 SHA は製品 core、in-src proof、接続回帰、mise selector を含む最後の commit である。
初回 lint の失敗（型パスの不要な修飾）を接続回帰ファイルの 1 行で解消した。
両 SHA 間に製品 core、proof、selector の差分がないことを `git diff --exit-code` で確認し、
変異検出は初回 SHA の結果を使う。`runs.jsonl` は変異を初回 SHA に、prek を最終の検証対象 SHA 上の
evidence-sync 作業ツリーに紐づける。

## Runtime Verification

Windows 11 x86_64（10.0.26300.9974）、rustc/cargo 1.99.1、mise 2026.11.2 で実行した。

| コマンド | UTC（2026-11-27） | exit | 結果 |
| --- | --- | ---: | --- |
| `mise run fmt-check` | 18:42:26–18:42:37 | 0 | Pass |
| `mise run check ruviform-io-archive` | 18:42:37–18:42:40 | 0 | Pass |
| `mise run lint ruviform-io-archive` | 18:42:40–18:42:45 | 0 | Pass（`-D warnings`） |
| `mise run test ruviform-io-archive` | 18:42:45–18:42:51 | 0 | 204 件 Pass（接続回帰を含む archive formal 26 件、doctest 10 件） |
| `mise run test-formal` | 18:42:51–18:42:57 | 0 | 変異の復元後に 83 件 Pass（IPC 8、transport 5、archive 26、structured 35、WAV 9） |
| `mise exec -- prek run --all-files` | 18:44:26–18:44:29 | 0 | evidence-sync 作業ツリーで Pass、自動修正なし |

`tar_metadata_block_connection_regression` は合成 fixture の通常ヘッダーとゼロ終端 block を使う。
上限 512 では通常エントリまたは終端を返し、511 では `max_metadata_bytes: declared 512 exceeds limit 511`
の `LimitExceeded` を返す。最初の 1 バイトの後で `BrokenPipe` を返す reader では、上限 511 でも
予算エラーより先に `Io` を返し、元のエラーを `source()` に保持する。
`read_extension` の実装、公開 API、依存関係、RT 契約は変更していない。

### I/O E2E

Windows 11 x86_64（10.0.26300.9974）、rustc/cargo 1.99.1、mise 2026.11.2 で、PR の
`1b024afa74710462adab090ba8497866bf7b5154` を検証した。この SHA と verified Sync Commit の差分は
docs/skill/evidence のみ。追加の feature 引数は指定せず、`ruviform-io-e2e` が定義する組み合わせを使った
（TAR の合成に関係する `ruviform-io-compression` の `gzip` / `zstd` を含む）。

| コマンド | UTC（2026-11-27） | wall | exit | 結果 |
| --- | --- | ---: | ---: | --- |
| `mise run check-io-e2e` | 19:12:11–19:12:32 | 20.961 s | 0 | Pass。全 test target の型検査のみ |
| `mise run test-io-e2e` | 19:12:32–19:12:54 | 21.670 s | 0 | Pass。88 passed、0 failed、2 ignored |

`test-io-e2e` のログで `ok` を確認した TAR 関連ケースを次に示す。

| test target | `TarArchiveReader` を通したケース |
| --- | --- |
| `archive_compression_staged` | `tar_gzip_staged_round_trip_with_layered_completion_proofs`, `tar_zstd_staged_round_trip`, `declared_size_hint_does_not_relax_streamed_budget` |
| `extraction` | `tar_round_trip_extracts_verified_tree`, `traversal_entry_is_refused_and_nothing_escapes`, `nested_traversal_is_refused`, `absolute_path_entry_is_refused`, `symlink_and_special_entries_are_refused`, `case_fold_collision_is_refused_and_tree_removed`, `extraction_budget_binds_total_output_bytes`, `failed_extraction_cleans_or_hands_out_recovery` |
| `fault_boundary` | `composite_publish_conflict_keeps_first_writer_and_recovers_temp` |
| `negative_fuzz` | `corrupted_tar_fixtures_fail_with_typed_archive_errors`, `mutation_sweep_never_publishes_unverified_composite` |
| `rooted_boundary` | `rooted_dir_reads_extraction_tree` |

`archive_compression_staged` の `dropped_encoder_without_finish_yields_unverifiable_bytes` と
`truncated_gzip_member_fails_finish_before_publish` も通過した。両ケースは TAR を書き込むが、
読み取り側の判定は gzip の完了検証までで、TAR Reader の検証には数えない。
`corpus_replay_bounded` と `large_staged_4gib_boundary` は明示的に ignored であり `NotRun`。
この実行は固定 fixture を用いた Ruviform 内部の合成回帰であり、consumer アプリの独立回帰や追加の形式証明ではない。

## Formal Verification

Windows 11 x86_64（10.0.26300.9974）から WSL2 Ubuntu 24.04.4 LTS x86_64（kernel
`6.6.87.2-microsoft-standard-WSL2`）の mise 分岐を実行した。WSL の rustc/cargo は 1.99.0、
Kani Rust Verifier は 0.68.0、CBMC は 6.11.0。

`mise run test-formal-kani arc-tar-block-budget` は `arc_tar_block_budget_decision_exactness`
だけを実行する。保証種別は Bounded Exhaustive。計上済みバイト数と設定上限はそれぞれ全 `u64` とし、
loop・assume・stub・製品実装の置換はない。`#[kani::unwind(2)]` で Unwinding Assertions を有効にした。

oracle は `u128` で `before + 512` を計算し、`u64` で表現不能な合計、上限以下の正確な更新値、
表現可能な超過の正確な要求値を区別する。cover では上限の 1 バイト手前・等号・1 バイト超過、
最大の表現可能値、上限を `u64::MAX` にしたときの最初の表現不能値、計上済みの値がすでに
上限を超える場合を、各判定結果と合わせて確認した。最後のケースを含めるために仮定を追加しておらず、
cover はその状態が実ストリームで到達可能であることを示さない。

| canonical run（初回 SHA） | UTC（2026-11-27） | exit | 結果 | verification time | lane wall |
| --- | --- | ---: | --- | ---: | ---: |
| `mise run test-formal-kani arc-tar-block-budget`（変異前） | 18:36:53–18:37:03 | 0 | `SUCCESSFUL`、0/27 failed、cover 6/6 | 0.20114551 s | 9.649 s |
| 同上（受理条件 `<=` → `<`） | 18:37:20–18:37:44 | 1 | 変異検出、1/27 failed、cover 4/6 | 0.4403503 s | 22.265 s |
| 同上（復元後） | 18:38:11–18:38:18 | 0 | `SUCCESSFUL`、0/27 failed、cover 6/6 | 0.17238195 s | 6.894 s |
| `mise run test-formal-kani invalid-selector` | 18:38:44–18:38:44 | 2 | 期待どおり usage を表示。環境探索前に終了（Windows） | — | 0.497 s |

| canonical run（verified Sync Commit） | UTC（2026-11-27） | exit | 結果 | verification time | lane wall |
| --- | --- | ---: | --- | ---: | ---: |
| `mise run test-formal-kani arc-tar-block-budget` | 18:42:19–18:42:25 | 0 | `SUCCESSFUL`、0/27 failed、cover 6/6 | 0.21418972 s | 5.539 s |
| `mise run test-formal-kani arc-tar-metadata-budget` | 18:42:25–18:42:31 | 0 | `SUCCESSFUL`、0/11 failed、cover 8/8 | 0.67932874 s | 5.917 s |
| `mise run test-formal-kani arc-tar-entry-progress` | 18:42:31–18:42:37 | 0 | `SUCCESSFUL`、0/43 failed、cover 14/14 | 1.0680268 s | 6.385 s |

変異で失敗したのは `in-budget sum must commit exactly, including equality` の assertion だけで、
unwinding failure はなかった。変異を commit せず元の `<=` に戻した。
`git diff --exit-code HEAD -- crates/ruviform-io-archive/src/tar/read.rs` の exit 0 と clean worktree を
確認してから、同じ selective lane と `mise run test-formal` を再実行した。

### コストと選択再実行

事前予算は 5 CPU-min / 1 GiB。2026-11-27 18:42:37–18:42:45 UTC に、
同じ mise selector を実行し、WSL に渡す選択 proof の `cargo kani` の直前に `/usr/bin/time -v` だけを挿入した。
target と filter は変えていない。これは補助証拠（canonical = mise）であり、WSL の invocation の
wall 6.95 s、CPU 1.21 s（user 0.62 s、sys 0.59 s）、最大 RSS 105,004 KiB、exit 0 だった。
最大 RSS は GNU time が記録した値で、Windows ホストや並行プロセスを含む合計メモリではない。
この固定 block 判定には対応する旧 Kani 実測がない。slice 14 の拡張判定や旧 ZIP parser-facing proof と
保証境界が異なるため、これらとの改善率は算出しない。

## Consumer 影響分類

[Consumer 影響分類の運用](../../docs/development-environment.md#consumer-影響分類の運用issue-219) に従い、
両 Consumer の `main` を別々に確認した。ここでの `no impact` は確認したリビジョンに対する
本 PR の TAR Reader 差分の静的判定であり、アプリを実行した結果ではない。

| Consumer | 確認した `main` SHA | 分類 | 根拠 | アプリ独立回帰 |
| --- | --- | --- | --- | --- |
| AudioLens | `b14746b7a34f790572ec1bb64f596b571c624559` | `no impact` | Rust ソース、Cargo manifest / lockfile、設定・スクリプトに archive クレートまたは TAR Reader の参照なし。全追跡テキストでも一致なし | `NotRun`。候補 Ruviform SHA に切り替えたアプリのビルド・精度回帰は未実施 |
| Synvelis | `141679277a9eb76e1de99ba20736daf4b2604544` | `no impact` | 同じ実装・依存範囲で一致なし。全追跡テキストの一致は `docs/` と `evidence/` に限られ、[Backup/Import の採用 ADR](https://github.com/scarecrow173/synvelis/blob/141679277a9eb76e1de99ba20736daf4b2604544/docs/adr/syn-io-01-backup-import-archive-zip-limits-path-link.md#decision-outcome) は ZIP のみを初期対象とし TAR を除外 | `NotRun`。候補 Ruviform SHA に切り替えたアプリのビルド・DAW/RT 回帰は未実施 |

各 Consumer のローカル clone を読み取り専用で検索した。`repo` は `G:/repositry/audio-products/` 配下の
`audio-lens` または `synvelis`、`sha` は上表の値、`pattern` は次のとおり。

```bash
pattern='ruviform[-_]io[-_]archive|TarArchiveReader|ArchiveReadLimits|tar_metadata_block_decision'
git -C "$repo" ls-remote origin refs/heads/main
git -C "$repo" grep -n -I -E "$pattern" "$sha" -- '*.rs' '*Cargo.toml' '*Cargo.lock' '*.toml' '*.yaml' '*.yml' '*.json' '*.sh' '*.ps1'
git -C "$repo" grep -n -I -E "$pattern" "$sha" --
```

`ls-remote` は両リポジトリで上表の SHA を返した。範囲を指定した `grep` は両方で exit 1（該当なし）。
全追跡テキストの `grep` は AudioLens で exit 1、Synvelis で exit 0（設計・証跡のみ）だった。
Ruviform 側でも `git grep -n -E 'ruviform-io-archive' 1b024afa74710462adab090ba8497866bf7b5154 -- '*Cargo.toml'`
で依存宣言を確認し、クレート自身と Workspace メンバー登録を除いた参照はテスト専用 `ruviform-io-e2e` のみだった。
この PR は manifest / lockfile を変更していない。未追跡の構成、将来の依存追加、消費側での候補 SHA の再解決は確認範囲に含まない。

## 未証明領域と未実施の検証

単一 block の算術と判定だけが proof の対象である。複数 entry にまたがる計上、拡張 payload 計上との合成、
TAR/ZIP parser、I/O、エラー優先順位、consumer は対象外とした。接続回帰は具体例の検査であり、
shell-core 接続の全域証明ではない。既存の Property Test や proof は退役・縮小していない。

| 項目 | 判定 | 理由・後続 |
| --- | --- | --- |
| selector なしの `mise run test-formal-kani` | NotRun | 今回の対象外で、verdict 未発行の既知 harness を含む |
| native Kani 分岐 | NotRun | Kani は WSL2 で実行。両分岐の selector と filter は差分で確認 |
| Workspace full / Feature Matrix / MSRV / 3 OS | NotRun | ローカル受入の対象外。統合検証は Daily CI の担当 |
| GUI-L2 / EXT-L3 | NotApplicable | GUI・外部接続の変更なし |
| AudioLens / Synvelis 独立回帰 | NotRun | 上記の静的影響分類のみ実施。候補 SHA を使うアプリの独立回帰と統合受入は未実施 |
| 手動 CI dispatch | NotRun | Daily CI の方針に従い、手動起動しない |

本記録は変更範囲のローカル証拠であり、merge 後 main の対象 SHA に対する CI の代替ではない。
Issue 全体の Wire/Archive の受入や両 consumer の受入をこの slice で完了扱いにしない。
docs index は Coordinator が統合時に整備する。

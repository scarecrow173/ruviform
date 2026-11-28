# rf#1015 slice 16: TAR メタデータ上限の複数エントリ回帰

## 検証対象

`ARC-IDX-LIMIT` のうち、TAR のヘッダーと拡張 payload の計上を複数エントリにまたがって検証した。保証種別は Runtime Verification（Example Tested）。base は `0fe0c22194280da012c9a85a6d31ab69bab30a37`、検証対象 SHA は `a644d31d932028dea3f079211c840efbb7add390`。後続の commit は本証跡だけを追加する。

[`metadata_bytes_limit_accumulates_across_pax_entries`](../../crates/ruviform-io-archive/tests/tar_reader.rs) は、既存 fixture で local PAX `path` と通常ファイルの組を 2 つ作る。PAX payload の宣言長は 22 バイトと 30 バイトで、どちらにもパディングがある。通常ファイルには空でない payload（`alpha`、`beta-beta`）を入れる。既存の `reader_with` から公開 API を呼び、製品内部の計上関数や状態の注入を使わない。

| 上限 | 独立した期待値 | 観測結果 |
| --- | --- | --- |
| 2,612 バイト | ヘッダー 4 個と最初のゼロブロックの `5 × 512` に、PAX 宣言長 `22 + 30` を足す | 2 つの PAX 適用後の名前とファイル内容が一致。`next_entry()` は `None`、`finish()` は残るゼロブロックを読み、EOF で成功 |
| 1,587 バイト | 2 番目の PAX ヘッダーまでの `3 × 512` と最初の PAX の 22 バイトを計上すると、残りは 29 バイト | 最初の名前と内容が一致し、エントリの `finish()` が成功。次は `LimitExceeded`。読み取り位置は第 2 PAX payload の先頭（2,560 バイト）で、payload を読む前に拒否 |

完全一致のケースはファイル payload やパディングをメタデータに加える誤りを検出する。1 バイト不足のケースは、最初のエントリまでの計上を次のエントリでリセットすると、期待した位置で拒否されないことを検査する。

## Runtime Verification

Windows 11 x86_64（10.0.26300.9974）、rustc/cargo 1.99.1、mise 2026.11.2、prek 0.5.7。追加の feature 引数は指定していない。`test-formal` 内の Structured Documents のみ、タスク定義どおり `--all-features` を使う。

| コマンド | UTC（2026-11-28） | wall | exit | 結果 |
| --- | --- | ---: | ---: | --- |
| `mise run fmt-check` | 01:36:46–01:37:12 | 25.817 s | 0 | Pass |
| `mise run check ruviform-io-archive` | 01:37:12–01:37:21 | 9.264 s | 0 | Pass |
| `mise run lint ruviform-io-archive` | 01:37:21–01:37:25 | 3.691 s | 0 | Pass（`-D warnings`） |
| `mise run test ruviform-io-archive` | 01:37:25–01:37:39 | 13.420 s | 0 | Pass、205 件（新規回帰を含む `tar_reader` 45 件、doctest 10 件） |
| `mise run test-formal` | 01:37:39–01:38:27 | 47.891 s | 0 | Pass、実行テスト 83 件（IPC 8、transport 5、Archive 26、Structured Documents 35、WAV 9）。Kani は起動しない |
| `mise exec -- prek run --all-files` | 01:46:20–01:46:26 | 5.896 s | 0 | Pass。本証跡をステージした作業ツリーで実行し、自動修正なし |

### 境界の検出確認

製品コードを変えず、テスト側の上限だけを一時的に 1 バイトずらした。期待結果は変えず、違う位置の `LimitExceeded` を成功と取り違えないことを確認した。選択実行には次のコマンドを使った。

```sh
mise run test ruviform-io-archive --test tar_reader metadata_bytes_limit_accumulates_across_pax_entries -- -- --exact
```

| 条件 | UTC（2026-11-28） | wall | exit | 結果 |
| --- | --- | ---: | ---: | --- |
| 完全一致ケースの上限を 2,611 バイトに変更 | 01:41:41–01:42:10 | 28.368 s | 101 | 期待した失敗。ゼロブロックで `max_metadata_bytes: declared 2612 exceeds limit 2611` |
| 不足ケースの上限を 1,588 バイトに変更 | 01:42:43–01:42:47 | 3.927 s | 101 | 期待した失敗。第 2 PAX payload 先頭の 2,560 バイトではなく、次の通常ファイルヘッダーを読み終えた 3,584 バイトの位置で拒否 |
| 元の上限に復元 | 01:42:57–01:43:00 | 2.569 s | 0 | Pass、1 件。実行前に `git diff --exit-code HEAD -- crates/ruviform-io-archive/tests/tar_reader.rs` の exit 0 を確認 |

初回の選択実行では `-- --exact` と指定したため、mise が区切りを消費して Cargo が `--exact` を拒否した（exit 1）。テスト本体に到達していないため、この結果は検出実績に含めない。

## 未証明領域と未実施の検証

[slice 14 の拡張 payload 判定](verification-slice14-tar-metadata-budget.md) と [slice 15 の固定ブロック判定](verification-slice15-tar-block-budget.md) は個別の形式証明である。この回帰は両判定を使う具体的なストリームを実行した結果であり、任意長・任意エントリ数の TAR ストリーム、他の拡張形式や不正入力、I/O エラーと予算判定の優先順位は証明しない。既存の proof、検証仕様、共有台帳の Sync Commit は変更していない。

| 項目 | 本 slice の要求状態 | 実行結果 | 理由・後続 |
| --- | --- | --- | --- |
| Kani・製品コードの mutation | NotApplicable | NotRun | 形式検証や製品コードを変えず、上限をずらす実行テストだけを実施 |
| Workspace full / Feature Matrix / MSRV / 他 OS | Deferred | NotRun | ローカルの変更範囲外。統合検証は merge 後の main SHA に対する Daily CI の結果で判断 |
| GUI-L2 / EXT-L3、AudioLens / Synvelis 独立回帰 | NotApplicable | NotRun | テスト・証跡のみの変更。製品 API・実装・依存関係は不変 |
| 手動 CI dispatch | NotApplicable | NotRun | Daily CI 方針に従い、手動起動しない |

この記録は変更範囲のローカル証拠であり、統合受入や Issue 全体の受入を完了扱いにしない。docs index は Coordinator が統合時に整備する。

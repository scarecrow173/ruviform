# Issue #721 検証記録（rf721）

作業日: 2026-11-17 / ブランチ: `feat/721-post-seal-temp-design`
環境: Windows 11 x86_64 / rustc 1.99.1 (58cf8f9c3 2026-10-13) /
cargo 1.99.1 / criterion 0.8.2（bench-smoke 実行時の bench 出力
環境行より）

## 残件 (a): bench-smoke 配線の実測照合

- コマンド: `mise run bench-smoke`（canonical = mise）
- 結果: **exit 0**。`crates/ruviform-io-e2e/benches/io_composition.rs`
  が実行され、`io_composition` の 6 case 全てが計測なし 1 回で
  `Testing ... Success`:
  `staged_write` / `staged_seal` / `staged_publish` /
  `staged_readback` / `tar_gzip_staged` / `tar_zstd_staged`。
- 証拠: `evidence/rf721/bench-smoke-io-composition-excerpt.txt`
  （環境行 + `Running benches\io_composition.rs` + 6 case Success）。
- `mise.toml` `[tasks.bench-smoke]` の `run` は
  `cargo bench -p ruviform-math -p ruviform-rt -p ruviform-viz
  -p ruviform-numeric -p ruviform-io-e2e -- --test` で
  `-p ruviform-io-e2e` を含む（PR #651 / commit `e69d92d0` で
  main 着地済み）。
- `docs/development-environment.md` との照合（read-only —
  同ファイルは `scarecrow173/ruviform#720` の並行 Dispatch が
  占有中のため編集しない）:
  - L20 `bench` 行: `cargo bench --workspace`、対象列挙に
    `ruviform-io-e2e` の `io_composition`（Issue #306 G05）を含む。
  - L21 `bench-smoke` 行: `mise.toml` と同一の `-p` 5 列挙
    （math/rt/viz/numeric/io-e2e）+ `-- --test`、説明は
    「`bench` と同じ対象の各ベンチを計測なしで 1 回実行」。
  - L91: bench/bench-smoke の役割記述が両 task と整合。
  - L119: CI `bench` job が `mise run bench-smoke` を呼ぶ記述と
    整合。
  - **判定: drift なし・記述一致。**

## 残件 (b): post-seal temp 改竄境界の設計判断

- 判断: **現行契約維持** — `publish()` は seal 後の temp 内容を
  再検証しない。post-seal の temp 外部改竄・削除は呼び出し側
  信頼境界内の事象として契約化する。
- 正本: `docs/designs/issue-721-post-seal-temp.md`（決定 1〜4・
  consumer 照合・強化案却下理由）。
- 根拠: `crates/ruviform-io-file/README.md` §信頼境界の既存宣言
  （同一境界内の悪意ある差替えは防衛しない・Temp identity は
  所有権証明ではない）、`docs/designs/issue-306-io-e2e-cases.md`
  Case 1/Case 2 の pin、consumer 全利用が同一信頼境界内の連続
  pipeline である実績（audio-lens `al-job`/`al-media`、synvelis
  `synvelis-library`/`synvelis-player` — read-only 参照）。
- doc 補強: `crates/ruviform-io-file/README.md` §信頼境界・保証
  範囲に「Post-seal temp 内容」1 項目を追加（実装差分なし）。

## 比例検証（canonical = `mise run`、全て実測）

| lane | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass | 0 |
| `mise run check ruviform-io-file` | pass | 0 |
| `mise run lint ruviform-io-file` | pass | 0 |
| `mise run test ruviform-io-file` | pass | 0 |
| `mise run check-io-e2e` | pass | 0 |
| `mise run lint-io-e2e` | pass | 0 |
| `mise run test-io-e2e` | pass | 0 |
| `mise run bench-smoke` | pass（`io_composition` 6 case） | 0 |

- `test-io-e2e` ログで `post_seal_temp_rewrite_is_not_
  reverified_before_publish`・`post_seal_temp_delete_fails_
  publish_not_published` の両試験が `ok` であることを確認
  （既存 suite 回帰なし・採用契約の pin 維持）。
- 補助証拠（canonical = mise）: cargo 直接実行は未使用。

## NotRun / Unsupported（POLICIES.md §4 による分離）

- NotRun: `mise run check-full` / `lint-full` / `test-full` /
  `build-full` / `verify-full`、`*-msrv`、3 OS 横断 lane —
  タスク指定になく日常の必須検証に含めない（Daily Full CI 委譲）。
- NotRun: 実 FS fault の OS 別挙動差（Windows ACL/readonly・
  POSIX share-lock 等）の環境別検証 — headless/fixture lane の
  Pass を実 OS 受入に昇格しない。本 slice は実装変更を伴わない
  ため新規の OS 別挙動差は発生しない。
- Unsupported: なし。

## review-protocol self-review

- 実施: 文書チェーン（spec/design/plan/task/test-spec/ir/exp）+
  README 差分 + evidence の self-review（詳細は PR 本文に記録）。
- 結果: blocker なし（指摘は全て反映済み or 意図的判断として
  文書化）。

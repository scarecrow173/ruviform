---
id: "IMPL-034XDuyLngwg5QXatIDUUO"
type: "impl"
status: "completed"
title: "Issue #306 G01 実4GiB opt-in lane + G05 性能 baseline lane"
created: "2026-11-14"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/large_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/benches/io_composition.rs"
      - type: "file"
        path: "evidence/rf306/large-bench-report.md"
      - type: "file"
        path: "docs/specs/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/designs/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/plans/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/tasks/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-lane-completion.md"
      - type: "file"
        path: "docs/impl/exp/issue-306-io-lane-completion.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-306-io-lane-completion.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-306-fault-seam-fuzz.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-306-io-lane-completion.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-306-io-lane-completion.md"
    rejected: []
---

# Issue #306 G01 実4GiB opt-in lane + G05 性能 baseline lane

## Summary

`ruviform-io-e2e` に Issue #306 の残り環境依存ゲート 2 本を追加した。
G01 は実ファイルシステム上で 4GiB−1 / 4GiB / 4GiB+1 の StagedFile
publish 経路を検証する `#[ignore]` opt-in lane
（`tests/large_staged.rs`）、G05 は staged write/seal/publish/readback
と TAR→gzip/zstd→StagedFile 合成の Criterion baseline
（`benches/io_composition.rs`）である。公開 API・本番コードは変更せず、
容量不足は `NotRun` として正直に報告する。

## Context

Issue #306（IO-IMP-12 共通 I/O 合成 E2E・受入ゲート）の Stage-1
環境非依存 slice は G07（fault seam）と G08（corpus replay）まで
main 統合済みで、残件は G01（実 4GiB 境界）と G05（性能 baseline）
のみであった。本 slice はその 2 lane を `crates/ruviform-io-e2e`
の test/bench のみで実装し、doc chain
（spec→design→plan→task→test-spec）と evidence
（`evidence/rf306/large-bench-report.md`）を揃える。
上流文書: `docs/specs/issue-306-io-lane-completion.md`、
`docs/designs/issue-306-io-lane-completion.md`（approved 相当）、
`docs/plans/`・`docs/tasks/`・`docs/test-specs/` の同 slug 文書。

## Implementation

### Changes

- `crates/ruviform-io-e2e/tests/large_staged.rs`（新規）:
  `#[ignore]` 付き単一テスト `large_staged_4gib_boundary`。
  サイズ定数 `SIZES = [4GiB−1, 4GiB, 4GiB+1]`、シナリオ表
  `SCENARIOS`（`create_new_roundtrip` / `replace_roundtrip` /
  `create_new_conflict_recovery` / `mid_write_fault_abort` /
  `post_seal_temp_delete`）をサイズごとに順次実行し、各 case 後に
  destination/temp を逐次 cleanup してピーク使用量を 1 case 分に
  抑える。payload は位置依存の deterministic pattern
  （`fill_pattern`/`mix64`）を 4MiB chunk で生成し、FNV-1a を
  chunked に畳み込む checksum で write→seal→readback→publish→
  reopen を照合する。容量 preflight は `set_len` + 末尾 byte 書込 +
  `sync_data` の sparse probe（必要量 = 4GiB+1+256MiB margin）で、
  失敗時は要求量・temp path を stdout に出して `NotRun` で終了する。
  実行中の `StorageFull`/`Io` 失敗も `NotRun` に分類し、scenario
  関数の `Result<(), NotRun>` 契約で Pass/Fail に偽装しない。
  mid-write fault は `tests/wav_fault.rs` 先例の `&mut StagedFile`
  fault wrapper（`DiskFull` writer、capacity=1/3 size で
  `io::ErrorKind::StorageFull` を注入）で再現し、abort 後に
  destination 不在・temp residue なしを確認する。
  post-seal temp delete は staging temp を削除して publish を
  `NotPublished` + `TempState::Absent` に分類させ、
  `RecoveryHandle::cleanup` で明示回収する。
- `crates/ruviform-io-e2e/benches/io_composition.rs`（新規）:
  Criterion `harness = false` bench。`BatchSize::PerIteration` +
  `iter_batched`/`iter_batched_ref` で fixture 準備・teardown を
  計測窓外に置く。cell: `staged_write`（8MiB pattern write）、
  `staged_seal`、`staged_publish`（seeded sealed fixture を
  CreateNew publish）、`staged_readback`（checksum 照合込み）、
  `tar_gzip_staged` / `tar_zstd_staged`（4×2MiB entry の TAR を
  encoder 経由で `&mut StagedFile` に合成、`Throughput::Bytes`）。
  `CountingAlloc`（`#[expect(unsafe_code)]` + SAFETY コメントの
  `GlobalAlloc` wrapper、計測区間のみ peak を追跡）で peak
  additional heap を 1GiB bound で sanity check し、環境 metadata
  （package/criterion/rustc/temp dir）を実行開始時に 1 回出力する。
  回帰閾値は provisional ±25%（同一マシン baseline 比、逸脱は
  調査トリガで自動 fail ではない）と bench 冒頭コメントに pin。
- `crates/ruviform-io-e2e/Cargo.toml`: dev-dep
  `criterion = { version = "0.8.2", default-features = false }`
  （`ruviform-rt` と同形。caret 指定・追跡済み `Cargo.lock` が
  再現性の錨・`default-features = false` で dev-dep の
  コンパイルコスト抑制・dev-dep 限定で `[dependencies]` に昇格
  しない旨をコメントで記録）と `[[bench]] name = "io_composition"`
  / `harness = false` を追加。
- `Cargo.lock`: criterion 0.8.2 は `ruviform-rt` dev-dep で既収録の
  ため差分は `ruviform-io-e2e` の依存リストへの `criterion` 追加
  1 行のみ（生成物としてコミット）。
- `crates/ruviform-io-e2e/src/lib.rs`: モジュール doc に G01/G05
  lane の説明を追記。
- `crates/ruviform-io-e2e/README.md`: 責務・検証コマンド
  （通常 test / corpus replay / G01 opt-in / `mise run bench`）・
  NotRun/Unsupported 境界・evidence 参照先を更新。
- `docs/*/issue-306-io-lane-completion.md`（spec/design/plan/task/
  test-spec）と本 ir/exp、`evidence/rf306/large-bench-report.md`。
- `docs/*/README.md` 各 index は generator（`doc-maintenance` /
  `sync_impl_index.js`）で再生成。

### Decisions During Implementation

- **G01 を単一 `#[ignore]` テスト + 内部シナリオ表にした理由**:
  サイズ×シナリオの直積を個別 `#[test]` にすると ignored test が
  15 個に増え、preflight も各 test で重複する。1 test 内で
  preflight 1 回 + 順次実行にすると容量判定と cleanup が一箇所に
  集約でき、ピーク使用量を 1 case 分に抑えられる。
- **checksum は新規依存なしの chunked FNV-1a**: 4GiB 全体を
  buffer せず 4MiB chunk で read→hash する。`DefaultHasher` は
  実装詳細が安定でないため FNV-1a（offset basis / prime は
  公開定数）を自前実装した。
- **payload pattern は位置依存 `mix64`**: 同一 offset が常に同一
  byte を返す deterministic 生成で、write と readback の照合に
  使う。テストと bench で同じ helper 構造（bench 側は独立実装）。
- **mid-write fault は wrapper 注入**: `tests/wav_fault.rs` の
  `&mut StagedFile` fault wrapper 先例に倣い、本番 API を変えずに
  `Write` impl で `StorageFull` を注入する `DiskFull` writer を
  test 内に置いた。
- **post-seal temp delete の期待値**: publish 前に staging temp
  を消すと `PublishFailure` は `NotPublished` /
  `PublishPhase::Publish` / temp `Absent` になり、recovery handle
  経由で cleanup できることを `staged_lifecycle.rs` の契約に
  合わせて検証した。
- **G05 の入力サイズは 8MiB**: G01 の 4GiB 境界は Criterion の
  反復計測には不適（1 iteration が長すぎて統計が取れない）ため、
  bench は 8MiB payload + 4×2MiB TAR entry に留め、4GiB は G01
  lane の責務として分離した（design 文書で pin 済み）。
- **peak alloc は bench 内自前 `CountingAlloc`**: 新規依存を
  足さず `GlobalAlloc` を wrap し、計測区間のみ `WINDOW_OPEN`
  で peak を追跡する。`unsafe impl` は `#[expect(unsafe_code)]`
  + SAFETY コメントの局所許可（AGENTS.md 規約どおり）。
- **encoder API の確認**: `gzip::Encoder::new(&mut W, ...)` /
  `finish(&mut self) -> EncodeReceipt`（writer を返さない）と
  `TarArchiveWriter::new(W)` / `finish() -> Result<W>` の実シグネ
  チャを `ruviform-io-archive`/`ruviform-io-compression` の source
  で確認してから bench を実装した（当初の仮定シグネチャを修正）。
- **WAV RIFF 4GiB 境界は本 lane で実装しない**: `wav_golden` の
  discard-sink 検証が classic RIFF 境界の既存根拠であり、実ファイル
  WAV 4GiB variant は design で NotRun として記録する判断をした。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-306-io-lane-completion.md`:
  encoder/tar API シグネチャの実確認、criterion `BatchSize` /
  `iter_batched` 意味論の vendored source 確認、clippy pedantic
  指摘への対応経緯を記録。

### Rejected

<!-- なし -->

## Validation

正本証拠: `evidence/rf306/large-bench-report.md`。

- G01（canonical `mise run test ruviform-io-e2e --test
  large_staged -- --ignored --nocapture`）: **Pass — 15/15 case**
  （4GiB−1/4GiB/4GiB+1 × 5 scenario）、1008.22s、preflight probe
  OK（required=4,563,402,753 bytes、NTFS temp_dir）。容量不足の
  NotRun は発生せず。
- G05（canonical `mise run bench` = `cargo bench --workspace`）:
  **Fail exit 101 — 本 slice 範囲外の既存破損**
  （`ruviform-numeric` lib test が bench profile でコンパイル不能。
  `cargo test -p ruviform-numeric --lib` は 116 tests pass で
  bench-profile 固有と確認。lease 外のため未修正、evidence/PR に
  記録）。
- G05 補助（`cargo bench -p ruviform-io-e2e --bench
  io_composition`、merge 後 tree で再実走）: **Pass — 全 6 cell
  完走**、panic/peak 違反なし。peak_delta 最大 ≈1.34 MiB
  （1 GiB bound 内）。数値は evidence に転記。
- 非回帰: `cargo check --locked`/`clippy -D warnings`/`fmt --check`/
  `cargo test -p ruviform-io-e2e`/`corpus_replay --ignored` 全て
  Pass（merge 後に check 再確認済み）。
- review-protocol self-review: `staged_publish` cell の destination
  不整合（bound dest `publish.bin` vs remove 対象 `published-N.bin`
  の不一致で初回 iteration panic する潜在 bug）を 1 件 CONFIRMED・
  修正済み — bench 実走で再検証。その他 P0/P1 なし。

## Risks

- G01 は temp drive の空き容量と I/O 速度に依存し、容量不足時は
  `NotRun` で終わる（Pass/Fail に偽装しない設計どおり）。
- `set_len` probe は NTFS の遅延割当で実空き容量を過小評価しない
  可能性があり、実行中の `StorageFull` が最終防衛線になる。
- G05 の ±25% 閾値は provisional で、consumer 固有閾値は #294 系の
  後続 gate に委譲される。

## Follow-ups

- `evidence/rf306/large-bench-report.md` に G01 実走結果（preflight
  観測・各 case Pass/NotRun）と G05 baseline 実測値を記録する。
- `docs/*/README.md` index を generator で再生成する。
- `docs/development-environment.md` の bench lane 対象一覧はリース外
  のため更新せず、PR 本文「未対応の作業」に記録する。
- Issue #306 は tracker open 継続（G02/G03/G04/G06 等の後続 gate）。

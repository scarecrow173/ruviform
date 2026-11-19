---
id: "TSPEC-034XCSPo2xzkvAijbgAfuA"
type: "test-spec"
status: "approved"
title: "Issue #306 G01/G05 試験仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-306-io-lane-completion.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-io-lane-completion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-306-fault-seam-fuzz.md"
    - "docs/test-specs/issue-306-io-e2e-cases.md"
  refines:
    - "docs/test-specs/issue-306-io-e2e-cases.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-306-io-lane-completion.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-306-io-lane-completion.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G01/G05 試験仕様

## Purpose

Issue #306 G01（実 4GiB 境界）と G05（性能 baseline）が満たすべき
検証契約を固定する。実装方法ではなく「何を保証し、何を保証しない
か」を規定し、NotRun/Unsupported の Pass 繰り上げを構造的に防ぐ。

## Feature

IO 合成層（`ruviform-io-e2e`）の実 FS 大容量境界と性能 baseline
計測 — 「staged publish が 4GiB 境界で型状態・実 FS 保証を破ら
ない」および「合成経路の性能を事前定義した条件で再現可能に計測
できる」。

## Rules

- **G01-R1 opt-in 分離**: `large_staged` の全試験は `#[ignore]`
  であり、通常 `test-io-e2e` lane（`cargo test -p
  ruviform-io-e2e`）では実行されない。実行入口は `mise run test
  ruviform-io-e2e --test large_staged -- --ignored
  --nocapture` のみ。
- **G01-R2 実 FS・公開 API のみ**: 試験は `StagedFile`/`SealedFile`/
  `PublishOptions`/`PublishReceipt`/`PublishFailure`/
  `RecoveryHandle` の公開 API と実 FS のみを消費する。本番 API・
  他クレートを変更しない。fault 注入は `&mut StagedFile` wrapper
  等の試験側実装に限定する。
- **G01-R3 境界サイズ網羅**: 4GiB−1 / 4GiB / 4GiB+1 の 3 サイズを
  対象とし、各サイズで CreateNew/Replace 両 publish 経路・
  readback checksum・reopen 照合・途中失敗→RecoveryHandle
  cleanup・post-seal temp delete→recovery を検証する。
- **G01-R4 容量不足は NotRun**: temp 配置ドライブの空き容量を
  preflight（`set_len`+末尾書き込み+sync の probe）で確認し、
  不足または write/publish 中の `StorageFull` 相当では残り case
  を NotRun として記録して打ち切る。skip 理由（要求量・観測
  結果・temp path/FS）は stdout と evidence に明記し、
  Pass/Fail に偽装しない。
- **G01-R5 逐次・bounded メモリ**: 全 case を逐次実行しピーク
  disk 使用量を 1 case 分に抑える。payload は chunked 生成・
  chunked checksum（FNV-1a・std のみ）で bounded memory を保つ。
- **G05-R1 事前定義**: Format/入力サイズ/Feature/Profile/FS/OS・
  baseline revision・回帰閾値・計測ブレ・許容不能時の判定を
  bench 冒頭コメント・design・本 spec に実装前に pin する。
- **G05-R2 canonical 入口**: bench は `mise run bench`
  （`cargo bench --workspace`）で実行し、環境 metadata
  （os/arch/cpu/cores/rustc/criterion/version/temp_dir）を
  実行開始時に stdout へ 1 回出力する。cargo 直接実行は補助
  証拠扱い。
- **G05-R3 fixture 窓外**: fixture 準備（create/write/seal/seed）
  は `iter_batched(_ref)` + `BatchSize` で計測窓外に置き、
  計測対象区間のみを時間計測する。
- **G05-R4 peak alloc の封じ込め**: counting `GlobalAlloc` は
  `enabled` フラグで計測区間のみ有効化し、他 bench・criterion
  自体への副作用を遮断する。heap のみ計測（mmap/OS buffer/
  page cache は対象外）である旨を明記する。
- **G05-R5 正直な記録**: 実測値は SHA/OS/FS/toolchain/feature/
  profile 付きで `evidence/rf306/` に記録し、NotRun/
  Unsupported/Deferred を Pass に繰り上げない。

## Examples

- **G01 容量不足**: temp drive の空きが `REQUIRED` 未満 →
  preflight probe が `StorageFull` 相当で失敗 → 全 case を
  NotRun として stdout + evidence に skip 理由を記録して終了
  （test は skip 相当で終了し、Pass/Fail を主張しない）。
- **G01 CreateNew conflict**: incumbent 配置済み dest へ 4GiB
  staged を publish → `DestinationExists`/`NotPublished` →
  `RecoveryHandle::cleanup` で temp 除去・incumbent は変更されない。
- **G01 途中失敗**: `DiskFull` wrapper が capacity=S/3 で
  `StorageFull` → `write_all` 途中失敗 → `abort` で temp/dest
  非残存。
- **G05 peak alloc**: `tar_gzip_staged` の `bench_function` 内で
  counter reset→enable→pipeline 3 回→disable→peak を
  `println!`（timing 対象外・1 回のみ）。

## Guarantees

- `cargo test -p ruviform-io-e2e`（通常 lane）は新規試験・
  bench を実行せず従来どおり終了する。
- G01 opt-in 実行時、4GiB 境界の publish/readback/recovery が
  公開 API 契約どおり動くか、または容量不足として NotRun と
  正直に分類されるかのどちらかが観測される。
- G05 bench は事前定義した cell を `mise run bench` で計測し、
  環境 metadata・throughput・peak alloc を evidence 転記可能な
  形で出力する。

## Non-goals

- 通常 lane への 4GiB 試験組み込み・専用 mise task 追加・CI
  変更。
- WAV RIFF 実ファイル 4GiB variant（`wav_golden` discard-sink
  が既存根拠 — NotRun）。
- bench の絶対性能合否判定・consumer 固有閾値（#127/#28/#84/
  #294 が正本）。
- SMB/Cloud FS・持続 fuzz・macOS/Linux 実機。

## Risk

- G01 の容量判定を誤ると NotRun を Pass/Fail に偽装する危険 —
  preflight + `StorageFull` 打ち切り + stdout/evidence 明記で
  緩和する。
- counting allocator の実装ミスで peak alloc が虚偽になる —
  alloc/dealloc/realloc の対称計上と enabled 区間限定で緩和し、
  heap 限定計測である旨を明記する。
- bench が `cargo bench --workspace` の実行時間を過度に伸ばす —
  入力 8MiB 級・reduced budget・cell 数限定で緩和する。

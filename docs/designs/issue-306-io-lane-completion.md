---
id: "DESIGN-034XCQLYJYZSmqpYRf01gv"
type: "design"
status: "approved"
title: "Issue #306 G01/G05 実4GiB lane・性能 baseline 設計"
created: "2026-11-15"
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
        path: "docs/designs/issue-306-io-lane-completion.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-io-lane-completion.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-io-e2e-cases.md"
    - "docs/designs/issue-306-fault-seam-fuzz-design.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  refines:
    - "docs/designs/issue-306-io-e2e-cases.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-lane-completion.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-lane-completion.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G01/G05 実4GiB lane・性能 baseline 設計

## Context

`ruviform-io-e2e` は IO-01〜05 の公開 API を層横断で合成する
test-only クレートであり、`StagedFile → seal → readback → publish`
の型状態ライフサイクルを小容量 fixture で検証済み。しかし Issue
#306 の G01（実 4GiB 境界）と G05（性能 baseline）は
`evidence/rf288/io06-gate-matrix.md` で「not run」のまま残る。
本設計はこの 2 lane を `ruviform-io-e2e` 内に opt-in/既存入口で
追加する形を決める。`mise.toml`・CI workflow は変更しない
（専用 task 追加禁止 — G01 は汎用 `mise run test`、G05 は既存
`mise run bench` で賄う）。

## Scope

### In Scope

- `tests/large_staged.rs`: 4GiB−1/4GiB/4GiB+1 の実 FS staged
  publish 境界試験（`#[ignore]` opt-in、単一 entry point）。
- `benches/io_composition.rs`: criterion bench（staged write /
  seal / publish / readback / TAR→gzip・zstd→StagedFile 合成 /
  peak alloc）。
- `Cargo.toml` への criterion dev-dep + `[[bench]]` 登録、
  lib.rs/README の lane 記述、`evidence/rf306/` 記録。

### Out of Scope

- 本番 API・他クレート・`mise.toml`・CI・`docs/development-
  environment.md` の変更。
- WAV RIFF 実ファイル 4GiB variant（下記「WAV RIFF 境界の扱い」）。
- Consumer 固有閾値・SMB/Cloud FS・持続 fuzz・他 OS 実機。

## Components and Boundaries

### G01 `tests/large_staged.rs`

構成は `tests/corpus_replay.rs` の先例に倣い、**1 つの
`#[ignore]`d test 関数**が全サイズ・全 scenario を逐次実行する
（cargo test の並列実行で複数 4GiB ファイルが同時に存在することを
構造的に防ぎ、ピーク使用量を 1 case 分 ≈4GiB+1+margin に抑える）。
`tests/common/mod.rs` の `TestDir`/`children`/`exists` を
`mod common` 経由で再利用し、新規依存は足さない。

**サイズ**: `SIZES = [4GiB−1, 4GiB, 4GiB+1]`（`u64` バイト）。

**preflight（容量ゲート）**: 試験冒頭で 1 回だけ実施。
`TestDir` 内に probe file を作り `set_len(REQUIRED)` →
末尾 1 byte 書き込み → `sync_data` → 削除、を試みる。
`REQUIRED = 4GiB+1 + MARGIN`（`MARGIN = 256MiB`: NTFS journal・
incumbent seed・rename の余裕。過大でも過小でもなく「probe が
通れば case が容量で死なない」ことを優先した保守値）。
失敗時は io error kind と probe 結果を stdout に出し、
**全 case を NotRun として記録して return**（test 自体は
skip 相当で終了 — Pass/Fail に偽装しない）。観測値
（temp path・probe 結果・error kind）は evidence へ転記する。
probe 成功は「set_len+末尾書き込み+sync が通る」実測であり、
論理空き容量の正確な値ではない — evidence には実行環境の
`df`/FS 情報を別途記録して補完する。

**scenario（各サイズで逐次・逐次 cleanup）**:

| ID | scenario | 検証内容 |
| --- | --- | --- |
| A | `create_new_roundtrip` | write(S bytes, chunked)→seal→readback で chunked FNV-1a+byte 数照合→`publish(CreateNew, FileAndDirectorySync)`→dest reopen で同照合 + `metadata().len()==S` |
| B | `replace_roundtrip` | incumbent を事前配置→write→seal→`publish(Replace, FileAndDirectorySync)`→reopen 照合（incumbent 置換を確認） |
| C | `create_new_conflict_recovery` | incumbent 事前配置→write→seal→`publish(CreateNew)` が `DestinationExists`/`NotPublished` で失敗→`take_recovery_handle`→`cleanup` で temp 除去・incumbent 無傷を確認（RecoveryHandle cleanup の実 FS 経路） |
| D | `mid_write_fault_abort` | `&mut StagedFile` を包む `DiskFull` wrapper（`tests/wav_fault.rs` 先例、capacity=S/3 で `StorageFull`）で write_all が途中失敗→`staged.abort()`→temp/dest 非残存 |
| E | `post_seal_temp_delete_recovery` | write→seal→temp を外部削除→`publish` が `NotPublished`→recovery handle `cleanup` が `TempState::Absent` で成功（`staged_lifecycle` の temp-delete 試験の 4GiB 版） |

各 scenario は独立した `TestDir` で、case 間で dest/temp を持ち
越さない。case ごとに `eprintln!` で `G01 <size> <id>: PASS …`
行を出し、途中 case の失敗でもそれ以前の観測が残る。
`StorageFull`/`StorageFull` 相当の io error（`ErrorKind::
StorageFull`、および raw OS error 112/28 が写る場合を考慮し
kind 判定を主とする）が write/publish 中に出た場合は、その時点で
残り case を **NotRun** として記録して打ち切る（容量不足を
Fail に偽装しない）。それ以外の error/panic は通常の test 失敗。

**payload と checksum**: 位置依存の決定的 pattern を 8-byte word
単位の splitmix64 派生で生成（`fill_pattern(buf, base)`）。
write 側で chunked FNV-1a（offset basis `0xcbf29ce484222325`、
prime `0x100000001b3`）を累積し、readback/reopen 側で同じく
累積して一致 + 総 byte 数一致を assert する。全て std のみ、
4MiB chunk でメモリは bounded。

**WAV RIFF 境界の扱い（design 判断）**: Issue G01 DoD の
「WAV RIFF サイズ境界」は `wav_golden.rs` が classic RIFF 4GiB
境界を discard sink で byte-exact 検証済み（実ファイル不要の
形式境界）。実ファイル WAV 4GiB variant は約 4GiB の PCM payload
を `WavMuxer` 経由で書く必要があり、検証する契約（RIFF size
field の 32bit 境界）は既に golden で pin 済みのため、**本 lane
では実装しない = NotRun** とし、evidence・README に理由
（既存根拠・追加分の限界効用）を記録する。

### G05 `benches/io_composition.rs`

criterion `0.8.2`（`default-features = false`、ruviform-rt と
同形 pin）+ `harness = false`。bench binary は本ファイルのみで
完結し、`tests/common` は参照しない（bench target からは届か
ないため、temp 管理・pattern・checksum の小 helper を bench 内に
自前実装する — 重複は許容、依存は増やさない）。

**事前定義（bench 冒頭コメント・本設計・test-spec に pin）**:

- Format: staged raw bytes / TAR→gzip / TAR→zstd → `&mut
  StagedFile` の合成。WAV・structured・secure extraction は対象外
  （G05 は IO 合成の代表経路として staged publish 系に限定）。
- 入力サイズ: `INPUT_BYTES = 8MiB`（staged write/readback/
  publish の payload）、TAR payload 合計 `8MiB`（4 entry ×
  2MiB）。4GiB は bench の反復計測に不適のため G01 が担当。
- Feature/Profile: bench profile（release 派生）、io-e2e の
  dev-dep feature 集合そのまま（io-file `fault-injection` は
  feature unification で有効だが hot path は不変 — 記録する）。
- FS/OS: `std::env::temp_dir()` 配下の実 FS（実行環境依存、
  evidence に path/FS を記録）。
- baseline revision: bench 実行時の git SHA を evidence に記録
  （bench 内からは `CARGO_PKG_VERSION` と環境 metadata のみ
  stdout 出力。SHA は実行者が記録）。
- 回帰閾値（暫定・本 slice で事前定義）: 同一マシン・同一
  profile での再計測は throughput が baseline の **±25%** を
  超えたら原因調査対象、peak alloc が下記 hard bound を超えたら
  Fail。consumer 固有閾値は #127/#28/#84/#294 が正本で本 baseline
  はそれを代替しない。
- 計測ブレ: criterion の標準出力（mean/median/throughput/CV）を
  evidence に転記し、CV が大きい cell はブレとして記録。
- 許容不能時の判定: peak alloc hard bound 超過・panic・
  deadline 相当の異常は Fail。環境起因（disk full 等）は
  G01 と同じく NotRun 相当として正直に記録。

**cells**（group `io_composition`、`poc_group` 相当の reduced
budget: warm_up 300ms・measurement 2s・sample_size 10 — IO cell
は 1 iter が ms 級のため rt poc より measurement を長めに取る）:

| cell | 内容 | Throughput |
| --- | --- | --- |
| `staged_write` | `iter_batched_ref`: setup=`StagedFile::create`、routine=`write_all(INPUT)`。fixture drop は計測窓外 | Bytes(INPUT) |
| `staged_seal` | setup=create+write(4KiB)（seal は payload サイズ不問の型遷移のため小 fixture）、routine=`seal()` を by-value `iter_batched` で消費。seal は I/O を持たない純粋遷移であり「ほぼ 0」を実測で示す cell | なし |
| `staged_publish` | setup=create+write(INPUT)+seal、routine=`publish(CreateNew, FileAndDirectorySync)`+`remove_file(dest)`（dest は iter ごと一意名）。by-value `iter_batched` | Bytes(INPUT) |
| `staged_readback` | setup=create+write(INPUT)+seal、routine=`open_readback`+chunked read+FNV（4MiB buffer 再利用で bounded） | Bytes(INPUT) |
| `tar_gzip_staged` | setup=`StagedFile::create`、routine=TAR 4 entry→`gzip::Encoder`→finish（二重 finish 証明含む） | Bytes(TAR payload) |
| `tar_zstd_staged` | 同上 zstd | Bytes(TAR payload) |

**peak alloc**: bench binary に `#[global_allocator]` の
`CountingAlloc`（System 包含・`enabled`/`current`/`peak` の
AtomicU64/AtomicBool）を置き、alloc/dealloc/realloc で live bytes
を追跡。`measure_peak` helper が counter を reset→enable→対象
pipeline を 3 回実行→disable→peak を返す。計測は各代表 cell
（`staged_write`・`tar_gzip_staged`）の `bench_function` 内で
`b.iter*` の**前に 1 回だけ**実行し `println!` で
`peak_alloc <cell>: <bytes>` を出力（criterion の timing 対象外、
evidence 転記用）。計測区間外は enabled=false で他 bench・
criterion 自体への影響を遮断する。`unsafe impl GlobalAlloc` は
`#[expect(unsafe_code)]` + SAFETY コメントで局所許可
（AGENTS.md 規約。bench target は `src/lib.rs` の
`#![forbid(unsafe_code)]` の適用外だが `workspace.lints` の
warn→`-D warnings` ゲート対象のため expect が必要）。
heap のみ計測 — mmap/OS buffer・page cache は対象外である旨を
bench コメントと evidence に明記。

**環境 metadata**: `rt_observability_poc.rs` と同型に、実行開始時
に os/arch/cpu model/core 数/rustc/criterion/io-e2e version/temp_dir
を stdout に 1 回出力（`Once` で重複防止）。FS 種別は env から
確実に取れないため path 表記に留め、evidence 側で `df`/FS 情報を
補完記録する。

## Data and Control Flow

- G01: `TestDir` → preflight probe → per-size per-scenario:
  `StagedFile::create` → chunked `write_all`(pattern+FNV) →
  `seal` → `open_readback`(chunked FNV) → `publish(mode)` →
  reopen FNV / `RecoveryHandle::cleanup` / `abort` → `TestDir`
  drop が残滓を除去。
- G05: criterion `bench_function` → （任意で peak probe 1 回）→
  `iter`/`iter_batched(_ref)` → setup（計測窓外）→ routine
  （計測窓）→ criterion が mean/throughput/CV を `target/
  criterion/` と stdout へ出力 → evidence 転記。

## Non-Functional Constraints

- G01 実行時のピーク disk 使用量 ≈ 1 case 分（≈4GiB+1+margin）。
  preflight probe は case 前に解放される。case は逐次であり
  temp/dest を case 間で持ち越さない。
- F: ドライブに中間生成物を置かない — temp は
  `std::env::temp_dir()`（実行環境依存、evidence に path を記録）。
- bench は `cargo bench --workspace` に含まれるため全量計測の
  実行時間を過度に伸ばさない（入力 8MiB 級・reduced budget・
  cell 6 本+probe）。
- `unsafe` は counting allocator のみ・局所許可。公開 enum は
  該当なし（bench/test 内の private enum は `#[non_exhaustive]`
  対象外）。

## Risks and Trade-offs

- **4GiB write の実行時間**: NVMe で数秒/GB 級を見込むが、
  opt-in lane のため許容。逐次実行でピークを抑える。
- **preflight probe の精度**: `set_len`+末尾書き込みは完全な
  空き容量保証ではない（FS によっては遅延割当）。途中で
  `StorageFull` が出た場合は NotRun 打ち切りで誠実に扱う
  （Fail にしない）。
- **bench の FS 依存性**: temp dir の FS・cache 状態で値は揺れる。
  baseline は「同一環境での相対比較」用であり、絶対値の
  性能保証ではないことを evidence に明記。
- **criterion `--test` smoke 非収録**: CI `bench` job の
  `bench-smoke` は math/rt/viz のみ対象で `io_composition` は
  CI smoke に載らない（`mise.toml`/CI 変更禁止のため受容、
  drift として記録）。
- **`docs/development-environment.md` の bench lane 記述 drift**:
  リース外のため更新せず、PR 本文「未対応の作業」に記録。

## Deferred Design Concerns

- WAV RIFF 実ファイル 4GiB variant | 形式境界は `wav_golden`
  discard-sink で pin 済み・追加分の限界効用が小さい | 実ファイル
  WAV publish が consumer 要件化した時点 | RIFF 境界の実 FS 経路が
  未検証のまま残る（現状は形式層で担保済み）。
- `bench-smoke`/`docs/development-environment.md` への
  io_composition 追加 | `mise.toml`/docs が本 slice のリース外 |
  Coordinator が範囲拡張または後続 slice で対応 | CI で bench の
  compile smoke が走らず、bench 破損はローカル `check --all-
  targets` でのみ検出される。
- Consumer 固有性能閾値・SMB/Cloud FS・他 OS | Issue Deferred/
  外部 gate | 各 Consumer Issue・後続 slice | baseline が
  consumer 受入を代替しない。

## Verification Notes

- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
  `test-io-e2e` / `test-io-e2e-corpus` 全 pass。
- [ ] `mise run test ruviform-io-e2e --test large_staged --
  --ignored --nocapture` 実走 → evidence 記録。
- [ ] `mise run bench` 実走 → `io_composition` 実測値を
  evidence 記録。

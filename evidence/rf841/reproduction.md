# evidence/rf841 — SpawnFailed{Timeout} spawn-deadline flake 定量記録（補助証拠）

`ruviform#841` — L1 runner spawn/handshake deadline（`command_timeout=500ms` が
spawn hand-off wait・Hello wait に共有される）による instantiate 時
`SpawnFailed { cause: Timeout }` の環境 flake。発生率・再現条件を補助 loop
`evidence/rf841/loop.sh` で定量した記録。canonical lane は
`mise run test-l1-real-plugin`（本ファイルは補助証拠 — 計測 run は fail
許容、失敗率の記録が目的）。

## 環境

- OS: Windows（MINGW64 `MINGW64_NT-10.0-26300`）、Git Bash 実行
- toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`（mise 管理）
- 対象 SHA: `6016bc64d329342c2452f4e680ae3f2d7827a78b`（origin/main）
- 計測対象 binary: `mise run test-l1-real-plugin` 1 回の成果物
  `target/debug/deps/l1_fault_test_plugin_e2e-*.exe`（両 crate）を
  `evidence/rf841/bin/{vst3,clap}-fault.exe` へ退避。runner binary +
  fixture cdylib は baked `CARGO_BIN_EXE_*` 経由で `target/debug/` を
  解決するため、fixture artifacts が残る限り copy は有効
  （`evidence/rf768/loop.sh` 先例どおり）。
- probe test: `real_test_fault_l1_hang_converges_on_timeout`（両 ABI 同名 —
  `command_timeout=500ms` を設定し instantiate 成功が必要な唯一の test）。
  `real_test_spawn_l1_hang_before_hello_times_out` も 500ms を使うが
  `SpawnFailed{Timeout}` を期待値とするため本 signature では可視に
  fail しない（後述）。

## 計測設計（arm）

`loop.sh` の `ARM` / `LOAD` / `N` で変えた条件:

| arm | binary 引数 | probe spawn の位置 | 1 run 当たり 500ms 脆弱 spawn |
| --- | --- | --- | --- |
| `cold` | `--ignored --exact <probe>` | process 内 1 回目（cold） | 1 |
| `warm` | `--ignored --test-threads=1 <warmup> <probe>` | 2 回目 | 1 |
| `warm3` | `--ignored --test-threads=1 abort exit_zero descriptor <probe>` | 4 回目 | 1 |
| `serial` | `--ignored --test-threads=1` | 全 suite 直列・8 番目前後 | 1 |
| `full` | `--ignored` | 全 suite 並列（default threads） | 1 |

- `LOAD=1`: sibling main checkout `/g/repositry/audio-products/ruviform` で
  `mise exec -- cargo build -p ruviform-plugin-test-vst3 -p
  ruviform-plugin-test-clap --features l1-runner --tests` を iter 中反復
  （rf763/rf768 先例 — 別 worktree/別 managed target のため、実行中
  binary が掴む fixture DLL は再ビルドされず `os error 5` を回避）。
- vst3/clap binary は各 iter で並走（先例どおり）。
- warmup は `command_timeout` default 10s の `descriptor_and_clean_block`
  等 — 自身は flake しない spawn 運動のみ行う。

## 結果

| arm | LOAD | vst3 | clap | 計 | fail signature |
| --- | --- | --- | --- | --- | --- |
| cold | 0 | 30/30 | 30/30 | **0/60 fail** | — |
| cold | 1 | 20/20 | 20/20 | **0/40 fail** | — |
| warm | 0 | 19/20 | 19/20 | **2/40 fail** | spawn_timeout×2（同一 iter 19 で両 ABI） |
| warm | 1 | 19/20 | 19/20 | **2/40 fail** | spawn_timeout×2（同一 iter 13 で両 ABI） |
| warm3 | 0 | 19/20 | 18/20 | **3/40 fail** | spawn_timeout×3（iter 2 で両 ABI + clap iter 6） |
| serial | 0 | 7/10 | 8/10 | **5/20 fail** | spawn_timeout×5 |
| full | 0 | 7/10 | 7/10 | **6/20 fail** | spawn_timeout×6 |
| full | 1 | 7/10 | 8/10 | **5/20 fail** | spawn_timeout×5 |

合計 240 binary run、21 fail — **全て同一 signature**:
`real_test_fault_l1_hang_converges_on_timeout` が `instantiate`
（`l1_fault_test_plugin_e2e.rs:193` の `.unwrap()`）で
`Err(SpawnFailed { cause: Timeout, diagnostics_id: 1 })` に panic。
他 signature（`warmup_fail`・`other_fail`・staleness 系）は 0 件。

## 観測された構造

1. **孤立 spawn はほぼ落ちない** — cold（process 内初回・1 spawn/iter・
   両 binary 並走）は cargo build 併走を含めて 0/100。500ms 制約を
   単独 spawn が超える事象は本環境では稀。
2. **suite 内に置かれた spawn で率が上がる** — probe 位置を 2 番目
   （warm 5%）→4 番目（warm3 7.5%）→suite 内（serial/full 25-30%）と
   進めると単調に上昇。直列実行でも落ちるため suite 内並列 spawn 競合
   だけが原因ではない — 先行 test の child 生成・kill/reap・DLL load/
   unload が連続する状態での spawn（または壁時計で後の時点）が脆弱。
3. **fail は iter 単位で両 ABI に相関して出る** — warm iter 19/13・
   warm3 iter 2 で両 binary が同時 fail。process 内要因ではなく
   machine 全体の stall 事象（scheduler/WER/loader 等）が
   500ms window に重なったときに発火する構造を示唆。
4. **LOAD=0/1 の差は本 N では小さい** — sibling `cargo build` 併走は
   cold では 0 件・full では 6/20→5/20 と有意差を出せていない。
   suite 内条件（spawn 周辺の先行活動）が外付け負荷より支配的。
   ただし本計測中も他 Orca worktree（rf839/rf842 等）の ambient 活動が
   あり得る — LOAD=0 は「本 loop が負荷を足さない」条件であり
   「無負荷保証」ではない点を正直に記録する。
5. **fail する test は常に `hang_converges_on_timeout` のみ** —
   `command_timeout=500ms` を設定し *instantiate 成功を必要とする* のは
   両 ABI でこの test のみ（`:638`/`:593`）。
   `real_test_spawn_l1_hang_before_hello_times_out`（`:1289`/`:1236`）も
   500ms を設定するが `SpawnFailed{Timeout}` が期待値のため、
   deadline 発火自体は fail として表面化しない。
   `diagnostics_id: 1` は各 test が独立 `PluginHost` を持ち id が
   host 毎に mint されるため常に 1（host 内初回 record）。

## signature と phase 帰属の限界

panic 出力は `SpawnFailed { cause: Timeout, diagnostics_id }` のみを含み、
発火した deadline（spawn hand-off wait `l1/mod.rs:2094` = phase `Spawn` か
Hello wait `l1/mod.rs:2148` = phase `Handshake` か）は log から確定
できない。帰属には `diagnostics_id` → `host.diagnostics_query` →
`DiagnosticRecord.phase`（`SpawnPhase`）を test 側で print する実装が
必要（本 slice の lease 外 — 分類ルール文書に手順のみ記述）。

## canonical lane での同時観測

`mise run test-l1-real-plugin` 本 worktree 初回 run（ビルド直後・
外部負荷なし）でも vst3 fault suite が同 signature
（`SpawnFailed { cause: Timeout, diagnostics_id: 1 }` @
`hang_converges_on_timeout`）で fail — `canonical-run2-4.log` と併せて
`verification.md` に記録。

## ファイル

- `loop.sh` — arm/load パラメータ化 loop（本記録の全 run を生成）
- `bin/{vst3,clap}-fault.exe` — 退避した test binary
- `logs-{arm}-load{0,1}/` — per-iter log + `<abi>-classes.txt`（iter 毎の
  分類）+ `<abi>-summary.txt`
- `canonical-run*.log` — canonical lane 実行 log

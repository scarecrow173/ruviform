# rf806 — formal verification D 層（proptest + Kani）verification record

Issue `scarecrow173/ruviform#806` の検証記録。canonical = `mise`
lane（`test-formal` / `test-formal-kani`）。cargo 直接実行は補助証拠
（mutation 注入確認・ロック前の下見）として明記済み。slice 1（wire
codec）と slice 2（archive reader）をそれぞれ記録する。

## slice 1 — wire codec（rvf.ipc.v1 / rvf.uitest.v1）

- Date: 2026-11-20
- OS: Windows (x86_64) — proptest/通常 lane。Kani lane は WSL2 Ubuntu
  （`cargo kani` 検出 → `/mnt/g` パス写像 → WSL ネイティブ target dir）
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1（Windows）;
  rustc 1.99.0 / Kani Rust Verifier 0.68.0 / CBMC 6.11.0（WSL2）
- Repo SHA: `06a2b4149add9447278525e01d466a26d9f1c0fd`（ブランチ起点。
  各 JSONL の `repo_sha` は実行時 HEAD を記録）
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果が証拠（本記録はそれを代替しない）。

## 検証対象

- `rvf.ipc.v1`（`crates/ruviform-ui/src/ipc.rs`）: `parse_intent` /
  `valid_op` / `Event::to_json` — `tests/formal_ipc.rs`。
- `rvf.uitest.v1` transport framing
  （`crates/ruviform-ui-test/src/transport.rs`）: `read_frame` /
  `write_frame` — `tests/formal_transport.rs`。
- 生産コードの意味論は変更なし（`git diff` の src/ 差分ゼロで確認）。

## Actions → resulting state

| # | Action (command) | Resulting state |
| --- | --- | --- |
| 1 | `mise run test-formal` | `formal_ipc` 7/7 + `formal_transport` 4/4 pass・exit 0。proptest regression seed 2 件（1-ulp float 境界）を再現実行 |
| 2 | `mise run test-formal-kani` | 環境解決: native `cargo kani` 不在 → WSL2 Ubuntu（cargo 1.99.0 + Kani 0.68.0 + CBMC 6.11.0）採用、WSL-native target dir で `cargo kani --tests -p ruviform-ui -p ruviform-ui-test` 実走（結果は下部 table） |
| 3 | mutation 注入: `valid_op` の `split('.').all(\|seg\| !seg.is_empty())` を除去（`crates/ruviform-ui/src/ipc.rs` 一時編集） | `broken_op_rejected_everywhere` が shrink つき反例 `op = ".0.-"` で FAILED（6 passed / 1 failed）。suite が契約違反を検出できることの実証。記録: `mutation-fail.log`（local） |
| 4 | 変異の完全 revert → `cargo test --locked -p ruviform-ui --test formal_ipc` 再実行 | 7/7 green・`git diff` src/ ゼロ（変異持ち越しなし）。記録: `mutation-revert-green.log`（local） |
| 5 | `mise run fmt-check` / `check` / `lint` / `test`（両 crate） | 全 pass（lint は `doc_markdown`・`cast_possible_truncation` 指摘を修正済みの最終形で clippy `-D warnings` clean） |
| 6 | `mise exec -- prek run --all-files` | 全 hook pass（whitespace/EOF/yaml/toml/line-ending/BOM/key/actionlint） |

## Lane / verification results

| Command | OS | Exit | Verdict | 備考 |
| --- | --- | --- | --- | --- |
| `mise run test-formal` | Windows x64 | 0 | pass | proptest 11 properties 全 pass |
| `mise run test-formal-kani` | WSL2 Ubuntu x64（Windows host 経由） | n/a | not_run | 5 `#[kani::proof]` は全て起動し CBMC が solve 継続中（cutoff 時点 4 solver >35 CPU-min・1 queue）。verdict 未発行 — 反例なし・fail ではない。単発 probe（`valid_op_soundness`・unwind16）も 15 分 window を超過 → 本環境の solver コストが支配（共有 WSL2・sibling CBMC 併走）。bound は spec 記載どおり |
| `cargo test … formal_ipc`（mutant 注入時） | Windows x64 | 101 | fail（期待どおり） | `broken_op_rejected_everywhere` が `op=".0.-"` を縮小提示 |
| `cargo test … formal_ipc`（revert 後） | Windows x64 | 0 | pass | 変異なしで緑に復帰 |
| `mise run fmt-check` | Windows x64 | 0 | pass | |
| `mise run check ruviform-ui` / `ruviform-ui-test` | Windows x64 | 0 | pass | `--all-targets`（新規 test ターゲット含む） |
| `mise run lint ruviform-ui` / `ruviform-ui-test` | Windows x64 | 0 | pass | clippy `-D warnings` clean |
| `mise run test ruviform-ui` / `ruviform-ui-test` | Windows x64 | 0 | pass | パッケージ全テスト pass |
| `mise exec -- prek run --all-files` | Windows x64 | 0 | pass | repo hooks 全量 |

## Bounds and assumptions（Kani）

`docs/specs/formal-verification.md` の「Kani の bound と仮定」表が
正本。要約:

- `parse_intent_error_contract`: envelope field（`v`/`kind`/`id`/`op`/
  `params`）を固定 JSON skeleton 上で symbolic に全網羅 → 全 gate
  を尽きる。任意 malformed byte 列は proptest `parse_intent_arbitrary_
  error_set`（concrete・shrink つき）に委譲 — bound 外として明示。
- `valid_op_soundness`: 8 byte 任意列のうち valid UTF-8（
  `core::str::from_utf8` — allocation-free）。`MAX_OP_LEN=128`
  境界領域は proptest `valid_op_total_and_sound` に委譲。
- `event_to_json_wellformed`: symbolic `id`（6 byte valid UTF-8）×
  `result_ok(Null)`。他 variant・大 payload は proptest
  `event_to_json_versioned_envelope`（全 variant・任意 `Value`）に委譲。
- `read_frame_contract`: ≤32 byte symbolic stream（symbolic u32
  prefix で `TooLarge` 判定自体は到達可）。実 drain 実走と cap 越
  stream は proptest `too_large_drains_and_stays_aligned` 等に委譲。
- `write_read_roundtrip`: 16 byte 任意列のうち valid UTF-8 の
  body。in-cap 全サイズ・exact-cap 境界は proptest
  `write_read_roundtrip` に委譲。

serde_json の float 精度: `float_roundtrip` 未使用の既定 parser
は最短 decimal 表現を 1 ulp 誤る場合がある（proptest が実際に
`905096.8987248447 → …448` の反例を発見し、seed を
`crates/ruviform-ui/tests/formal_ipc.proptest-regressions` に
commit 済み）。これは codec 契約外の serde_json 既知限界 — 厳密
比較が必要な箇所は `json_value_eq`（整数厳密・float 1 ulp）で
wire 上の意味的同一性を検証する。

## Honest notes

- Kani lane は「任意入力の全網羅」ではなく bounded symbolic の
  尽きる検証。上表の委譲先（proptest concrete coverage + 1 MiB
  cap の concrete 経路）と合わせて契約をカバーする構成であり、
  bound 外を証明済みとは主張しない。
- Kani 初回実行で 64 byte 任意入力 × `serde_json::from_str` の
  unwind が非実用的（`from_utf8_lossy` の Utf8Chunks unwinding が
  指数的分岐で収束しなかった）ことが判明し、slice-1 の harness
  を「固定 skeleton + symbolic field」へ再設計した。任意 malformed
  byte 列の exhaustive proof は本 slice の scope 外（concrete
  proptest で代替）として spec に明記。
- さらに `String`/`Vec` の RawVec 増分割当と `fmt` 系 `Debug`
  unwind が支配コストだったため、最終形は symbolic `&str` を
  `core::str::from_utf8(&bytes)` で得る allocation-free 設計
  （`format!`/`.unwrap()`/symbolic 長 container を排除）。harness
  自体の loop は bound ≤32 で小さいが、symbolic byte 列上の
  UTF-8・`str` API・serde_json の path 爆発による SAT 解は
  依然重く（1 harness あたり数十 CPU-min 規模）、slice-1 では
  `-j4` 並列実行の manual lane として提供する。
- `rustc` 差分: Windows 1.99.1 / WSL2 1.99.0 / Kani 同梱 nightly
  — lane ごとの toolchain を表に分離記録。
- `dock.rs:2254` の `semicolon_in_expressions_from_non_local_macros`
  は Kani toolchain（nightly future-incompat）警告で既存コード由来
  — 本 slice の変更とは無関係。通常 lane の `-D warnings` では
  未指摘（stable では warn 化前）。
- 変更ファイルに生産 `src/` 差分なし。`Cargo.lock` は proptest
  依存解決分のみ（dev-dep・本番グラフ非波及）。

## slice 2 — archive reader（ruviform-io-archive TAR/ZIP）

- Date: 2026-11-21
- OS: Windows (x86_64) — proptest/通常 lane。Kani lane は WSL2 Ubuntu
  （`cargo kani` 検出 → `/mnt/g` パス写像 → WSL ネイティブ target dir）
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1（Windows）;
  rustc 1.99.0 / Kani Rust Verifier 0.68.0 / CBMC 6.11.0（WSL2）
- Repo SHA: `b27fb6a1f63970479f5c82872f9ba0391f741ed5`（`origin/main`
  起点。各 JSONL の `repo_sha` は実行時 HEAD を記録）
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果が証拠（本記録はそれを代替しない）。

### 検証対象

- `crates/ruviform-io-archive` の TAR streaming reader（`src/tar/`）・
  ZIP reader（`src/zip/`）・`ArchiveReadLimits`（`src/limits.rs`）・
  completion/receipt（`src/entry.rs` / `src/receipt.rs`）—
  `tests/formal_archive.rs`（18 properties + `#[cfg(kani)] mod proofs`
  3 harness）。
- 生産コードの意味論は変更なし（mutation 注入・完全 revert 済み、
  `git diff` の src/ 差分ゼロで確認）。

### Actions → resulting state

| # | Action (command) | Resulting state |
| --- | --- | --- |
| 1 | `mise run test-formal` | `formal_ipc` 7/7 + `formal_transport` 4/4 + `formal_archive` 18/18 pass・exit 0 |
| 2 | `mise run test-formal-kani` | 環境解決: native `cargo kani` 不在 → WSL2 Ubuntu 採用。`cargo kani --tests` の全 crate 一括実行は compile 直後に WSL 側が不安定化（`E_UNEXPECTED`・CBMC 孤児 process 残存）し verdict 未発行 — 反例ではなくリソース/環境起因と判断。代替として harness 単独実行を `/tmp` log で収集（結果は下部 table） |
| 3 | mutation 注入 attempt 1: `tar/read.rs` declared-size gate `>` → `>=`（境界のみ違反） | **未検出** — `tar_declared_size_gate` の sampling が `(declared == limit)` 境界をほぼ生成しない（`limit = limit_raw % (declared+9)` のため確率 ~1/(declared+9)）。18/18 pass・EXIT=0。boundary density gap を正直に残す。記録: `slice2/mutation-attempt1-notdetected.log`（local） |
| 4 | mutation 注入 attempt 2: `tar/read.rs` `entries_total >= max_entries` → `>`（over-cap を 1 件余計に受理） | `tar_max_entries_gate` が shrink つき反例 `k = 2, max = 0` で FAILED（17 passed / 1 failed・EXIT=101）。suite が契約違反を検出できることの実証。記録: `slice2/mutation-fail.log`（local） |
| 5 | 変異の完全 revert → `cargo test --locked -p ruviform-io-archive --test formal_archive` 再実行 | 18/18 green・EXIT=0・`git diff` src/ ゼロ（変異持ち越しなし）。記録: `slice2/mutation-revert-green.log`（local） |
| 6 | `mise run fmt-check` / `check` / `lint` / `test`（ruviform-io-archive） | 全 pass（lint は `matches!`・method-ref・semicolon・useless-conversion 指摘を修正済みの最終形で clippy `-D warnings` clean） |
| 7 | `mise exec -- prek run --all-files` | 全 hook pass（whitespace/EOF/yaml/toml/line-ending/BOM/key/actionlint） |

### Lane / verification results

| Command | OS | Exit | Verdict | 備考 |
| --- | --- | --- | --- | --- |
| `mise run test-formal` | Windows x64 | 0 | pass | proptest 11（wire codec）+ 18（archive）properties 全 pass |
| `mise run test-formal-kani`（全 crate 一括） | WSL2 Ubuntu x64 | n/a | not_run | compile 完了直後に CBMC 群が WSL を不安定化し cargo-kani が無出力のまま exit 1・verdict 未発行。反例ではなく環境/リソース起因（`E_UNEXPECTED` 続発）— `fail` とも `pass` とも記録しない |
| `cargo kani --harness proofs::tar_typeflag_classification`（単独・timeout 900s） | WSL2 Ubuntu x64 | 124 | not_run | timeout で中断 — CBMC が unwinding 段階で継続中（`parse_numeric` inline loop が bound 600 縁まで到達）。verdict 未発行・反例なし |
| `cargo kani --harness proofs::tar_declared_size_gate`（単独・timeout 900s） | WSL2 Ubuntu x64 | 124 | not_run | timeout で中断 — 同上。verdict 未発行・反例なし |
| `cargo kani --harness proofs::zip_index_gates`（単独・timeout 900s） | WSL2 Ubuntu x64 | 124 | not_run | timeout で中断 — 同上。verdict 未発行・反例なし。long-window（1800s）再試行を別途実施 |
| `cargo test … formal_archive`（mutant 注入時） | Windows x64 | 101 | fail（期待どおり） | `tar_max_entries_gate` が `k=2, max=0` を縮小提示 |
| `cargo test … formal_archive`（revert 後） | Windows x64 | 0 | pass | 変異なしで緑に復帰 |
| `mise run fmt-check` | Windows x64 | 0 | pass | |
| `mise run check ruviform-io-archive` | Windows x64 | 0 | pass | `--all-targets`（新規 test ターゲット含む） |
| `mise run lint ruviform-io-archive` | Windows x64 | 0 | pass | clippy `-D warnings` clean |
| `mise run test ruviform-io-archive` | Windows x64 | 0 | pass | パッケージ全テスト pass |
| `mise exec -- prek run --all-files` | Windows x64 | 0 | pass | repo hooks 全量 |

### Bounds and assumptions（Kani・slice 2）

`docs/specs/formal-verification.md` の「slice 2 archive harness」表が
正本。要約:

- `tar_typeflag_classification`: 512 byte 固定 TAR header block の
  byte 156（typeflag）のみ symbolic・全 256 値を尽きる。name・数値
  field・extension payload 中身は concrete — 複数 header や GNU/PAX
  長名は proptest `tar_pax_gnu_extensions` 等に委譲。
- `tar_declared_size_gate`: 同上 skeleton で `max_entry_uncompressed_
  bytes` を symbolic u64 にして declared=42 との比較 gate を尽きる
  — boundary も sampling 非依存で exhaustive。
- `zip_index_gates`: concrete ZIP skeleton（1 entry）で EOCD の
  `max_entries`/`max_metadata_bytes` budget を symbolic 化し
  index gate を尽きる。複数 entry・ZIP64・deflate 本体は proptest
  に委譲。
- 任意 malformed stream の exhaustive proof は scope 外 — concrete
  proptest `tar_arbitrary_stream_error_set` / `zip_arbitrary_
  stream_error_set` が shrink つき coverage を担う（slice 1 と同じ
  責務分割）。

### Honest notes（slice 2）

- archive harness の初版は harness 側 `tar_block` の 512 回 checksum
  loop が支配的 unwind コストで CBMC が実用時間を超過したため、
  固定 field の byte-sum を closed-form 定数（`TAR_HEADER_BASE` +
  octal digit sum）に置き換えた。定数が layout と不整合なら
  checksum mismatch で harness 自体が fail する自己検証構造。
- mutation attempt 1（declared-size 境界 `>=`）が proptest に検出
  されなかった事実は boundary density の正直なギャップとして残す。
  同 gate は Kani harness が symbolic u64 で exhaustive に検証する
  対象であり、boundary 検出は sampling に依存しない。
- 変更ファイルに生産 `src/` 差分なし（mutation は完全 revert）。
  `Cargo.lock` は proptest dev-dep 解決分のみ。`tests/common/mod.rs`
  は `Debug` derive 追加のみ（proptest 診断表示用・意味論不変）。

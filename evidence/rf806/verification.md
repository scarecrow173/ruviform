# rf806 — formal verification D 層（proptest + Kani）verification record

Issue `scarecrow173/ruviform#806` の検証記録。canonical = `mise`
lane（`test-formal` / `test-formal-kani`）。cargo 直接実行は補助証拠
（mutation 注入確認・ロック前の下見）として明記済み。slice 1（wire
codec）・slice 2（archive reader）・slice 3（structured readers）・
slice 4（WAV muxer/serializer）をそれぞれ記録する。

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
| `cargo kani --harness proofs::zip_index_gates`（単独・timeout 900s → 1800s 再試行） | WSL2 Ubuntu x64 | 124 | not_run | 900s/1800s いずれも timeout で中断 — unwinding/symex 段階で継続中、BMC の `Runtime` 行未到達。verdict 未発行・反例なし |
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

## slice 3 — structured document readers（ruviform-io-structured JSON/YAML/TOML/CSV/XML）

- Date: 2026-11-21
- OS: Windows (x86_64) — proptest/通常 lane。Kani lane は WSL2 Ubuntu
  （`cargo kani` 検出 → `/mnt/g` パス写像 → WSL ネイティブ target dir）
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1（Windows）;
  rustc 1.99.0 / Kani Rust Verifier 0.68.0 / CBMC 6.11.0（WSL2）
- Repo SHA: `e7fcf22ec2683cc518e2e3b6c64ad26dbe280b9a`（`origin/main`
  起点。各 JSONL の `repo_sha` は実行時 HEAD を記録）
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果が証拠（本記録はそれを代替しない）。

### 検証対象（slice 3）

- `crates/ruviform-io-structured` の serde-typed JSON/YAML/TOML reader・
  pull-based CSV `CsvReader`・pull-based XML `XmlEventReader`・
  共有 `InputLimits`/`BoundedRead` byte budget（`src/limits.rs` /
  `src/input.rs`）・安定 `DocumentErrorKind` 分類 —
  `tests/formal_structured.rs`（35 properties + `#[cfg(kani)] mod
  proofs` 2 harness + `src/input.rs` の in-src `BoundedRead` proof
  1 harness）。
- 生産コードの意味論は変更なし。`src/input.rs` への追加は末尾の
  `#[cfg(kani)] mod proofs` のみ（`cfg(kani)` 下でしか存在しない
  検証コード — `cargo kani --tests` が lib target をカバーするため
  in-src 配置。mutation 注入・完全 revert 済み、`git diff` の
  意味論差分ゼロで確認）。

### Actions → resulting state（slice 3）

| # | Action (command) | Resulting state |
| --- | --- | --- |
| 1 | `mise run test-formal` | `formal_ipc` 7/7 + `formal_transport` 4/4 + `formal_archive` 18/18 + `formal_structured` 35/35 pass・exit 0。記録: `slice3/test-formal.txt` |
| 2 | `mise run test-formal-kani` | invocation 1（wire/archive）は CBMC 4 並列が WSL2 の 31 GiB を枯渇させ途中 kill（per-harness verdict なし）。invocation 2（structured）は初回 `csv_max_*_gate` が `csv_core::build_dfa` の unwinding assertion で FAILED → bound を 128 + `--no-unwinding-checks`（skeleton byte < 128 のみ到達）に調整後、3/3 harness SUCCESSFUL。記録: `slice3/kani-io-structured.txt`・`slice3/kani-bounded-read.txt` |
| 3 | mutation 注入: `src/input.rs` `BoundedRead::read` の probe 比較 `remaining == 0` → `remaining <= 1`（inclusive 境界を 1 byte 早く効かせる） | `json_max_bytes_gate` が shrink つき反例 `tail = [], offset = 0`（`limit == len` の丁度 budget 文書が `LimitExceeded` を返す）で FAILED（34 passed / 1 failed）。suite が契約違反を検出できることの実証。記録: `slice3/mutation-fail.txt` |
| 4 | 変異の完全 revert → `cargo test -p ruviform-io-structured --all-features --test formal_structured` 再実行 | 35/35 green・EXIT=0・`git diff` src/ は `#[cfg(kani)]` 追加のみ（変異持ち越しなし）。記録: `slice3/post-revert-green.txt` |
| 5 | `mise run fmt-check` / `check-io-structured` / `lint-io-structured` / `test-io-structured` | 全 pass（feature matrix: default・xml・xml+file・all-features の 4 系統。lint は `doc_markdown`・`single_match`・`int_plus_one`・`match_same_arms`・`semicolon` 指摘を修正済みの最終形で clippy `-D warnings` clean） |
| 6 | `mise exec -- prek run --all-files` | （下部 lane table を参照） |

### Lane / verification results（slice 3）

| Command | OS | Exit | Verdict | 備考 |
| --- | --- | --- | --- | --- |
| `mise run test-formal` | Windows x64 | 0 | pass | proptest 7+4+18+35=64 properties 全 pass |
| `mise run test-formal-kani` | WSL2 Ubuntu x64 | 1 | fail | invocation 1（wire/archive）: CBMC OOM kill・per-harness verdict なし（slice 2 と同じ WSL2 31 GiB 枯渇 — 環境起因）。invocation 2（structured）: 3/3 harness SUCCESSFUL（下記の個別行を参照） |
| `cargo kani --tests --all-features --no-unwinding-checks -p ruviform-io-structured`（structured invocation 単独再実行） | WSL2 Ubuntu x64 | 0 | pass | `bounded_read_budget_gate` 0/532 failed（62.7s）・`csv_max_field_bytes_gate` 0/5812（17.4s）・`csv_max_records_gate` 0/5794（17.5s） — "3 successfully verified harnesses, 0 failures" |
| `cargo test … formal_structured`（mutant 注入時） | Windows x64 | 101 | fail（期待どおり） | `json_max_bytes_gate` が `tail=[], offset=0` を縮小提示 |
| `cargo test … formal_structured`（revert 後） | Windows x64 | 0 | pass | 変異なしで緑に復帰 |
| `mise run fmt-check` | Windows x64 | 0 | pass | |
| `mise run check-io-structured` | Windows x64 | 0 | pass | `--all-targets` × feature matrix 4 系統（新規 test ターゲット含む） |
| `mise run lint-io-structured` | Windows x64 | 0 | pass | clippy `-D warnings` clean × matrix |
| `mise run test-io-structured` | Windows x64 | 0 | pass | パッケージ全テスト × matrix（all-features で structured 129 + formal 35 + doctests） |
| `mise exec -- prek run --all-files` | Windows x64 | 0 | pass | repo hooks 全量（trailing whitespace の auto-fix 後の再実行で green） |

### Bounds and assumptions（Kani・slice 3）

`docs/specs/formal-verification.md` の「slice 3 structured harness」
表が正本。要約:

- `bounded_read_budget_gate`（in-src `src/input.rs`）: concrete
  6 byte input × symbolic `max_bytes:u64`（全値）— delivered byte
  総量が `min(len, max)` に一致・`Ok(0)` が真の EOF のみ・
  trip flag ⇔ byte が budget 外に存在・error/EOF とも sticky、を
  exhaustive に尽きる。
- `csv_max_records_gate` / `csv_max_field_bytes_gate`（
  `tests/formal_structured.rs`）: concrete skeleton（`h\nr1\nr2\nr3\n`
  / `a,bb\n`）× symbolic limit — gate comparison のみ symbolic で、
  buffer 成長系 option は concrete に抑える（RawVec unwind 回避は
  slice 1/2 の教訓どおり）。

### Honest notes（slice 3）

- mutation（probe 比較 `== 0` → `<= 1`）を検出したのは
  `json_max_bytes_gate` のみ — 発火条件は「read() が丁度
  `remaining == 1` で呼ばれる」ことで、serde_json の逐 byte
  trailing check は頻繁にその境界に着地するが、yaml/toml/csv/xml
  の backend は大きな fill で `remaining` を 1 跨いで 0 まで一度に
  落とすため、この変異は read 粒度の関係でマスクされる。検出能力
  の差は変異の性質によるもので gate 欠陥ではない — `limit < len`
  → `LimitExceeded` の本線は全形式で pin 済み（probe を越えて読む
  必要がある input では backend 非依存）。
- `max_record_bytes` の実効境界は `r >= m`（terminator 付き record
  では buffer が丁度満杯でも `OutputFull` が発火 — terminator 処理に
  出力 headroom が必要）。`max_field_bytes`・`max_fields` の
  `s > m` / `f > m` と異なり buffer-capacity 由来の半開区間 —
  spec 側の slice 3 表に実測どおり記録した。
- JSON の strict-EOF 空白集合は {space, tab, LF, CR} — ASCII 空白の
  form feed（0x0C）は `TrailingData`。`is_ascii_whitespace` との差は
  property が実測で検出（seed `tail=[12]` は
  `formal_structured.proptest-regressions` に保存済み・毎回再生）。
- in-budget tail byte が probe 発火より先に非 limit error
  （`Syntax`/`InvalidEncoding`/`Io`）を出し得るのは backend の
  読み順依存 — そのため各形式の `*_max_bytes_gate` は
  「in-budget で誤分類し得ない tail」を選んで deterministic に
  pin し、任意 tail の kind 集合 membership は arbitrary drive に
  分離した（JSON のみ逐 byte モデルで正確な規則を証明）。

## slice 4 — WAV muxer/serializer（ruviform-io-wav）

- Date: 2026-11-22
- OS: Windows (x86_64) — proptest/通常 lane。Kani lane は WSL2 Ubuntu
  （`cargo kani` 検出 → `/mnt/g` パス写像 → WSL ネイティブ target dir）。
  WSL2 VM は本 slice の検証中に `Wsl/Service/E_UNEXPECTED` で繰り返し
  落ち、CBMC RSS ~1-4.5 GiB で service 死亡・fresh boot では ~31 GiB
  envelope まで生存する状態依存を示した（environment-caused。
  反例ではない）。
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1（Windows）;
  rustc 1.99.0 / Kani Rust Verifier 0.68.0 / CBMC 6.11.0（WSL2）
- Repo SHA: `c58fc94697be00a718b21dc47d6de795d5430e7c`（`origin/main`
  起点。各 JSONL の `repo_sha` は実行時 HEAD を記録）
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果が証拠（本記録はそれを代替しない）。

### 検証対象（slice 4）

- `crates/ruviform-io-wav` の wire-format gate（`Wire::new`）・
  RIFF/WAVE serialization（`WAVEFORMATEX` 16B / `WAVEFORMATEXTENSIBLE`
  40B `fmt ` body・`data` chunk 配置）・canonical planar `f32` → wire
  sample 変換（NaN→silence・clamp・全レンジ scaling・round・F32 LE 直通
  ・no dither）・encoder packet/frame 整合・muxer finalize/backpatch/
  `WavReceipt` one-shot —
  `tests/formal_wav.rs`（9 properties + `#[cfg(kani)] mod proofs`
  3 harness）+ `src/convert.rs` / `src/wav.rs` 末尾の in-src
  `#[cfg(kani)] mod proofs`（7 harness）。
- 生産コードの意味論差分は 1 箇所のみ: `src/wav.rs` の `Unsupported`
  診断構築を `unsupported_sample_format` helper 経由に分離し
  （`cfg(kani)` 時は `format!` を持たない軽量診断 — `ErrorKind`・
  分岐条件は不変、診断文面のみ `cfg` 差分）。`std::io::Error::new`
  + `format!("{sample_format:?}")` を symbolic domain に載せると
  WSL2 envelope を超えたための feasible 化であり、ゲート契約自体は
  `format_tag_membership_is_wire_formats` が exhaustive に証明する。
  mutation 注入・完全 revert 済み、`git diff` の意味論差分ゼロで確認。

### Actions → resulting state（slice 4）

| # | Action (command) | Resulting state |
| --- | --- | --- |
| 1 | `mise run test-formal` | `formal_ipc` 7 + `formal_transport` 4 + `formal_archive` 18 + `formal_structured` 35 + `formal_wav` 9 = 73 properties 全 pass・exit 0。記録: `slice4/test-formal.txt` |
| 2 | `cargo kani --tests -j1 -p ruviform-io-wav`（lane の wav invocation 単独・lib harness 7 本を先に個別実行） | lib 側 7/7 harness SUCCESSFUL（下記 lane table）。記録: `slice4/kani-io-wav-lib.txt` |
| 3 | test 側 3 harness を個別 `-j1` で実行（環境分裂回避のため harness 毎に分割） | `open_muxer_spec_contract_pcm` 0/4530（15.8s）・`open_muxer_spec_contract_extensible` 0/4536（29.5s）SUCCESSFUL。`open_muxer_rejects_invalid_value` は Err-direction（Ok 到達不能の証明）が fresh VM でも ~31 GiB envelope に張り付き verdict 未発行 — `not_run`（environment-blocked）。記録: `slice4/kani-open_muxer_spec_contract_{pcm,extensible}.txt`・`kani-open_muxer_rejects_invalid_value.txt` |
| 4 | mutation 注入: `src/convert.rs` S16 arm `f32::round` → `trunc`（round-half-away-from-zero 契約の破壊） | `sample_conversion_matches_documented_policy` が shrink つき反例 `spec{S16, rate=1, Mono}, samples=[0.4151574]`（`0.4151574*32768=13603.97` → round 13604 vs trunc 13603）で FAILED（8 passed / 1 failed・EXIT=101）。seed は `formal_wav.proptest-regressions` に永続化済み。記録: `slice4/mutation-fail.txt` |
| 5 | 変異の完全 revert → `cargo test --locked -p ruviform-io-wav --test formal_wav` 再実行 | 9/9 green・EXIT=0・`git diff` src/ は `#[cfg(kani)]` 追加と kani 限定診断 helper のみ（変異持ち越しなし）。記録: `slice4/post-revert-green.txt` |
| 6 | `mise run fmt-check` / `check ruviform-io-wav` / `lint ruviform-io-wav` / `test ruviform-io-wav` | 全 pass（lint は `checked_conversions`・`explicit_counter_loop`・`single_match` 指摘を修正済みの最終形で clippy `-D warnings` clean） |
| 7 | `mise exec -- prek run --all-files` | （下部 lane table を参照） |

### Lane / verification results（slice 4）

| Command | OS | Exit | Verdict | 備考 |
| --- | --- | --- | --- | --- |
| `mise run test-formal` | Windows x64 | 0 | pass | proptest 7+4+18+35+9=73 properties 全 pass |
| `cargo kani -j1 --output-format terse -p ruviform-io-wav`（lib harnesses） | WSL2 Ubuntu x64 | 0 | pass | 7/7 SUCCESSFUL: `write_sample_documented_policy` 0/482・`format_tag_membership_is_wire_formats` 0/88・`wire_new_gate_is_exact_{small,mid}_align` 0/362+0/362・`wire_new_gate_is_exact_large_align` 0/368・`wire_new_non_wire_is_unsupported` 0/251・`fmt_and_header_layout` 0/890 |
| `cargo kani --tests -j1 -p ruviform-io-wav --harness open_muxer_spec_contract_{pcm,extensible}` | WSL2 Ubuntu x64 | 0 | pass | 0/4530（15.8s）・0/4536（29.5s）SUCCESSFUL — KaniSink（固定 80B `Write+Seek` sink）化で `Cursor<Vec<u8>>` splice/growth の SSA 膨張を回避 |
| `cargo kani --tests -j1 -p ruviform-io-wav --harness open_muxer_rejects_invalid_value` | WSL2 Ubuntu x64 | — | not_run | Err-direction（Ok 不可達性）の SAT instance が untrimmed ~31.7 GiB 張り付き。unwind 96→16→4・成功分岐 assert 除去・KaniSink でも支配コストは `open_muxer` 本体の encoding で不変。verdict 未発行のため pass 扱いしない（fail-closed）。契約は `wire_new_gate_is_exact_*`（`InvalidValue` の唯一 rejection を exhaustive 証明）+ proptest `muxer_rejections_are_invalid_data_and_not_sticky` で union 補完 |
| `mise run test-formal-kani`（全 invocation 一括） | WSL2 Ubuntu x64 | 1 | fail | `-j4` 時代の初回一括実行は invocation 1（wire/archive）で WSL service が `E_UNEXPECTED` 死亡・JSON verdict `fail` を honest に出力 — 環境起因（反例なし）。記録: `slice4/test-formal-kani-invocation-crashed.txt` |
| `cargo kani --tests -j1 -p ruviform-io-wav`（lane invocation 3 を aggregate 実行） | WSL2 Ubuntu x64 | 137 | partial | aggregate 順（lib 7 → `open_muxer_spec_contract_extensible` → `open_muxer_rejects_invalid_value` → `open_muxer_spec_contract_pcm`）で 8 harness SUCCESSFUL 後、`rejects` が ~12 GiB まで伸長した時点で中断（SIGKILL — 31 GiB envelope 張り付き経路は別個個別実行で実測済み）。`pcm` は queue 順で未到達だが個別実行で verified（上行）。記録: `slice4/kani-io-wav-aggregate.txt` |
| `cargo test … formal_wav`（mutant 注入時） | Windows x64 | 101 | fail（期待どおり） | `sample_conversion_matches_documented_policy` が `samples=[0.4151574]` を縮小提示 |
| `cargo test … formal_wav`（revert 後） | Windows x64 | 0 | pass | 変異なしで緑に復帰 |
| `mise run fmt-check` | Windows x64 | 0 | pass | |
| `mise run check ruviform-io-wav` | Windows x64 | 0 | pass | `--all-targets`（新規 formal_wav test target 含む） |
| `mise run lint ruviform-io-wav` | Windows x64 | 0 | pass | clippy `-D warnings` clean |
| `mise run test ruviform-io-wav` | Windows x64 | 0 | pass | パッケージ全テスト（formal 9 + 既存 tests + doctests） |
| `mise exec -- prek run --all-files` | Windows x64 | 0 | pass | repo hooks 全量（新規 evidence・test file を含め auto-fix 不要で green） |

### Bounds and assumptions（Kani・slice 4）

`docs/specs/formal-verification.md` の「slice 4 wav harness」表が
正本。要約:

- `write_sample_documented_policy`: symbolic `f32` 全値（NaN/inf/
  範囲外含む）× wire 5 format 全て — 変換後 byte 列が documented
  policy の oracle と bit-exact。symbolic float は `f32::round`/
  clamp を含むが bounded・無 loop で feasible。
- `format_tag_membership_is_wire_formats`: symbolic `SampleFormat`
  全 discriminant — `format_tag` が `Some` を返す集合がちょうど
  {U8,S16,S24,S32,F32} に一致（`Wire::new` のゲート oracle を分離
  証明 — 実 `Wire::new` 経路の `Unsupported` kind は
  `wire_new_non_wire_is_unsupported` + proptest が担当）。
- `wire_new_gate_is_exact_{small,mid,large}_align`: concrete matrix
  — distinct `block_align` 値 13 代表 × rate ∈ {u32::MAX/align−1,
  u32::MAX/align, u32::MAX/align+1}。byte-rate overflow 境界の両側を
  全 align class で尽くす。symbolic `SampleRate::new` は
  `TimeBase::new` Euclid loop が nested symbolic `%` を生成し
  infeasible（slice-1/2 教訓どおり concrete matrix + proptest が
  任意 rate/layout/format の interior を担当）。3-way split は
  solver-RAM 対策（semantic ではない）。
- `wire_new_non_wire_is_unsupported`: non-wire 5 format 各 concrete
  に `Wire::new` が `ErrorKind::Unsupported` を返すこと。
  `unsupported_sample_format` helper 分離（`cfg(kani)` で `format!`
  を持たない診断）により `io::Error::new`+`{:?}` encoding を回避し
  feasible 化。
- `fmt_and_header_layout`: concrete matrix — layout class（named
  ≤2ch/5.1/7.1/custom mask）× format — 16B/40B `fmt ` body の全
  byte layout・`data` chunk 位置・cbSize/valid-bits/mask/GUID を
  byte 列で証明。`unwind(64)`（memcmp/slice 比較の builtin unroll。
  当初 16 で unwinding assertion が loud fail → 引き上げ後 verified）。
- `open_muxer_spec_contract_{pcm,extensible}`: concrete success
  matrix（16B form 1 spec・40B form 3 spec — `fmt ` body は
  `fmt_and_header_layout` で全 layout×format 証明済みのため
  open_muxer glue は代表のみ）— `KaniSink`（固定 80B 配列の
  `Write+Seek` 最小 sink）で `Cursor<Vec<u8>>` の splice/growth
  encoding を排除。
- `open_muxer_rejects_invalid_value`: byte-rate overflow spec が
  `ErrorKind::InvalidValue` に写像されることの Err-direction 証明
  — 本 environment で verdict 未取得（`not_run`、上表参照）。
  Ok 不可達性の SAT core は成功 direction の witness 探索より
  構造的に重く、`open_muxer` 全 CFG encoding が ~31 GiB envelope
  を超過。

### Honest notes（slice 4）

- `open_muxer_rejects_invalid_value` は 5 形態（untrimmed・
  trimmed body・KaniSink・unwind 16・unwind 4）で計 6 回試行し、
  fresh VM でも cbmc が ~31 GiB envelope に張り付いたまま
  verdict を発行しない — 環境起因の `not_run` と正直に記録する
  （`pass` 扱いしない）。対象契約（overflow → `InvalidValue`
  伝播）は `wire_new_gate_is_exact_*` が source gate を
  exhaustive に証明し、`open_muxer` の `?` 伝播は proptest
  `muxer_rejections_are_invalid_data_and_not_sticky` が任意 spec
  で pin する union 補完で意味を持つ。
- WSL2 service は累積不安定で cbmc ~1 GiB でも `E_UNEXPECTED`
  死亡し得る一方、fresh boot では ~31 GiB まで生存した —
  同一 box で閾値が数倍振れる。`-j1` 逐次化で並列圧は排除済み。
- symbolic `SampleRate`（`TimeBase::new` Euclid `while`）は全
  harness で infeasible だったため、rate 軸は常に concrete —
  gate 比較自体は `u32::MAX/align` 境界の matrix で尽きる設計。
  任意 rate/layout/format の interior は proptest が担当。
- `fmt_and_header_layout` の check 数は `unsupported_sample_format`
  分離後 3912 → 890 に減少（diagnostic encoding が全 unroll に
  混入していた）。
- mutation（S16 `round`→`trunc`）を検出したのは
  `sample_conversion_matches_documented_policy` — model が
  production scaling と同一式を独立実装しているため契約逸脱に
  正確。seed `0.4151574` は regressions file 永続化済み。
- Kani 検証コードは全て `#[cfg(kani)]` 下 — `cargo test`/lint/
  通常 lane の compiled surface に影響しない（`check-cfg` は
  workspace で `cfg(kani)` を既に known 宣言）。

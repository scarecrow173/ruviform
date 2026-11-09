---
id: "IMPL-034UDl0o66TbT5Ovx4NeVe"
type: "impl"
status: "completed"
title: "Issue #398/#400 残件 audit — rooted capability・sealed readback decoder bridge の受入照合と検証証拠"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/398"
    - "https://github.com/scarecrow173/ruviform/issues/400"
    - "docs/specs/rooted-read-capability.md"
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/test-specs/rooted-read-capability.md"
    - "docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-398-400-audit-rooted-capability-sealed-readback-decoder-bridge.md"
    modified:
      - type: "file"
        path: "docs/plans/rooted-read-capability.md"
      - type: "file"
        path: "docs/test-specs/rooted-read-capability.md"
      - type: "file"
        path: "docs/plans/issue-400-borrowed-readback-decoder-bridge.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/rooted-read-capability.md"
    - "docs/plans/issue-400-borrowed-readback-decoder-bridge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/rooted-read-capability.md"
    - "docs/designs/issue-400-borrowed-readback-decoder-bridge.md"
    - "evidence/rf398-rooted-read/report.md"
    - "docs/impl/ir/issue-442-443-435-verification-closeout.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #398/#400 残件 audit — rooted capability・sealed readback decoder bridge の受入照合と検証証拠

## Summary

Issue #398（rooted read-only filesystem capability — handle 相対の
open/metadata/browse 境界、SV85/PR180 由来）と Issue #400（借用 sealed
readback を独立 decoder へ bounded 接続する共通 API）の全受入行を
`origin/main` `b36f6de2` の実装・tests・merged PR と行単位で照合した。
**src 変更は不要** — 全行について充足証拠を確認した
（cross-repo gate・実 OS runtime 行は対象外規約どおり NotRun 記録）。
docs チェーンの status 不整合（plan status `done`→`completed`、
test-spec `verifies` への design 追加、#400 plan の stale checkbox）
と本記録のみを差分とする。`crates/**` は無変更。最終 head は
`origin/main` `662ba7ed` の merge 後であり、io 系 lane は merge 後
head で再検証済み。

## Context

- 対象 Issue: <https://github.com/scarecrow173/ruviform/issues/398> /
  <https://github.com/scarecrow173/ruviform/issues/400>
- 既統合 PR: #401（`f483fe1c` rooted read-only fs capability、Refs #398）、
  #407（`6d7a0f3e` 借用 sealed readback→独立 decoder bounded 接続、
  Refs #400）、#412（`d0ed6208` #409 scoped re-root `open_scoped_root`/
  `read_dir_each`、Refs #398）、#513（#409 doc chain 完了反映）、
  #516（#305 WAV golden/fault）。Consumer 側: Synvelis PR #180
  （MERGED・`RootedDir` 消費）、Synvelis PR #181（MERGED・
  `with_borrowed_demuxer` 消費 + 独自 proxy 撤去）。
- 文書 chain: #398 = `docs/{specs,adr,designs,test-specs,plans,tasks}/
  rooted-read-*` + `evidence/rf398-rooted-read/report.md`。
  #400 = `docs/{specs,designs,plans,tasks,test-specs}/
  issue-400-borrowed-readback-decoder-bridge*`。
- 作業ブランチ: `chore/io-398-400-audit`（`origin/main` `b36f6de2` 起点、
  docs のみの差分）。
- 先行例: `docs/impl/ir/issue-442-443-435-verification-closeout.md`（#474）
  と同型の検証クローズアウト記録。

## Implementation

### Changes

コード変更なし。`docs/**` のみを更新した。

- `docs/plans/rooted-read-capability.md`: status `done` → `completed`
  （plan type の有効 status は `completed` であり `done` は audit error
  `invalid-status` 相当 — doc_status audit で検出・修正）。
- `docs/test-specs/rooted-read-capability.md`: `relations.verifies` に
  `docs/designs/rooted-read-capability.md` を追加（design の
  `verified-by` 宣言と整合。#400 chain と同じ spec+design 両 verify
  形状へ揃えた）。
- `docs/plans/issue-400-borrowed-readback-decoder-bridge.md`:
  Implementation Sequence / Verification の stale な `- [ ]` を実績で
  更新（merge 済み・再検証 pass）。
- managed index（`docs/plans/README.md`・`docs/specs/README.md`・
  `docs/tasks/README.md`）を `doc_maintenance.js apply` で再生成。
  rooted plan の status 修正に伴う settled セクション移動のほか、
  無関係な #278 系 spec/task の stale status が front-matter 実値へ
  同期された（index-metadata-mismatch の機械的修復）。本 IR は
  `docs/impl/ir/README.md`（hand-curated）へ手動で行追加した。

### Decisions During Implementation

- 受入照合は「実装が存在する」ではなく「Issue の受入行の文言を
  満たす証拠が main に存在し、今回の head で再検証できる」ことを
  基準に行った。各行の判定根拠は下記「受入条件との対応」に記す。
- cross-repo gate（Synvelis 側の互換 tuple 検証・実機/OS/OOM 計測）と
  未実行 OS の runtime 行は本タスクの対象外規約に従い NotRun として
  記録し、Pass 扱いしない。
- `mise run test`（workspace）の `ruviform-plugin-host` `l1_proxy_e2e`
  flaky は既知の process-boundary timing 問題であり、本差分（docs のみ）
  とは無関係。baseline では再実行で全 suite pass、merge 後 head では
  高並列下で散発失敗したが `--test-threads=4` で 184/184 pass を確認。
  失敗 test は実行ごとに異なり deterministic な回帰ではない。
- doc_status audit が報告する repo 全体の既存 finding（無関係な
  broken relation・reciprocal warning 等）は対象外 chain のみ本
 タスクで修復し、リポジトリ全体の audit clean は主張しない。
  rooted-read chain の `plan derives-from spec` に対する spec 側
  `derived-by` 非対称 warning は非 blocking であり、`implemented-by`
  で既に双方向リンク済みのため据え置いた。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実行環境: Windows 11 x64 / Git Bash、mise 管理 toolchain
（`mise.toml` pin、Rust 1.99.0）。照合 baseline は `origin/main`
`b36f6de2`、最終検証 head は `origin/main` `662ba7ed` merge 後
（merge delta は docs と `#[ignore]`d plugin-host 計測 test のみで
io 系コード差分なし）。`chore/io-398-400-audit` ブランチ
（docs のみの差分）で実施。すべて `mise run` /
確立済み `mise exec -- cargo` 入口で実行した。

| タスク | 結果 |
| --- | --- |
| `mise run fmt-check` | pass（exit 0、merge 後 head で再実行） |
| `mise run check`（workspace `--all-targets`） | pass（exit 0、merge 後 head で再実行） |
| `mise run lint`（workspace `--all-targets -D warnings`） | pass（exit 0、merge 後 head で再実行） |
| `mise run check-io-file` / `lint-io-file` | pass（exit 0、baseline で実行。merge 後は workspace `check`/`lint` が同一 crate を `--all-targets` で包含） |
| `mise run test-io-file` | pass（exit 0、merge 後 head で再実行 — io-file unit 92 + `tests/rooted_read.rs` 実 FS 25 + io-archive 全 suite + doctests） |
| `mise run check-io-e2e` / `lint-io-e2e` | pass（exit 0、baseline で実行。merge 後は workspace `check`/`lint` が包含） |
| `mise run test-io-e2e` | pass（exit 0、merge 後 head で再実行 — `wav_staged`・`rooted_boundary`・fault/fuzz/os_fs_probe 含む全 suite） |
| `mise exec -- cargo test -p ruviform-io-symphonia -p ruviform-media-io -p ruviform-media-reader` | pass（exit 0、merge 後 head で再実行 — borrowed 12・borrowed_source 3・`SourceReader`/prefetch/reader/rt_fill 全件・`with_borrowed_demuxer` doctest + escape `compile_fail` doctest 含む） |
| `mise run test-rt` | pass（exit 0、`rt_` filter、merge 後 head で再実行） |
| `mise run check-linux` | pass（exit 0、baseline で実行 — io-file/io-archive の `x86_64-unknown-linux-gnu` check+clippy scoped lane。merge delta に io 系コード差分なし） |
| `mise run test`（workspace） | baseline head: 初回 `l1_proxy_e2e` 3 件 flaky 失敗 → 再実行 exit 0（191 suite・FAILED 0）。merge 後 head: `l1_proxy_e2e` で 2 件 flaky 失敗（`SpawnFailed{Timeout}` 系・失敗 test は毎回異なる）。同 binary の `--test-threads=4` 再実行は 184/184 pass — 高並列負荷下の timing flake であり、本差分（docs のみ・当該 crate 無変更）とは無関係。io 系全 lane は merge 後 head で個別に pass |
| `mise run test-msrv` 相当 | NotRun — io 系 crate 向けの MSRV task は `mise.toml` に未定義（`test-ofx-msrv`/`test-ui-test-msrv` のみ存在し対象外） |

### 受入条件との対応

#### Issue #398（rooted read capability）

| # | 受入行 | 判定 | 根拠 |
| --- | --- | --- | --- |
| 1 | leaf/中間 component 交換の決定的 fault/barrier 回帰で read/stat/pin/browse の契約を検証 | 充足 | `src/rooted/tests.rs` の scripted mock VFS が call 間 mutation を許す決定的 barrier（`leaf_swapped_to_link_never_opens_outside_object`・`intermediate_dir_swapped_to_link_is_notdirectory`・`open_pinned_roundtrip_and_recreate_mismatch`・`outside_node_bytes_never_surface`・`read_dir_each_*` 系）。実 FS 側は `tests/rooted_read.rs` の `leaf_link_swap_never_reads_outside`・`concurrent_leaf_swap_tolerance`・`pin_open_pinned_roundtrip_and_recreate_mismatch`・browse 系 `read_dir_each_*`/`opens_and_enumerates_a_real_tree`。io-e2e `rooted_boundary.rs` 4 件も pass。今回 `test-io-file`/`test-io-e2e` で再検証 |
| 2 | root 内 link 正常系、escape/loop/limit/nonregular/FIFO/reparse 等の型付き失敗を検証 | 充足 | `withinroot_*`/`absolute_link_*`（unit+実 FS）で root 内 link 受理、`link_loop_is_loop_detected`/`self_link_is_loop_detected`/`hop_budget_limits_expansions`/`depth_and_component_budgets_limit`/`absolute_link_*_escapes`/`dotdot_*_escapes`/`nonregular_leaves_are_not_regular_file` で typed 失敗（`EscapesRoot`/`LoopDetected`/`LimitExceeded`/`NotRegularFile`/`NotFound`/`Unsupported`）。FIFO は `fifo_leaf_is_not_regular_file`（`#[cfg(unix)]` — Windows では compile 対象外、unix lane の CI/実機で検証済み証跡は `evidence/rf398-rooted-read/report.md`）。reparse は Windows junction 系 `junction_absolute_inside_resolves_via_nt_bridge`/`junction_to_outside_escapes`/`absolute_link_alias_spelling_escapes_root` が実 FS で pass |
| 3 | reader と metadata/length/identity を同じ handle へ結び付け、open 前後の path 再解決による回避を防ぐ | 充足 | `RootedDir::open_regular`/`open_pinned` は resolve→open を同一 handle chain で行い、`RootedFile{file, info}` が同一 opened object の `probe` 由来 `NodeInfo` を保持（`src/rooted.rs:325-411`）。`pin`/`open_pinned` は per-object Strong identity gate（弱い FS は typed `Unsupported`）。全 resolve は fd-stack/handle walk で path 文字列の再解決を行わない（`src/rooted/resolve.rs` two-state TEXTUAL/FD walk） |
| 4 | archive 既存契約を回帰させず、正規 mise fmt/check/lint/test・独立 A/B→Validator→Judge を完了 | 充足 | archive の extract backend は共有 `vfs` の薄い adapter 化で公開契約不変（PR #401）。`test-io-file` が io-file+io-archive を同一 lane で全 pass（今回再検証）。design doc `review:` 節に cycle 2 `ACCEPT`（F1-F9・R-1〜R-12 解消）を記録済み、PR #401 body に独立 review 記録あり。workspace `check`/`lint`/`fmt-check` も今回 pass |
| 5 | OS ごとに実測と NotRun/Unsupported を区別し、保証できない条件を成功扱いしない | 充足（記録） | `evidence/rf398-rooted-read/report.md` が Windows 実測・unix compile-only・macOS NotRun を区別済み。本 audit でも同区分を維持: Windows 11 x64 実測（本 run）、unix/macOS runtime は NotRun（`check-linux` の linux-gnu check+clippy は通過するが runtime 未実測）。弱 identity FS は typed `Unsupported` を返す設計で silent degrade なし |

#### Issue #400（借用 sealed readback → 独立 decoder bounded 接続）

| # | 受入行 | 判定 | 根拠 |
| --- | --- | --- | --- |
| 1 | `media-io`/`io-symphonia`/`io-file` の責務を確認し、借用 `Read+Seek` と `'static` decoder の境界を解決する共通 API を追加 | 充足 | `ruviform_io_symphonia::with_borrowed_demuxer`（`src/borrowed.rs`）が `&mut (impl Read + Seek + ?Sized)` を `&mut dyn Demuxer` の scoped closure へ接続。`media-io` に `impl<D: Demuxer + ?Sized> Demuxer for &mut D` reborrow port（`demux.rs:438`）、`media-reader` に `SourceReader<'a>`（`reader.rs:79`）。責務診断は design doc に記録。io-file の `SealedFile::open_readback`（`ReadbackFile`）は変更なしでそのまま消費可能 |
| 2 | Sealed/read-only/publish 排他・所有権/`Send`/lifetime を守り、raw OS handle・unsafe lifetime 延長・temp path 再 open・全 WAV RAM copy・新 parser/外部依存を導入しない | 充足 | `borrowed.rs` に `unsafe` なし。`Cargo.toml`/`Cargo.lock` は PR #407 で無変更（依存追加なし）。port は `&mut dyn Demuxer` で escape 不能（`compile_fail` doctest で固定）、`R: Send`/`f: Send` のみ要求で `!Send` source を caller thread に留める。`io-e2e wav_staged` が実 `SealedFile` の `&mut` readback 借用中に publish 不能であることを型で保持し、scope 後に publish する経路を実測（`wav_finalize_borrow_release_seal_decode_publish_reopen` pass） |
| 3 | read/seek/EOF/Interrupted/partial error と decoder の早期終了/error/panic で、scope 終端・リソース解放・deadlock 不在・bounded in-flight memory を回帰テストで確認 | 充足 | `tests/borrowed.rs` 12 件: 逐語 seek（`seeks_are_forwarded_verbatim`）・byte-exact read（`borrowed_wav_demuxes_and_decodes_byte_exact`）・EOF/short read（`SparseSource`）・`Interrupted` 転送（`interrupted_error_is_forwarded_not_swallowed`）・途中 fault（`injected_read_fault_surfaces_as_io_error`）・早期 `Err`（`early_callback_error_terminates`）・invalid source（`invalid_source_fails_probe`）・callback panic（`callback_panic_propagates`）・source panic + scope join（`source_panic_propagates_and_scope_joins`）・`!Send` 受理（`non_send_source_is_accepted`）・再利用=deadlock 不在（`source_is_reusable_after_scoped_call`）・bounded（`large_logical_source_is_bounded`: 64 MiB 論理 source で `max_read_request <= 64 KiB`、`sync_channel(1)`×2）。`media-reader/tests/borrowed_source.rs` 3 件で `SourceReader` 経由 canonical decode。全件 pass（今回再検証） |
| 4 | 正規 mise による関連 crate/workspace 検証と独立レビューの結果を記録 | 充足 | 本記録の Validation 節に今回 head での実測を記録。PR #407 body に merge head での検証表と「独立 bridge／SourceReader review で新 P0/P1 なし」の記録あり。design doc に escape 防止・channel disconnect・panic 伝搬の設計根拠を記録済み |
| 5 | Synvelis #181 の Export/player 回帰を新共通 API で実行し、未実行の実機/OS/OOM 検証は Pass とせず NotRun 等で明示 | consumer 側実施済み・tuple gate は NotRun | Synvelis main の `synvelis-render/src/verify.rs` は `with_borrowed_demuxer` を消費（独自 ReadProxy/channel/scoped-thread 撤去済み）。SV PR #181（MERGED）の検証記録は `l2_e2e` 17/17・`x_e2e` 5/5・`pc_b` 7/7 pass（bridge 経路の PCM-exact 含む）。RF↔SV 候補 tuple 互換 gate は coordinator 管理の cross-repo gate であり本タスク対象外 → **NotRun**。実 OS/OOM/Native 実測は両側とも未実施で NotRun のまま |

## Risks

- `mise run test`（workspace）の `ruviform-plugin-host` `l1_proxy_e2e`
  flaky は既知の process-boundary timing 問題であり、本差分（docs のみ）
  とは無関係だが、workspace gate の安定性としては残課題。
- unix/macOS runtime の rooted capability 実測は依然 NotRun
  （`check-linux` は型/lint のみ）。実 OS 行は Issue 側でも
  cross-repo/実機 gate として扱われる。
- #468（RootedDir canonical premise の設計判断）は別 Issue で進行中
  であり本 audit の対象外。`WithinRoot` の非 canonical alias 保守的
  `EscapesRoot` は意図的縮小として spec/design に明記済み。

## Follow-ups

- Issue #398/#400 の Close 判断は Issue owner（cross-repo gate・実 OS
  行の最終判定を含む）の責務。本記録は受入行の充足証拠と検証実測を
  提供するのみで、Issue 契約の緩和・Close は行わない。
- consumer 側の残件: SV PR #181 checklist の「Consumer 互換 tuple
  検証」（coordinator 管理）と、synvelis #184 系の canonical 綴り
  fixture 規則（#468 chain）が残る。
- `ruviform-plugin-host` `l1_proxy_e2e` の flaky 安定化は別 Issue
  候補（process-boundary timing test の負荷依存）。

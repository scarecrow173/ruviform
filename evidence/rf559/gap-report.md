# RF-SCRIPT-01 Gap Report — Luau+Rhai 共存 PoC

Issue: https://github.com/scarecrow173/ruviform/issues/559
上位契約: Issue #558（RF-SCRIPT-00、main 統合済み）
検証環境: Windows 11 x86_64 / mise 管理 Rust toolchain
実施日: 2026-11-14

## 判定サマリ

**共存: 成立（PoC 範囲）**。同一実行ファイル内で Luau(mlua vendored
`luau0-src`) と Rhai を同時リンク・初期化し、`ScriptBackendRegistry`
（契約型ミラーの PoC ローカル実装）経由の language/backend requirement
で実行時選択した。交互評価・並行評価の双方で動作し、同一意味の
Versioned Typed Definition が canonical digest 一致で両 VM から出力
された。混種 Workflow 参照は digest+kind+revision の typed reference
のみで解決し、`vm_reference`/`handle`/`pointer`/`direct` の
`ref_type` は `DirectReferenceRejected` で拒否した。

**No-go 項目なし（PoC 範囲内）**。ただし以下の境界を明記する:
本 PoC は headless であり、実 OS 依存最適化・Consumer 組込み・
製品正式サポートは後続 Gate（#558/#553/#294/synvelis#264）の領域。

## Contract-covered / PoC-proven / NotRun の区別

| 領域 | 状態 | 根拠 |
| --- | --- | --- |
| 単一バイナリ同時リンク・初期化 | PoC-proven | `cargo build` が mlua+luau0-src と rhai を 1 crate に link。`registry()` で 2 backend 同時登録、demo binary が `registered backends: 2` を出力 |
| 実行時明示選択（language/backend/version） | PoC-proven | `fx_p02`/`fx_p03`/`fx_p04`、demo `resolve` 出力 |
| 交互・並行実行 | PoC-proven | `fx_p03`（交互 4 回）、`fx_p04`（2 thread scope 並行、digest 一致） |
| 同一意味 artifact の digest 一致 | PoC-proven | `fx_p01`/`fx_p06`、demo `SAME_DIGEST_OK`/`WORKFLOW_SAME_DIGEST_OK`（digest `sha256:v1:254a…`） |
| 混種 Workflow 参照（Host Typed Port） | PoC-proven | `fx_p05`: Luau 産 action_recipe を Rhai 産 workflow_definition が digest 参照し `build_workflow_plan` が 2 node plan を構築 |
| VM ポインタ直渡し・循環拒否 | PoC-proven | `fx_n25`（`vm_reference`/`handle`/`pointer`/`direct` → `DirectReferenceRejected`）、`fx_n24`（未知/自己参照 → `UnknownReference`）。真の循環は内容アドレス digest 上構築不能（SHA-256 不動点が必要）— `visiting` set は防御的に実装済み |
| i64 厳密・Null/Absent・ID・Time | PoC-proven | `fx_p06`: `i64::MAX` が tag record 経路で厳密往復、Absent≠Null、Time{domain,timebase,count}・ResourceId{domain,kind,value}・Bytes 保持、両 backend canonical digest 一致 |
| Unsafe Host API 拒否 | PoC-proven | `fx_n14`（`os.system` binding → `InvalidHostSurface`）、`fx_n18`（宣言/handler 不整合 → `InvalidHostSurface`） |
| Quota（instructions/wall/memory/output/host-calls） | PoC-proven | `fx_n07`〜`fx_n11`: `BudgetExceeded(MaxXxx)` を両 backend で確認。Rhai は `MaxMemoryBytes` を広告しない → `fx_n06` で `UnsupportedBudget`（silent ignore しない） |
| 協調 Cancel | PoC-proven | `fx_n12`: `CancelToken` 共有 flag + Luau `set_interrupt`/Rhai `on_progress` safepoint で `Cancelled` |
| Session Isolation | PoC-proven | `fx_n20`: Luau session A の global が session B に漏れない（`shared_vm_state=false`）。各 session は独立 VM |
| Fail-Closed registry（F-01〜F-05 相当） | PoC-proven | `fx_n01`〜`fx_n05`: UnknownLanguage/UnknownBackend/DuplicateBackend/AmbiguousBackend/UnsupportedContractVersion/MissingCapability |
| Fail-Closed session（digest/language/size/RT） | PoC-proven | `fx_n15`〜`fx_n19`: `ContractViolation`/`SourceDigestMismatch`/`LanguageMismatch`/`InvalidHostSurface`/`SourceTooLarge` |
| Plan Fail-Closed（unknown/stale/kind/digest/schema） | PoC-proven | `fx_n22`/`fx_n23`/`fx_n26`/`fx_n27` + `fx_n25` |
| Vendor 型の公開面非漏出 | PoC-proven（構造的） | `ScriptValue`/`ScriptError`/`ScriptArtifact` に mlua/Rhai `Dynamic`/DAW 型なし。`fx_n21` で error kind の Debug 出力に vendor 名が出ないことを確認。codec が `$rv` tag record で値を正規化 |
| Build/Link/Run — Windows | PoC-proven | `evidence/rf559/verification.md` のコマンド記録 |
| Build/Link/Run — macOS | NotRun | 本環境（Windows）では実行不能。技術非対応は未確定のため Unsupported にしない |
| Build/Link/Run — Linux | NotRun | 同上。Luau C++ は POSIX toolchain で build 可能な設計だが本環境で未実測 |
| OS 依存・Feature 非依存・GPU 非依存 | PoC-proven（Windows） | Feature 非依存: 排他 feature なしで `luau`+`rhai` を同時有効化した単一 crate。GPU 非依存: headless、wgpu/GPU 参照なし。OS 依存: Windows では MSVC + vendored Luau C++ で link 成功 |
| 実 Runtime への製品移管 | Contract-covered（#558）/ Deferred | 本 PoC は契約ミラーの最小実装。本番 `ScriptBackendRegistry`・Runtime 統合は #558/#553 Gate |
| Consumer 適用受入（synvelis 等） | Deferred（#294/synvelis#264） | Consumer repo は読み取り専用参照のみ。適用受入は統合 Gate |
| 3OS 配布レーン | NotRun（#560 領域） | real-OS/配布検証は本スライス対象外 |

## #558/#553 へ渡す結論

### 共存成立の根拠（Option C の実現可能性）

- `mlua 0.12.2`（vendored Luau、`send` feature）と `rhai 1.27.0`（`sync`）
  は同一 crate・同一バイナリで競合なく link・初期化できる。
- 両 VM は per-session 独立インスタンスで運用でき、session 間の状態
  漏洩は観測されなかった（`shared_vm_state=false` を capability で
  正直に広告）。
- canonical serialization + SHA-256 digest により、言語非依存の
  semantic equality を VM 間で検証できる。`$rv` reserved tag record
  で f64 非表現の i64・Time・ResourceId・Absent/Null 区別を往復できる。

### 残る制約・注意（#558/#553 への引き継ぎ）

1. **Luau の数値表現**: Luau は binary64 のみ。i64 厳密値は tag record
   経路が必須（script が素の number で大整数を扱うと精度落ち）。
   製品実装でも typed port 境界で tag を強制する必要がある。
2. **Rhai の memory budget**: Rhai 1.27 embedded には allocator 計測
   hook がなく `MaxMemoryBytes` を強制できない。capability 広告で
   非対応を明示し、要求時は `UnsupportedBudget` で fail-closed
   （silent ignore 禁止を PoC で実証）。製品で memory quota が必須なら
   Rhai 側に計測 wrapper か engine fork が必要。
3. **instruction budget の粒度**: Luau は `set_interrupt` の呼出回数、
   Rhai は `on_progress` の op counter を「命令数」として近似計上。
   厳密な命令数ではない — 契約の `CancelGranularity::Safepoint` と
   整合するが、quota 意味論は backend ごとの近似と明記すべき。
4. **host call は同期**: 両 backend とも host binding は同期呼出。
   blocking host call 中の即時 cancel は保証しない（契約どおり）。
5. **workflow 参照の構造制約**: artifact payload は record で
   `references[]` が typed reference のみ。digest は内容アドレスのため
   循環参照は構造上構築不能（防御的 `CycleDetected` は実装済み）。
   `ref_type` 語彙の拡張は契約側の versioning で扱う。
6. **Luau safe state**: PoC は標準 library を公開しない最小 safe state。
   製品で `string`/`table`/`math` 等を公開する場合、host surface 検査
   （`os.*`/`io.*`/`debug.*`/`loadstring` 系の遮断）を別途設計要。

### 最小移行・代替方式（#553 への提案材料）

- **推奨**: Option C（language-neutral typed Port + runtime
  Registry/Dispatch）をそのまま製品化。PoC が契約ミラーで実証した
  ため、残作業は本番 crate への移植・capability 行列の確定・
  consumer 配線のみ。
- **代替 A（backend 分離 process）**: Rhai memory quota が製品要件に
  なった場合、Rhai backend を別 process に逃がし typed port を IPC 化
  する方式。複雑度が上がるため、まず Rhai 側 allocator hook の
  有無を #553 で再調査するのが先。
- **代替 B（単一言語選択）**: Rhai のみ or Luau のみ採用。共存不要なら
  本 PoC の registry/codec 層は過剰。Issue #558 の動機（両言語の
  使い分け）を再確認してから判断。

## Fixture 対応表（evidence → test）

| Fixture | test | 結果 |
| --- | --- | --- |
| FX-P-01 同一 digest | `fx_p01_same_meaning_same_digest` | Pass |
| FX-P-02 明示選択 | `fx_p02_explicit_selection` | Pass |
| FX-P-03 交互評価 | `fx_p03_alternating_evaluation` | Pass |
| FX-P-04 並行評価 | `fx_p04_parallel_evaluation` | Pass |
| FX-P-05 混種 workflow | `fx_p05_mixed_workflow_reference` | Pass |
| FX-P-06 値モデル往復 | `fx_p06_value_model_roundtrip` | Pass |
| FX-P-07 typed host api | `fx_p07_typed_host_api` | Pass |
| FX-P-08 analyzer | `fx_p08_analyzer_registry` | Pass |
| FX-N-01〜05 registry fail-closed | `fx_n01`〜`fx_n05` | Pass |
| FX-N-06〜11 quota/budget | `fx_n06`〜`fx_n11` | Pass |
| FX-N-12 cancel | `fx_n12_cancel` | Pass |
| FX-N-13 値域外 | `fx_n13_value_out_of_range` | Pass |
| FX-N-14〜19 session fail-closed | `fx_n14`〜`fx_n19` | Pass |
| FX-N-20 session isolation | `fx_n20_session_isolation` | Pass |
| FX-N-21 error 分類 | `fx_n21_error_classification` | Pass |
| FX-N-22〜28 plan/analyzer fail-closed | `fx_n22`〜`fx_n28` | Pass |

全 36 test Pass（`cargo test --locked --all-targets`、Windows）。

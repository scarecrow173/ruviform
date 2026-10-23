---
id: "TASK-034KV6AzIxXBCyJdl6BFjk"
type: "task"
status: "todo"
title: "RT 観測計測ハーネスの実装（PoC scaffolding・criterion ベンチ・スモーク配線）"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/104"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/observability_poc.rs"
      - type: "file"
        path: "crates/ruviform-rt/benches/rt_observability_poc.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
  blocks:
    - "docs/tasks/rt-observability-poc-run-and-report.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-observability-poc-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# RT 観測計測ハーネスの実装（PoC scaffolding・criterion ベンチ・スモーク配線）

## Work

plan `PLAN-034KV6r7RDBR7CSvMSIB8e`（[../plans/implement-issue-104-rt-observability-poc.md](../plans/implement-issue-104-rt-observability-poc.md)）の Phase A。`crates/ruviform-rt` に PoC 用 criterion dev-dep・`#[doc(hidden)] pub mod observability_poc` scaffolding・計測ベンチ・self-consistency / カバレッジ隙間実証テストを実装し、`bench-smoke` の対象拡大と文書整合を行う。

- [ ] A-1. dev-dep 導入前のコンパイルコスト baseline を記録: `mise run check`（`cargo check --workspace --all-targets`）の所要時間を計測し、T2 のレポート補助データ（dev-dep 前後比較）の基準値として記録する
- [ ] A-2. `crates/ruviform-rt/Cargo.toml` に `[dev-dependencies]`（`criterion = { version = "0.8.2", default-features = false }` — ruviform-math と同一 pin 方針: caret 指定・追跡済み `Cargo.lock` を再現性の錨・`default-features = false` でコンパイルコスト抑制）と `[[bench]] name = "rt_observability_poc", harness = false` を追加（`[dependencies]` の `arc-swap`・`rtrb` は不変）。dev-dep 追加理由を Cargo.toml インラインコメントに記録する
- [ ] A-3. `src/observability_poc.rs` を新規作成:
  - `Counter`/`CounterReader` — telemetry スケッチ形状の共有 `Arc<AtomicUsize>` 計数ペア（`increment` = `fetch_add(1, Relaxed)`・`read` = `load(Relaxed)`）
  - `bridge<R: Copy + Send + 'static>(capacity) -> (RtRecorder<R>, RecordSink<R>)` — rt_bridge スケッチの実験実装。構築は非 RT のみ・capacity 構築時固定・リング全 pre-alloc（既存 private dep `rtrb` 上に構築してよい）。`RtRecorder::record(&mut self, rec: R)` は wait-free/no-alloc/bounded・満杯時 drop + 共有カウンタ加算、`overflows()`、`RecordSink::drain(&mut self, f: impl FnMut(R)) -> usize` bounded drain・`overflows()`
  - `ObsRecord` — 固定サイズ Copy レコード（kind 識別子・`i64` タイムスタンプ・`u64` 値・`reserved: [u64; 2]`、コンストラクタは `[0; 2]` 初期化・`reserved()` accessor 経由のみ読取。`#[non_exhaustive]` 相当の扱い — 設計の規約どおり）
  - オーバーフロー方針バリアント — 同一満杯シナリオで drop / aggregate / count の 3 方針を測れる API バリアント
  - タイムスタンプ取得 helper — `Instant::now()` + `Instant` → epoch 相対 nanos → `i64` 変換 → レコード格納（`ruviform-audio-device` の `instant_to_media_time` と同形の数学を std `Instant` 上で行う代理形）
  - `unsafe` 0・pub enum なし・pure std + crate 内依存（`rtrb`）のみ。`src/lib.rs` に `#[doc(hidden)] pub mod observability_poc;` を追加し、モジュール doc に「実験 API・本番 RT 契約の対象外・#103 判定材料の計測用 scaffolding であり本番 API を確定しない」旨を明記
- [ ] A-4. `#[cfg(test)]` テストを `src/observability_poc.rs` に実装:
  - self-consistency — crate の `alloc_counter` で `record`/`drain`/`overflows`/`Counter` 操作・タイムスタンプ helper の測定窓内 alloc/dealloc が 0 であることを検証（fixture 確保は `enable()` 前 = 窓外）
  - カバレッジ隙間実証 — 呼出側スレッドで `alloc_counter::enable()` → alloc する spawned thread を join → 呼出側 `count()` = 0 維持を検証、かつ spawned thread 内で enable→alloc→count すれば N>0 が観測できることを検証。3 組（呼出側=0・spawned 側=N・N>0）を標準出力に出して T2 がレポートへ転記できるようにする。ヘルパ名に `rt_` を含めない（`rt_` は RT 契約テスト lane 予約済みの既存規約どおり。テスト関数名自体は lane 対象外の命名でよい — 本検証は機構の実証であり RT 契約テストではない）
  - scaffolding 意味論 — record 成功 / drop-on-full で overflow 計数 / drain bounded（1 呼出 ≤ capacity）/ 方針バリアントの各意味論を検証
- [ ] A-5. `benches/rt_observability_poc.rs` を実装 — plan「計測マトリクス」節の列構成どおり: `counter`（計装なし / フラグ付き / 無条件計装）・`rt_bridge`（record 非満杯 / record 満杯 / drain / overflows）・`overflow_policy`（drop / aggregate / count + `retire_channel` leak-on-full 基準列）・`rt_primitives`（`channel` try_push/try_pop・`retire`/`overflows`・`MasterClock::tick`/`publish_tick`・`slot` publish/read）・`timestamp`（`Instant::now()` / 同形変換 / `SystemTime::now()` 対照列）。`BenchmarkId` 規約・ns 級操作のバッチ反復（N 回/iteration + `black_box`）・削減時間設定（warm_up/measurement/sample — 全量目安 ≈10–20 分）・run 冒頭に環境メタデータ（OS・CPU 型名・コア数・rustc 版・criterion 版・対象クレート版）を標準出力
- [ ] A-6. `mise.toml` の `bench-smoke` を `cargo bench -p ruviform-math -p ruviform-rt -- --test` に更新（タスク記述も実態に合わせる）し、`AGENTS.md` の検証入口対応表 `bench`/`bench-smoke` 行を更新。`mise run bench-smoke` スモーク完走と `mise exec -- cargo bench -p ruviform-rt -- counter` の quick-run filter を確認

コミット粒度: ①dev-dep + scaffolding + テスト（A-1〜A-4）、②ベンチ + smoke 配線 + 文書整合（A-5〜A-6）の 2 コミットを目安とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-104-rt-observability-poc.md](../plans/implement-issue-104-rt-observability-poc.md) — `PLAN-034KV6r7RDBR7CSvMSIB8e`（approved）。Phase A の正本
- [../designs/foundation-observability-minimal-api.md](../designs/foundation-observability-minimal-api.md) — `DESIGN-034KQXr3w4dYgKO79dklG9`（approved）。rt_bridge 形状・Counter 選定・`reserved: [u64; 2]` 規約の正本
- [../designs/shared-foundations-and-modularization-policy.md](../designs/shared-foundations-and-modularization-policy.md) — `DESIGN-034Kx6CVMPDNgzKhJwfgSp`（approved）。RT/非 RT 観測データフローの正本
- [../test-specs/rt-observability-poc-tests.md](../test-specs/rt-observability-poc-tests.md) — `TSPEC-034KV6hs4k3mZ6jVBgDaKf`。計測方法論契約（測定窓純粋性・有効データ条件・隙間実証）の正本

## Dependencies

- 前提タスク: なし（依存グラフのルート。plan `depends-on` の `foundation-observability-minimal-api` が提供する rt_bridge 形状・Counter 選定を scaffolding の正本として利用）
- 後続をブロック: [rt-observability-poc-run-and-report.md](rt-observability-poc-run-and-report.md) — 計測ハーネスが計測実行の前提
- 外部前提: crates.io 上の `criterion` 0.8.2 の入手可能性
- MSRV 検証は `mise exec -- rustup run 1.89 cargo check -p ruviform-rt --lib --locked` を使用（dev-dep は `--lib` 範囲に影響しない — simd_poc と同じ封じ込め検証形）

## Affected Files

- `crates/ruviform-rt/Cargo.toml` — `[dev-dependencies]` と `[[bench]] harness = false` 追加（`[dependencies]` は不変）
- `crates/ruviform-rt/src/observability_poc.rs` — 新規（`#[doc(hidden)]` PoC scaffolding + `#[cfg(test)]` テスト）
- `crates/ruviform-rt/src/lib.rs` — `#[doc(hidden)] pub mod observability_poc;` 追加（crate doc・公開 re-export は変更しない）
- `crates/ruviform-rt/benches/rt_observability_poc.rs` — 新規（criterion ハーネス）
- `Cargo.lock` — dev-dep 追加に伴う再生成（mise pin の 1.99.0 上で再生成してから `rustup run 1.89 ... --locked` 検証に入る）
- `mise.toml` — `bench-smoke` 対象に `ruviform-rt` 追加 + 記述更新
- `AGENTS.md` — 検証入口対応表の `bench`/`bench-smoke` 行を実態に更新

## Done When

- [ ] `crates/ruviform-rt/Cargo.toml` に criterion dev-dep と `[[bench]] name = "rt_observability_poc", harness = false` が宣言され、`[dependencies]` が不変のまま維持されている
- [ ] `src/observability_poc.rs` が `#[doc(hidden)] pub mod` として露出し、Counter ペア・bridge・方針バリアント・タイムスタンプ helper・`ObsRecord` を含み、`unsafe` 0・pub enum なし・pure std + crate 内依存のみである
- [ ] self-consistency（窓内 alloc 0）・カバレッジ隙間実証・scaffolding 意味論の `#[cfg(test)]` テストが存在し `mise run test` でグリーン
- [ ] `benches/rt_observability_poc.rs` が計測マトリクス全列（counter / rt_bridge / overflow_policy / rt_primitives / timestamp）を実装し、`mise run bench-smoke` でスモーク完走する
- [ ] `mise.toml` の `bench-smoke` が ruviform-math + ruviform-rt を対象とし、`AGENTS.md` の記述が実態と整合している
- [ ] baseline 計測値（A-1）の記録がある

## Verification

- [ ] `mise run check` がグリーン（dev-dep 解決・全ターゲット型チェック・`Cargo.lock` 再生成）
- [ ] `mise run lint` がグリーン（`unsafe_code` ゲート通過・lint 局所化のみ）
- [ ] `mise run test` がグリーン（self-consistency・隙間実証・意味論テスト）
- [ ] `mise exec -- rustup run 1.89 cargo check -p ruviform-rt --lib --locked` がグリーン
- [ ] `mise run bench-smoke` がスモーク完走し、`mise exec -- cargo bench -p ruviform-rt -- counter` の quick-run filter が機能する

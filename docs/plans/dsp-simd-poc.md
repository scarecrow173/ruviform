---
id: "PLAN-034JdOWI3vblJW11kIueY0"
type: "plan"
status: "completed"
title: "DSP/SIMD 比較ベンチマーク PoC 実装"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/src/simd_poc.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/mod.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/inputs.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/equiv.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/fearless_impl.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/pulp_impl.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_poc/multiversion_impl.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_equiv.rs"
      - type: "file"
        path: "crates/ruviform-math/benches/simd_poc.rs"
      - type: "file"
        path: "docs/adr/dsp-simd-backend-adoption.md"
      - type: "file"
        path: "docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl"
      - type: "file"
        path: "docs/impl/exp/dsp-simd-poc-<yyyymmdd>.md"
      - type: "file"
        path: "docs/impl/ir/dsp-simd-poc-benchmark.md"
      - type: "file"
        path: "docs/test-specs/simd-equivalence-matrix-contract.md"
      - type: "file"
        path: "docs/test-specs/bench-tasks-and-ci-smoke.md"
    modified:
      - type: "file"
        path: "crates/ruviform-math/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-math/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "docs/specs/dsp-simd-poc-adr.md"
      - type: "file"
        path: "docs/designs/dsp-simd-poc.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "../specs/dsp-simd-poc-adr.md"
  implemented-by:
    - "../tasks/add-bench-harness-and-dev-deps.md"
    - "../tasks/implement-simd-poc-scalar-kernels.md"
    - "../tasks/implement-simd-poc-candidate-backends.md"
    - "../tasks/add-simd-equivalence-matrix-tests.md"
    - "../tasks/implement-criterion-benchmarks.md"
    - "../tasks/run-simd-bench-and-collect-results.md"
    - "../tasks/write-simd-adoption-adr.md"
    - "../tasks/record-impl-and-sync-docs.md"
    - "../impl/ir/dsp-simd-poc-benchmark.md"
  depends-on:
    - "dev-foundation-bootstrap.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/dsp-simd-poc.md"
    - "../discovery/dsp-simd-poc-issue-19.md"
    - "../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/dsp-simd-poc-adr.md"
    - "../designs/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/simd-equivalence-matrix-contract.md"
    - "../test-specs/bench-tasks-and-ci-smoke.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# DSP/SIMD 比較ベンチマーク PoC 実装 Implementation Plan

## Goal

Issue #19 の PoC として、`crates/ruviform-math` 内にスカラー参照カーネルと 3 候補バックエンド（`fearless_simd` / `pulp` / `multiversion`+autovec）を同一カーネル API で比較するベンチマークハーネスを構築し、dispatch level 強制指定の等価性 matrix テスト・criterion による候補別計測・experiment log への結果記録を経て、採否 ADR（SIMD 二層構造の採否・バックエンド選定・等価基準確定値・AVX-512・denormal 方針・MSRV 判断）までを一連の成果物として出す。

## Scope

単一の実装ストリームとして扱う。ハーネス骨格 → スカラー参照・候補カーネル実装 → 等価性 matrix → criterion ベンチ → 計測・結果収集 → 採否 ADR → impl record・文書整合は同一クリティカルパス上の工程であり、「Issue #19 PoC の採否判断材料一式」という単一のリリース意図を共有するため plan 分割はしない。

範囲外（spec の Out of Scope と一致）: 候補クレートの本番 `[dependencies]` 昇格と `ruviform-math` 公開 API 確定、`ruviform-dsp` クレート新設・実用 DSP カーネル、`std::simd`（nightly 限定）と `wide` の評価、他 OS / アーキでのベンチ比較（Windows x64 が主計測）、denormal 対策の実装そのもの（FTZ/DAZ 設定 — 方針記録までが本 PoC の範囲）。

## Upstream Documents

- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`（status: proposed）。本 plan が実装する spec。成果物・受け入れ基準の正本
- [GitHub Issue #19](https://github.com/scarecrow173/ruviform/issues/19) — 一次ソース（`relations.source`）

## Design Inputs

- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（status: approved）。ファイル配置・カーネル API 抽象形・等価性契約・ベンチ methodology・採否 ADR への入力形式の正本。本 plan のタスク分解・ファイルマップ・検証条件は同書を直接トレースする
- [../adr/dsp-simd-poc.md](../adr/dsp-simd-poc.md) — `ADR-034JcihlNe8WCZVpp9RBCq`（status: proposed）。評価アプローチの決定（ハーネス配置 = ruviform-math dev-deps、比較対象 = 3 系統、ベンチ基盤 = criterion）。設計書には本 ADR からの deviation（`pub(crate)` 配置が dev-dep 制約で成立しないため `#[doc(hidden)] pub` + `tests/` 共有 mod へ修正）が明記されており、本 plan は設計書側の配置を正として採用する
- [../discovery/dsp-simd-poc-issue-19.md](../discovery/dsp-simd-poc-issue-19.md) — 候補比較・MSRV 調査値・Open Questions の canonical 記録（参照）
- [dev-foundation-bootstrap.md](dev-foundation-bootstrap.md) — `depends-on`。mise 検証タスク・ワークスペース構成を検証基盤として利用する（確立済み）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-math/Cargo.toml` | dev-dependencies（`criterion` 0.8.2 default-features=false・`fearless_simd` 1.1 + `force_support_fallback`・`fearless_simd_macros` 0.1・`pulp` 0.22 + `x86-v4`,`macro`・`multiversion` 0.9・`approx` 0.5）と `[[bench]] harness = false` 宣言 | `[dependencies]` は空のまま維持。feature 選定根拠は設計書 dev-dependencies 節どおり |
| `crates/ruviform-math/src/lib.rs` | `#[doc(hidden)] pub mod simd_poc;` の追加 | crate doc は変更しない（モジュール側で実験 API 宣言） |
| `crates/ruviform-math/src/simd_poc.rs` | スカラー参照カーネル 8 本（add / scale / clamp_unit / fade / dot / fir / biquad / sine）・共通型（`BiquadCoeffs` / `SineCoeffs` / `SineState` / `FIR_TAPS=64`）・deterministic 入力生成（xorshift64*）・denormal 生成 | pure std のみ・候補クレートを import しない。モジュール doc に「実験 API・RT 契約対象外・入力生成 helper は allocate する」旨を明記 |
| `crates/ruviform-math/tests/simd_poc/mod.rs` | 共有サポート mod の再エクスポート + matrix ランナー | `tests/` 直下以外はテストターゲットにならないことを利用 |
| `crates/ruviform-math/tests/simd_poc/inputs.rs` | 境界入力生成（normal / edge / denormal・端数長・非アラインオフセット） | denormal は `f32::from_bits(0x0040_0000)` や `f32::MIN_POSITIVE * 0.5` で生成（素朴な係数乗算は +0.0 に丸まるため不採用） |
| `crates/ruviform-math/tests/simd_poc/equiv.rs` | 等価性判定（`to_bits()` 比較・`approx::UlpsEq`・失敗報告フォーマット） | 失敗時に `(kernel, impl, level, input, len, offset, index, expected_bits, actual_bits, ulps)` を出力 |
| `crates/ruviform-math/tests/simd_poc/fearless_impl.rs` | `fearless_simd` カーネル群 + `Fallback::new()` / `Level::new().as_*()` 強制ラッパ + `dispatch!` 経路 | x86 限定 `as_*` API は `#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]` ゲート。`assume_supported`（unsafe）は不使用。clamp は `min_precise_*` / `max_precise_*` 系 |
| `crates/ruviform-math/tests/simd_poc/pulp_impl.rs` | `pulp` カーネル群（`WithSimd` 直接 impl 基本形）+ `Scalar::new()` / `x86::V2/V3/V4::try_new()` / `Arch::new().dispatch` ラッパ | x86 限定 `V2`/`V3`/`V4` は cfg ゲート。`new_unchecked`（unsafe）は不使用。NaN 伝播は fearless 側と揃える方針 |
| `crates/ruviform-math/tests/simd_poc/multiversion_impl.rs` | `#[multiversion(targets("x86_64+avx2+fma","x86_64+sse4.2","aarch64+neon"))]` カーネル群 | autovec スカラー本文は `src::simd_poc` と同一ループ。個別 clone 強制 API は存在しない（matrix 制限として記録） |
| `crates/ruviform-math/tests/simd_equiv.rs` | 等価性テストターゲット本体（カーネル × 実装列 × 入力 × 長さ × オフセットの matrix） | `mise run test` で実行。構成不能 level は panic せず skip 記録 |
| `crates/ruviform-math/benches/simd_poc.rs` | criterion ベンチ本体 | `#[allow(dead_code)] #[path = "../tests/simd_poc/mod.rs"] mod simd_poc_support;` で共有 mod を include |
| `Cargo.lock` | dev-dep 追加に伴う更新 | mise pin の 1.99.0 上（`mise run check`）で再生成してから `rustup run 1.85 ... --locked` 検証に入る |
| `mise.toml` | `bench` タスク記述の実態更新（no-op 記述の撤廃）+ スモーク用 `bench-smoke` タスク追加 | `bench` は全量計測入口として維持（spec AC） |
| `.github/workflows/ci.yml` | `bench` ジョブをスモーク化（`mise run bench-smoke`） | 全量計測 ≈400–560 ケースはローカル手動実行に分離 |
| `AGENTS.md` | 検証入口対応表の `bench` 行・CI 記述の実態更新 | 「実質 no-op」記述を更新 |
| `README.md` | `bench` 関連記述の実態整合確認・必要箇所の更新 | 現状はタスク一覧言及のみで大きな矛盾はない見込み（実装時に確認） |
| `docs/adr/dsp-simd-backend-adoption.md` | 採否 ADR（最終成果物） | ファイル名は adr-doc skill での生成時に確定可。結論: 二層構造採否・バックエンド選定・等価基準確定値・AVX-512・denormal 方針・MSRV |
| `docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl` | 計測結果（1 行 = 1 計測点） | `estimates.json` 集約物。コミット対象 |
| `docs/impl/exp/dsp-simd-poc-<yyyymmdd>.md` | experiment log 文書（環境メタデータ・判定表・観察メモ） | impl record から参照される |
| `docs/impl/ir/dsp-simd-poc-benchmark.md` | impl record（実施内容・検証実績・experiment log 参照） | impl-doc skill で生成 |
| `docs/specs/dsp-simd-poc-adr.md` | `derived-by` に本 plan（`../plans/dsp-simd-poc.md`）を追記（relation バックリンク整合）+ AC checkbox の実績反映 | Phase H の文書整合タスクで実施 |
| `docs/designs/dsp-simd-poc.md` | `derived-by` に本 plan を追記（relation バックリンク整合） | approved design への relation 追記のみ・本文変更なし |
| `docs/adr/README.md`・`docs/impl/ir/README.md`・`docs/specs/README.md`・`docs/plans/README.md` | doc-suite 生成索引の再生成（採否 ADR・impl record 追加に伴う索引更新） | 各 skill の index 更新手順で再生成。`changes.generated` 分類 |
| `docs/test-specs/simd-equivalence-matrix-contract.md`・`docs/test-specs/bench-tasks-and-ci-smoke.md` | 本 plan の検証意図を正本化する test-spec（等価性 matrix 契約 / ベンチ・CI スモーク運用） | `verified-by` relation で参照する検証意図の正本。planning 段階で作成済み |

ファイル一覧に未確定要素はない。採否 ADR・experiment log・impl record のファイル名は生成時に確定するが、上記の計画名を既定とする。

## Implementation Sequence

### Phase A: ベンチハーネス骨格と dev-dependencies（Task T1）

- [x] A-1. dev-dep 導入前のコンパイルコスト baseline を記録: `mise run check`（`cargo check --workspace --all-targets`）の所要時間を計測し、Phase G のコンパイルコスト前後比較の基準値として記録する（dev-dep 導入後では baseline を取れないため本 Phase の先頭で実施）
- [x] A-2. `crates/ruviform-math/Cargo.toml` に `[dev-dependencies]`（`criterion = { version = "0.8.2", default-features = false }`、`fearless_simd = { version = "1.1", features = ["force_support_fallback"] }`、`fearless_simd_macros = "0.1"`、`pulp = { version = "0.22", features = ["x86-v4", "macro"] }`、`multiversion = "0.9"`、`approx = "0.5"`）と `[[bench]] name = "simd_poc", harness = false` を追加（`[dependencies]` は空のまま）。計測対象 dev-dep のバージョン方針: caret 指定とし、追跡済み `Cargo.lock` を再現性の錨とする（`=x.y.z` exact pin は採用しない — workspace は `Cargo.lock` をコミット管理しており lockfile が解決版を固定するため）。各 dev-dep の追加理由・feature 選定根拠は Cargo.toml のインラインコメント（設計書 dev-dependencies 節のコメントを転記）と PR 説明文に記録する
- [x] A-3. `benches/simd_poc.rs` に最小 criterion 骨格（`criterion_group!` / `criterion_main!` + `std::hint::black_box` を使う placeholder ベンチ 1 本）を作成
- [x] A-4. `mise run check` で dev-dep 解決・`Cargo.lock` 再生成・全ターゲット型チェックを確認
- [x] A-5. `mise exec -- rustup toolchain install 1.85` で 1.85 ツールチェーンを調達し、`mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーンであることを確認（MSRV 検証手順の確立。`cargo +1.85` は rustup proxy 前提のため `rustup run` 経由の呼出形に統一）
- [x] A-6. `mise.toml` に `bench-smoke` タスク（`cargo bench -p ruviform-math -- --test`）を追加し、`.github/workflows/ci.yml` の `bench` ジョブを `mise run bench-smoke` へ変更（CI スモーク分離方針）

### Phase B: スカラー参照カーネルと共通型（Task T2）

- [x] B-1. `src/simd_poc.rs` を新規作成し、共通型（`BiquadCoeffs` / `SineCoeffs` / `SineState` / `pub const FIR_TAPS: usize = 64`）とカーネルシグネチャ抽象形（設計書 Data and Control Flow 節どおり: `add` / `scale` / `clamp_unit` / `fade` / `dot` / `fir` / `biquad` / `sine`）を定義。`simd_poc` モジュールに pub enum を置かない（`KernelId` 等の識別子は `&'static str` または構造体定数で代替 — 設計書 Non-Functional Constraints の公開 enum 規約適用）
- [x] B-2. 8 カーネルのスカラー参照実装を pure std ループで記述（autovec を意図的に抑制しない・`#[inline]` 不指定・`unsafe` 0・長さ不一致は `debug_assert`）。clippy pedantic 系 lint（`cast_precision_loss` / `suboptimal_flops` / `imprecise_flops` / `wildcard_imports` 等）との衝突は、意図的抑制を `#[expect(...)]`（未対応なら `#[allow(...)]`）＋理由コメントで対象 item に局所化する（ファイル・モジュール単位の lint allow は置かない）
- [x] B-3. deterministic 入力生成（xorshift64*・固定 seed・`[-1.0, 1.0)` 一様乱数）と denormal 値生成（`f32::from_bits(0x0040_0000)` / `f32::MIN_POSITIVE * 0.5`）を実装
- [x] B-4. `src/lib.rs` に `#[doc(hidden)] pub mod simd_poc;` を追加し、`simd_poc` のモジュール doc に「実験 API・本番 RT 契約の対象外・入力生成 helper は allocate する」旨を明記
- [x] B-5. `mise run check` / `mise run lint` / `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーンであることを確認

### Phase C: 候補バックエンド実装と共有サポート（Task T3）

- [x] C-1. `tests/simd_poc/mod.rs` を作成（共有サポートの再エクスポート + matrix ランナー。`tests/simd_poc/` は cargo のテストターゲットにならない構造を利用）
- [x] C-2. `tests/simd_poc/inputs.rs` に入力生成を実装（`normal` / `edge` = denormal・±0・NaN・±Inf 混合 / `denormal` 混入。長さ `{7,15,16,17,31,33}` + `{100,255,1023,1024,4096}`、先頭オフセット `{0,1,3}`。`biquad` は `channels = 8` 固定で frames 側を同系列に変える — 設計書 methodology 節どおり）
- [x] C-3. `tests/simd_poc/equiv.rs` に等価性判定を実装（bit 完全一致 = `to_bits()` 比較、ulp 上限 = `approx::UlpsEq`、失敗報告に `(kernel, impl, level, input, len, offset, index, expected_bits, actual_bits, ulps)` を含める）
- [x] C-4. `tests/simd_poc/fearless_impl.rs` を実装（`#[simd]` 付き portable 層カーネル + `dispatch!` 経路 + `Fallback::new()` / `Level::new().as_sse2()` 等の `Option` ベース強制ラッパ。x86 限定 API 参照は `#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]` でゲート。clamp は `*_precise` 系で NaN 伝播を揃える）
- [x] C-5. `tests/simd_poc/pulp_impl.rs` を実装（`WithSimd` trait 直接 impl を基本形とするカーネル + `pulp::Scalar::new()` / `pulp::x86::V2,V3,V4::try_new()` の `Option` ベース強制 + `Arch::new().dispatch` 経路。`V2`/`V3`/`V4` 参照は cfg ゲート）
- [x] C-6. `tests/simd_poc/multiversion_impl.rs` を実装（`#[multiversion(targets("x86_64+avx2+fma","x86_64+sse4.2","aarch64+neon"))]`。本文はスカラー参照と同一ループ）
- [x] C-7. `mise run check`（`--all-targets` で dev ターゲット込みコンパイル）と `mise run lint` がグリーンであることを確認（`unsafe` 0・候補 API の unsafe 要求経路不使用・公開 API に候補クレートの型が露出しないことをレビュー）。pedantic lint 衝突（`cast_precision_loss` / `suboptimal_flops` / `imprecise_flops` / `wildcard_imports`）と共有 mod の dead_code は `#[expect]` / `#[allow]` + 理由コメントで対象 item に局所化する（B-2 と同方針）

### Phase D: 等価性 matrix テスト（Task T4）

- [x] D-1. `tests/simd_equiv.rs` を作成し、カーネル × 実装列（`scalar` / `fearless_fallback` / `fearless_neon`（aarch64 cfg ゲート列）/ `fearless_sse2` / `fearless_sse4_2` / `fearless_avx2` / `fearless_avx512` / `fearless_dispatched` / `pulp_scalar` / `pulp_v2` / `pulp_v3` / `pulp_v4` / `pulp_dispatched` / `multiversion`）× 入力セット × 長さ × オフセットの matrix をカーネルごと 1 テスト関数で集約検証（x86 限定列は cfg ゲート。aarch64 では `fearless_neon` と portable 列のみ — 設計書 matrix 節どおり）
- [x] D-2. 構成不能な level（トークン `None`）は panic ではなく skip として記録する振る舞いを実装。カーネル契約上 NaN/Inf を扱わない edge セル（`dot` の有限値契約等）は除外条件をコードコメントで明記
- [x] D-3. `mise run test` で matrix がグリーンであることを確認し、観測 max ulp を集計（観測値 > 8 は実装バグか契約不適合として調査し、数値を合わせ込む事後緩和はしない。`biquad` で固定上限超過が正当と判明した場合のみ設計書の代替契約形へ切替）

### Phase E: criterion ベンチ実装（Task T5）

- [x] E-1. `benches/simd_poc.rs` を実装: `#[allow(dead_code)] #[path = "../tests/simd_poc/mod.rs"] mod simd_poc_support;` で共有 mod を include し、カーネルごと `BenchmarkGroup` + `group.throughput(Throughput::Elements(len as u64))` + `BenchmarkId::new(<impl>, "{input}/{len}")` を構成
- [x] E-2. ベンチ matrix を実装（サイズ `64/256/1024/4096` + 端数 `100/1000/1023`、入力 `normal` + `denormal` — `add`/`scale`/`clamp`/`fade` は `normal` のみ、`biquad` は `channels = 8` 固定で frames を同系列、計測識別子は設計書 methodology 節の一覧どおり `fearless_sse2`/`pulp_v2` を除外）
- [x] E-3. `biquad` / `sine` は `iter_batched` / `iter_with_setup` で反復ごとに状態を初期化（normal 側は非 denormal の決定的初期状態、denormal-cliff 観測側は denormal 状態を維持・再注入する setup）
- [x] E-4. PoC 用の `BenchmarkGroup` 時間設定（`warm_up_time` / `measurement_time` / `sample_size` 削減 — 全量 ≈400–560 ケースを criterion 既定の 1h 超から ≈20–30 分以内へ短縮する目安）と quick-run filter 規約（`mise exec -- cargo bench -p ruviform-math -- <filter>`）を実装。run 冒頭に OS・CPU 型名・コア数・rustc 版・criterion 版・`Level::new()` / `pulp::Arch::new()` 検出結果を標準出力する
- [x] E-5. `mise run bench-smoke`（`cargo bench -p ruviform-math -- --test`、スモーク）と `mise exec -- cargo bench -p ruviform-math -- dot`（quick-run 例）が実行できることを確認

### Phase F: ベンチ実行と結果収集（Task T6）

- [x] F-1. Windows x64 主計測環境で全量ベンチを実行（`mise run bench` = `cargo bench --workspace`）。同一マシンで 2–3 run 実施して分散を記録（省電力モード・負荷状況を環境メタデータに記録）。criterion の `estimates.json` は run ごとに上書きされるため「run 実行 → `estimates.json` 集約（F-2）→ 次 run」の順序で実施する。削減設定後の全量予想所要時間（E-4 の目安 ≈20–30 分）と実測所要時間を記録する
- [x] F-2. `target/criterion/**/new/estimates.json` から `median` / `mean` を抽出する軽量集約（node ワンショット or 手集計）を行い、`docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl`（1 行 = 1 計測点）を生成してコミット。jsonl は LF 改行・末尾改行ありで生成する（prek の end-of-file-fixer / mixed-line-ending 系 hook 適合 — コミット対象のため）
- [x] F-3. experiment log 文書 `docs/impl/exp/dsp-simd-poc-<yyyymmdd>.md` を作成（環境メタデータ・カーネル × impl のスカラー比判定表・denormal 崖観察・判定目安との照合結果を記録）

### Phase G: 採否 ADR（Task T7）

- [x] G-1. 採否基準の補助データを収集: コンパイルコスト（A-1 の baseline に対する `mise run check` 所要時間の dev-dep 前後比較 + `mise exec -- cargo check --timings --workspace --all-targets` の候補クレート分抽出）、unsafe 範囲（`mise run lint` の `unsafe_code` lint 通過を正の検証とし、`assume_supported` / `new_unchecked` / `std::arch` を使わずに済んだことを確認。`grep -rn "unsafe"` はコメント・doc 言及を拾う偽陽性があるため採用しない）、MSRV 適合（候補各クレートの `rust-version` 列挙 + pulp `x86-v4` の推移的下限 ≈1.89 の確認 + `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` 実績）、保守状況（discovery 記録値の更新確認）
- [x] G-2. `docs/adr/` に採否 ADR を作成（adr-doc skill で scaffold。ファイル名既定: `dsp-simd-backend-adoption.md`）。設計ベースラインの採否基準に沿い、SIMD 二層構造の採否・バックエンド選定・等価基準確定値（実測 max ulp に余裕を持たせた値）・AVX-512 の扱い・denormal 方針・MSRV 判断を記録（不採用・条件付き採用も正当な結論。先に決めた結論へ数値を寄せない）
- [x] G-3. ADR の front matter relations を設定（`source` = Issue #19、`derives-from` = 本 plan / design / spec / approach ADR）し、`node .agents/skills/doc-status/scripts/audit_docs.js --type adr --json` がクリーンであることを確認

### Phase H: impl record と文書整合（Task T8）

- [x] H-1. `docs/impl/ir/` に impl record を作成（実施内容・検証実績・experiment log への参照・`rt_` テスト未追加の判断記録を含む）
- [x] H-2. `mise.toml` の `bench` タスク記述を実態に更新（no-op 記述の撤廃）し、`AGENTS.md` の検証入口対応表 `bench` 行と CI 記述（`bench-smoke`）を更新、`README.md` の bench 関連記述を確認・必要箇所を更新
- [x] H-3. 上流文書バックリンクと生成索引を整合: `docs/specs/dsp-simd-poc-adr.md` と `docs/designs/dsp-simd-poc.md` の `derived-by` に本 plan（`../plans/dsp-simd-poc.md`）を追記し、`docs/adr/README.md`・`docs/impl/ir/README.md`・`docs/specs/README.md`・`docs/plans/README.md`（生成索引）を該当 skill の index 更新手順で再生成する
- [ ] H-4. 最終検証スイートを実行: `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench-smoke`、`mise exec -- prek run --all-files`、`mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked`、`node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` がすべてグリーン。spec AC-6 の `mise run bench` グリーン要求は Phase F の全量計測実績（experiment log）で充足済みとし、ここでは `bench-smoke` でハーネス健全性を再確認する（全量再実行は実行時間予算上行わない）。併せて `git diff --name-only` で変更ファイル群が front matter `changes` 宣言のスコープ内であることを確認（検証スイートの最終実行はオーケストレータの最終検証ステップに委譲 — 本チェックはその完了をもって確定）

## Dependencies and Risks

### Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| A（dev-deps + ハーネス骨格 + CI スモーク） | — | C, E |
| B（スカラー参照 + 共通型 + 入力生成） | — | C, D |
| C（候補実装 + 共有サポート） | A（dev-dep 解決）, B（共通型・参照 API） | D, E |
| D（等価性 matrix） | B, C | G |
| E（criterion ベンチ） | A, C | F |
| F（計測・結果収集） | E | G |
| G（採否 ADR） | D（等価性結果）, F（計測結果） | H |
| H（impl record + 文書整合） | G + 全 Phase の検証完了 | — |

- クリティカルパス: A/C → D/E → F → G → H（B は C の前提として早期に必要）
- 並列可能: A と B（B は pure std のため dev-dep 非依存）、C 内の C-4 / C-5 / C-6（候補別ファイルは独立）、D と E（共有 mod 完成後はテスト側と bench 側で並行実装可能）
- 外部前提: crates.io 上の `fearless_simd` 1.1 / `fearless_simd_macros` 0.1 / `pulp` 0.22 / `multiversion` 0.9 / `criterion` 0.8.2 / `approx` 0.5 の入手可能性、rustup 経由の 1.85 ツールチェーン調達、Windows x64 計測環境

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | データ損失 | — | なし | 変更は新規ファイル追加と設定追記のみで不可逆操作なし。rollback は git revert / ファイル除去で完結 |
| 2 | 破壊的変更（`#[doc(hidden)] pub` で実質公開面増加） | 中 | 低 | `publish = false` の内部クレートで外部契約化しない。暫定措置をモジュール doc・採否 ADR に明記し、後続 Issue で正式 API 化または除去 |
| 3 | 外部依存の入手可否（候補クレート 4 本 + criterion + approx） | 低 | 中 | バージョン pin で導入。候補クレートの MSRV 超過（fearless_simd 1.89 / multiversion 1.86 / criterion 1.86 / pulp 未宣言≈1.89 推移的）は既知。MSRV 検証は `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` に限定し、pin or MSRV 引上げの判断は採否 ADR へ送る |
| 4 | dev-dep の MSRV 超過で `rustup run 1.85` 系検証が dev ターゲット込みで壊れる | 高 | 低 | `--lib --locked` 検証手順に限定（設計書 Non-Functional Constraints どおり）。`Cargo.lock` は先に 1.99.0 側（`mise run check`）で再生成 |
| 5 | Windows x64 計測ノイズ（turbo/省電力・負荷・タイマ分解能） | 高 | 中 | criterion の warmup・統計処理・同一 run 内相対比較で緩和。2–3 run の分散と省電力モード・負荷状況を環境メタデータに記録 |
| 6 | criterion 0.8 の `alloca` 非対応ターゲット問題 | 低 | 低 | `0.8.2` 以上を要求（同バージョンで修正済み）。Windows x64 / Linux / macOS は対応ターゲット |
| 7 | `fearless_simd` の新しさ・`min`/`max`/`mul_add` の implementation-defined 意味 | 中 | 中 | `*_precise` 系・NaN 契約・カーネル内コメントで明示。等価性 matrix が検出網として機能 |
| 8 | `pulp` の正式 MSRV 未宣言 | 中 | 低 | 実測（`mise exec -- rustup run 1.85 cargo check ...` / `rustup run 1.86 ...` での check）で記録し採否 ADR の判断材料にする |
| 9 | `multiversion` が個別 clone 強制を API 上提供しない | 確定 | 低 | matrix 設計で「dispatched のみ」列として扱い、等価性検証網羅性の構造的差として採否 ADR に記録 |
| 10 | AVX-512 環境が手元 / CI にない | 高 | 低 | `Option` ベース強制で skip 記録。採否 ADR で「AVX-512 は実測不在」判断材料を明示 |
| 11 | `#[path]` 共有 mod の非標準性（二重コンパイル・lint 通過性） | 中 | 中 | bench 側 `mod` 宣言への `#[allow(dead_code)]` を第一回避策（mod 属性は配下に波及）。`mise run lint` で実際にゲート通過を確認し、通らなければ独立 PoC support クレート案へ切替（Deferred Concerns に記録） |
| 12 | 全量ベンチの実行時間（≈400–560 ケース・既定設定で 1h 超） | 高 | 中 | PoC 用 `BenchmarkGroup` 時間設定削減・quick-run filter 規約・CI スモーク / ローカル全量の分離で対処 |
| 13 | denormal 収束で `biquad` / `fir` のベンチが性能崖を踏む | 中 | 低 | denormal バリアントを別途計測して崖を「観測対象」にする。FTZ/DAZ 実装はしない（採否 ADR の denormal 方針節へ） |
| 14 | 等価性 matrix で観測 ulp が初期上限 8 を超過 | 中 | 中 | 実装バグか契約不適合として先に調査。`biquad` は設計書の代替契約形（先頭 N フレーム ulp + 以降相対誤差 / フレーム長依存上限）へ切替可。事後緩和はしない |

## Task Breakdown

| Task | 内容 | 対応 Phase | verified-by |
| --- | --- | --- | --- |
| T1 | dev-dependencies 追加 + bench ハーネス骨格（`[[bench]] harness=false`・criterion 骨格・Cargo.lock 再生成・MSRV 検証手順確立・CI スモーク分離）。コミット分割: ①dev-dep + harness 骨格、②`bench-smoke` タスク + ci.yml スモーク分離 | A | [bench-tasks-and-ci-smoke](../test-specs/bench-tasks-and-ci-smoke.md) |
| T2 | `src/simd_poc.rs`: スカラー参照カーネル 8 本 + 共通型 + deterministic 入力生成 + lib.rs への `#[doc(hidden)]` モジュール配線 | B | — |
| T3 | `tests/simd_poc/` 共有モジュール: 境界入力生成・等価性判定 helper・候補 3 実装（fearless_simd / pulp / multiversion+autovec）・cfg ゲート方針適用 | C | — |
| T4 | 等価性テスト: `tests/simd_equiv.rs` でカーネル × level × 入力 matrix を `mise run test` 経由で検証 | D | [simd-equivalence-matrix-contract](../test-specs/simd-equivalence-matrix-contract.md) |
| T5 | criterion ベンチ実装: `BenchmarkId` 規約・状態持ちカーネルの `iter_batched`・時間予算設定・環境情報出力 | E | [bench-tasks-and-ci-smoke](../test-specs/bench-tasks-and-ci-smoke.md) |
| T6 | ベンチ実行・結果収集: 全量計測（Windows x64）・`estimates.json` 集約 → experiment log（jsonl + 文書） | F | [bench-tasks-and-ci-smoke](../test-specs/bench-tasks-and-ci-smoke.md) |
| T7 | 採否 ADR 作成: ベンチ・等価性・補助データ証拠から二層構造採否・バックエンド選定・MSRV・等価基準・denormal/AVX-512 を結論 | G | — |
| T8 | impl record + 文書整合: `docs/impl/ir/` 記録・README/AGENTS/mise.toml bench 記述の実態更新・上流文書バックリンク・最終検証。コミット分割: ①impl record、②文書同期（バックリンク・索引・記述整合） | H | [bench-tasks-and-ci-smoke](../test-specs/bench-tasks-and-ci-smoke.md) |

タスク doc（`docs/tasks/`）への具体化は dispatch 時に必要に応じて行う。本 plan の Phase 分解・検証条件を実装の正本とする。コミット粒度は上表の分割指定を基本とし、それ以外は 1 タスク 1 コミットを目安とする。

## Verification

| ID | 対象 | 検証コマンド / 確認内容 | 合格条件 |
| --- | --- | --- | --- |
| V-1 | Phase A | `mise run check`（`cargo check --workspace --all-targets`）+ `Cargo.toml` 目視 + `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` | dev-deps 宣言・`[dependencies]` 空・`harness = false`・`Cargo.lock` 再生成済み・MSRV check グリーン・baseline 計測値の記録あり |
| V-2 | Phase B | `mise run check` / `mise run lint` / `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` | グリーン。`src/simd_poc.rs` が pure std（候補クレート import なし）・`unsafe` 0・pub enum なし |
| V-3 | Phase C | `mise run check` / `mise run lint` + 公開 API レビュー | グリーン。dev ターゲットが全候補依存でコンパイル。PoC コードの `unsafe` 0（`mise run lint` の `unsafe_code` lint ゲート通過で検証 — `grep` 全文検索は偽陽性のため採用しない）。候補クレートの型が `#[doc(hidden)]` 含む公開 API に露出しない |
| V-4 | Phase D | `mise run test`（`cargo test --workspace`） | `tests/simd_equiv.rs` の matrix がグリーン。denormal / ±0 / NaN / Inf / 非アライン先頭 / 端数長入力を含む。構成不能 level は skip 記録（panic しない）。観測 max ulp が集計される |
| V-5 | Phase E | `mise run bench-smoke`（= `cargo bench -p ruviform-math -- --test`、スモーク）+ `mise exec -- cargo bench -p ruviform-math -- dot`（quick-run 例） | スモーク完走・quick-run filter が機能。`target/criterion/<group>/<impl>/<input>/<len>/new/estimates.json` の出力階層が `BenchmarkId` 規約どおり生成される |
| V-6 | Phase F | `mise run bench`（`cargo bench --workspace`、全量）+ experiment log レビュー | Windows x64 で計測完了。`docs/impl/exp/` に jsonl（LF・末尾改行）+ 環境メタデータつき experiment log 文書が存在。2–3 run の分散が記録される |
| V-7 | Phase G | `node .agents/skills/doc-status/scripts/audit_docs.js --type adr --json` + ADR 内容レビュー | 採否 ADR が `docs/adr/` に存在し、二層構造採否・バックエンド選定・MSRV 判断・等価基準確定値・denormal / AVX-512 方針を記録。audit クリーン |
| V-8 | Phase H | `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench-smoke` + `mise exec -- prek run --all-files` + `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` + `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` + `git diff --name-only <base>...HEAD`（変更ファイル群が `changes` 宣言スコープ内か照合） | すべてグリーン。`mise.toml` / `AGENTS.md` / `README.md` の bench 記述が実態と整合。impl record が experiment log を参照。spec AC-6 の `mise run bench` グリーンは Phase F の全量計測実績で充足（`bench-smoke` はハーネス健全性の再確認）。変更ファイルが `changes` 宣言スコープ内 |

## Rollback Strategy

| 対象 | Trigger | Procedure | Verification |
| --- | --- | --- | --- |
| PoC 変更全体 | 採否判断に関わらず PoC 実装を取り下げる場合 | PoC コミット群の `git revert`。または新規ファイル（`src/simd_poc.rs`・`tests/simd_poc/`・`tests/simd_equiv.rs`・`benches/`）の削除 + `Cargo.toml` / `lib.rs` / `mise.toml` / `ci.yml` / `AGENTS.md` / `README.md` の編集差分を `git checkout --` で戻し、`mise exec -- cargo generate-lockfile` で `Cargo.lock` を再生成 | `mise run check` / `lint` / `test` がグリーン、`Cargo.toml` に dev-deps・`[[bench]]` が残っていないこと |
| 部分的戻し（特定候補クレートのみ除去） | 候補クレートの導入自体が問題になった場合（解決不能・MSRV 破壊等） | 当該 dev-dep 行と対応する `tests/simd_poc/*_impl.rs`・matrix 列・ベンチ識別子を一組で除去し `Cargo.lock` 再生成 | `mise run check` / `lint` / `test` / `bench-smoke` グリーン。matrix が残存候補で完結すること |
| ドキュメントのみの戻し | ADR / experiment log / impl record の誤り | 文書の `superseded` 化または修正コミット（文書は履歴として残す運用のため削除しない） | `audit_docs.js --type all --json` クリーン |

## Open Questions / Gaps

- 上流 status: spec `SPEC-034Jci19siJZexLs53Xh5d` と approach ADR `ADR-034JcihlNe8WCZVpp9RBCq` はともに `proposed`（design は `approved`）。plan 作成のゲートは design approved で満たすが、spec/ADR の status 確定は実装着手前にレビュー対象とする
- `approx` vs `float_eq`: design は「実装時に API 使い勝手で 1 つに絞る」とし、本 plan は `approx` 0.5 を第一選択として pin。切替時は dev-dep のみ変更
- `pulp` の実効 MSRV: 未宣言のため `x86-v4` feature 有効時の推移的下限（bytemuck `avx512_simd` 経由 ≈1.89）を含め実測で記録（Phase G の採否 ADR 入力）
- `fearless_simd` の `*_precise` 系 API の level 別 availability: 強制 level 実装時に API 確認が必要（`Fallback` 等で precise 系が揃うか）
- `biquad` 等価性契約: 固定 ulp ≤ 8 で正当な超過が出た場合のみ、設計書の代替契約形へ切替（切替判断は実測に基づき採否 ADR に記録）
- `#[path]` 共有 mod が `mise run lint`（`-D warnings`）を通らない場合の fallback: 独立 PoC support クレート案（workspace `members` 一時追加）— 設計書 Deferred Concerns と同じ据え置き
- bench 実行環境の CPU（AVX-512 搭載可否・コア数）: 計測時に環境メタデータとして記録。AVX-512 非搭載なら該当列は skip 記録のみ

## Self-Review

- [x] spec の受け入れ基準 7 件（ベンチハーネス存在・`mise run bench` 計測可能・等価性契約 + matrix グリーン・採否 ADR 存在・experiment log・mise/prek グリーン・文書整合）が Phase A–H と V-1〜V-8 に対応している
- [x] design の決定事項（2 層配置・dev-dep/feature 選定・`profile.bench` 不追加・カーネル 8 本と等価性契約・matrix 軸・cfg ゲート・ベンチ methodology・experiment log 形式・採否 ADR 入力形式）がタスク・ファイルマップ・検証条件に反映されている
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は Deferred 性質を Open Questions / Gaps に記録し、各項目に判断先を明記）
- [x] ファイルパス・relation・status・用語が spec / design / approach ADR と整合している（`simd_poc` モジュール名・dev-dep バージョン・等価基準値・検証コマンドが設計書どおり）

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation
- Independent work streams: Phase A と B は並列可能。Phase C 内の候補別実装（C-4 / C-5 / C-6）は独立ファイルのため並列可能。共有 mod 完成後は Phase D（等価性テスト）と Phase E（ベンチ）が並行可能。Phase F 以降は直列（計測 → ADR → 記録）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー重点: (1) spec / approach ADR が `proposed` のまま実装に入る点（plan は design approved を根拠に作成。spec/ADR の status 扱いを判定してほしい）、(2) `bench-smoke` mise タスク追加という CI スモーク分離の実現方式（ci.yml が `mise run bench` でなく `mise run bench-smoke` を呼ぶ形。cargo 直接呼出しとのトレードオフで mise 入口原則を優先した）、(3) dev-dep feature 選定（`force_support_fallback` / `x86-v4`+`macro` / criterion `default-features = false`）の妥当性
- 明示的に残した前提: 採否 ADR・experiment log・impl record のファイル名は生成時に確定可（File Map の計画名を既定）、タスク doc は dispatch 時に必要に応じて生成（plan の Phase 分解が正本）
- 上流参照: spec `SPEC-034Jci19siJZexLs53Xh5d`（受け入れ基準の正本）、design `DESIGN-034JcqP2KiWsdes88fgvtt`（配置・契約・methodology の正本）、approach ADR `ADR-034JcihlNe8WCZVpp9RBCq`（評価アプローチ決定。設計書に deviation 明記あり）、discovery（調査値の正本）、Issue #19・親 Issue #1

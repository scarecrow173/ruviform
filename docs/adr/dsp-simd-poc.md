---
id: "ADR-034JcihlNe8WCZVpp9RBCq"
type: "adr"
status: "proposed"
title: "DSP/SIMD PoC 評価アプローチ"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/dsp-simd-poc-issue-19.md"
  derived-by:
    - "dsp-simd-backend-adoption.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# DSP/SIMD PoC 評価アプローチ

## Context and Problem Statement

Issue #19 は、設計ベースライン「DSP／SIMD 設計（A.4）」節で「要 PoC」と分類された論点 — SIMD 二層構造（スカラー参照実装 + ランタイム分岐 + フォールバック）の採否、`fearless_simd` / `pulp` / `multiversion` 系バックエンドの選定、数値等価基準の具体値 — を比較ベンチマークで解消し、採否を ADR 化することを要求する。discovery 調査（[dsp-simd-poc-issue-19.md](../discovery/dsp-simd-poc-issue-19.md)）でエコシステムの候補・確定制約・仮説は整理済みだが、「どこに・何を・どう測るか」という評価方法そのものは未決定である。評価方法の選択は PoC コードの配置・依存解決・等価性検証の自動化可能性・採否 ADR が得る判断材料の品質を左右するため、実装着手前に ADR として確定する。本 ADR が決めるのは評価アプローチのみであり、SIMD バックエンドおよび二層構造の最終採否は、ベンチマーク結果を受けた後続の採否 ADR が担う。

## Decision Drivers

* 公開 API 非露出: SIMD バックエンドクレートの型・API を Ruviform 公開 API に露出しない（設計ベースライン A.4 および dispatch 層を `ruviform-math` に集約する構造仮説と整合）。dev-dependencies 化により候補クレートの型を lib API から構造的に隔離できるか
* dev-dep の波及範囲: `resolver = "2"` により dev-dep の feature unify は test/bench ビルド時のみで本番ビルドに波及しない（discovery 確認済み）。この性質を活かせる配置であること
* 等価性 matrix の自動化可能性: 「スカラー参照 vs 各 dispatch level」の matrix を、ランタイム自動検出ではなく dispatch level 明示指定で強制実行できること（`fearless_simd` は `Level` を明示構築可能、`pulp` は `Arch`、`multiversion` は target feature 指定）。自動検出のみに依存すると CI 実行環境の ISA で検証対象が変わり、等価性検証が非決定的になる
* MSRV 1.85 契約との整合: ワークスペース確定値 `rust-version = "1.85"` に対し、主要候補の MSRV が上回る（`fearless_simd` 1.x = 1.89、`multiversion` = 1.86、`criterion` = 1.86）。dev-dep の MSRV 超過は `cargo test` / `--all-targets` ビルドで resolution エラーになりうるため、pin するか MSRV 引上げを伴う判断とするかを採否 ADR へ送れる構造にする
* unsafe 原則禁止: `unsafe_code = "warn"` + `-D warnings` で実質 deny。候補クレート内部の unsafe はクレート側の責務だが、こちらが書く PoC コード（カーネル・ベンチ・等価性テスト）は safe のみで書ける候補を比較対象とする
* 比較の網羅性と実効性: 採否 ADR が「なぜこの方式か」を対抗評価付きで説明できるだけの候補を残しつつ、ランタイム dispatch 方針と構造的に不整合な候補にはベンチコストをかけず理由付きで除外する

## Considered Options

**ハーネス配置:**

* Option A — `crates/ruviform-math` の `benches/` + dev-dependencies に配置する
* Option B — 独立 PoC クレート `crates/ruviform-simd-poc` を新設する
* Option C — ワークスペース外のスクラッチで実施する

**比較対象候補:**

* Option A — `fearless_simd` 1.1.0 / `pulp` 0.22.3 / `multiversion` 0.9.0 + autovec スカラーの 3 系統を比較する
* Option B — `fearless_simd` のみを評価する
* Option C — `wide` 1.7.1 を含む全 4 候補を比較する

**ベンチマーク基盤:**

* Option A — `criterion` 0.8
* Option B — `divan` 0.1.21

## Decision Outcome

ハーネス配置・比較対象・ベンチ基盤ともに Option A を採用する。以下の組み合わせで PoC を実施する。

* **ハーネス**: `crates/ruviform-math` の `benches/` に `[[bench]] harness = false` + dev-dependencies で配置する。スカラー参照実装・各バックエンドカーネル・等価性テストは ruviform-math 内部に置き、候補クレートの型が公開 API に一切露出しない構造を dev-deps で保証する。`resolver = "2"` により dev-dep の feature は本番ビルドに波及しない
* **比較対象**: `fearless_simd` 1.1.0 + `fearless_simd_macros` / `pulp` 0.22.3 / `multiversion` 0.9.0 + autovec スカラーの 3 系統を同一カーネル群で比較する。`wide` はランタイム dispatch 非対応が公式 docs に明記されており、x86_64 SSE2 baseline → AVX2/FMA 起動時検出という配布バイナリ方針と構造的に不整合なため、ベンチコストをかけずに比較対象から除外する（本 ADR に理由を記録）。`std::simd` は nightly 限定のため stable-only 方針で対象外
* **ベンチ基盤**: `criterion` 0.8 を採用する。`Throughput::Elements`・統計処理・入力配置のランダム化等の機能を優先し、dev-dep の重さ（rayon + plotters）は `publish = false` の内部クレートで影響が局所なため許容する。`divan` は代替候補として記録するが並行評価はしない
* **等価性検証**: カーネルごとに「ビット完全一致（`to_bits()` 比較）」または「ulp 上限」の等価性契約を定義し、スカラー参照実装を正として検証する。テスト入力に denormal・±0・NaN・Inf・非アライン先頭・レーン幅 ±1 の端数を含める。dispatch level を明示指定して強制実行し、対象 ISA 別に「スカラー vs 各 dispatch level」の matrix を自動化する。等価性判定には `approx`（`UlpsEq`）または `float_eq` を使用する
* **採否・MSRV 判断の送り**: バックエンド採否・SIMD 二層構造の採否・MSRV 1.85 維持または引上げの判断は、ベンチマーク結果と等価性 matrix の結果を受けた後続の採否 ADR で行う。本 ADR は評価アプローチのみを確定し、採否を先取りしない

### Consequences

* 良い点: dev-deps 配置により候補クレート 3 本が公開 API および本番ビルドの依存木に一切混入せず、「公開 API 非露出」制約が構造的に保たれる。dispatch level 強制指定の等価性 matrix により、CI 実行環境の ISA に依らず決定的に等価性を検証できる。採否 ADR が性能・等価性・unsafe カプセル化範囲・MSRV 適合の定量的根拠を得られる
* 悪い点: dev-dependencies が増える（`criterion` + 候補 3 クレート + 等価性判定クレート）。`Cargo.lock` が肥大化するが `publish = false` クレートのため影響は局所。`mise run bench` が実質 no-op でなくなるため、`mise.toml` の bench タスク記述と AGENTS.md の検証入口対応表の更新が必要になる。MSRV 1.85 を超える dev-dep（`criterion` 1.86・`fearless_simd` 1.89・`multiversion` 1.86）は `cargo test` / `--all-targets` ビルドの MSRV 検証に影響しうる — pin するか MSRV 判断とするかは採否 ADR で扱う

### Confirmation

下記 Verification のチェックリストで確認する。ハーネス配置・依存宣言・`harness = false` 設定は `Cargo.toml` と bench ファイルの実在で確認し、公開 API 非露出は ruviform-math の公開 API に候補クレートの型が出ないことのレビューで、等価性 matrix の自動化は dispatch level 強制指定のテストが `mise run test` で実行されることで確認する。

## Implementation Plan

* Affected paths:
  * `crates/ruviform-math/Cargo.toml` — dev-dependencies（`criterion`、`fearless_simd` + `fearless_simd_macros`、`pulp`、`multiversion`、等価性判定用 `approx` または `float_eq`）と `[[bench]] harness = false` 設定
  * `crates/ruviform-math/benches/` — カーネル別比較ベンチマーク（新規）
  * `crates/ruviform-math/src/` — スカラー参照カーネルと各バックエンドカーネル（`pub(crate)` 以下の公開度。公開 API にしない）
  * `crates/ruviform-math/tests/` — dispatch level 強制指定の等価性 matrix テスト
  * `mise.toml` — `bench` タスクの記述更新（no-op 前提の記述を実ベンチ実行に合わせる）
  * `AGENTS.md` — 検証入口の対応表（`bench` が実質 no-op である旨の記述を更新）
* Patterns to follow: スカラー参照実装を先に書き、全バックエンドが同一シグネチャ・同一入力で比較される構造にする。dispatch level は自動検出に任せず明示指定で強制する（等価性検証の決定性）。等価性契約（bit 完全一致 / ulp 上限）はカーネル単位でコード上に明示する
* Patterns to avoid: 候補クレートの型・トレイトを `pub` API に露出しない（`pub use`・公開関数の引数・戻り値への混入）。ベンチ入力をアライン済み・denormal なしのみにしない（非現実的な最良条件になる）。ランタイム自動検出頼みの等価性テストにしない（実行環境で検証対象 ISA が変わる）。`std::simd`・nightly feature・`std::arch` 直接呼出しは使わない（stable-only・unsafe 原則禁止）
* Dependencies or configuration: 上記 dev-dep 追加。MSRV 1.85 を超える dev-dep の扱い（バージョン pin か `resolver.incompatible-rust-versions = "fallback"` か）は実装時に検証し、MSRV 引上げ自体の是非は採否 ADR に送る

## Verification

* [ ] `crates/ruviform-math` の `Cargo.toml` に候補 3 系統・`criterion`・等価性判定クレートが dev-dependencies として宣言され、通常 dependencies に候補クレートが存在しない
* [ ] `[[bench]] harness = false` の criterion ベンチが `mise run bench` で実行される
* [ ] 等価性テストが dispatch level 明示指定で「スカラー vs 各 dispatch level」の matrix を構成し、入力に denormal / ±0 / NaN / Inf / 非アライン先頭 / 端数を含む
* [ ] 各カーネルに bit 完全一致または ulp 上限の等価性契約がコード上に明示されている
* [ ] PoC コードが `unsafe` を含まず `mise run lint` がグリーン
* [ ] 候補クレートの型が ruviform-math の公開 API に露出していない
* [ ] `mise.toml` の `bench` タスクと AGENTS.md の対応表が実ベンチ実行に合わせて更新されている

## Pros and Cons of the Options

**ハーネス配置:**

### Option A — ruviform-math `benches/` + dev-deps（採用）

* Good, because dev-deps は lib の依存木・公開 API に混入せず「公開 API 非露出」が構造的に保たれる。`resolver = "2"` で feature unify も本番に波及しない。dispatch 層を `ruviform-math` に集約する構造仮説と同じクレート内で検証でき、スカラー参照と同一クレートで等価性 matrix を組める。
* Bad, because ruviform-math の dev-dep が膨らみ `cargo test` / `--all-targets` の解決・ビルド時間が増える。MSRV 超過 dev-dep が MSRV 検証に影響しうる。

### Option B — 独立 PoC クレート `crates/ruviform-simd-poc`

* Good, because PoC 依存が ruviform-math の `Cargo.toml` を汚染せず、PoC 終了時にクレートごと除去できる。`publish = false` のワークスペースメンバーとして管理できる。
* Bad, because dispatch 層の `ruviform-math` 集約という構造仮説とクレート境界がずれ、PoC 成果を本実装へ移す際に再配置が必要になる。ワークスペースに一時的メンバーが増え `members` 管理が複雑化する。

### Option C — ワークスペース外スクラッチ

* Good, because ワークスペース・`Cargo.lock`・CI に一切影響しない。
* Bad, because mise タスク・workspace lints・CI 検証の枠外になり、等価性 matrix の自動検証と結果の再現性が失われる。採否 ADR の根拠として審査可能な成果物が残らない。

**比較対象候補:**

### Option A — 3 系統比較（採用）

* Good, because dispatch 方式の異なる 3 系統（`fearless_simd` の `Level` + portable API / `pulp` の `Arch` + `WithSimd` / `multiversion` の関数 multiversioning + autovec）を同一カーネルで比較でき、採否 ADR が「なぜこの方式か」を対抗評価付きで説明できる。いずれもこちら側のコードを safe に書ける候補であり unsafe 原則禁止と整合する。
* Bad, because 3 系統分のカーネル実装と等価性 matrix の組み合わせが増え、PoC の実装量が最大になる。

### Option B — `fearless_simd` のみ

* Good, because 実装量が最小で最速に結論が出る。第一候補の実力は測れる。
* Bad, because 対抗比較がなく「`fearless_simd` が最適か」は検証できない。`pulp` の dispatch 方式や `multiversion` + autovec の構成単純さという現実的な代替を退けた根拠が採否 ADR に残らない。

### Option C — `wide` を含む全 4 候補

* Good, because 網羅性は最大。
* Bad, because `wide` はランタイム dispatch 非対応が公式明記されており、x86_64 SSE2 baseline → AVX2/FMA 起動時検出という配布方針と構造的に不整合。結果を採否判断に使えないと分かっている候補にベンチコストを払うのは不経済（「multiversion + wide」組合せ案も同理由で否定済み）。

**ベンチマーク基盤:**

### Option A — `criterion` 0.8（採用）

* Good, because `Throughput::Elements` による要素レート測定・統計的処理・入力配置のランダム化等、カーネルベンチに必要な機能が揃う。設計ベースライン「criterion 等でカーネル別ベンチマーク」の記述と整合する。
* Bad, because 依存が重い（rayon + plotters）・MSRV 1.86 でワークスペース 1.85 を超過する。dev-dep のため本番影響はないが、MSRV 検証への扱いが課題として残る。

### Option B — `divan` 0.1.21

* Good, because MSRV 1.80 で 1.85 契約と衝突せず依存が軽い。`cargo bench -- --test` で CI 煙試験が容易。
* Bad, because 機能が criterion より少なく、スループット・統計処理の表現力で劣る。採否判断の定量材料の質が落ちる可能性がある。

## More Information

* 一次ソース: GitHub Issue [#19 [poc] DSP/SIMD 比較ベンチマークと採否 ADR](https://github.com/scarecrow173/ruviform/issues/19)（`relations.source` — 比較ベンチマーク・等価性基準・採否 ADR 化の要求の正本）
* 上流ドキュメント: [DSP/SIMD バックエンド比較 PoC の探索（Issue #19）](../discovery/dsp-simd-poc-issue-19.md)（`relations.derives-from` — エコシステム調査・候補比較表・仮説・未検証点の正本。MSRV・バージョン・依存の数値は同書の調査値に基づく）
* 設計制約: [ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)「DSP／SIMD 設計（A.4）」節（`relations.related` — 二層構造の有力仮説・候補一覧・等価性基準・採否基準の確定済み制約）
* 親 Issue: GitHub Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（`relations.references` — 設計ベースラインの発生源）
* 作業契約: [AGENTS.md](../../AGENTS.md)（`relations.related` — stable-only・MSRV 1.85・unsafe 原則禁止・公開 enum 規約の確定制約の所在）
* 後続: PoC 実装完了後、ベンチマーク・等価性 matrix の結果を受けて採否 ADR（SIMD バックエンド・二層構造の採否、MSRV 判断）を本 ADR から派生させる

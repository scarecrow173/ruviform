---
id: "TSPEC-034Jeo1j5l7fBhYc8iYsTN"
type: "test-spec"
status: "draft"
title: "ベンチ・検証タスクと CI スモーク挙動の契約保証"
created: "2026-10-20"
updated: "2026-10-20"
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
    - "../../AGENTS.md"
    - "manual-ci-workflow.md"
    - "mise-verification-tasks.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/dsp-simd-poc.md"
  derived-by: []
  verifies:
    - "../specs/dsp-simd-poc-adr.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ベンチ・検証タスクと CI スモーク挙動の契約保証

## Purpose

Issue #19 PoC のベンチ検証が保証すべき運用意図——「全量計測（`mise run bench`）と CI/短時間スモーク（`mise run bench-smoke`）の二層分離」「実行時間予算の制御」「experiment log のコミット可能な記録形式」——を、[spec](../specs/dsp-simd-poc-adr.md) の受け入れ基準（「`mise run bench` で候補別・サイズ別の計測が実行できる」「mise 検証と `prek run --all-files` がグリーン」）と[設計書](../designs/dsp-simd-poc.md)のベンチ methodology・実行時間予算・計測結果記録形式の節から固定する。ベンチハーネスが「存在するが実行できない／遅すぎて運用に乗らない」状態に退行しないことを保証する。廃止してよいのは、採否 ADR 完了後に PoC ベンチハーネスが除去されたとき、または CI bench 運用方針を変更する ADR / spec 改訂が行われたとき。

## Feature

ベンチ・検証タスクの二層運用: `mise run bench`（`cargo bench --workspace`）を全量計測入口、`mise run bench-smoke`（`cargo bench -p ruviform-math -- --test`）を CI・短時間スモーク入口として `mise.toml` に定義し、CI bench ジョブはスモークのみを実行する。計測結果は `docs/impl/exp/*.jsonl` として prek hook 適合・コミット可能な形式で記録されること。

## Rules

- **`mise run bench` は全量計測入口**: spec 受け入れ基準「`mise run bench` で候補別・サイズ別の計測が実行できる」の正本。`bench` タスクは「ベンチマーク未実装のため実質 no-op」から脱却し、`cargo bench --workspace` の実計測を実行して全 matrix（8 カーネル × 約 10 impl × 7 サイズ × 入力バリアント ≈400–560 ケース）を対象とする。
- **`bench-smoke` は CI/短時間スモーク**: `cargo bench -p ruviform-math -- --test`。criterion の `--test` モードは各ベンチを計測せず 1 反復だけ実行して完走性を検証するため、短時間（分数オーダー）で終わる。
- **CI bench ジョブはスモークのみ実行**: `.github/workflows/ci.yml` の `bench` ジョブ（`ubuntu-latest`・`workflow_dispatch` 専用）は `mise run bench` ではなく `mise run bench-smoke` を呼ぶ。CI で全量計測は実行しない（実行時間・ノイズのため）。全量計測は Windows x64 主計測環境でのローカル手動実行に分離する。
- **cargo 呼び出しは mise 経由**: 部分計測（quick-run filter）は `mise exec -- cargo bench -p ruviform-math -- <filter>`（例: `dot`・`1024`）とし、検証・計測入口として裸の `cargo` を使わない（AGENTS.md「cargo コマンドは直接実行せず `mise run` / `mise exec --` 経由」の規約）。
- **実行時間予算を削減設定で制御**: 全 matrix は criterion 既定設定（warmup・measurement・sample 100）で 1 時間超が見込まれるため、PoC 用 `BenchmarkGroup` 設定（`warm_up_time` / `measurement_time` / `sample_size` 削減）で全量 ≈20–30 分以内を目安に短縮し、quick-run filter で更に絞り込める。採用した設定値と予想・実測の所要時間は experiment log の環境メタデータに記録する。
- **マルチランは「run → 集約 → 次 run」順**: criterion の `estimates.json` は run ごとに上書きされるため、分散記録のための 2–3 run は各 run 完了直後に `estimates.json` を集約（jsonl へ退避）してから次 run を実行する。
- **experiment log は hook 適合形式**: `docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl` は 1 行 = 1 計測点の JSON Lines とし、LF 改行・末尾改行ありで生成してコミット対象とする（prek の end-of-file-fixer / mixed-line-ending 系 hook を通過する形式）。環境メタデータ（OS・CPU 型名・コア数・rustc 版・criterion 版・`Level::new()` / `pulp::Arch::new()` 検出結果）を run 冒頭で標準出力に出し、experiment log 文書へ転記する。
- **検証入口記述の実態整合**: `bench` タスクが no-op でなくなった点・CI が `bench-smoke` を呼ぶ点を含め、`mise.toml` のタスク記述・`AGENTS.md` 検証入口対応表・`README.md` の bench 関連記述を実態に合わせて更新する。

## Examples

- Given: 実装完了後のローカル環境（Windows x64）。When: `mise run bench` を実行する。Then: criterion が全 matrix の計測を開始し、`target/criterion/<group>/<impl>/<input>/<len>/new/estimates.json` に統計値が出力される。
- Given: `workflow_dispatch` で CI を手動起動。When: `bench` ジョブを確認する。Then: `mise run bench-smoke` が実行され、全量計測ではなく各ベンチの 1 反復スモークで完走する。
- Given: `mise run bench-smoke`（= `cargo bench -p ruviform-math -- --test`）。When: 実行する。Then: criterion が計測を行わず全ケースを 1 回ずつ実行して終了し、ハーネスの破損（panic・設定不備）があれば失敗する。
- Given: 部分計測を行いたい。When: `mise exec -- cargo bench -p ruviform-math -- dot` を実行する。Then: `dot` を名前に含むベンチケースのみ計測される（quick-run filter 規約）。
- Given: 2 回目のベンチ run 直前。When: 前回 run の `estimates.json` を確認する。Then: 次 run で上書きされるため、先に jsonl へ集約済みであることが必要となる（run → 集約 → 次 run）。
- Given: `docs/impl/exp/dsp-simd-poc-<yyyymmdd>-<seq>.jsonl` をコミット。When: `prek run --all-files` を実行する。Then: end-of-file-fixer / mixed-line-ending 系 hook を通過する（LF・末尾改行あり）。
- Given: spec AC-6 の `mise run bench` グリーン要求。When: plan の最終検証（V-8）を確認する。Then: `bench-smoke` 実行はハーネス健全性の再確認であり、AC-6 の全量計測実績は Phase F の experiment log で充足済みと扱う根拠が記録されている。

## Guarantees

- `mise run bench` が実際に全量計測を実行する入口として機能し続ける（no-op 記述の残置・タスク名だけの存在を防ぐ）。
- CI でベンチハーネスの破損が短時間・決定的に検出される（スモーク lane がハーネスの腐敗を防ぐ）。
- 全量計測が CI をブロックしない（スモーク分離により CI 実行時間・計測ノイズの問題を回避）。
- 計測結果が環境メタデータつき・hook 適合・コミット可能な形式で残り、採否 ADR の根拠データとして再利用できる。
- 部分計測（quick-run filter）とマルチラン集約の手順が規約として固定され、実装者がコマンドを再発明しなくてよい。

## Non-goals

- 計測値の性能判定・スカラー比の評価（採否 ADR の責務。判定目安の数値は設計書 methodology 節「判定目安の数値化」が正本）。
- 等価性契約の内容（[simd-equivalence-matrix-contract](simd-equivalence-matrix-contract.md) の責務）。
- criterion の統計処理の正しさ・`estimates.json` スキーマ（上流ツールの責務）。
- 他 OS / アーキでのベンチ再現（Windows x64 が主計測。spec の Out of Scope）。
- CI ワークフローの手動ディスパッチ方針・他ジョブの検証内容（[manual-ci-workflow](manual-ci-workflow.md)・[mise-verification-tasks](mise-verification-tasks.md) の責務）。

## Risk

- スモーク分離を欠くと、CI で全量計測（既定設定で 1h 超）が走りジョブが実質使用不能になるか、逆に CI 検証を外してハーネス破損が未検出のまま残る。
- `bench` タスクの記述が実態と乖離すると、検証入口の信頼が崩れ「`mise run bench` が通った」という報告の意味が失われる。
- 実行時間予算の削減設定がなければ全量計測が現実的に実行不能になり、採否判断の計測データ自体が得られない。
- jsonl が hook 非適合（CRLF・末尾改行なし）だとコミット時にフック失敗または無言の整形差分が混入し、experiment log の証拠性が損なわれる。
- マルチランの集約順序を誤ると `estimates.json` の上書きで前回 run の生データを失い、分散記録が成立しない。

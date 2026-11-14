# RF-SCRIPT-01 PoC Task

plan: `docs/plan.md`。各タスクは commit 可能な論理単位。

- [x] T1: `poc/script-coexist/` 骨格（Cargo.toml/.gitignore/lib.rs）+ contract/value/canonical/codec
- [x] T2: `backends/mod.rs` 共通 session 検証 + `backends/luau.rs`（codec・interrupt budget・memory limit・host handlers）
- [x] T3: `backends/rhai.rs`（codec・on_progress budget/cancel・call_fn entry）
- [x] T4: `plan.rs`（DefinitionStore + workflow plan 検証）+ `analyzer.rs`（parse-only analyzer registry）
- [x] T5: `tests/fixtures.rs` — FX-P-01〜08・FX-N-01〜28 の実測（36 test）
- [x] T6: `src/main.rs` デモ実行（registry 登録・交互評価・同一 digest・fail-closed 表示）
- [x] T7: `mise.toml` poc-script-coexist-{check,lint,test,run} 追加 + fmt/check/lint/test 検証
- [x] T8: `evidence/rf559/`（gap report・verification・OS matrix）
- [x] T9: `docs/impl/{ir,exp}/rf-script-01-coexist-poc.md` + index 再生成 + review

## 実装時の差分メモ

- 計画の `consumer.rs` は `plan.rs` の `DefinitionStore`/`build_workflow_plan`
  に集約した（Consumer 側 Typed Port の最小実装として十分なため別 file に
  分けなかった）。
- `src/main.rs` は JSONL sweep ではなく、人間可読のデモ出力 + assert に
  留めた。機械可読証拠は `tests/fixtures.rs` の 36 test が担う。

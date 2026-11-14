# RF-SCRIPT-01 PoC Plan

spec: `docs/spec.md` / design: `docs/design.md`（同ディレクトリ）。

## File map

| path | 内容 |
| --- | --- |
| `poc/script-coexist/Cargo.toml` | 独立 workspace + package `script-coexist-poc`、lints は workspace 規約を参考に `unsafe_code=warn` |
| `poc/script-coexist/.gitignore` | `/target/`（Cargo.lock は追跡 — 再現性のため） |
| `poc/script-coexist/src/lib.rs` | module 公開 |
| `poc/script-coexist/src/contract.rs` | Id/Version/Capabilities/Registry/Requirement/ResolveError/Session/Config/Budget/Cancel/HostApiSurface |
| `poc/script-coexist/src/value.rs` | ScriptValue/ScriptError/ScriptSource/ScriptArtifact/各 newtype |
| `poc/script-coexist/src/canonical.rs` | canonical serialization + sha256 digest |
| `poc/script-coexist/src/plan.rs` | WorkflowDefinition/Artifact 検証 |
| `poc/script-coexist/src/analyzer.rs` | AnalyzerRegistry + SyntaxCheckAnalyzer |
| `poc/script-coexist/src/consumer.rs` | ArtifactStore・freshness・host surface 検査 |
| `poc/script-coexist/src/backends/{mod,luau,rhai}.rs` | ScriptBackend/ScriptSession impl + codec |
| `poc/script-coexist/src/main.rs` | fixture sweep CLI（JSONL 証拠行） |
| `poc/script-coexist/tests/fixtures.rs` | FX-P/N テスト |
| `poc/script-coexist/docs/{spec,design,plan,task}.md` | 本チェーン |
| `mise.toml` | `poc-script-coexist-{check,lint,test,fmt,run}` タスク追加 |
| `docs/impl/ir/rf-script-01-coexist-poc.md` | IR（impl-doc 規約） |
| `docs/impl/exp/rf-script-01-coexist-poc.md` | exp（検証計画+結果） |
| `evidence/rf559/{gap-report,verification}.md` + 記録 | 証拠 |

## Dependency graph

```
contract (Id/Registry/Session/Budget/Host)
value (ScriptValue/Error/Source/Artifact)
canonical -> value
plan -> value, canonical
consumer -> value, canonical, contract
analyzer -> contract, value
backends::{luau,rhai} -> contract, value, canonical, consumer(host store)
lib -> all
main/tests -> lib
```

## 実装順序

1. contract/value/canonical（型と digest）
2. backends trait + luau backend（codec・budget・cancel）
3. rhai backend
4. plan/consumer/analyzer
5. tests/fixtures.rs（FX-P→FX-N）
6. main.rs 証拠 sweep
7. mise タスク・fmt/lint/test 検証
8. evidence + impl docs + index 再生成

## Verification matrix

| 検証 | コマンド | 判定 |
| --- | --- | --- |
| fmt | `cargo fmt --manifest-path poc/script-coexist/Cargo.toml -- --check`（poc タスク経由） | exit 0 |
| check | `mise run poc-script-coexist-check` | exit 0 |
| lint | `mise run poc-script-coexist-lint`（`-D warnings`） | exit 0 |
| test | `mise run poc-script-coexist-test` | 全 fixture Pass |
| run 証拠 | `mise run poc-script-coexist-run` | JSONL 出力を evidence へ |
| repo fmt | `mise run fmt-check`（ルート workspace — poc は独立のため別途 poc fmt を実施） | exit 0 |
| doc lint | `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/impl` | 自スコープ error なし |
| index | `doc_maintenance.js index apply --dir docs/impl/ir` / `--dir docs/impl/exp`（sync_impl_index 経由） | regenerated |

`*-msrv`/`*-full`/3OS は実行しない（タスク指定なし・Daily CI 委譲）。
macOS/Linux Build/Run は NotRun。

## Risk register

| risk | 緩和 |
| --- | --- |
| vendored Luau の Windows MSVC build 失敗 | cc が VS2022 BuildTools を検出。失敗時は再現ログを evidence に残し No-go/代替を記録 |
| mlua `set_interrupt` が budget/cancel に十分か | safepoint 保証を documentation で確認済み。命令数は近似として正直に記録 |
| codec の tag 衝突（`$rv` key） | 予約 key として文書化。pull は tag 形式を厳格検査 |
| Rhai `on_progress` API 差異 | 実装時に docs.rs で確認。非対応なら cancel は host call 経由のみと正直に記録 |
| canonical 数値正規化の曖昧さ | design.md に規則を明記。FX-P-02 で digest 一致を実測 |

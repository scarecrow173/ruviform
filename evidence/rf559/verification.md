# RF-SCRIPT-01 Verification Record — Build/Link/Run・OS Matrix

Issue: https://github.com/scarecrow173/ruviform/issues/559
対象: `poc/script-coexist/`（独立 workspace、`[workspace] members = ["."]`）
判定台帳: [`gap-report.md`](gap-report.md)

## 実行環境

- OS: Windows 11 x86_64（worktree: `rf559-script1`）
- Toolchain: mise 管理 Rust（repo `mise.toml` 正本）。`rustc --version`
  は検証時の toolchain を使用（`cargo` は mbx managed target 経由）
- 検証日時: 2026-11-14
- Feature 構成: `mlua = { version = "0.12.2", features = ["luau", "send"] }`、
  `rhai = { version = "1.27.0", features = ["sync"] }` — 排他 feature なし、
  単一 crate で両 VM を同時有効化

## OS Matrix（Build / Link / Run / OS依存 / Feature非依存 / GPU非依存）

| OS | Build | Link | Run | OS依存 | Feature非依存 | GPU非依存 | 判定 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Windows 11 x86_64 | Pass | Pass | Pass | Pass（MSVC + vendored Luau C++） | Pass（排他 feature なし同時 link） | Pass（headless、GPU 参照なし） | **PoC-proven** |
| macOS | NotRun | NotRun | NotRun | NotRun | NotRun | NotRun | **NotRun** — 本環境で実行不能。技術非対応は未確定 |
| Linux | NotRun | NotRun | NotRun | NotRun | NotRun | NotRun | **NotRun** — 同上。vendored Luau C++ は POSIX toolchain で build 可能な設計だが未実測 |

NotRun の扱い: POLICIES.md の規範どおり、実行できない OS は NotRun。
原因で技術非対応と確定した場合のみ Unsupported とする — 本 PoC では
macOS/Linux の非対応は一切確定していない。

## 実行コマンドと結果（canonical = mise）

すべて repo root で実行。PoC は独立 workspace のため専用タスク経由。

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | Pass（root workspace。poc/ は独立 workspace のため `cargo fmt --all` は poc を対象外 — poc 側は `cargo fmt --all` を poc/script-coexist で個別実行し `cargo fmt --check` 相当を確認） | 0 |
| `cd poc/script-coexist && cargo fmt --all -- --check` | Pass（補助証拠） | 0 |
| `mise run poc-script-coexist-check` | Pass（`cargo check --locked --all-targets`） | 0 |
| `mise run poc-script-coexist-lint` | Pass（`cargo clippy --locked --all-targets -- -D warnings`） | 0 |
| `mise run poc-script-coexist-test` | Pass（`cargo test --locked --all-targets` — 36 fixture test 全 Pass） | 0 |
| `mise run poc-script-coexist-run` | Pass（demo binary、下記出力） | 0 |

※ `cargo` 直接実行の結果は補助証拠（canonical = mise）。上記は
mise タスク定義が `cd poc/script-coexist && cargo ...` を実行する
もので、実コマンドは同一。

## Demo binary 出力（`mise run poc-script-coexist-run`）

```
== RF-SCRIPT-01 coexist PoC demo ==
poc_contract_version = 1
registered backends: 2
  backend=luau-mlua language=luau contract_v1 caps=21
  backend=rhai-embedded language=rhai contract_v1 caps=21
round=0 lang=luau backend=luau-mlua artifact_kind=ActionRecipe digest=sha256:v1:254a2a27d6922d416a0f31c4622e0602fa92f0d292fbfcdd105d56593ab48af3
round=0 lang=rhai backend=rhai-embedded artifact_kind=ActionRecipe digest=sha256:v1:254a2a27d6922d416a0f31c4622e0602fa92f0d292fbfcdd105d56593ab48af3
round=1 lang=luau backend=luau-mlua artifact_kind=ActionRecipe digest=sha256:v1:254a2a27d6922d416a0f31c4622e0602fa92f0d292fbfcdd105d56593ab48af3
round=1 lang=rhai backend=rhai-embedded artifact_kind=ActionRecipe digest=sha256:v1:254a2a27d6922d416a0f31c4622e0602fa92f0d292fbfcdd105d56593ab48af3
distinct digests across backends: 1
SAME_DIGEST_OK
distinct workflow digests across backends: 1
WORKFLOW_SAME_DIGEST_OK
fail-closed unknown language -> no matching backend (UnknownLanguage): unknown language: python
demo complete
```

（全 4 回の action_recipe 評価と workflow 評価で digest が一致。）

## Fixture test 一覧（`tests/fixtures.rs`、36 test 全 Pass）

FX-P（正例）: `fx_p01`〜`fx_p08` — 同一 digest・明示選択・交互/並行評価・
混種 workflow・値モデル往復・typed host api・analyzer。
FX-N（反例）: `fx_n01`〜`fx_n28` — registry/session/plan/analyzer の
fail-closed、quota（instructions/wall/memory/output/host-calls）、
cancel、session isolation、error 分類。

対応表は `gap-report.md` の Fixture 対応表を参照。

## Vendor 型非漏出の確認

- 公開型（`ScriptValue`/`ScriptError`/`ScriptArtifact`/`ScriptResult`/
  `BackendDescriptor`/`SessionConfig` 等）に mlua 型・Rhai `Dynamic`・
  DAW 固有型は存在しない（`src/` 全公開面を確認）。
- `fx_n21_error_classification` が error kind の Debug 表現に
  `mlua`/`Dynamic` が出ないことを assert。
- codec（`src/codec.rs`）が `$rv` reserved tag record で vendor 非依存の
  typed value を両 backend 共通で materialize/dematerialize する。

## 既知の PoC 限界（evidence として明記）

- Rhai backend は `MaxMemoryBytes` を強制できない（capability で非対応を
  広告し `UnsupportedBudget` で拒否 — `fx_n06`）。
- instruction/wall-time budget は VM safepoint 近似（`CancelGranularity::
  Safepoint` を capability で広告）。
- Luau safe state は標準 library 非公開の最小構成。`os`/`io`/`debug`/
  `loadstring` 系は存在しないため unsafe host API の遮断は自明だが、
  製品で stdlib を開く場合は host surface 設計が別途必要。
- workflow artifact の `references` は `definition_ref` のみ受理。
  真の循環参照は内容アドレス digest 上構築不能（`CycleDetected` は
  防御的実装）。

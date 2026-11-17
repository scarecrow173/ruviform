# Issue #725 — consumer 影響評価（rf725 Phase A）

- 日付: 2026-11-17
- ブランチ: `feat/725-serde-vocab-phase-a`
- 対象変更: `EnvironmentKind::RealAbiTestPlugin` の serde 語彙
  `real-abi-test-plugin` ↔ schema 語彙 `test-plugin` の gap 解消
  （Phase A = deserialize 側 alias 受容。emit flip は第2 slice）
- 照合先 Issue: `scarecrow173/audio-lens#127`・
  `scarecrow173/synvelis#28`・`scarecrow173/synvelis#126`
  （consumer 側リポジトリは read-only 照合のみ — 変更なし）

## 語彙 gap の所在（確認済み）

- emit/serde 側: `crates/ruviform-audio-test/src/metadata.rs`
  `EnvironmentKind::RealAbiTestPlugin` — `#[serde(rename_all =
  "kebab-case")]` により canonical wire 綴りは
  `real-abi-test-plugin`。`as_str()` も同綴りを返す。
- schema 正本（read-only）: `ci/consumer-verify/evidence.mjs`:
  `ENVIRONMENT_KINDS`（:73-79）は `test-plugin` を契約語彙とし、
  `environment_kind` の lane record 検証（:362
  `if (!ENVIRONMENT_KINDS.includes(rec.environment_kind))`）で
  `real-abi-test-plugin` は未知語彙として拒否される。
- 契約語彙の出典: `docs/designs/issue-627-virtual-first-verification-
  contract-design.md`:99,105・`docs/specs|designs/issue-294-virtual-
  first-evidence-schema.md`・`.github/workflows/consumer-verify.yml`:
  1239 の静的 lane 行 `EXT-L3|test-plugin|Required`。
- emit 経路: `crates/ruviform-plugin-dock-test/src/scenario.rs` の
  `json!(kind.as_str())` が evidence JSONL の `environment_kind`
  に `real-abi-test-plugin` を流す。`tests/plugin_dock.rs` は同
  綴りを literal assert（rf#631 占有のため本 slice では変更不可）。

## consumer 消費箇所の実測照合

### audio-lens（`scarecrow173/audio-lens`）

- 依存: `crates/al-analysis/Cargo.toml` の `[dev-dependencies]` に
  `ruviform-audio-test.workspace = true`（workspace 依存は
  ルート `Cargo.toml`:92 で git 依存
  `https://github.com/scarecrow173/ruviform.git`）。
  `Cargo.lock` の解決 rev: `cbcc83437616fdd7418da37183c6be66b6cfe225`。
- コード消費は `crates/al-analysis/tests/rf628_oracle.rs` のみ:
  - `ruviform_audio_test::oracle::{PcmCompare, pcm_assert_eq,
    pcm_reference_block}`
  - `ruviform_audio_test::pcm::{PcmFixture, PcmPattern,
    PcmScenario, PcmSegment}`
- `metadata` module・`EnvironmentKind`・`RunMetadata`・
  `RunParameters`・`environment_kind`・`as_str` の使用は
  **ゼロ**（`rg` で crates/ 全体を照合。`environment_kind`/
  `environmentKind` はリポジトリ内に一切出現しない）。

### synvelis（`scarecrow173/synvelis`）

- 依存: `crates/synvelis-recording/Cargo.toml` の
  `[dev-dependencies]` に `ruviform-audio-test.workspace = true`
  （ルート `Cargo.toml`:75 で同じく git 依存）。
  `Cargo.lock` の解決 rev: 同上 `cbcc8343…`。
- コード消費は `crates/synvelis-recording/tests/` の 2 ファイルのみ:
  - `rf628_oracle.rs`: `backend::{DeviceSpec, InputSpec,
    ScriptedBackend, StreamScript, StreamDirection}`・
    `clock::*`・`oracle::{PcmCompare, pcm_assert_eq,
    pcm_reference_block}`・`pcm::{PcmFixture, PcmPattern,
    PcmScenario, PcmSegment}`
  - `rf628_oracle_rt_alloc.rs`: `backend`・`clock`・`pcm` のみ
- `metadata` module・`EnvironmentKind`・`RunMetadata`・
  `environment_kind` の使用は **ゼロ**。
- `test-plugin` 文字列は `crates/synvelis-render/tests/
  rcp01_recipe.rs`:286・`rcp03_render_policy.rs`:199 に出るが、
  これは `ProcessorBinding::Resolved(PluginRef { descriptor:
  "test-plugin", .. })` の plugin descriptor 名であり、
  `EnvironmentKind` の serde 語彙とは無関係。
- `environment_kind`/`environmentKind` の出現は
  `evidence/sv86/fault-injection-followup-verification.md` と
  `evidence/reports/syn-io-02-windows-real-fs.md` の表ヘッダ
  2 箇所のみ — 過去 run の記録文書でありコード消費ではない。

## 影響評価の結論

| 観点 | 結果 |
| --- | --- |
| consumer コードへの compile 影響 | **なし** — 両 consumer とも `metadata` module を import していない。Phase A（deserialize alias 追加）も第2 slice（emit flip）も `oracle`/`pcm`/`backend`/`clock` の消費面に触れない |
| consumer の証拠 JSONL への影響 | **なし** — consumer 側が `environment_kind` を生成・検証するコード経路は存在しない。`ci/consumer-verify` の schema 検証は ruviform repo 内の workflow が担い、consumer は lane を実行する側 |
| consumer pin bump の要否 | **不要** — consumer は git 依存の追従 pin であり、変更は後方互換（deserialize は新旧両綴りを受容、serialize/`as_str` は Phase A で不変）。仮に第2 slice で emit が `test-plugin` に flip しても、consumer コードは `EnvironmentKind` を消費しないため影響なし。pin 更新自体は consumer 側の通常の追従フローに委ねる |
| ruviform 内部の直接 consumer | `crates/ruviform-plugin-dock-test`（rf#631 占有）が `EnvironmentKind` を消費し `as_str()` の emit 綴りを evidence に流す + literal assert — emit flip は同クレートの更新を必須とするため第2 slice に分離（本 slice の編集範囲外） |

## 判定

consumer（AudioLens / Synvelis）への影響は **ゼロ** と評価する。
`real-abi-test-plugin`↔`test-plugin` の語彙 gap 解消は
consumer 側のコード・pin・証拠経路に影響を与えず、consumer 側の
pin bump 作業は不要（Issue #725「対象外」の「consumer 側 pin bump
実作業」に該当する作業は発生しない）。

schema 側（`ci/consumer-verify/`）は `test-plugin` が正本で
変更不要 — read-only 照合のみ実施し、本 slice では一切触れていない。

## 検証方法の記録

- `rg -l "ruviform-audio-test|ruviform_audio_test"` で両 consumer
  リポジトリの消費ファイルを列挙 → `Cargo.toml`/`Cargo.lock` と
  `tests/rf628_oracle*.rs` を実読。
- `rg -n "EnvironmentKind|RunMetadata|environment_kind|
  real-abi-test-plugin|test-plugin|CaseRequirement|CaseResult|
  VerificationLevel"` を両 consumer の `crates/` + ルート
  `Cargo.toml` に対して実行 → metadata 系の使用ゼロを確認
  （synvelis の `test-plugin` 2 件は `PluginRef.descriptor` で
  無関係と確認）。
- `rg -ln "environment_kind|environmentKind"` を両 consumer
  リポジトリ全体（`target`/`Cargo.lock` 除外）に実行 → コード上の
  使用ゼロ、evidence 文書の表ヘッダ 2 件のみ。
- ruviform 側の emit 経路・schema 正本・契約語彙出典・
  plugin-dock-test の literal assert は本 worktree で実測確認
  （read-only 参照 — `crates/ruviform-plugin-dock-test/`・
  `ci/consumer-verify/`・`.github/workflows/` は編集していない）。

実施日: 2026-11-17 / 実施者: Devin worker（task_3ab7dd9af9dd）
照合時点の consumer ローカル HEAD は作業時点の checkout —
リモート最新との乖離があり得るが、対象 API の消費構造
（metadata module 未使用）は両 repo の main 系で確認済み。
必要に応じて Coordinator が統合時に再照合する。

# RF-SCRIPT-01 PoC Spec — Luau+Rhai 単一バイナリ共存・Backend Registry/Typed Port

- Issue: https://github.com/scarecrow173/ruviform/issues/559
- 上位契約: `docs/specs/issue-558-script-contract.md`、
  `docs/designs/issue-558-script-contract-design.md`、
  `docs/adr/issue-558-script-backend-contract.md`（RF-SCRIPT-00、main 統合済み）
- 状態: 内部文書（poc/ 配下。canonical docs ツリー外・doc-status 監査対象外）

## 目的

RF-SCRIPT-00 契約の型・意味論をミラーした**隔離 PoC crate** で、Luau(mlua)
と Rhai を同一実行ファイルへ同時リンクし、実行時 Registry/Dispatch・
Versioned Typed Definition・Fail-Closed を headless で実証する。
`ScriptBackendRegistry` の本番実装は存在しないため、本 PoC は契約型を
ミラーした最小ローカル registry を PoC 印付きで実装する。

## 受入条件（Issue #559 転記）

1. 同一実行ファイルに Luau+Rhai VM を同時リンク・初期化し、交互/並行
   Script 実行で Language/Backend Registry の明示選択を確認する
   （排他 Feature ビルドは不合格）。
2. 同一意味の Versioned Typed Definition を両 VM から出力し、混成
   Workflow 参照を Host Typed Port 経由で扱い、VM ポインタ直渡し/循環は
   拒否する。
3. 公開型に mlua/Rhai Dynamic/DAW 固有型を漏出させず、i64/Null/Absent/
   ID/Time/Unsafe Host API/Quota/Cancel/Session Isolation を Fixture で
   判定する。
4. Windows/macOS/Linux について Build/Link/Run/OS 依存・Feature/GPU
   非依存を個別に記録し、実行できない OS は NotRun、技術非対応確定時
   のみ Unsupported。
5. 共存成立/No-go・残る OS/Consumer 制約と必要な最小移行・代替方式を
   #558/#553 へ渡す。製品正式 Luau/Rhai サポートは別 Gate。

## スコープ

### In Scope

- `poc/script-coexist/` 配下の独立ワークspace PoC crate（lib+bin+tests）。
- 契約ミラー型: `ScriptLanguageId`/`ScriptBackendId`/`contract_version`/
  `BackendCapabilities`/`ScriptBackendRegistry`/`ScriptRequirement`/
  `ResolveError`、`ScriptBackend`/`ScriptSession`/`SessionConfig`/
  `ExecutionBudget`/`ScriptCancel`、`ScriptValue`/`ScriptError`/
  `ScriptSource`/`ScriptArtifact`、`HostApiSurface`/`HostBinding`、
  `ScriptAnalyzer`/`AnalyzerRegistry`（最小）。
- Luau backend（mlua `luau` feature、vendored Luau）と Rhai backend
  （rhai crate）の同一バイナリ同時登録・実行時選択。
- Versioned Typed Definition（ActionRecipe/WorkflowDefinition）の
  canonical form + `DefinitionDigest`（sha256）による同一 digest 実証。
- Fixture: FX-P-01〜P-08・FX-N-01〜N-24 のうち本スライスで実測可能な
  全行をテスト化。Consumer Plan 検証（F-09〜F-11, F-17）は PoC 内の
  型付き文書検証として実装。
- `evidence/rf559/` への gap report・Build/Link/Run・OS 別記録。
- `mise.toml` への poc タスク追加、`docs/impl/{ir,exp}/rf-script-01-*.md`。

### Out of Scope

- 本番 crate 化・公開 API 確定・`crates/**` 変更・Cargo.lock（ルート）・
  CI workflow 変更。
- macOS/Linux での実 Build/Run（NotRun として記録）。
- 3OS 配布レーン（rf#560）、Consumer（synvelis/audio-lens）への適用
  （読み取り専用参照のみ）、製品正式 Luau/Rhai サポート判断。
- Lua 5.4、JIT/性能比較、VM stack/coroutine 永続化、Workflow Engine/
  Coordinator/Job 正本。

## Fixture 割当（本スライスの実施範囲）

| Fixture | 内容 | 本 PoC での扱い |
| --- | --- | --- |
| FX-P-01/02 | Luau/Rhai が同一意味 Action Recipe を生成 | 実測（同一 DefinitionDigest） |
| FX-P-03 | 同一意味 Workflow Definition | 実測（同一 digest） |
| FX-P-04 | 混合言語 Workflow（型付き参照・Host Typed Port 経由） | 実測 |
| FX-P-05 | Absent/Null 区別の往復 | 実測 |
| FX-P-06 | i64/Time/ResourceId 往復 | 実測（backend 別の正直な意味論を記録） |
| FX-P-07 | Luau+Rhai 同時登録・実行時選択 | 実測（交互+並行） |
| FX-P-08 | Analyzer Diagnostics/Completion | 実測（Syntax のみ、Completion=Unsupported） |
| FX-N-01〜N-08 | Registry/Version/Capability/digest Fail Closed | 実測 |
| FX-N-09〜N-11 | CrossVmReference/Cycle/UnknownReference | 実測（PoC plan validator） |
| FX-N-12 | i64 非表現 → ValueOutOfRange | 実測（Luau push 拒否 + decode 範囲外） |
| FX-N-13/14/24 | Analyzer Capability/stale/unavailable | 実測 |
| FX-N-15 | RT 呼出し契約違反 | 実測（non-RT ガードの TLS 模擬） |
| FX-N-16 | Session 状態漏洩 | 実測（分離確認） |
| FX-N-17 | Grant/Secret 混入 | 実測 |
| FX-N-18 | Unsafe Host binding | 実測（consumer 側拒否） |
| FX-N-19 | ブロッキング Host + cancel | 実測（協調 cancel・即時中断非保証を記録） |
| FX-N-20/21 | Budget 超過（instructions/memory） | 実測（Luau memory limit、Rhai operations） |
| FX-N-22/23 | 巨大 source/敵対的入力 | 実測 |
| M-07/M-08 | Analyzer crash/FFI 障害・VM panic | Deferred（#560/#559 本番判断材料として記録のみ） |

## 成功判定

- 全 Fixture が `mise run` の poc タスク経由で Pass/Fail を機械的に
  出力し、未実測を Pass にしない。
- gap report が contract-covered / PoC-proven / NotRun を区別し、
  共存成立/No-go と残る制約・最小移行を #558/#553 へ渡せる形で残る。

---
id: "IMPL-692ScriptPort0001"
type: "impl"
status: "completed"
title: "Issue #692 ruviform-script neutral contract port"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "devin-worker"
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-script/"
      - type: "directory"
        path: "evidence/rf692/"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-692-script-port.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/designs/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-692-script-port.md"
  references: []
  defers:
    - "実 Luau backend / Analyzer 製品 port（Issue #692 AC2 本体 — s2 slice）"
    - "synvelis thin adapter 接続（synvelis#271/#272/#273/#274 consumer gate）"
    - "Luau.Analysis native FFI 統合（Issue #560 系 — s2 以降）"
    - "mlua vendored vs native bridge の依存選定（s2 slice で RF-SCRIPT-02 結果を参照）"
    - "*-full / *-msrv / 3OS 横断検証（CI lane 委譲）"
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-692-script-port-exp.jsonl"
      - "docs/impl/exp/issue-692-script-port.md"
    rejected: []
---

# Issue #692 ruviform-script neutral contract port

## Summary

Issue #692 first slice（RF-SCRIPT-00）として、headless・vendor 中立の
契約 crate `crates/ruviform-script` を新設した。`docs/designs/
issue-558-script-contract-design.md`（approved）の design 完全形を
`poc/script-coexist`（#559）・`ffi-poc`（#560）の意味論から port し、
C1 registry / C2 runtime port / C3 値モデル / C4 budget・cancel /
C5 analyzer port / C6 host surface 記述子 + canonical digest +
`$rv` codec を実装。実 VM・FFI・unsafe・vendor 型は含まない。

## Context

- spec: `docs/specs/issue-692-script-port.md`（approved 相当 — doc chain）
- design: `docs/designs/issue-692-script-port.md`
- plan: `docs/plans/issue-692-script-port.md`
- test-spec: `docs/test-specs/issue-692-script-port.md`
- task: `docs/tasks/issue-692-script-port.md`
- 契約正本: `docs/designs/issue-558-script-contract-design.md`（#558 approved）
- 移植元（READ-ONLY）: `poc/script-coexist/src/`、`ffi-poc/src/`、
  synvelis `crates/synvelis-script/src/`（cross-repo 参照のみ）

## Implementation

### Changes

- `crates/ruviform-script/Cargo.toml` — workspace metadata 継承、
  `[lints] workspace = true`。依存は `serde_json`（canonical 文字列の
  JSON escape に内部利用のみ）と `sha2`（digest 計算）のみ — 両方
  Cargo.lock 既在で新規外部依存なし。理由・ライセンスをコメント記録。
- `src/lib.rs` — `#![forbid(unsafe_code)]`、module 構成、
  `CONTRACT_VERSION = 1`、root `pub use` 再エクスポート + `prelude`。
- `src/id.rs` — opaque ID newtype 一式（`ScriptLanguageId`/
  `ScriptBackendId`/`AnalyzerId`/`CapabilityId`/`TimeDomainId`/
  `ResourceDomainId`/`SessionId`）と `ScriptBackendVersion`
  （`impl_version` + `contract_version` 分離）。
- `src/value.rs` — `ScriptValue` 閉じた variant 集合（`Absent`/`Null`/
  `Bool`/`Integer(i64)`/`Number(f64)`/`String`/`Bytes`/`Array`/
  `Record(BTreeMap)`/`Time{domain,timebase,count}`/
  `ResourceId{domain,kind,value}`）、`Rational`、`SourcePosition`/
  `SourceRange`、`SourceDigest`/`DefinitionDigest`（
  `"sha256:v1:<hex>"`）、`SourceRevision`、`ScriptSource`/
  `ScriptArtifact`/`ArtifactKind`/`ArtifactSource`/`ProducedBy`、
  `ScriptError`/`ScriptErrorKind`、`ScriptDiagnostic`/
  `DiagnosticSeverity`、`ScriptResult`。
- `src/canonical.rs` — `canonical_value`（authoritative 単一実装:
  record key は UTF-8 byte 順 sort・`Absent` field 省略・array 順序
  保持・string は JSON escape・`Time`/`ResourceId`/`Bytes`/非有限
  `Number` は codec と同一の `$rv` tag record 形）+
  `canonical_number` + `definition_digest`/`source_digest`/
  `sha256_hex`/`hex_encode`/`hex_decode`。
- `src/codec.rs` — `materialize`/`dematerialize`（`$rv` tag codec:
  `int`/`time`/`res`/`bytes`/`number`/`null`/`absent`）。未知 tag・
  非文字列 tag・field 過不足・内容不正は fail-closed `CodecError`。
  `Integer` は `|i| > 2^53` のみ tag 化。`F64_SAFE_I64`/`is_tagged`。
- `src/budget.rs` — `ExecutionBudget`（全 `Option`、`None`=無制限）、
  `BudgetKind`、`CancelGranularity`、`CancelToken`（`Arc<AtomicBool>`
  の cloneable 協調キャンセル）、`PlacementHint`。
- `src/capability.rs` — `ValueKind`（host signature 用中立型語彙）と
  `BackendCapabilities`（artifact_kinds/capability_ids/
  owner_thread_only/send/sync/shared_vm_state/cancel_granularity）。
- `src/host.rs` — `HostBinding`/`HostSignature`/`HostEffect`/
  `HostApiSurface`/`HOST_SURFACE_SCHEMA_VERSION`。
  `validate_neutral` が空名・予約 prefix（`os.`/`fs.`/`net.`/
  `network.`/`shell.`/`process.`/`env.`/`sys.`/`danger.`、大小文字
  非区別）・重複名を拒否。handler 実行 seam は契約に含めない（s2）。
- `src/session.rs` — `ScriptBackend`（`descriptor()` +
  `open_session(&SessionConfig)`）と `ScriptSession`
  （`evaluate`/`cancel`/`cancel_token`/`close`）。Send/Sync を無条件
  要求しない（配置は capabilities + `placement_hint` で表現）。
  `SessionError`（`UnsupportedBudget`/`InvalidHostSurface`/
  `SourceDigestMismatch`/`SourceTooLarge`/`LanguageMismatch`/
  `IsolationViolation`/`ResourceExhausted`/`Backend`）。
- `src/registry.rs` — `BackendDescriptor`/`ScriptRequirement`/
  `ScriptBackendRegistry`（`register`/`resolve`/`list`/`len`/
  `is_empty`）。resolve は fail-closed: `UnknownLanguage`→
  `UnknownBackend`→`NoCandidates`→`UnsupportedContractVersion`→
  `MissingCapability`→`AmbiguousBackend` の順で理由を区別。
  `contract_version == CONTRACT_VERSION && >= min_contract_version`。
- `src/analyzer.rs` — `ScriptAnalyzer`（`analyze(&self,…)` —
  内部可変性は実装責務）、`AnalyzerRegistry`（backend registry と
  同一 fail-closed 規則の `resolve` + `analyze` convenience が
  wanted-capability 検査と `verify_result_revision` stale 検出を
  内包）、`AnalyzeRequest`/`AnalyzeResult`/`Diagnostic`/`Completion`/
  `AnalyzerError`/`TypeDefinition`/`WantedCapabilities`/
  `DiagnosticLevel`。
- `tests/common/mod.rs` + `tests/{registry,codec,analyzer,host_surface,
  session}.rs` — deterministic fake backend/analyzer fixture
  （test 専用、本番公開面に fake なし・registry は空で出荷）と
  54 test。fail-closed 行列・codec 全 variant 往復・digest 同一性/
  mismatch・stale revision・host surface 中立性・session open/
  evaluate/cancel/close を網羅。
- `Cargo.toml`（root）`members` 追加、`crates/README.md` 索引行追加
  （計40クレート）、`Cargo.lock` 再生成（新規 package 1 件のみ）。
- `README.md`（crate）— 責務・非目標・依存記録。

### Decisions During Implementation

- **canonical 表現は手書き writer 1 箇所**: synvelis `canon.rs` の
  serde-derive 方式ではなく poc の手書き方式を採用（wire format 未決定、
  serde 型を公開署名に出さない — design D6）。`serde_json` は文字列
  escape のみに内部利用。
- **非有限 `Number` は tag record で canonical 化**: 引用符 sentinel は
  `String` と衝突するため `{"$rv":"number",…}` 形に変更（exp seq 2-3）。
- **`Integer`/`Number` の canonical 一致**: `Number(1.0)` と
  `Integer(1)` は同一 canonical text `1` → 同一 digest（設計どおり
  「同一意味は同一 digest」）。`>2^53` の `Integer` は typed canonical
  が正確な十進を保持し、codec tag 形とは意図的に異なる（文書化済みの
  唯一の例外）。
- **root `pub use` 再エクスポート**: intra-doc link 76 件切れを解消し、
  consumer path を `ruviform_script::Type` 形でも提供（exp seq 7-8）。
- **`ScriptSession`/`ScriptBackend` に Send/Sync を要求しない**: poc の
  `ScriptSession: Send` は PoC 形状。配置特性は `BackendCapabilities`
  （`send`/`sync`/`owner_thread_only`/`shared_vm_state`）+
  `placement_hint` で表現（design C2）。
- **`AnalyzerRegistry::analyze` は `&self`**: ffi-poc の `&mut self`
  は PoC 形状。内部可変性は実装責務（synvelis mirror 先例）。
- **fake fixture は `tests/` のみ**: `ScriptBackendRegistry::new()` は
  空で出荷、fake のプリ登録経路なし（test/fixture 専用の境界）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-692-script-port-exp.jsonl` — 実装過程の観察・
  判断記録（canonical NaN 衝突修正、decode_tag 集約、expect_err の
  Debug 制約、doc link 修復、Rational/host-version の境界判断など）。
- `docs/impl/exp/issue-692-script-port.md` — 採用/棄却の判断サマリ。

### Rejected

- serde derive canonical（wire format 未決定のため公開型に serde を
  露出しない）。
- `Rational::new` での den=0 拒否（PoC parity・契約は非解釈）。
- `validate_neutral` での schema_version 検査（backend 責務）。

## Validation

canonical 検証（`mise run`、Windows 11 x64・rustc 1.99.1 — 補助として
`cargo doc -p ruviform-script --no-deps` で intra-doc link 0 warning を確認）:

- `mise run fmt-check` — pass
- `mise run check ruviform-script` — pass（`--locked --all-targets`）
- `mise run lint ruviform-script` — pass（`-D warnings`、clippy pedantic
  clean）
- `mise run test ruviform-script` — pass（54 tests、0 failed）
- `mise exec -- prek run --all-files` — pass（全 hook）

証拠: `evidence/rf692/verification.md`

## Risks

- `$rv` 予約 key を user payload が使った場合、tag record と canonical
  が衝突し得る — 契約上 reserved であり `dematerialize` は fail-closed
  で拒否するが、consumer が規約を破った payload を作ると digest 衝突の
  余地がある（poc と同一の既知限界、文書化済み）。
- `HostApiSurface` の `schema_version` 検査は backend 責務 — contract
  層は命名中立性のみ保証。s2 backend 実装時に version 拒否を実装する
  必要がある。
- `CancelToken` は協調キャンセル — blocking host call は return まで
  中断不能（design どおり、consumer が結果を捨てる責務）。

## Follow-ups

- 実 Luau backend / Analyzer 製品 port（Issue #692 AC2 — s2 slice）。
- synvelis thin adapter 接続（synvelis#271/#272/#273/#274 gate）。
- Luau.Analysis native FFI 統合（#560 系）。
- mlua vendored vs native bridge の依存選定（s2 で RF-SCRIPT-02 の
  版整合・3OS 配布結果を参照 — 本 slice では選定しない、PR 本文に記録）。
- `*-full`/`*-msrv`/3OS 横断は CI lane 委譲。

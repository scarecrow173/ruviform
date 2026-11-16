---
id: "DESIGN-034ZkLmNpQrStUvWxYzCd2"
type: "design"
status: "approved"
title: "Issue #692 設計 — ruviform-script 中立契約 port crate（headless）"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/specs/issue-692-script-port.md"
    - "docs/designs/issue-558-script-contract-design.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-692-script-port.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-692-script-port.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-692-script-port.md"
    - "docs/designs/issue-558-script-contract-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-692-script-port.md"
    - "docs/discovery/issue-692-script-port.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-692-script-port.md"
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  defers: []
  deferred-by: []
---

# Issue #692 設計 — ruviform-script 中立契約 port crate（headless）

## Context

Issue #692 first slice。RF-SCRIPT-00 中立契約
（`docs/designs/issue-558-script-contract-design.md`、approved）の
design 完全形を、新規 crate `crates/ruviform-script` として headless に
実装する。実 Luau backend/Analyzer・synvelis 接続・依存選定は後続
slice / consumer gate。

移植元・参照元（全て READ-ONLY、merge 済み）:

- `poc/script-coexist/src/{contract,value,canonical,codec,analyzer,lib}.rs`
  — #559 PoC。design 完全形に最も近い意味論の移植元。独立 workspace
  であり crate 依存・コード共有はせず、意味論を port する。
- `ffi-poc/src/{contract,analyzer}.rs` — #560。analyzer 契約の FFI 側
  shape（`AnalyzeRequest.type_definitions`/`WantedCapabilities`/
  `TypeDefinition`/`Cursor`/`AnalyzerError`/`verify_result_revision`）。
- `G:/repositry/audio-products/synvelis/crates/synvelis-script/src/
  {backend,analyzer,canon}.rs` — cross-repo READ-ONLY。consumer 側
  shape の確認のみ（`analyze(&self)` 先例、`canonical_json`/
  `digest_of`/`sha256_hex` 先例）。synvelis 側は変更しない。
- `crates/ruviform-ui-theme/Cargo.toml` — crate 雛形先例
  （`*.workspace = true` 継承・`[lints] workspace = true`・依存理由
  コメント）。

## Scope

### In Scope

- crate 骨格: `Cargo.toml`（serde_json/sha2 のみ、理由コメント）、
  `src/lib.rs`（crate-level doc・module 公開・prelude）、`README.md`。
- C1 識別子/Registry・C2 Runtime Port・C3 値モデル・C4 Budget/Cancel・
  C5 Analyzer Port・C6 Host API 記述子の中立契約型一式。
- `canonical.rs`（canonical 表現 + digest utils）・`codec.rs`
  （`materialize`/`dematerialize`）。
- `tests/` fake fixture + 契約検証（fail-closed 行列・codec 往復・
  digest・analyzer・host surface）。
- workspace 登録・docs chain・`evidence/rf692/`。

### Out of Scope

- 実 backend/Analyzer 実装・外部依存追加・synvelis 接続・unsafe・
  実 VM/FFI・RT 経路・GUI（spec Out of Scope 参照）。

## Module Layout

```
crates/ruviform-script/
  Cargo.toml          — serde_json/sha2 のみ、[lints] workspace = true
  README.md           — 責務・層・依存境界・検証手順
  src/
    lib.rs            — crate doc・CONTRACT_VERSION・module 公開・prelude
    id.rs             — 文字列 ID newtype + SessionId + ScriptBackendVersion
    value.rs          — ScriptValue・位置/範囲・digest/revision・
                        ScriptSource・ScriptArtifact 系・ScriptError 系・
                        DiagnosticSeverity・ScriptDiagnostic・ScriptResult
    budget.rs         — BudgetKind・ExecutionBudget・CancelGranularity・
                        CancelToken・PlacementHint
    capability.rs     — ValueKind・BackendCapabilities(+capability_ids)
    host.rs           — HostEffect・HostSignature・HostBinding・
                        HostApiSurface(+validate_neutral)・HostSurfaceError
    session.rs        — SessionConfig・SessionError・ScriptBackend・
                        ScriptSession
    registry.rs       — BackendDescriptor・ScriptRequirement・
                        RegistryError・ResolveError(Reason)・
                        ScriptBackendRegistry
    canonical.rs      — canonical_value・definition_digest・source_digest・
                        sha256_hex・hex_encode/decode・CanonicalError・TAG_KEY
    codec.rs          — materialize・dematerialize・CodecError・F64_SAFE_I64
    analyzer.rs       — C5 analyzer port 一式
  tests/
    common/mod.rs     — fake backend/analyzer fixture（test 専用）
    registry.rs       — backend registry fail-closed 行列
    codec.rs          — codec 往復・canonical/digest
    analyzer.rs       — analyzer registry/analyze/stale
    host_surface.rs   — validate_neutral
    session.rs        — open_session/evaluate/cancel/close 契約
```

依存方向（crate 内）: `id` ← `value` ← `budget`/`capability`/`host`/
`session`/`registry`/`analyzer`、`canonical` ← `value`、
`codec` ← `value`+`canonical`。循環なし。

## 型設計の決定

### D1. design 完全形を採用（synvelis 簡約形は正本ではない）

`Time{domain: TimeDomainId, timebase: Rational, count: i64}`・
`ResourceId{domain: ResourceDomainId, kind, value}`・
`HostApiSurface`・`SessionConfig.placement_hint`・
`ScriptSource`/`ScriptArtifact`/`HostBinding` の `schema_version` を
保持する。synvelis ミラーの簡約形（`Time(i64)` 等）は consumer 都合の
簡約であり、port 後に consumer 側が整合する前提で design 形を
authoritative にする。

### D2. `ScriptBackendVersion` を分離

`{ impl_version: String, contract_version: u32 }` を `id.rs` に置き、
`BackendDescriptor.version` と `AnalyzerDescriptor.version` の双方が
使う。`contract_version` は本契約の版を表す `u32` で、crate 定数
`CONTRACT_VERSION: u32 = 1`（RF-SCRIPT-00 の初版）と一致することを
resolve が検査する。`impl_version` は semver/opaque 文字列。

### D3. Send/Sync を無条件に要求しない

`ScriptBackend`/`ScriptSession`/`ScriptAnalyzer` の trait に
`Send`/`Sync` supertrait を付さない。poc の `ScriptSession: Send` は
PoC 形状。配置特性は `BackendCapabilities`（`owner_thread_only`/
`send`/`sync`）が広告し、`SessionConfig.placement_hint` で consumer が
選択する。registry 自体も Send/Sync を要求しない（consumer が Mutex
等で包む）。

### D4. `ScriptResult.artifact` は `Option`

設計 C2 は `artifact: ScriptArtifact`（非 Option）と読めるが、値
モデル往復など型付き定義文書を生じない評価（FX-P-05/06 相当）を
表現できない。poc と同じく `Option<ScriptArtifact>` とし、
`return_value: Option<ScriptValue>`・`diagnostics: Vec<
ScriptDiagnostic>` と併せて「artifact を生じない評価」を正直に
表現する。design の非 Option 読みは artifact 必須の script 種別を
示唆するもので、契約型が全評価に artifact を強制すると fake/空
document が必要になるため採用しない。

### D5. SourceRange/SourcePosition の統合形

- `SourcePosition { offset: u64, line: u32, col: u32 }` — design C3 の
  offset/line/col 形。`line`/`col` は 0 始まり、`col` は行内 UTF-8
  byte offset（Luau `Position` と同じ意味論）。
- `SourceRange { start: SourcePosition, end: SourcePosition }` —
  poc の byte-offset range と ffi-poc/synvelis の position range を
  包含する統合形。`ScriptDiagnostic.range`・analyzer `Diagnostic.
  range`・`Completion.replace_range` は `Option<SourceRange>`。

poc の `SourceRange{start:u64,end:u64}`（byte offset のみ）では
editor 向け line/col が失われ、synvelis/ffi-poc の `{line,column}`
では絶対 offset が失われる。統合形は両者の情報を保持し、synvelis
adapter の写像が直接的。backend は line/col 既知・offset 未計算の
場合 source text から offset を計算する責務を負う（non-RT のため
許容）。

### D6. serde derive を公開契約型に付さない

canonical 表現の authoritative 実装は `canonical_value(&ScriptValue)
-> Result<String, CanonicalError>` 一箇所（poc `canonical.rs` の手書き
方式を port）。公開契約型へ serde derive を付さず、serde_json は
canonical 文字列 escape（`serde_json::to_string` on `String`/`&str`）
の内部利用に限定する。serde 型を公開シグネチャへ露出しない
（wire format は consumer が未決定 — YAGNI）。`canonical_json`/
`digest_of` の serde_json `Value` 経由方式（synvelis `canon.rs`
先例）は採用しない — `ScriptValue` 直結の手書き canonical が
`$rv` tag 意味論（Absent 省略・数値正規化・i64 10 進文字列）を
直接表現でき、tag record codec と canonical 形が一致する poc 設計を
維持できる。

### D7. codec は `$rv` tag record 方式 + 型付きエラー

`materialize(&ScriptValue) -> ScriptValue`・`dematerialize(ScriptValue)
-> Result<ScriptValue, CodecError>` を port。tag kind:
`"int"`（|i|>2^53 の i64 を 10 進文字列で厳密運搬）・`"time"`・
`"res"`・`"bytes"`（hex）・`"null"`・`"absent"`。`$rv` は予約 key で、
未知 tag・field 欠落/型不一致/parse 失敗・`$rv` 非文字列は
fail-closed `CodecError`。poc の String error を型付き
`CodecError{UnknownTag{tag}, MalformedTag{reason}}` へ格上げ。
`canonical_value` の失敗（トップレベル Absent）は `CanonicalError`。

### D8. analyzer 契約は ffi-poc/synvelis shape を採用 + `&self`

`ScriptAnalyzer::analyze(&self, request: &AnalyzeRequest) ->
Result<AnalyzeResult, AnalyzerError>`。ffi-poc の `&mut self` は PoC
形状（FFI ハンドルの可変状態）、synvelis mirror が `&self` 先例。
内部可変性は実装責務。

`AnalyzeRequest { source: ScriptSource, language: ScriptLanguageId,
type_definitions: Option<Vec<TypeDefinition>>, wanted:
WantedCapabilities, cursor: Option<Cursor> }`。design の
`type_definitions: Option<DefinitionSnapshotRef>` に対応させ、
`Option<Vec<TypeDefinition{package_name, source}>>` とする
（`None` = snapshot 未提供、`Some(vec![])` = 空 snapshot を区別）。

`AnalyzerCapabilities { diagnostics: DiagnosticLevel, completion: bool,
max_source_bytes: Option<u64>, supported_languages: Vec<
ScriptLanguageId> }` — `DiagnosticLevel{Unsupported,Syntax,
SyntaxAndType}` は能力段階の順序付き enum（`Ord` 導出で
`needed > supported` 比較が可能）。`max_source_bytes` は
`Option<u64>`（`None`=無制限 — `ExecutionBudget`/`BackendCapabilities`
と同じ語彙）。`supported_languages` は広告情報であり resolve の
key は `descriptor.language`（backend registry と同一規則）。

`AnalyzerRegistry::resolve` は C1 と同一 fail-closed 規則を analyzer
語彙で行う: `AnalyzerRequirement{language, analyzer,
required_capabilities, min_contract_version}` →
`AnalyzerResolveError{NoMatchingAnalyzer{reason, detail},
AmbiguousAnalyzer{candidates: Vec<AnalyzerId>}}`。
`AnalyzerResolveReason{UnknownLanguage, UnknownAnalyzer,
UnsupportedContractVersion, MissingCapability, NoCandidates}`。
`required_capabilities` は `BTreeSet<CapabilityId>` で、
`AnalyzerCapabilities::capability_ids()` が導出する広告 ID 集合
（`"analyze.diagnostics"`/`"analyze.types"`/`"analyze.completion"`）
の部分集合であることを検査する。registry の `analyze` 利便
メソッドは resolve + `verify_result_revision`（stale 検出 helper、
`AnalyzerError::StaleResult`）を行う — ffi-poc 先例どおり consumer
役の fail-closed 既定だが、Consumer 側の stale 拒否責務は契約上
consumer に残る（helper は提供のみ）。

`AnalyzerError { Unavailable, Unsupported{capability},
StaleResult, InvalidRequest{reason}, VersionMismatch{field, expected,
actual}, Internal{message} }` — ffi-poc 語彙から FFI 専用の
`Bridge{status,message}` を除いた形。`DuplicateAnalyzer` 登録拒否は
`AnalyzerRegistryError`（backend の `RegistryError` と対称の別型 —
poc の `AnalyzerId→ScriptBackendId` 変換 hack を解消）。

### D9. HostApiSurface は記述子のみ + `validate_neutral`

`HostBinding{name, signature: HostSignature{args: Vec<ValueKind>,
returns: ValueKind}, effect: HostEffect{Pure,Query,Effect},
schema_version, timeout_hint: Option<Duration>}`・
`HostApiSurface{bindings, schema_version}`。中立契約に OS/FS/Network/
Shell binding を含めない — `validate_neutral()` が禁止 prefix
（`os.`/`fs.`/`net.`/`shell.`/`process.`/`env.`/`sys.`/`danger.`）を
持つ binding 名を `HostSurfaceError::ForbiddenBindingName{name}` で
拒否する。`effect` は助言情報で権限根拠にしない（design C6）。

**handler 実行 seam は本 slice に含めない**: C6 は記述子のみ契約化
し、実 backend/consumer が必要とする呼出し transport（host call の
実際の dispatch 経路）は実 backend 実装時に確定する。poc の
`HostHandler`/`host_handlers`/`inputs` は PoC 拡張であり契約外。
`SessionConfig` は `host_bindings: HostApiSurface`（宣言面）のみを
持ち、backend は surface の `validate_neutral`・`schema_version` を
検査する責務を負う。seam の要否は s2 以降の実 backend slice で再検討
（Deferred Design Concerns に記録）。

### D10. SessionConfig/SessionError/ScriptSession

`SessionConfig { budget: ExecutionBudget, host_bindings:
HostApiSurface, artifact_kinds: BTreeSet<ArtifactKind>, source:
ScriptSource, placement_hint: Option<PlacementHint>, resume:
Option<SourceDigest> }`。`resume` は poc の F-08（Source digest
不一致検出）用で、design C2 の列挙にないが fail-closed 判定表 F-08
の `SessionError::SourceDigestMismatch` を backend が生成するための
期待 digest 入力として保持する（小さな追加で契約の完全性を保つ）。
`host_handlers`/`inputs`（poc 拡張）は契約外として除外。

`SessionError { UnsupportedBudget{kind}, InvalidHostSurface{reason},
SourceDigestMismatch{expected, actual}, SourceTooLarge{bytes, max},
LanguageMismatch{expected, actual}, IsolationViolation{detail},
ResourceExhausted{detail}, Backend{detail} }` — poc 語彙を port。

`ScriptSession` は Send/Sync 無条件要求なし（D3）。`session_id()`・
`backend_id()`・`evaluate(entry: Option<&str>)`・`cancel()`・
`cancel_token()`・`close()` を持つ。`CancelToken` は公開
`CancelToken::new()`/`Default` で生成可能な `Arc<AtomicBool>` 共有
フラグ（backend が session 内に保持し `cancel()`/`cancel_token()` で
共有 — 別スレッド evaluate 中の協調的 cancel を可能にする poc 設計を
維持。`from_flag` は公開しない）。

### D11. resolve アルゴリズム（C1 fail-closed を製品へ厳密化）

`ScriptBackendRegistry::resolve(&ScriptRequirement)`:

1. `language`/`backend` 指定で候補をフィルタ。
2. 要求 `language` が未登録 → `NoMatchingBackend{UnknownLanguage}`、
   要求 `backend` が未登録 → `NoMatchingBackend{UnknownBackend}`
   （理由区別のため既知判定を先に行う）。
3. `candidates` が空（既知 ID 同士の矛盾要求等）→
   `NoMatchingBackend{NoCandidates}`。
4. `contract_version == CONTRACT_VERSION && min_contract_version
   以上` の版互換フィルタ → 空なら `NoMatchingBackend{
   UnsupportedContractVersion}`。
5. `required_capabilities ⊆ capability_ids()` かつ
   `artifact_kinds ⊆ capabilities.artifact_kinds` の capability
   フィルタ → 空なら `NoMatchingBackend{MissingCapability}`。
6. 1 件 → `Ok`、2 件以上 → `AmbiguousBackend{candidates}`。

poc との差分: poc は `candidates` 空を `UnsupportedContractVersion`
に潰していたが、製品では `NoCandidates` を独立理由として区別する
（poc `ResolveErrorReason::NoCandidates` の語彙を活用 — 既知
language/backend の矛盾要求を version 問題と誤認しない）。
`artifact_kinds` の `ConsumerExtension{domain}` は domain 一致を
要求する（capability ID `artifact.consumer_extension` は domain 非
依存のため `caps.artifact_kinds.contains` で別途検査 — poc と同じ
二重検査を維持）。

`AnalyzerRegistry::resolve` は同一手順を analyzer 語彙で行う
（D8）。

### D12. canonical/digest 規則（poc を port・エラー型のみ格上げ）

- `canonical_value(&ScriptValue) -> Result<String, CanonicalError>`:
  Record は key を UTF-8 byte 順ソート・`Absent` field は省略、
  `Time`/`ResourceId`/`Bytes` は codec と同じ `{"$rv":…}` tag 構造を
  canonical key で展開、`Number` は `canonical_number`（整数値 f64
  → 整数 10 進・-0.0→0・NaN/±Inf 特別形）、`Integer` は 10 進、
  String は JSON escape（serde_json）。トップレベル `Absent` は
  `CanonicalError::Absent`。
- `definition_digest(&ScriptValue) -> Result<DefinitionDigest,
  CanonicalError>`: canonical bytes の sha256 を `"sha256:v1:<hex>"`
  形式で返す。
- `source_digest(&str) -> SourceDigest`: 同形式。
- `sha256_hex(&[u8]) -> String`・`hex_encode`/`hex_decode`（codec の
  bytes tag と共有する公開 utility）。
- invariant: `canonical_value(v) == canonical_value(materialize(v))`
  は `Integer(|i|>2^53)` を除き成立（大きな Integer は typed canonical
  では 10 進数値、materialize 後は `{"$rv":"int"}` tag — codec は
  vendor 境界表現であり canonical 空間では typed 形が正本）。
  codec 往復は `dematerialize(materialize(v)) == v`（全 variant）を
  保証し、payload digest は typed `ScriptValue` 上で計算する。

### D13. 品質契約

- `#![forbid(unsafe_code)]`（poc 先例より強い禁止 — 本 crate は
  unsafe を一切含まない）。
- 全公開 enum に `#[non_exhaustive]`（`exhaustive_enums` workspace
  lint + `docs/adr/public-enum-non-exhaustive-policy.md`）。
- `BackendCapabilities` は bool flag 5 個を持つため
  `clippy::struct_excessive_bools` を reason 付き `#[expect]` で局所
  許可（capability 広告は本質的に bool 集合 — bitflags 化は語彙を
  損なう）。`canonical_number` の f64 比較は `clippy::float_cmp` を
  reason 付き `#[expect]`（canonical 正規化は bit 級の IEEE-754
  意味論を要求 — NaN/±Inf/整数判定/`-0.0` は意図的な厳密比較）。
  その他の pedantic lint は `#[expect]` + reason で局所対応し、
  `#[allow]` は使わない。
- 依存は serde_json `"1"`・sha2 `"0.10"` のみ（Cargo.lock 既在 —
  `ruviform-plugin-dock-test`・`poc/script-coexist` 先例。
  serde_json は canonical 文字列 escape の内部利用、sha2 は digest
  計算。ともに MIT/Apache-2.0 系、公開 API に serde 型を露出しない）。

### D14. fake fixture 境界

fake backend/analyzer は `tests/common/mod.rs` に置く deterministic
test double で、`ScriptBackend`/`ScriptAnalyzer` trait の実装として
契約面を検証する。本番経路（`ScriptBackendRegistry::new()`・
`AnalyzerRegistry::new()`）に fake をプリ登録する公開 API・feature・
モジュールは作らない。`#[cfg(test)]` は integration test から見えない
ため `tests/common` パターンを採用（各 test file が `mod common;`
で共有）。

### D15. workspace 配置

`docs/designs/ruviform-architecture-design-baseline.md` の層構造に
照らし、`crates/README.md` の「利用側層」セクションへ配置する
（engine 非依存・consumer が実装する契約面を提供する中立 port で、
workspace 内 path 依存を持たない）。`Cargo.toml` `members` には
責務コメント付きで追加。

## Fail-Closed 判定の対応（design C1 表 → 本実装）

| # | 条件 | 検出点 | 本実装の結果 |
| --- | --- | --- | --- |
| F-01 | 未知 language | `ScriptBackendRegistry::resolve` | `NoMatchingBackend{UnknownLanguage}` |
| F-02 | 未知 backend | 同上 | `NoMatchingBackend{UnknownBackend}` |
| F-03 | 候補 0 件 | 同上 | `NoMatchingBackend{NoCandidates}` |
| F-04 | 候補 2 件以上 | 同上 | `AmbiguousBackend{candidates}` |
| F-05 | `contract_version` 不一致 | `resolve` | `NoMatchingBackend{UnsupportedContractVersion}` |
| F-06 | 必須 Capability 未対応 | `resolve`/`open_session` | `NoMatchingBackend{MissingCapability}` / `SessionError::UnsupportedBudget` |
| F-12 | i64 非表現整数の越境 | `dematerialize`/backend | `CodecError` / `ScriptError::ValueOutOfRange`（backend 責務） |
| F-13 | Analyzer Capability 未実装 | `analyze` | `AnalyzerError::Unsupported{capability}` |
| F-14 | Analyzer 結果の stale revision | `verify_result_revision`/registry `analyze` | `AnalyzerError::StaleResult` |
| F-15 | RT パスからの呼出し | 呼出し側責務 | 契約違反（non-RT 保証 — 検知機構は作らない） |
| F-16 | Session 間状態漏洩 | `shared_vm_state` 未広告 | `SessionError::IsolationViolation`（backend 責務） |
| F-18 | Analyzer 未登録 | `AnalyzerRegistry::resolve`/`analyze` | `AnalyzerResolveError::NoMatchingAnalyzer` / `AnalyzerError::Unavailable` |

F-07/F-09/F-10/F-11/F-17 は Consumer/Plan 検証側の責務（本 crate は
Artifact の形と `schema_version` を保持するのみ — 検証ロジックは
consumer）。F-08 は `SessionConfig.resume` + `SessionError::
SourceDigestMismatch` で backend が生成可能。M-01 は
`HostApiSurface::validate_neutral`、M-02/M-03/M-04/M-09/M-11 は
`CancelGranularity`/`ExecutionBudget`/`ScriptErrorKind::
BudgetExceeded`/`Cancelled` の語彙で表現（強制は backend 責務）。

## AC4 記録（正直な範囲）

本 slice は中立契約のみで実 backend/Analyzer を含まない。
**mlua vendored vs native bridge の依存選定は s2 slice で
RF-SCRIPT-02（ffi-poc）の版整合・3OS 配布結果を参照して決定する。
s1 では選定を行わない。** `docs/adr/` は本 slice の lease 外のため、
本節と spec・PR 本文に記録する（選定済みのように装飾しない）。

## Deferred Design Concerns

- 実 Luau backend/Analyzer の製品 port（AC2）・依存選定 — s2 以降。
- synvelis thin adapter 接続（AC3）— synvelis#271/#272/#273/#274。
- `synvelis-automation` `AnalyzerRegistry` の本 crate への移行 —
  consumer 側後続。
- Host binding handler 実行 seam — 実 backend 必要時に確定。
- `ScriptSession` の Send/Sync 提供形態 — backend 実装依存、
  `BackendCapabilities` で広告（design C2 Deferred と同じ）。
- 3OS/`*-full`/`*-msrv` — CI lane 委譲。

## Verification Notes

- [ ] `mise run check|lint|test ruviform-script`・`fmt-check`・
  `prek run --all-files` が Pass（headless unit/integration、
  Windows ローカル — macOS/Linux は CI lane NotRun）。
- [ ] fake fixture で registry fail-closed 行列・codec 往復・digest・
  analyzer・host surface が検証される。
- [ ] 実 Luau VM/FFI・実 OS・GUI 経路は本 slice 非含有 — NotRun/
  Deferred（所有 gate: s2 slice・synvelis#271〜#274・CI lane）。

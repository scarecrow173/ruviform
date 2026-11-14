# RF-SCRIPT-01 PoC Design — Luau+Rhai 単一バイナリ共存

spec: `poc/script-coexist/docs/spec.md`。契約正本:
`docs/designs/issue-558-script-contract-design.md`（以下「契約設計」）。
本書の型名・Fixture 語彙は契約設計 C1〜C7・Fail-Closed 表・Fault Matrix
に従う。**本 PoC の型は契約のミラーであり、本番 API を意味しない。**

## コンポーネント

```
poc/script-coexist/            (独立 workspace — ルート members に入れない)
  Cargo.toml                   mlua{luau} / rhai / serde_json / sha2
  src/
    lib.rs                     公開面（contract ミラー + backends + plan + analyzer）
    contract.rs                C1/C2/C4/C6 の型（Id/Registry/Session/Budget/Host）
    value.rs                   ScriptValue/ScriptError/ScriptSource/ScriptArtifact
    canonical.rs               canonical serialization + sha256 digest
    plan.rs                    WorkflowDefinition 検証（F-09/10/11/17）
    analyzer.rs                ScriptAnalyzer/AnalyzerRegistry ミラー + SyntaxCheckAnalyzer
    consumer.rs                consumer 側責務の模擬（artifact store・freshness・host surface 検査）
    backends/mod.rs            ScriptBackend trait + descriptor
    backends/luau.rs           mlua Luau backend + value codec
    backends/rhai.rs           rhai backend + value codec
  src/main.rs                  証拠出力用 CLI（fixture sweep → JSONL 行）
  tests/fixtures.rs            FX-P/N fixture テスト
docs/{spec,design,plan,task}.md 本内部文書チェーン
```

独立 workspace とする理由: `poc/ui`・`poc/plugin-gui` の先例どおり、
重い native 依存（vendored Luau C++）をルート workspace/CI の
`--workspace` build 対象から隔離する。ルート `Cargo.toml`/`Cargo.lock`
は変更しない。

## C1 Registry/Dispatch（契約設計 §C1 のミラー）

- `ScriptBackendRegistry { backends: Vec<Box<dyn ScriptBackend>> }`。
  `register` は `ScriptBackendId` 重複を `RegistryError::DuplicateBackend`
  で拒否。`resolve(&ScriptRequirement)` は契約の 5 手順をそのまま実装:
  language/backend フィルタ → contract_version/capabilities/artifact_kinds
  検査 → 0 件 `NoMatchingBackend{reason}`（UnknownLanguage/
  UnknownBackend/UnsupportedContractVersion/MissingCapability を区別）→
  1 件選択 → 2 件以上 `AmbiguousBackend`。フォールバックなし。
- `contract_version` は PoC 契約版 `1`。`ScriptBackendVersion{impl_version,
  contract_version}` を descriptor が保持。
- `BackendCapabilities` は値種別・強制 Budget 種別・配置特性
  （`owner_thread`/`send`/`sync` フラグ）・`max_source_bytes`・
  artifact 種別・`cancel_granularity`・`shared_vm_state`・
  `native_integer`（backend が i64 を VM ネイティブで保持できるか、
  情報提供用）を持つ。

## C2 Session/Runtime Port

- `trait ScriptBackend { descriptor(); open_session(&SessionConfig)
  -> Result<Box<dyn ScriptSession>, SessionError> }`。
  vendor 型をシグネチャに出さない。
- `trait ScriptSession { session_id(); evaluate(entry: Option<&str>)
  -> Result<ScriptResult, ScriptError>; cancel(); close(); }`。
  Send/Sync 境界は持たない（配置は capabilities が広告）。
- `SessionConfig { budget, host_bindings: HostApiSurface,
  host_handlers: BTreeMap<String, HostHandler>, artifact_kinds,
  source: ScriptSource, resume: Option<SourceDigest>,
  placement_hint }`。`host_handlers` は PoC 拡張（実装は consumer
  責務であることを示すモック。中立契約の `HostApiSurface` は記述子のみ）。
- `open_session` の Fail-Closed: 要求 Budget 種別 ⊄ 広告 →
  `SessionError::UnsupportedBudget`。unsafe host binding 名 →
  `SessionError::InvalidHostSurface`。`resume` digest ≠ source.digest →
  `SourceDigestMismatch`。source bytes > `max_source_bytes` →
  `SourceTooLarge`。source.language ≠ backend.language →
  `LanguageMismatch`。
- `evaluate` は chunk を load → 実行 → 戻り値を pull codec で
  `ScriptValue` 化 → `{kind, definition}` Record 規約で `ScriptArtifact`
  を構成（`schema_version=1`、digest=canonical(payload) の sha256）。

## C3 値モデルと codec（本 PoC の中核判断）

`ScriptValue` は契約の閉じた variant 集合をそのまま実装
（Absent/Null/Bool/Integer(i64)/Number(f64)/String/Bytes/Array/Record/
Time{domain,timebase,count}/ResourceId{domain,kind,value}）。

VM 越境は backend 内部の **codec** が担い、公開面には出さない。
codec は backend ごとに VM の最良表現を使い、ScriptValue 境界で
意味を保存する:

| ScriptValue | Luau (mlua) | Rhai |
| --- | --- | --- |
| Absent | 値として push 不可（Record では key 省略） | 同左 |
| Null | tag `{"$rv":"null"}`（Record 内で nil=欠落と区別するため） | tag 同様（`()` との混同を避ける） |
| Bool | bool | bool |
| Integer(i64) | `|v| ≤ 2^53` → number、超過 → `ValueOutOfRange` | INT（i64 ネイティブ・全域） |
| Number(f64) | number | FLOAT |
| String | string（UTF-8 strict） | String |
| Bytes | tag `{"$rv":"bytes","hex":"…"}`（byte 列を hex 化） | Blob（native `Vec<u8>`） |
| Array | integer key 1..=n の table | Array |
| Record | string key table | Map（key は String のみ） |
| Time | tag `{"$rv":"time","domain","timebase":{"num","den"},"count"}`（i64 は 10 進 string） | 同 tag |
| ResourceId | tag `{"$rv":"res","domain","kind","value"}` | 同 tag |

- tag は `{"$rv": <kind>, …}` 形式。`$rv` を含む Record の pull は tag
  として decode し、未知/不正 tag は `ScriptError::Type` で Fail Closed。
  tag 内の i64 は 10 進文字列で運び、parse 失敗・範囲外は
  `ValueOutOfRange`。
- pull 方向: VM の function/thread/coroutine/userdata/fnptr/blob 以外の
  独自型 → `ScriptError::Unsupported`（VM 固有型の越境禁止）。
- Integer の非対称は**意図的**: Luau は f64 のみなので `|v|>2^53` を
  `ValueOutOfRange` で拒否（F-12/FX-N-12 の実測点）。Rhai は i64 を
  保持。pull で Luau number → `Number`（整数への黙った矯正をしない）、
  Rhai INT → `Integer`。
- `capabilities.native_integer` でこの差を広告する。

## C3' canonical form / DefinitionDigest

`canonical.rs` が `ScriptValue` を決定的バイト列へ直列化する:

- Record: key を UTF-8 バイト順ソート、`"key":value` を `,` 連結、`{}` 囲み。
- Array: `[v,…]`。String: JSON string escape（serde_json 準拠の最小実装）。
- Bool: `true`/`false`。Null: `null`。Absent: 直列化対象外（Record では
  key 自体を出さない）。
- Integer: 10 進。Number: `v==trunc && |v|≤2^53` → 整数 10 進に正規化
  （`Number(2.0)` と `Integer(2)` を同一 canonical に揃え、backend 間の
  数値表現差を吸収 — 契約の「数値正規化」）。それ以外は `{:e}` 最短表現。
  NaN→`nan`、±Inf→`inf`/`-inf`、-0.0→`0`。
- Time/ResourceId/Bytes: tag と同じ構造を canonical key で展開。
- `DefinitionDigest` = `"sha256:v1:" + hex(sha256(canonical(payload)))`。
  `SourceDigest` も同形式で source text を hash。`SourceRevision` は u64。

## C4 Budget/Cancel

`ExecutionBudget` は契約どおり全項目 `Option`。backend 広告:

- Luau: `max_instructions`（`set_interrupt` の safepoint 訪問回数 —
  厳密な VM 命令数ではないことを evidence に明記）、`max_wall_time`
  （interrupt 内で経過時間検査）、`max_memory_bytes`
  （`set_memory_limit(used+budget)` → `MemoryError`）、
  `max_output_bytes`（artifact payload canonical サイズ検査）、
  `max_host_calls`（handler 呼出カウンタ）。
- Rhai: `max_instructions`（`Engine::set_max_operations` — operation
  数）、`max_wall_time`/`cancel`（`on_progress` コールバックで
  flag/経過時間検査）、`max_output_bytes`、`max_host_calls`。
  `max_memory_bytes` は**非対応**（rhai に総メモリ上限 API なし）→
  要求時 `UnsupportedBudget`（FX-N-06 の非対称実測点）。
- cancel は冪等・協調的。blocking host call 中の即時中断は保証しない
  （M-02/M-04）— FX-N-19 で「cancel 後も host return まで待ち、結果を
  破棄して close」を実測記録する。
- `ensure_non_rt()` を全公開 API 入口で呼ぶ。PoC では TLS フラグ
  `poc_simulate_rt_context()` で RT 経路を模擬し、呼出しを
  `ScriptError::ContractViolation` で拒否（FX-N-15。debug assert 相当の
  契約違反検出として記録）。

## C5 Analyzer（最小ミラー）

- `AnalyzerRegistry` は `ScriptBackendRegistry` と独立
  （register/resolve 同一の Fail-Closed）。
- `SyntaxCheckAnalyzer`: capabilities `diagnostics=Syntax`、
  `completion=false`。analyze は VM の compile/load のみを実行し
  （評価しない）、構文エラーを `source_revision` 付き
  `Diagnostic` へ写像。completion 要求 →
  `AnalyzerError::Unsupported{completion}`（FX-N-13）。
- 未登録 → `AnalyzerError::Unavailable`（FX-N-24）。stale revision は
  consumer 側 `check_result_freshness` が `StaleResult` で拒否
  （FX-N-14）。

## C6 Host API / consumer 模擬

- `HostApiSurface::validate_neutral()` が binding 名 prefix
  `os.`/`fs.`/`net.`/`shell.`/`process.`/`env.` を拒否（FX-N-18。
  実運用では consumer ポリシー層の責務 — PoC では模擬実装）。
- `effect` は助言情報として保持するのみ。
- PoC の mock handler: `host.echo(v)`、`host.put_artifact(rec)` /
  `host.get_artifact(digest)`（consumer 側 `ArtifactStore` への typed
  port — FX-P-04 の混成参照経路）、`host.sleep_ms(n)`（FX-N-19）。

## C7 plan.rs — 型付き文書検証（consumer 責務の PoC 模擬）

`validate_workflow_definition(payload, known_artifacts)`:

- payload は Record、`steps` は Array、各 step は `{id, ref:{kind,digest}}`。
- ref digest が `known` に無い/ kind 不一致 → `PlanError::UnknownReference`。
- payload 内を再帰走査し `vm` 系 direct-reference marker
  （`$rv:"vm.direct"`、key `vm_handle`/`vm_ptr`/`native_handle`）→
  `PlanError::CrossVmReference`（F-09。VM ハンドルは ScriptValue で
  表現不能なため、marker 自体を拒否する形で実装）。
- step→ref の参照グラフを DFS し循環 → `PlanError::CycleDetected`。
- `verify_artifact`: `schema_version != 1` → `UnsupportedSchemaVersion`、
  payload 内の `grant`/`secret`/`token`/`apikey`/`password`（key を
  lowercase 正規化）→ `ForbiddenField`、digest 再計算不一致 →
  `DigestMismatch`。

## 依存

| crate | version | 用途 | ライセンス |
| --- | --- | --- | --- |
| mlua | 0.12.2 (`luau`) | Luau VM（vendored `luau0-src` で C++ 同梱 build） | MIT |
| rhai | 1.27.0 | Rhai VM（pure Rust） | MIT OR Apache-2.0 |
| serde_json | 1 | string escape / 証拠 JSON | MIT OR Apache-2.0 |
| sha2 | 0.10 | SourceDigest/DefinitionDigest | MIT OR Apache-2.0 |

MLua `send` feature は不要（session を thread 間で共有しない設計。
並行実証は各 thread が自前の registry/session を持つ）。

## Feature/GPU/OS 依存

- feature: default のみ。排他 feature なし（両 backend は常時登録）。
- GPU: 不使用。OS 依存: Windows で mlua-sys が vendored Luau を MSVC で
  build（VS2022 BuildTools 確認済み）。macOS/Linux は本環境で NotRun、
  OS 依存として vendored C++ build が必要な点を evidence に記録。

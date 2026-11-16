# Issue #692 — ruviform-script 中立契約 port 検証記録

- Issue: https://github.com/scarecrow173/ruviform/issues/692（first slice:
  RF-SCRIPT-00 中立契約 crate 新設。Issue は継続 open — 本 slice は
  AC2 実 backend / AC3 synvelis 接続を含まない）
- 対象: `crates/ruviform-script`（新設、headless・vendor 中立契約 crate）
- 環境: Windows 11 x64 / rustc 1.99.1（mise pin）/ cargo workspace
- 実施日: 2026-11-16
- canonical 検証はすべて `mise run` 経由。cargo 直接実行は補助のみ。

## 実行した検証（canonical = mise）

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass（`cargo fmt --all -- --check` 差分なし） | 0 |
| `mise run check ruviform-script` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run lint ruviform-script` | pass（`cargo clippy … -D warnings`、pedantic warning 0） | 0 |
| `mise run test ruviform-script` | pass（54 tests + 0 doctests、0 failed） | 0 |
| `mise exec -- prek run --all-files` | pass（trim/eof/yaml/toml/line-ending/BOM/private-key/illegal-names/actionlint 全 hook） | 0 |

補助（canonical ではない）:

- `cargo doc -p ruviform-script --no-deps` — intra-doc link warning 0
  （root `pub use` 追加で解消済みを確認）。
- `cargo clippy -p ruviform-script --all-targets` — warning 0
  （mise lint と同一ゲートの事前確認）。

## Test case 一覧と結果（`mise run test ruviform-script`、54件全 pass）

- `tests/registry.rs`（13）: duplicate 登録拒否・unique resolve・
  `UnknownLanguage`/`UnknownBackend`/`NoCandidates`/
  `UnsupportedContractVersion`/`MissingCapability` 各 fail-closed・
  `AmbiguousBackend`（候補 2 件列挙）・capability+artifact kind 充足
  による一意選択・`min_contract_version` フィルタ・list/len/is_empty。
- `tests/codec.rs`（14）: `materialize`→`dematerialize` 全 variant
  往復（`Time{domain,timebase,count}`/`ResourceId`/`Bytes`/非有限
  `Number`/`>2^53 Integer` 含む）・未知 tag/非文字列 tag/field 過不足/
  不正 hex・不正 int・不正 number spelling の fail-closed・canonical
  == materialized canonical（`2^53` 安全範囲）・`Absent` record field
  省略・`Number(1.0)`≡`Integer(1)` digest 一致・意味差 payload の
  digest 不一致。
- `tests/analyzer.rs`（9）: duplicate 登録拒否・unique resolve・
  fail-closed 行列（UnknownLanguage/UnknownAnalyzer/
  UnsupportedContractVersion/MissingCapability/AmbiguousAnalyzer）・
  `analyze` happy path・stale `source_revision` 拒否・wanted
  capability 未充足拒否・`verify_result_revision` 単体・
  `analyze(&self)` 非 `&mut` 呼出し。
- `tests/host_surface.rs`（5）: 中立名受理・`os.`/`fs.`/`net.`/
  `shell.` 等予約 prefix 拒否（大小文字非区別）・空名・重複名拒否・
  `HostApiSurface::new`/`validate_neutral`。
- `tests/session.rs`（13）: open happy path・evaluate 既定/設定結果・
  設定 error・`LanguageMismatch`/`UnsupportedBudget`/`SourceTooLarge`/
  `InvalidHostSurface`/`SourceDigestMismatch` 各 fail-closed・
  resume digest 一致受理・supported budget 受理・協調 cancel
  （idempotent・clone token）・session_id 一意・close 冪等。

## 品質契約の確認

- `unsafe`: `src/lib.rs` に `#![forbid(unsafe_code)]`。grep で crate
  内 `unsafe` 使用 0 を確認。
- vendor 型: 依存は `serde_json` + `sha2` のみ（Cargo.lock 既在）。
  `mlua`/`rhai`/FFI/consumer 具象型は署名・依存ともに非含有
  （`luau`/`rhai` は test fixture の言語 ID 文字列と doc 参照のみ）。
- 実 VM/FFI/OS binding: 非含有。`HostApiSurface` は記述子のみで
  handler 実行 seam なし。`os.`/`fs.`/`net.`/`shell.` 等の予約 prefix
  を `validate_neutral` が拒否。
- 公開 enum: 全て `#[non_exhaustive]`（`clippy::exhaustive_enums`
  warn→`-D warnings` ゲート通過）。
- fake backend/analyzer: `tests/common` のみ。`ScriptBackendRegistry::
  new()`/`AnalyzerRegistry::new()` は空で出荷、fake プリ登録経路なし。

## NotRun / Deferred（正直な記録）

- **実 Luau backend / Analyzer 製品 port（Issue #692 AC2 本体）**:
  s2 slice Deferred。本 slice は中立契約 + fake fixture のみ。
- **synvelis thin adapter 接続（AC3）**: synvelis#271/#272/#273/#274
  consumer gate 所有。cross-repo 参照は READ-ONLY のみ実施。
- **Luau.Analysis native FFI 統合（#560 系）**: s2 以降 Deferred。
- **mlua vendored vs native bridge の依存選定（AC4）**: s2 slice で
  RF-SCRIPT-02（ffi-poc）の版整合・3OS 配布結果を参照して決定。
  本 slice では選定を行わず、PR 本文に記録（`docs/adr/` 新設は
  lease 外のため PR 本文記録に留めた）。
- **`*-full` / `*-msrv` / Feature Matrix / 3OS 横断**: タスク固有表に
  より本 slice の必須検証対象外。Daily Full CI / 対象 SHA の CI lane
  委譲。
- **macOS/Linux build**: 本環境（Windows）では実行不可。pure-Rust
  契約 crate で OS 依存はないが、3 OS 実測は CI lane で実施。
- **実 OS・実 VM・GUI 経路**: consumer gate（synvelis 側 Issue）に委譲。
- **doc test**: 公開 API 例の doctest は本 slice では未追加
  （`cargo test` doc-tests 0 件）。crate-level doc は link 検証済み。

## 生成物

- `Cargo.lock`: 新規 workspace member `ruviform-script`（deps:
  `serde_json` + `sha2`、いずれも lock 既在 version）の
  `[[package]]` 1 件追加のみ。外部依存の新規追加なし。
- docs index（`docs/**/README.md`）: 一切未編集・未再生成
  （Coordinator が統合時に再生成）。

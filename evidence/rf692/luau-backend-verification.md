# Issue #692 s2 — ruviform-script-luau 検証記録

- Issue: https://github.com/scarecrow173/ruviform/issues/692（s2 slice:
  RF-SCRIPT-03 実 Luau analyzer backend。Issue は継続 open — 本 slice は
  AC2 の analyzer 側のみ。runtime backend / AC3 synvelis 接続を含まない）
- 対象: `crates/ruviform-script-luau`（新設）+ workspace 配線
  （root `Cargo.toml` / `Cargo.lock` / `crates/README.md`）
- 環境: Windows 11 x64 / rustc 1.99.1（mise pin）/ MSVC / cargo workspace
- 実施日: 2026-11-17
- canonical 検証はすべて `mise run` 経由。cargo 直接実行は補助のみ。

## 実行した検証（canonical = mise）

| コマンド | 結果 | exit |
| --- | --- | --- |
| `mise run check ruviform-script-luau` | pass（`cargo check --locked --all-targets`） | 0 |
| `mise run lint ruviform-script-luau` | pass（`cargo clippy … -D warnings`、warning 0） | 0 |
| `mise run test ruviform-script-luau` | pass（48 tests + 0 doctests、0 failed） | 0 |
| `mise run fmt-check` | pass（`cargo fmt --all -- --check` 差分なし） | 0 |
| `mise exec -- prek run --all-files` | pass（全 hook） | 0 |

補助（canonical ではない）:

- `mise exec -- cargo check -p ruviform-script-luau`（`--locked` なし）—
  新規 workspace member の `Cargo.lock` エントリ生成に使用。
  lock 差分は `[[package]] ruviform-script-luau` 追加 11 行のみで、
  既存依存 version の変更なしを `git diff Cargo.lock` で確認。

## Test case 一覧と結果（`mise run test ruviform-script-luau`、48件全 pass）

### lib unittests（16）— `src/`

- `analyzer::tests`（7）: `position_offset` の改行意味論
  （Luau Lexer 規約どおり `\n` のみ — `\r` は空白扱いで単独 `\r` は
  改行にならず、`\r\n` は `\n` で 1 回計数。shim `offsetToPosition`
  との一致を両側で固定）、malformed JSON → `Internal`（panic
  しない）、parse 可能だが schema 違反の JSON（非 object /
  `diagnostics` 非 array / completion 要求時 `completions` 非 array）
  → `Internal`（fail-visible）、未知 severity → Error 写像、
  completion kind 写像・label 必須、diagnostic range 再構成。
- `ffi::create_negotiation_tests`（2）: ABI version 不一致 →
  `VersionMismatch`、struct_size 不一致 → reject（create fail-closed）。
- `ffi_boundary_tests`（7）: `bridge_versions` が Luau revision を
  報告 + 報告 version と `vendor/luau/REVISION` tag の一致検査
  （vendor 再 pin 時の shim 更新漏れを fail-visible 化）、
  invalid UTF-8 byte 列 reject、cursor 範囲外 reject、
  source size 上限 enforce、completion disabled → status 5、
  thread mismatch reject（shim 境界の生 fixture — PoC 移植）。

### `tests/bridge.rs`（22）— 製品契約統合

- descriptor が中立契約を広告（language=`luau`、analyzer_id=
  `luau-analysis`、contract_version=CONTRACT_VERSION、impl_version に
  `luau0.740` 含有）。
- 正常 source → `AnalyzeResult` に `source_revision` エコー +
  `verify_result_revision` 受理、completion 未要求 → 空 Vec、
  Error 級 Diagnostic なし。
- 構文 error → `code="parse"`・severity=Error・SourceRange 再構成。
- strict mode で型注釈違反 → Error diagnostic、nonstrict では非 Error
  （options が shim へ実効している証拠）。
- completion: `al` prefix → `alpha` 候補・`replace_range` 付き、
  enable_completion=false → `Unsupported{analyze.completion}`、
  cursor=None → `InvalidRequest`。`wanted.completion=false` に
  cursor が付いた request は diagnostics のみの成功結果
  （native completion を起動しない — review cycle 1 P1 の
  regression test）。
- 型定義: request 同梱 `TypeDefinition` で未知グローバル解決
  （Error → 非 Error）、`add_type_definition` 永続化 + package_name
  dedup、completion 候補への typedef 反映。同名 package の
  `add_type_definition` 再定義は Luau の global binding 上書きに
  従う last-write-wins（型注釈 mismatch で実測固定）。
- schema_version 不一致（source / type_definitions /
  type_definitions.source）→ `VersionMismatch{field}` fail-closed。
- cursor 範囲外・UTF-8 char-boundary 外・`max_source_bytes` 超過 →
  `InvalidRequest`。非対応言語 → `InvalidRequest`。
- 同一 request の determinism、stale revision の consumer 側拒否
  （`verify_result_revision` → `StaleResult`）。
- `\r\n`/単独 `\r` 混在 source の diagnostic offset end-to-end
  round-trip（shim emit の line/col → `position_offset` → byte
  offset が Luau Lexer 規約と一致することを実測）。
- `module_time_limit_sec` 超過 → `timeoutHits` を shim が検出し
  部分診断を成功返却せず `Internal` で fail-closed（実測:
  `MIN_POSITIVE` limit + 200 locals で timeout 発火）。
  負値/NaN/`-inf` の `module_time_limit_sec` → connect 時
  `InvalidRequest`。
- mlua VM（`return 20 + 22` → 42）と Analysis が同一バイナリで動作
  （AC4 共存実証）。
- `AnalyzerRegistry` への実 analyzer 登録→解決→解析の疎通。

### `tests/faults.rs`（1）— fault injection seam

- `RF_LUAU_FAIL_INIT` → `connect` が `Internal{status 7}`、
  `RF_LUAU_FAIL_ANALYZE` → `analyze` が同、`RF_LUAU_FAIL_LOADDEF` →
  `add_type_definition` が同。注入解除後に正常系へ復帰。
  環境変数はプロセス全体のため単一 #[test] に集約。

### `tests/registry.rs`（9）— registry fail-closed 行列

- 空 registry → `Unavailable` / `UnknownLanguage`。
- 登録→resolve→analyze end-to-end、duplicate analyzer_id 拒否。
- 複数 instance は hint 必須（`AmbiguousAnalyzer` → analyze 経路では
  `Unavailable{ambiguous}`）、hint 指定で一意解決。
- 未登録言語・未登録 analyzer hint → `UnknownLanguage`/
  `UnknownAnalyzer`。
- contract_version ゲート（stub v0・min_contract_version 超過）→
  `UnsupportedContractVersion`。capability ゲート
  （`analyze.completion` 要求 vs enable_completion=false）→
  `MissingCapability` / `Unsupported`。
- wanted ⊄ caps → registry.analyze が `Unsupported` で事前拒否。
- stub の stale `source_revision` → `StaleResult`。

## レビュー記録（review-protocol）

- Cycle 1: Reviewer A（cooperative）+ Reviewer B（adversarial）の
  独立レビューで P1×1 / P2×2 / P3×複数を検出。CONFIRMED 分を修正:
  - P1: `wanted.completion=false` でも cursor 存在だけで native
    completion path が起動し得た → `cursor_offset` を completion
    要求時のみ shim へ渡すよう修正 + regression test 追加。
  - P2: parse 可能だが schema 違反の shim JSON が silent な空成功に
    なり得た → トップレベル object / `diagnostics` array /
    completion 要求時 `completions` array を必須化し違反は
    `AnalyzerError::Internal`。
  - P2: 改行意味論が Luau Lexer（`\n` のみ）と shim/Rust 変換
    （単独 `\r` も改行）で乖離 → 両側を `\n` のみ計数に統一、
    spec/README/テストを同期。
  - P3: `add_type_definition` の owner-thread 事前検査欠落、
    connect 時の空 version 文字列未拒否、dead `"method"` kind arm、
    `rf_luau_analyzer_destroy` の thread 未検査（assert 追加）、
    status 6 message の spec 不一致、build.rs の include dir
    `rerun-if-changed` 欠落、test-only seam の文書化、
    型定義再定義セマンティクス（last-write-wins を実測 + 文書化）。
- Cycle 2: Reviewer A（cooperative）+ Reviewer B（adversarial）の
  独立再レビュー。Reviewer A は cycle 1 修正 10 件を全て VERIFIED
  と判定し BLOCKER COUNT: 0。Reviewer B が新規 P2 を 1 件検出:
  - P2: `module_time_limit_sec` 超過時に Luau `CheckResult.
    timeoutHits` が shim で捨てられ、部分/空 diagnostics が
    `Ok` 成功として返り得た（default 5.0s で到達可能）→ shim で
    `timeoutHits` 非空を status 6 → `Internal` に fail-closed 化
    + 実測 test 追加。
  - P3 群も併せて修正: diagnostics 非 object エントリの silent
    drop → `Internal`、present-but-garbage `range`/`position` の
    (0,0,0) 偽装 → `None` 伝播、`position_offset` の col clamp を
    行末へ修正 + saturating add、`load_definition` 例外時の
    `globalTypes` freeze 保証、`module_time_limit_sec` 負値/NaN
    の connect 時拒否、`kLuauVersion` と `REVISION` tag の一致
    test 追加、CR/LF diagnostic offset の end-to-end test 追加、
    design doc の改行意味論記述を `\n` のみへ同期。
  - 残存 P3（非 blocking、記録のみ）: 空 version 時の
    `VersionMismatch.field` が常に `"impl_version"` を指す cosmetic
    問題、shim `impl_version` 文字列の crate version との drift
    余地、request 同梱 typedef の accumulate 意味論（spec 記載済み
    の dedup 仕様どおり）。

## 品質契約の確認

- `unsafe`: `src/ffi.rs` に局所許可（`#[expect(unsafe_code)]` +
  SAFETY コメント）で隔離。公開 API・`analyzer.rs`・`contract.rs` は
  safe code のみ。`crates/ruviform-script` 側は無変更
  （`#![forbid(unsafe_code)]` 維持、`git status` で差分なし）。
- 公開 enum: 本 crate の新規公開 enum なし（`LuauAnalyzerOptions` は
  struct、結果型は中立契約の再利用）。
- `Cargo.lock`: `ruviform-script-luau` package エントリ追加のみ
  （11 行）。`mlua 0.12.2`/`mlua-sys 0.13.0`/`luau0-src
  0.22.0+luau740`/`cc`/`find-msvc-tools`/`serde_json` は既存 lock
  再利用、新規 version 解決なし。
- vendor: `vendor/luau/REVISION` に Luau tag 0.740
  （commit `c0e346edd89066b44dca174c9f54ce84c746a540`）と
  mlua/luau0-src 整合根拠を記録。LICENSE.txt / lua_LICENSE.txt 同梱。

## 未検証・Deferred（正直な記録）

- macOS/Linux の build/link/test — **Pass**（merge 後 `ci.yml`
  run `43932614405` @ `06c6d664`: `verify-macos`・
  `verify (ubuntu-latest)`・`verify (windows-latest)` success。
  Luau C++ shim の macOS 初回 build も green）。実機固有の
  環境差分は `evidence/rf560/platform-matrix.md` の分類を継承。
- release-size / performance / memory 実測 — **NotRun**。
- runtime `ScriptBackend`（eval/budget/artifact）— 別 slice。
- Synvelis thin adapter（AC3）— consumer gate（synvelis#271〜274）。
- Issue #692 の AC checkbox 更新・Issue close — 本 slice の範囲外。

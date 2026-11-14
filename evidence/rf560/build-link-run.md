# Build / Link / Run 記録 — Windows（Issue #560 PoC）

## 環境

| 項目 | 値 |
| --- | --- |
| OS | Windows（ホスト: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf560-script2`） |
| Rust toolchain | mise 管理（`mise.toml` 正本。edition 2024 / rust-version 1.89） |
| C++ toolchain | MSVC cl.exe（Visual Studio Build Tools 2022）— `find-msvc-tools` で build.rs が検出 |
| locale | CP932 系（日本語 Windows）— shim の UTF-8 日本語コメントで `/utf-8` 必須と判明 |
| Luau | 0.740（vendored Analysis + luau0-src VM） |

## Build

| 項目 | 値 |
| --- | --- |
| 方式 | `ffi-poc/build.rs` が `cc` crate で Analysis `src/*.cpp` 76 本 + `native/rf_luau_bridge.cpp` を C++17 でコンパイルし `rf_luau_bridge.lib` を生成 |
| defines | `LUAI_MAXCSTACK=1000000`, `LUA_VECTOR_SIZE=3`, `LUA_API=extern "C"`, debug 時 `LUAU_ENABLE_ASSERT` |
| include | Analysis/include → vendored `Ast|Bytecode|CodeGen|Common|Compiler|Config|Inliner|Require|VM`/include → native/ |
| MSVC 環境 | build.rs が `INCLUDE` 未設定時に `find-msvc-tools` の環境を `unsafe { env::set_var }`（edition 2024 の unsafe 規約に SAFETY コメント付きで適合）で補う |
| `/utf-8` | MSVC target 時のみ付与。CP932 既定では shim の日本語コメントが C1020（header 終端 `#endif` でのパース崩壊）を起こすことを実測で確認し、対処済み |

## Link（重要な発見と対処）

- **症状**: 初回リンクで 118 件の未解決 externals
  （`luaopen_utf8`/`luaL_sandbox`/Luau 文字列 utility 等、
  `rf_luau_bridge.lib` からの参照）。
- **原因**: rustc は native lib を依存グラフ順に並べるため、
  `rf_luau_bridge.lib` が mlua-sys の `luau*.lib`（9 本:
  `luauast`/`luaubytecode`/`luaucommon`/`luaucompiler`/`luauconfig`/
  `luaucustom`/`luauinliner`/`luaurequire`/`luauvm`）より後に置かれ、
  MSVC link.exe の左→右走査で bridge の未解決参照を解決できなかった。
- **対処**: build.rs が同名 9 lib を `cargo:rustc-link-lib=static=...`
  で bridge 直後に再 emit。`/LIBPATH` は mlua-sys が供給済み。
  MSVC は各 `.lib` 出現ごとに未解決シンボルを検索するため重複は無害。
  lib 名は mlua-sys 0.13.0 / luau0-src 0.22.0+luau740 の生成物に対応する
  **PoC 固定値**（コメントに明記。mlua-sys 更新時は要再照合）。
- **結果**: 全テストバイナリのリンク成功。

## Run（実測）

| コマンド | 結果 |
| --- | --- |
| `cargo test -p ruviform-ffi-poc`（補助証拠） | **22/22 Pass**（lib unit 8 + bridge integration 13 + faults 1） |
| `mise run check ruviform-ffi-poc` | Pass（`cargo check --locked --all-targets`、exit 0） |
| `mise run lint ruviform-ffi-poc` | Pass（`cargo clippy --locked --all-targets -D warnings`、exit 0） |
| `mise run test ruviform-ffi-poc` | Pass（`cargo test --locked`、22/22 tests） |
| `mise run fmt-check`（`cargo fmt --all -- --check`） | Pass |
| `node .agents/skills/doc-status/scripts/doc_status.js lint` | 本スライスの文書に finding なし（repo 全体の既存 finding 751 件は main 由来で本タスク範囲外） |

### fixture 別結果（tests/bridge.rs, ffi_boundary_tests.rs, tests/faults.rs）

| fixture | 内容 | 結果 |
| --- | --- | --- |
| FX-P-08 正常系 | 有効 Luau source → `AnalyzeResult`（`source_revision` 付き diagnostics/completions） | Pass |
| FX-P-08 構文 Error | 構文エラー source → 構造化 `Diagnostic` | Pass |
| FX-P-08 型 Error | 型エラー source → 構造化 `Diagnostic` | Pass |
| FX-P-08 Completion 更新 | 外部型定義追加で completion 候補が変化 | Pass |
| FX-N-13 | completion 未対応要求 → `Unsupported{completion}` 相当の明示エラー | Pass |
| FX-N-14 | stale `source_revision` → Consumer 側で拒否 | Pass |
| F-18/FX-N-24 | Analyzer 未登録 → `AnalyzerError::Unavailable` | Pass |
| cursor 範囲外 | shim が status 2 で拒否 | Pass |
| source サイズ超過 | `max_source_bytes` で明示エラー | Pass |
| 不正 UTF-8 | shim 境界で status 1 | Pass |
| ABI/contract version 不一致 | create 時 status 8 fail-closed | Pass |
| struct_size 不一致 | create 時に拒否 | Pass |
| native thread 不一致 | 別スレッドからの呼出しで status 4 | Pass |
| completion disabled | completion 無効時の要求で Unsupported | Pass |
| fault 注入 | `RF_FFI_POC_FAIL_INIT`/`FAIL_ANALYZE`/`FAIL_LOADDEF` → status 7、解除後に正常復帰 | Pass |
| VM/Analysis 共存 | mlua `Lua` でスクリプト実行 + Analysis が同一バイナリで動作 | Pass |
| 解析≠実行許可 | analyzer 成功が runtime Grant を意味しないことを型・テストで分離 | Pass（設計上の分離、`analyzer_independent_of_runtime_backend`） |

## 既知の問題・制約（正直な記録）

1. **build-script stdout が非 UTF-8**: mbx が `build-script stdout is not
   UTF-8` を警告し cache を bypass する（cl.exe が CP932 系バイトを
   出力することが原因と推定）。ビルド自体は成功。原因箇所の特定と
   対処は未実施 — PoC では警告として記録。
2. **env fault injection の Windows 固有対応**: Rust の
   `env::set_var`/`remove_var` は Win32 PEB env block
   （`SetEnvironmentVariableW`）を更新するが、C++ `std::getenv` が読む
   CRT `_environ` は起動時 snapshot のまま。shim の `envFlag` を
   Windows では `GetEnvironmentVariableA` で読むよう修正済み
   （非 Windows は `std::getenv`）。この相違自体が fault fixture の
   有用な発見であり、`build-link-run.md` に記録する。
3. **debug `.lib` サイズ**: `rf_luau_bridge.lib` debug は約 377 MB
   （77 TU の debug info）。配布見積には release 計測が必要（未実施、
   `license-ledger.md` 参照）。
4. **OS 範囲**: Windows のみ実測。macOS/Linux は toolchain 不在のため
   NotRun（`platform-matrix.md`）。
5. **review-protocol 所見（P3・非 blocking）**: completion 要求時に
   shim が `frontend.check` を diagnostics 用と autocomplete 用で
   2 回実行する（`retainFullTypeGraphs` 付き再 check）。結果は正しいが
   計算コストは約 2 倍。`load_definition` の unfreeze/freeze は
   `loadDefinitionFile` が例外を投げた場合に凍結状態へ戻らない
   （debug の `LUAU_ENABLE_ASSERT` 下での影響は未検証、PoC 範囲）。
   `completion_context` は JSON 文字列として出力される（Rust 側は
   未使用のため影響なし）。

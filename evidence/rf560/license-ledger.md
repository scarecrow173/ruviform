# License / 互換性台帳 — Luau.Analysis vendored source（Issue #560）

## vendored source

| 項目 | 値 |
| --- | --- |
| upstream | `luau-lang/luau`（https://github.com/luau-lang/luau） |
| tag / commit | `0.740` / `c0e346edd89066b44dca174c9f54ce84c746a540` |
| vendored path | `ffi-poc/vendor/luau/`（manifest: `REVISION`） |
| 内容 | `Analysis/`（src+include）、`Ast`/`Bytecode`/`CodeGen`/`Common`/`Compiler`/`Config`/`Inliner`/`Require`/`VM` の public headers、`LICENSE.txt`、`lua_LICENSE.txt` |
| source サイズ | 約 7.9 MB（`du -sh`、2026-11-14 計測） |
| License | MIT（`LICENSE.txt`）。`lua_LICENSE.txt` は Lua 5.x 系 MIT License 由来の同梱 notice で、Luau VM 側の帰属表示用 |
| vendored TU 数 | Analysis `src/*.cpp` 76 本 + shim `native/rf_luau_bridge.cpp` = 77 本を本 PoC の build.rs が C++17 でコンパイル |

## 依存解決チェーン（Cargo.lock 正本）

| crate | version | 役割 |
| --- | --- | --- |
| `mlua` | 0.12.2 | Lua/Luau VM 埋め込み（PoC では VM 共存証明用） |
| `mlua-sys` | 0.13.0 | mlua の FFI 層。`luau` feature で `luau0-src` を build-dependency に持つ |
| `luau0-src` | 0.22.0+luau740 | Luau 0.740 の VM/Ast/Compiler/Bytecode/Common/Config/Require/Inliner を静的ライブラリ化（`luau-build/*.lib`） |
| `serde_json` | 1.x | shim→Rust の JSON 結果解析 |
| `cc` / `find-msvc-tools` | build-dependency | Analysis+shim の C++ ビルドと MSVC 環境検出 |

## 版整合の根拠

- `luau0-src 0.22.1+luau744` は crates.io で **yanked** のため、
  mlua 0.12.2 が解決する VM は Luau **0.740** に固定される。
- したがって Analysis 側も upstream tag `0.740`
  （`c0e346edd89066b44dca174c9f54ce84c746a540`）で vendored し、
  VM と Analysis の Source Revision を一致させた。
- 詳細は `revision-alignment.md`。

## 必要 C++ toolchain

| toolchain | 用途 | 状況 |
| --- | --- | --- |
| MSVC cl.exe（Visual Studio Build Tools 2022, 17.x） | Windows での Analysis+shim コンパイル | 本 worktree で実測 Pass（`/utf-8` 必須 — shim コメントが UTF-8 日本語で、CP932 既定では C1020 エラー） |
| clang++ / g++（Linux） | Linux での同等ビルド | **NotRun** — 本環境に toolchain なし |
| Apple clang++（macOS） | macOS での同等ビルド | **NotRun** — 本環境に macOS なし |

## 配布サイズ増分（概算・debug 計測値）

| 対象 | サイズ | 備考 |
| --- | --- | --- |
| vendored source | 7.9 MB | リポジトリ増分（release 配布物には含まれない） |
| `rf_luau_bridge.lib`（debug） | 約 377 MB | 77 TU の debug info 込み。**debug 計測値であり配布見積には使わない** |
| `rf_luau_bridge.lib`（release） | 未計測 | `cargo build --release` 未実施 — 後続 Gate で計測 |
| mlua-sys `luau-build/*.lib`（debug） | 約 99 MB（9 ライブラリ合計） | VM 側。Analysis 追加による増分は bridge lib 分のみ（VM libs は #559 側と共通） |
| test exe（debug、Analysis+VM+Rust std 込み） | 約 10–11 MB | `bridge-*.exe` / `faults-*.exe` 実測 |

release ビルドでの最終バイナリ増分・strip 後サイズ・3 OS 別の
配布物差分は **NotRun / Deferred**（real-OS 配布実行は統合 Gate の
責務）。debug 値を配布見積として昇格しない。

## 互換性・既知の制約

- **CRT**: Analysis/shim は MSVC CRT（動的）でコンパイル。mlua-sys の
  luau0-src も同一 MSVC CRT を使用するため、同一バイナリでの混在は
  Windows では整合する。他 OS の CRT 組合せは未検証（NotRun）。
- **Exception**: shim は `try/catch(...)` で Luau 例外・std 例外を
  status code に変換し、C++ 例外を FFI 境界へ漏出させない。
- **Allocator**: Luau の `lua_Alloc`/Analysis 内部 allocator は
  shim 内に閉じ、Rust 側へ公開しない。`rf_luau_free` は shim が
  `new char[]` で返した文字列のみを `delete[]` する対称 API。
- **ODR**: Analysis が必要とする VM/Ast/Common/Compiler 等のシンボルは
  mlua-sys の luau0-src 静的ライブラリから解決し、Analysis 側では
  再コンパイルしない（同一 revision のため ODR 上整合）。
- **static link 順序**: Windows/MSVC では `rf_luau_bridge.lib` が
  `luau*.lib` より後に置かれると未解決シンボルになるため、build.rs が
  同名 lib を bridge 直後に再 emit する（詳細 `build-link-run.md`）。
- **外部 luau-lsp / system-wide install 不要**: 解析は vendored source
  の in-process 実行であり、外部プロセス・言語サーバー install を前提
  としない（Issue 受入条件どおり）。

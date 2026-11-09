---
id: "DESIGN-034Uoh91RxVTOnim395q6R"
type: "design"
status: "approved"
title: "Issue #418 — theme-codegen 設計（manifest 駆動の ResolvedTheme 静的型生成）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-418-theme-codegen.md"
    - "https://github.com/scarecrow173/ruviform/issues/418"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-theme/src/bin/theme_codegen.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/codegen/themes.json"
      - type: "file"
        path: "crates/ruviform-ui-theme/src/generated.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/codegen.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-theme/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-theme/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-418-theme-codegen.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-418-theme-codegen.md"
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-418-theme-codegen.md"
  derived-by:
    - "docs/plans/issue-418-theme-codegen.md"
    - "docs/test-specs/issue-418-theme-codegen.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-418-theme-codegen.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  defers: []
  deferred-by: []
---

# Issue #418 — theme-codegen 設計（manifest 駆動の ResolvedTheme 静的型生成）

## Context

[spec](../specs/issue-418-theme-codegen.md) の要求を実装する。#385 の `ThemeResolver::resolve` は Git 管理 DTCG JSON → `ResolvedTheme`（`BTreeMap<String, ResolvedValue>` + sources/scale/mode）を実行時に生成する。本 Issue は同じ resolve を**生成時に 1 回実行**し、結果を Rust リテラルとして emit することで「token 集合がコンパイル時に既知」な静的型を提供する。resolve 規則の第二正本は作らない — codegen は `ruviform-ui-theme` の公開 API（`TokenDocument::from_json` + `ThemeResolver::resolve`）だけを呼び、merge/alias/mode の意味論を複製しない。

配置の制約（Worker Spec）: 新規 workspace クレートは追加不可。入口は既存 crate 内の bin に収める。`ruviform-ui-theme` は serde/serde_json のみに依存する Driver 非依存 crate であり、codegen bin もこの crate に同居させる（bin は lib の公開 API のみを使い、crate の依存集合は増やさない — manifest の JSON parse は既存の `serde_json` を `serde_json::Value` 経由で使う）。

## Scope

### In Scope

- `src/bin/theme_codegen.rs`: manifest JSON → 解決 → Rust ソース emit の CLI（`--manifest` / `--out` / `--check`）。
- `codegen/themes.json`: 既存 fixture（`crates/ruviform-ui-test/fixtures/themes/*.dtcg.json`、synvelis-dark / audiolens-dark）を入力とする manifest。
- `src/generated.rs`: 生成物の commit 先。`lib.rs` に `pub mod generated;` を追加。
- `tests/codegen.rs`: byte-identical 決定性・committed 生成物の鮮度（`--check` 相当）・生成型と実行時 resolve の値一致を検証。
- `mise.toml` への `codegen-theme` / `codegen-theme-check` タスク追加（共有ファイル変更。理由: Issue 受入「codegen の入口が mise タスクとして定義」の充足。影響: 新規タスク追加のみで既存タスク・CI lane に変更なし）。

### Out of Scope

- `ResolvedTheme` への公開コンストラクタ追加、生成物の crate 外 emit、build.rs / proc-macro 化。
- 生成型の Driver mapper への配線（#386 側の消費設計）。
- token JSON 自体の追加・変更（入力は既存 fixture をそのまま使う）。

## Components and Boundaries

```text
crates/ruviform-ui-theme/
  src/bin/theme_codegen.rs   — CLI + manifest parse + emit（lib の公開 API のみ使用）
  codegen/themes.json        — manifest 正本（name/mode/documents[]、path は manifest 基準）
  src/generated.rs           — 生成物（commit 対象・手編集禁止）
  src/lib.rs                 — `pub mod generated;` 追加のみ
  tests/codegen.rs           — 決定性・鮮度・実行時 resolve 一致テスト
```

### manifest 形式（v1）

```json
{
  "themes": [
    { "name": "synvelis-dark", "mode": "dark",
      "documents": ["../ruviform-ui-test/fixtures/themes/synvelis-dark.dtcg.json"] }
  ]
}
```

- root は `themes`（非空配列）のみ。theme 要素は `name`・`mode`・`documents`（非空配列）のみ。未知 key・型違い・欠落はエラー（fail-fast、最初の 1 件）。
- `documents` の path は manifest ファイルのある directory 基準で解決（cwd 非依存の決定性）。
- `name` は出力 module 名の素。`mode` は `ThemeResolver::resolve` の mode 引数。

### CLI

```
theme-codegen --manifest <path> --out <path> [--check]
```

- 通常: 生成テキストを `--out` へ書き込む（parent dir は `create_dir_all`）。
- `--check`: 書き込まず、生成テキストと既存 `--out` の byte 比較。不一致・不存在は exit 1（CI/鮮度検査用）。
- 失敗時は stderr に 1 行の分類済みメッセージを出し exit 1。引数不正は usage を出し exit 2。
- 依存追加なし: 引数 parse・manifest parse は手書き + `serde_json::Value`。

### 生成コードの形

各 theme につき 1 つの `pub mod <snake_case(name)>` を emit し、内部に `pub struct Theme` を置く:

```rust
pub mod synvelis_dark {
    //! <theme doc>
    pub struct Theme {
        surface_bg: crate::ResolvedColor,
        space_sm: crate::ResolvedLength,
        // ... token 毎に 1 field（private）
        theme: crate::ResolvedTheme,
    }
    impl Theme {
        pub const MODE: &str = "dark";
        #[must_use] pub fn new() -> Self { /* literal から field + ResolvedTheme を構築 */ }
        #[must_use] pub fn resolved_theme(&self) -> &crate::ResolvedTheme;
        #[must_use] pub fn into_resolved_theme(self) -> crate::ResolvedTheme;
        #[must_use] pub fn surface_bg(&self) -> crate::ResolvedColor;   // token 毎の typed accessor
        // ...
    }
    impl Default for Theme { fn default() -> Self { Self::new() } }
}
```

- **値の出所**: `ThemeResolver::resolve` が返した `ResolvedTheme` を codegen が走査し、`ResolvedValue` 各 variant を Rust 式へ写像して field 初期化子と `ResolvedTheme::new` の token map 挿入の**両方**に emit する（`ResolvedTheme::new` は `pub(crate)` のため crate 内生成でのみ利用可能 — crate 外 emit は spec の Out of Scope）。field accessor は map lookup を経由せず保持値を返すため panic 経路を持たない。
- **field/accessor 型**: `Color→ResolvedColor`、`Length→ResolvedLength`、`FontFamily→Vec<String>`（accessor は `&[String]`）、`FontWeight→u16`、`Number→f64`、`Duration→ResolvedDuration`、`CubicBezier→[f64;4]`、`StrokeStyle→StrokeStyle`。いずれも `ResolvedValue` の payload 型と一致。
- **accessor 命名**: token path を `.` で区切り各 segment を ASCII 英数字の snake_case に正規化して `_` 連結。Rust keyword・空 segment・先頭数字・他 token との衝突・private `theme` field との衝突は生成時エラー（決定的）。
- **module 名**: `name` を同規則で snake_case 化。keyword は末尾 `_`、衝突はエラー。
- **決定性**: theme は module 名昇順、token は `BTreeMap` 昇順、sources は resolve の precedence 順で emit。f64 は `{:?}`（最短 round-trip 表現）で出力。文字列リテラルは `{:?}` エスケープ。生成ファイル先頭に「generated・手編集禁止・再生成コマンド」を記す。
- **lint 適合**: 生成コードは `#[must_use]`・`Default` impl・doc comment を備え、pedantic lint（`new_without_default`/`must_use_candidate`/`missing_panics_doc`（panic 経路なし）/`module_name_repetitions` 等）を通過する形で emit する。token 数が `clippy::too_many_lines`（pedantic、閾値 100 行）を超え得るため、生成ファイル先頭に理由付きの `#![allow(clippy::too_many_lines)]` を置く — 生成物の機械的な行数超過に対する明示的・局所的な例外であり、警告の隠蔽ではない（理由は本節と PR 本文に記録）。

### 実行時 resolve との関係（整合方針）

- **値の同一性**: codegen は resolve を再実装しない。生成時に `ThemeResolver::resolve` を呼ぶため、同一入力に対する値は実行時 resolve と**定義上一致**する。`tests/codegen.rs` が生成型の `resolved_theme().to_normalized_json()` と実行時 resolve の `to_normalized_json()` の一致を検証する。
- **使い分け**: token 集合・mode が Git 管理で既知・固定の Theme → 生成型（コンパイル時に token 名・型が確定、typo はコンパイルエラー）。Consumer/アプリが実行時に document を登録・差替える動的 Theme → 従来どおり `ThemeRegistry`/`ThemeResolver`（実行時 resolve）。**非推奨化はしない** — 両経路は同一パイプラインの別の具現化タイミングであり、置き換え関係ではない。
- **差異として残るもの**: 生成型は resolve 時点の snapshot であり、入力 JSON を後から変更しても再 codegen するまで古い値を保持する（`--check`/テストが鮮度を検出）。`ScaleContext` は生成時既定値で、`resolved_theme().with_scale()` で実行時差替え可能。sources の fingerprint は生成時の doc 内容を反映する。

## Data and Control Flow

```text
codegen/themes.json (manifest)
  → theme_codegen --manifest --out
      ├─ manifest parse（serde_json::Value・strict）
      ├─ documents[] → TokenDocument::from_json（path は manifest dir 基準）
      ├─ ThemeResolver::resolve(&docs, mode) → ResolvedTheme
      ├─ identifier sanitize + collision 検査（決定的エラー）
      └─ Rust source emit（BTreeMap/ソート順で決定的）
  → src/generated.rs（commit）
  → lib.rs `pub mod generated;` → cargo check/lint/test の対象
```

## Non-Functional Constraints

- 決定性: 同一 manifest + 同一入力 → byte-identical 出力（実装内に時刻・乱数・環境依存を持ち込まない）。
- 依存追加なし（serde_json は既存 dep を流用）。`unsafe` なし。公開 enum 追加なし（生成 struct/module は非 enum）。
- 生成コードは `cargo fmt --check` を通る形で emit（rustfmt 規約に合わせた整形済みテキストを出力）。

## Risks and Trade-offs

- 生成物の手編集ドリフト → `tests/codegen.rs` が committed ファイルと再生成結果の byte 比較を行い、`mise run codegen-theme-check` でも検出可能。
- token 命名が Rust 識別子に落ちない入力 → 衝突・不正名は生成時エラーで fail-fast（token JSON 側の命名規約を実質要求する。v1 採用 token 群は既に snake 互換の命名）。
- `too_many_lines` allow の乱用懸念 → 対象は生成ファイル 1 つに限定し、token 数増大時も構造を変えずに済む。生成コードを人手で分割するより機械的一貫性を優先。
- crate 内生成限定による Consumer 活用の制約 → spec の Deferred として明示。公開 materialize API は YAGNI で今回は追加しない。

## Deferred Design Concerns

- crate 外 emit・`ResolvedTheme` 公開 materialize API → spec Deferred（実需時）。
- 複数 mode の一括生成（現状は theme 毎に mode を 1 つ宣言。必要なら manifest の `modes` 配列拡張で対応可能だが YAGNI で v1 は単一 mode）。
- 生成型からの `ResolvedTheme` 再構成コスト（`new()` は都度 BTreeMap を構築。Theme は少数・構築頻度は低い前提。必要なら `OnceLock` 化は利用側で可能）。

## Verification Notes

- [ ] `mise run codegen-theme` で生成 → `src/generated.rs` が更新され、`--check` が一致を返す
- [ ] `tests/codegen.rs`: 2 回生成の byte 一致・committed ファイルとの一致・生成型 vs 実行時 resolve の `to_normalized_json` 一致
- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` が green（生成コードを含む）
- [ ] 生成コードに `gpui`/Toolkit/DOM/wry 依存・型がない（crate 境界不変）

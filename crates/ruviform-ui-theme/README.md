# ruviform-ui-theme

Driver 非依存の DTCG（Design Tokens Community Group 形式）design-token インタプリタ。
Git 管理の token JSON を **deterministic パイプライン**（`parse → schema/type validation → alias 解決 → theme/mode/consumer override merge → normalized typed ResolvedTheme`）で処理し、型付き不変の `ResolvedTheme` を生成する。

- 正本: `docs/specs/issue-385-dtcg-resolved-theme.md` / `docs/designs/issue-385-dtcg-resolved-theme.md`（Issue #385）
- 構造: `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（Issue #384 — Driver-neutral 契約層 + GPUI 標準 Driver 仮決定）

## 層・依存境界

- 依存は `serde` + `serde_json` のみ。`ruviform-ui`（host 契約）・GPUI・Toolkit・DOM/wry 型への依存を持たない。GPUI 固有の Style への変換は Driver 側 mapper（#386 の成果物）が `ResolvedTheme` を一方向消費して行う。
- 本 crate は DTCG 解釈・Theme/asset/mode・reference/alias 解決の**唯一の正本**。Driver や Toolkit が alias/override 優先順位を独自解釈する第二正本を作らない。

## 採用 DTCG profile（`rvf-dtcg-profile.v1`）

- 各 document はルート `$extensions."dev.ruviform.dtcg-profile"` に `profile`・`layer`（`primitive|semantic|component|product`）・`id`・任意 `modes` を宣言する。
- 採用 `$type`（v1）: `color`・`dimension`・`fontFamily`・`fontWeight`・`number`・`duration`・`cubicBezier`・`strokeStyle`。複合型（`typography`/`border`/`shadow`/`gradient`/`transition`）は v1 対象外で strict error。
- alias は値全体の `"{path.to.token}"` 参照のみ（部分文字列埋め込みは v1 対象外）。mode 値は `$extensions."dev.ruviform.modes"`。
- strictness: 未知 `$type`・未知参照・循環・mode 不一致・override 型不一致はエラー。未知 `$extensions` key・非 `$` 未知 key は ignore。未知の `$` prefix key は `ReservedKey` エラー。
- precedence: documents は layer 昇順（`primitive < semantic < component < product`）で評価され、同 layer は渡し順で後者優先。override は同一 token key・同一 mode scope にのみ有効。
- schema migration: 未対応の `$type`・部分 alias・複合型は profile version を上げて追加する。`ResolvedTheme` が schema_version を保持するため Consumer/Driver 側で差分判定できる。
- 色空間・単位: color は srgb components[0..1]+alpha に正規化。length は `px`/`rem` の単位を保持し rem→px 換算は mapper 側規約（`ScaleContext::rem_base_px`・`scale_factor`）。`to_normalized_json()` が key 辞書順の正準 golden を出す。
- Figma 連携方針: API/実行時依存は持たない。将来の Import は `export → DTCG 変換 → validate → diff → PR レビュー` の一方向フロー。

## 実行手順

```rust
let base = ruviform_ui_theme::TokenDocument::from_json(base_json)?;
let product = ruviform_ui_theme::TokenDocument::from_json(product_json)?;
let theme = ruviform_ui_theme::ThemeResolver::resolve(&[base, product], "dark")?;
assert_eq!(theme.schema_version(), "rvf-dtcg-profile.v1");
```

## Theme Registry

`ThemeRegistry` は theme 名 → document stack の対応を保持するだけで、解決規則は持たない（`ThemeResolver::resolve` へ委譲）。Product Theme ごとに base 文書と Consumer override 文書を `register` し、`resolve(name, mode)` で `ResolvedTheme` を得る。未登録名は `ResolveError::UnknownTheme`。

## Golden fixtures と境界

- 入力: `tests/fixtures/`。出力正本: `tests/golden/*.json`（`to_normalized_json()` のバイト一致）。
- 検証対象（`tests/golden.rs`）: valid（`valid-dark`）・Consumer override（`consumer-override-dark`）・OS font 不在（`os-font-fallback-dark`）・DPI scale（`dpi-scale-2x-dark`）の golden、invalid（`err-invalid-value`）・ref-cycle（`err-cycle`）・type mismatch（`err-type-mismatch`）・missing mode（`product-light-only`）の分類済みエラー。
- OS font は本 crate では **利用可否を判定しない**。`fontFamily` は宣言された fallback 列を順序どおり保持し、インストール有無による置換・除外は Driver（#386/#390）の capability 判定に委ねる。
- DPI scale は `ResolvedTheme::with_scale` で `ScaleContext` のみ差し替える。token 値は変化せず、device pixel 換算は mapper 側。
- 出力は入力文書の順序に依存しない（`tests/golden.rs::output_is_independent_of_input_order_and_repetition`）。fingerprint は値の正規化 JSON に基づき、`Debug` 表現に依存しない。

## 静的 theme codegen（Issue #418）

`codegen/themes.json` が宣言する theme について、`theme-codegen` bin が生成時に `ThemeResolver::resolve` を 1 回実行し、結果を `src/generated.rs` の型付き Rust リテラルとして emit する。resolve 規則の第二実装は持たず、意味解釈の正本は引き続き本 crate の runtime pipeline のみ。

```sh
mise run codegen-theme         # src/generated.rs を再生成
mise run codegen-theme-check   # commit 済み生成物の鮮度を byte 比較（非破壊）
```

生成物は `generated::<module>::Theme` を公開する。`Theme::new()` が typed field（token path を snake_case 化した accessor）と内部 `ResolvedTheme` を同時に構築し、`resolved_theme()` / `into_resolved_theme()` で runtime snapshot へアクセスできる。`#[non_exhaustive]` enum の新規 variant 追加時は codegen が明示エラーで停止するため、生成物と runtime の乖離は fail-closed。

### runtime resolve との使い分け

- **静的 codegen（本経路）**: 出荷が確定した token 集合に対し、compile-time の発見可能性・型安全な accessor・zero-map-lookup アクセスが欲しい場合。生成物は commit され、入力変更時は `mise run codegen-theme` で再生成する（`tests/codegen.rs` が決定性・鮮度・runtime 一致を検証）。
- **runtime resolve（`ThemeResolver` / `ThemeRegistry`）**: 実行時に文書・mode・consumer override が決まる場合、開発中の token 編集、未知 theme の解決。動的入力は引き続き runtime pipeline を使う。codegen は runtime resolve の置き換えでも非推奨化でもなく、固定入力に対する同一結果の静的具現化である。

## 検証

`mise run check` / `mise run test`（ワークスペース正規入口）。codegen は `mise run codegen-theme` / `mise run codegen-theme-check`。Driver 境界は `mise run check-ui-driver-boundary` / `mise run test-ui-driver-contract`。

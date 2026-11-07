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

検証は `mise run check` / `mise run test`（ワークスペース正規入口）。fixtures は `tests/fixtures/`。

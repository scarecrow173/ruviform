---
id: "DESIGN-034T6SbNaGu3l7owAp7FCX"
type: "design"
status: "approved"
title: "DTCG Interpreter/Resolver 設計 — alias・mode・override merge・typed ResolvedTheme（Issue #385）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "https://github.com/scarecrow173/ruviform/issues/385"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-theme"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://www.designtokens.org/"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  defers: []
  deferred-by: []
---

# DTCG Interpreter/Resolver 設計 — alias・mode・override merge・typed ResolvedTheme（Issue #385）

## Context

[spec](../specs/issue-385-dtcg-resolved-theme.md) の要求を実装する `ruviform-ui-theme` の設計。DTCG JSON → deterministic pipeline → typed `ResolvedTheme`（不変 snapshot）。**唯一の正本**であり、GPUI・Toolkit・`ruviform-ui`（host 契約）への依存を持たない。依存は `serde` + `serde_json` のみ。GPUI mapper は #386 の Driver 側成果物として `ResolvedTheme` を一方向消費する（`ResolvedTheme → Driver-private Style` のみ許可し、双方向・正本委譲はしない）。

## Scope

### In Scope

- `crates/ruviform-ui-theme` のモジュール構成・公開 API・エラー分類・fixture。
- DTCG profile `rvf-dtcg-profile.v1` の採用範囲（型・参照・mode・override・strictness）。
- 4 層 precedence の merge 規約と ResolvedTheme の不変 snapshot 設計。

### Out of Scope

- GPUI/Toolkit mapper、Component spec 統合（#387）、codegen、Figma 実連携、Consumer 製品 Theme 内容。

## Profile（`rvf-dtcg-profile.v1`）

採用する DTCG 形式のサブセット:

- **Document**: ルートに `"$extensions": { "dev.ruviform.dtcg-profile": { "profile": "rvf-dtcg-profile.v1", "layer": "primitive|semantic|component|product", "id": "<source identity>" } }` を必須とする。profile 不一致・layer 未知はエラー。`modes: [...]` を同 extension で宣言可 — **省略（空）の doc は mode-agnostic であり任意 mode を受理する**。非空 `modes` を宣言する doc で要求 mode が不在なら `MissingMode`。
- **Token**: `{ "$type": <type>, "$value": <value>, "$description"?: string, "$extensions"?: {...} }`。`$type` は token または祖先 group で宣言（継承）。effective `$type` がどこにも無い token は `InvalidValue`（"missing $type"）。token node に非 `$` key の子があっても読まず ignore（group 化しない）。
- **Group**: 入れ子 object。`$type`/`$description`/`$extensions` を持てる。`$extends` 等他の `$` prefix key は `ReservedKey` エラー（v1 対象外）。
- **Alias**: `$value` が **値全体** `"{path.to.token}"` の文字列（内側は非空・空白・`{}` なし）の場合のみ参照。部分埋め込み（`"{a}-{b}px"`）や `{`/`}` を含む他の文字列は全 `$type` で `InvalidValue`（文字列型でも literal 化させない）。参照先が merge 後の token 集合に存在しない → `UnknownReference`（group path・存在しない path 共通）。参照先の `$type` が参照側と異なる → `TypeMismatch`。循環は `AliasCycle`（安定 DFS、cycle path を列挙）。path は `.` 連結 — `.` 含む JSON key と group 入れ子が同一 path に衝突する場合は `InvalidValue`（duplicate token path）として検出する。
- **Mode 値**: token の `"$extensions": { "dev.ruviform.modes": { "<mode>": <value-or-alias> } }`。mode 値は base と同じ `$type` として parse 時に検証される（形状違反は `InvalidValue`）。doc 宣言 `modes` に無い token 側 mode key は到達不能のため ignore（エラーにしない）。
- **Rounding 拡張**: `dimension` token の `$extensions."dev.ruviform.rounding"` = `"none"|"nearest"|"floor"|"ceil"`（省略時 `none`）。`ResolvedLength::rounding` の唯一の入力元。mapper が device 単位へ変換する際の規約値で、resolve 側は値を変換しない。
- **採用 `$type`（v1）**: `color`, `dimension`, `fontFamily`, `fontWeight`, `number`, `duration`, `cubicBezier`, `strokeStyle`（対象外: `border`, `transition`, `shadow`, `gradient`, `typography` 複合型 = v1 で未知 `$type` 扱い → strict error）。
- **値の形**: `color` = `"#rgb"|"#rrggbb"|"#rrggbbaa"` または `{ "colorSpace": "srgb", "components": [0..1 ×3], "alpha"?: 0..1 }`（DTCG object 形、alpha 省略=1）。`dimension` = `{ "value": number, "unit": "px"|"rem" }`。`fontFamily` = string または非空 string[]（fallback は配列順）。`fontWeight` = `1..=1000` の整数または DTCG 命名表（`thin|hairline`→100、`extra-light|ultra-light`→200、`light`→300、`normal|regular`→400、`medium`→500、`semi-bold|demi-bold`→600、`bold`→700、`extra-bold|ultra-bold`→800、`black`→900、`extra-black|ultra-black`→950）。`number` = 有限数値。`duration` = `{ "value": number, "unit": "ms"|"s" }`（負値は v1 では拒否しない — mapper 規約）。`cubicBezier` = `[x1,y1,x2,y2]` 4 要素有限数値（DTCG の x∈[0,1] 制約は v1 では強制しない — mapper 側で clamp 可）。`strokeStyle` = `"solid"|"dashed"|"dotted"|"double"|"groove"|"ridge"|"outset"|"inset"`。
- **strictness（v1 明示規約）**: 未知 `$type`・alias 未定義・循環・mode 不一致・override 型不一致・malformed alias・duplicate path はエラー。未知の `$extensions` key・非 `$` 未知 key（token 内の未知非 `$` key を含む）は ignore（fatal にしない）。`$` prefix を持つ未知 key（`$extends` 等）は `ReservedKey` エラー。
- **エラー列挙**: fail-fast — 最初の 1 件のみ返す。`serde_json::Map` のキーは常に辞書順で walk も BTreeMap のため、「最初のエラー」は入力に対し一意に決まる（multi-error collection は v1 で提供しない）。`TokenError`（構造/型）は `from_json` 段階、`ResolveError`（merge 意味論）は `resolve` 段階の責務。`ResolveError::Document` は `From<TokenError>` 変換用の包み（`?` で document エラーを resolve 系の caller へ伝播する構成用）。

## Components and Boundaries

```text
crates/ruviform-ui-theme/src/
  lib.rs       — crate doc・再 export・version()
  error.rs     — TokenError / ResolveError（分類済み・#[non_exhaustive]）
  document.rs  — TokenDocument（profile metadata + raw token tree・layer）
  parse.rs     — JSON → raw token/group tree（serde 手書き deserialize、deterministic）
  validate.rs  — schema/type validation（$type 解決・値形状・strictness）
  alias.rs     — {a.b.c} 参照の存在検証・DFS 循環検出（安定順エラー）
  resolve.rs   — layer merge（precedence）+ mode/override 合成 → ResolvedTheme
  theme.rs     — ResolvedTheme / ResolvedValue / 型付き accessor
tests/
  fixtures/    — *.json token docs（valid/invalid/cycle/type-mismatch/missing-mode/override）
  pipeline.rs  — fixture → normalized output golden / error 分類テスト
```

### 公開 API（骨格）

```rust
pub struct TokenDocument { /* layer, id, modes, token tree */ }
pub enum DocumentLayer { Primitive, Semantic, Component, Product }        // #[non_exhaustive]
impl TokenDocument {
    pub fn from_json(json: &str) -> Result<Self, TokenError>;
    pub fn layer(&self) -> DocumentLayer;
    pub fn id(&self) -> &str;
}

pub struct ThemeResolver;
impl ThemeResolver {
    /// mode 名で解決。documents は precedence 昇順（primitive→product）。
    /// mode が各 document の宣言 modes に存在しない場合 MissingMode。
    pub fn resolve(docs: &[TokenDocument], mode: &str) -> Result<ResolvedTheme, ResolveError>;
}

pub struct ResolvedTheme { /* schema_version, mode, scale, sources, tokens: BTreeMap */ }
impl ResolvedTheme {
    pub fn schema_version(&self) -> &str;          // "rvf-dtcg-profile.v1"
    pub fn mode(&self) -> &str;
    pub fn scale(&self) -> ScaleContext;           // { scale_factor: f64, rem_base_px: f64 }
    pub fn with_scale(&self, scale: ScaleContext) -> Self;  // token 値は不変・context のみ複製
    pub fn sources(&self) -> &[TokenSource];       // 各 doc の id+layer+fingerprint
    pub fn tokens(&self) -> impl Iterator<Item = (&str, &ResolvedValue)>;
    pub fn get(&self, key: &str) -> Option<&ResolvedValue>;
    pub fn color(&self, key: &str) -> Option<ResolvedColor>;
    pub fn length(&self, key: &str) -> Option<ResolvedLength>;
    pub fn font_family(&self, key: &str) -> Option<&[String]>;
    pub fn number(&self, key: &str) -> Option<f64>;
    pub fn duration(&self, key: &str) -> Option<ResolvedDuration>;
    pub fn to_normalized_json(&self) -> String;    // deterministic golden 用（key ソート）
}

pub enum ResolvedValue {                                                          // #[non_exhaustive]
    Color(ResolvedColor), Length(ResolvedLength), FontFamily(Vec<String>),
    FontWeight(u16), Number(f64), Duration(ResolvedDuration),
    CubicBezier([f64; 4]), StrokeStyle(StrokeStyle),
}
pub struct ResolvedColor { pub space: ColorSpace, pub components: [f64; 3], pub alpha: f64 }
pub enum ColorSpace { Srgb }                                                      // #[non_exhaustive] v1
pub struct ResolvedLength { pub value: f64, pub unit: LengthUnit, pub rounding: Rounding }
pub enum LengthUnit { Px, Rem }                                                   // #[non_exhaustive]
pub enum Rounding { None, Nearest, Floor, Ceil }                                  // #[non_exhaustive]（mapper 適用規約）
pub struct ResolvedDuration { pub value: f64, pub unit: DurationUnit }
pub enum StrokeStyle { Solid, Dashed, Dotted, Double, Groove, Ridge, Outset, Inset } // #[non_exhaustive]
pub struct ScaleContext { pub scale_factor: f64, pub rem_base_px: f64 }           // 既定 1.0/16.0
pub struct TokenSource { pub layer: DocumentLayer, pub id: String, pub fingerprint: u64 }
```

### merge / override 規約

- documents は caller が任意順で渡し、**resolver が layer rank（Primitive→Semantic→Component→Product）で stable sort** して評価する（誤順序を入力側ミスにしない）。同一 layer の複数 doc は渡し順で後者優先。同じ token path が下位層に存在しないのに上位層で定義されることは許可（追加 token）。
- override は **token 単位の wholesale 置換**（base `$value` と mode map が一体で入替わる。上位 doc が `{dark: X}` のみ供給した場合、下位層の `{light}` 値は引き継がず上位の `$value` へ落ちる）。作用範囲は同一 token key・同一 mode scope のみ — 異なる mode への影響なし、子孫への暗黙継承なし。
- override で `$type` が基底と異なる → `TypeMismatch` エラー。`$description`・`$extensions` の未知 key は置換対象外（値の意味だけが上書きされる）。
- alias は **merge 後の最終 token 集合**で解決する（上位層が指す参照を含む）。`{a.b}` が存在しない → `UnknownReference`。循環 → `AliasCycle`（検出した cycle path を安定順で報告）。
- mode 解決: `resolve(docs, mode)` で各 doc が宣言する modes に要求 mode が含まれない → `MissingMode{doc_id, mode}`。token に mode 値がない場合は base `$value` を使用（宣言 modes 全てで同値）。
- Rem 換算は resolve 時には行わない。`ResolvedLength { unit: Rem }` を保持し、`ScaleContext::rem_base_px` と scale_factor は mapper 側の換算規約として context に保持する（勝手な単位変換をしない要求に整合）。

### エラー分類

```rust
pub enum TokenError {                                                              // #[non_exhaustive]
    MalformedJson { line: usize, column: usize, message: String },
    MissingProfile, InvalidProfile { found: String },
    UnknownLayer { found: String },
    UnknownTokenType { path: String, found: String },
    InvalidValue { path: String, reason: String },        // 値形状・範囲違反
    ReservedKey { path: String, key: String },            // 未知 $-prefix key
}
pub enum ResolveError {                                                            // #[non_exhaustive]
    Document(TokenError),
    MissingMode { document: String, mode: String },
    TypeMismatch { path: String, base: String, override_: String },
    UnknownReference { path: String, reference: String },
    AliasCycle { cycle: Vec<String> },
    DuplicateSource { id: String },                       // 同一 (layer,id) の重複入力
    EmptyDocuments,
}
```

- エラー列挙は入力順・token path の辞書順で安定（BTreeMap + 安定 DFS）。
- `ResolveError: From<TokenError>`。doc の構造的問題は parse/validate 時に TokenError として早く失敗させ、resolve は merge 意味論の問題だけを扱う。

### 決定性・正本ルール

- 全 map は `BTreeMap`。`to_normalized_json()` は key 辞書順・数値は serde_json の `f64` 出力の正準形。
- `TokenSource::fingerprint` は document 正規化後の FNV-1a 64bit（外部 hash 依存を増やさない）。
- Theme 差分が業務数値・操作状態・Timing を変えない: `ResolvedTheme` は見た目値のみ保持し、値の semantic は Consumer 側キー規約。crate 内に project/dsp/timing の概念を持ち込まない（design 上の分離を spec AC で要求）。

## Data and Control Flow

```text
JSON docs (Git 正本) → TokenDocument::from_json → validate（$type・値・strictness）
  → ThemeResolver::resolve(docs, mode)
      ├─ layer 昇順 merge（precedence・型一致検証）
      ├─ mode scope 値選択（missing mode → error）
      └─ alias DFS 解決（存在・循環検証）
  → ResolvedTheme { schema_version, mode, scale, sources, BTreeMap<key, ResolvedValue> }
  → （別 crate/Driver）mapper へ不変受渡し
```

## Non-Functional Constraints

- deterministic: 同一入力 → バイト同一 normalized output・同一エラー列。
- `unsafe` なし。公開 enum `#[non_exhaustive]`。doctest あり。
- 依存: serde（derive 不要でも可、derive 使用）+ serde_json のみ。`ruviform-*` 依存なし。

## Risks and Trade-offs

- DTCG mode の標準が未確定 → `dev.ruviform.modes` extension で profile 化。将来 DTCG 標準が確定した場合は schema_version v2 で移行（schema_version は ResolvedTheme が保持）。
- `typography`/`border`/`shadow`/`gradient`/`transition` 複合型を v1 対象外にした → 必要時に schema version を上げて追加（unknown type は strict error なので偽解釈しない）。
- 部分文字列 alias 非対応 → DTCG 一部ツールの `"{a}-{b}px"` 形式は v1 で拒否。fixture で明記。

## Deferred Design Concerns

- GPUI mapper 消費契約の具体形（#386）・Component spec からの参照規約（#387）。
- `ResolvedTheme` の diff/バージョン比較 API（token rename/compat guide 用）→ 実需時に追加。
- OS font missing・DPI scale の Capability 表現 → Driver capability 側（#386/#390）で検証。crate は `with_scale` で context 差替えのみ提供。

## Verification Notes

- [ ] 公開 API に `gpui`/Toolkit/DOM/wry 型・依存がない（Cargo.toml + API 目視 + #390 guard 方針と整合）
- [ ] merge precedence・override 型一致・alias cycle・missing mode が spec の初期規約どおり
- [ ] ResolvedTheme が schema_version/mode/scale/sources を保持し不変
- [ ] エラー分類が type mismatch/cycle/unknown ref/missing mode/strictness を網羅
- [ ] fixtures が spec AC の 6 ケースを網羅

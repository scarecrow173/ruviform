# Issue #692 script port — experiment notes

実装過程の観察と判断の記録。イベント列の正本は
`issue-692-script-port-exp.jsonl`（schema `experiment_event.v1`）。

## 採用した判断

- **非有限 `Number` の canonical 形は tag record**: 当初の引用符付き
  sentinel（`"__rv_nan__"` 等）は通常の `String` と衝突し得るため、
  `{"$rv":"number","value":"nan"|"inf"|"-inf"}` を採用。codec の
  `materialize` が出力する形と一致させ、`canonical_value(v) ==
  canonical_value(materialize(v))` を有限範囲で保つ。
- **`decode_tag` の検証を一箇所に集約**: 既知 tag の期待 field 集合
  （tag key + 宣言 field のみ、過不足なし）を `check_fields` で検査し、
  tag ごとの decode に分岐。未知 tag・非文字列 tag・field 過不足・
  内容不正は全て fail-closed の `CodecError`。
- **test の `expect_err` は match helper に置換**: `Result<&dyn
  ScriptBackend, _>` / `Result<Box<dyn ScriptSession>, _>` は Ok 側が
  `Debug` を持たず `expect_err` が使えない。`tests/common` に
  `expect_resolve_err` / `expect_open_err` を追加。`evaluate` /
  `analyze` の戻り値は Ok 型が `Debug` を持つため `expect_err` を維持。
- **crate root の `pub use` 再エクスポート追加**: module doc が
  `crate::Type` 省略形で 76 件の intra-doc link 切れを起こしたため、
  `ruviform-ui-theme` 先例どおり root re-export を追加。consumer は
  `ruviform_script::ScriptValue` 形でも到達可能。module path は
  canonical namespace として維持し、`prelude` は glob import 用に残す。
- **`canonical_number` は `f64` Display shortest round-trip**: 有限値は
  Rust の決定論的最短往復表記をそのまま使い、整数値は小数点なしで
  出力されるため `Number(1.0)` と `Integer(1)` が同一 canonical/digest
  を共有する。`-0.0` は `to_bits() << 1 == 0` で検出（float `==` 不使用、
  `clippy::float_cmp` clean・MSRV 1.89 で動作）。

## 棄却した案

- **serde derive による canonical 実装（synvelis `canon.rs` 方式）**:
  wire format 未決定の現段階で公開契約型に serde を露出しない判断
  （design D6）。canonical は手書き writer 1 箇所に集約し、JSON escape
  のみ `serde_json` を内部利用する。
- **`Rational::new` での `denominator == 0` 拒否**: PoC と同一形状で
  contract は timebase を非解釈データとして扱う。検証を入れると
  typed 生成と codec decode で非対称になるため、rustdoc の不変条件
  記述に留めた（den=0 は consumer/backend の責務）。
- **`HostApiSurface::validate_neutral` での schema_version 検査**:
  surface/binding の version 判定は `SessionConfig` 契約どおり
  backend の `open_session` 責務とし、neutral 検証は命名規則と一意性に
  限定した。

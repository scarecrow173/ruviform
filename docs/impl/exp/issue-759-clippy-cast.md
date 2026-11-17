# Issue #759 plugin-clap `GuiSize` f64→u32 境界契約 — 実装実験記録（2026-11-18）

`docs/tasks/issue-759-clippy-cast.md` の slice（`set_gui_size` の
`f64 as u32` cast lint 修復）で試行・観測した事項の記録。
実装結果の正本は
[`docs/impl/ir/issue-759-clippy-cast.md`](../ir/issue-759-clippy-cast.md)、
イベントログは
[`issue-759-clippy-cast-exp.jsonl`](issue-759-clippy-cast-exp.jsonl)
を参照。

## 試行 1: lint error の再現と帰属確定

- **目的**: Issue 記載の「4 error」が `set_gui_size` の 2 cast に
  帰属することを実測で確認する。
- **観測**: `mise run lint ruviform-plugin-test-clap` が
  `host.rs` の `width: size.width as u32` /
  `height: size.height as u32` に対し
  `cast_possible_truncation` + `cast_sign_loss` を各 1 件ずつ
  報告（計 4 events・exit 101）。workspace
  `clippy::pedantic = warn` + lint task の `-D warnings` で
  error 化する構成と一致。
- **結論**: 帰属確定。`f64 as u32` は 2 lint が必ず発火する
  組合せであり、cast を残す限り局所 `#[expect]` が不可避。

## 試行 2: 境界契約の選定（飽和 vs 拒否）

- **目的**: 負値・NaN・非有限・`u32::MAX` 超過の扱いを契約として
  決める。
- **観測**: neutral `GuiSize`（plugin-core `boundary.rs`）は
  「scale-independent な論理単位」の `f64` で定義域の明文化が
  ない。`HostError::InvalidValue`（caller-supplied value の定義域外
  拒否）は存在するが、「定義域外」と断定する契約根拠がない。
  兄弟 adapter の VST3 host は同一 verb で
  `size.width.round() as i32` の飽和写像 + `#[expect]` を
  採用済み。
- **結論**: **飽和写像を採用** — round → `clamp(0.0,
  f64::from(u32::MAX))` → `as u32`。負値・NaN・非有限 → `0`、
  `u32::MAX` 超過 → `u32::MAX`。verb を全域で定義し、異常サイズの
  最終判定は plugin の `set_size` accept/decline に委ねる。
  拒否案は VST3 adapter との verb 挙動分岐を生むため棄却
  （discovery `docs/discovery/issue-759-clippy-cast.md` の
  Alternatives 表に記録）。

## 試行 3: `#[expect]` の適用範囲と lint の連鎖

- **目的**: clamp で実際に安全化した上で、残る構文 lint を
  `#[expect]` で局所許可する形を確定する。
- **観測**: `clamp` 後も clippy は値域を追跡せず
  `cast_possible_truncation`/`cast_sign_loss` を発火するため、
  helper 関数 `gui_size_component` に
  `#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]`
  を付与。初回実装の `u32::MAX as f64` は別 lint
  `cast_lossless`（`f64::from` への置換を要求）に抵触したため
  `f64::from(u32::MAX)` に修正 — `unit_f64` の既存スタイルと一致。
- **結論**: `#[allow]` は使わず `#[expect]` + doc comment の契約
  明記で lint を通す。`#[expect]` は lint が実際に発火しなく
  なった場合 unfulfilled expectation として検出されるため、
  恒久的な隠蔽にならない。

## 試行 4: 境界契約の unit test 固定

- **目的**: 「境界挙動がコードまたは doc に明記される」受入条件を
  実行可能な形でも担保する。
- **観測**: host feature の `mod tests` に
  `gui_size_component_saturates_at_pixel_bounds` を追加 —
  800.4→800 / 800.5→801（round）、-3.0/NaN/-inf→0、
  +inf/1e30/u32::MAX+0.4→u32::MAX を assert。
- **結論**: `cargo test -p ruviform-plugin-clap --features host`
  で当該 test が pass（auxiliary 実行）。canonical lane は
  `mise run test-l1-real-plugin` の `--lib --bins` 段が担う。

# RF384/RF385 実行証拠レポート (Issue #384 Driver-neutral GUI ADR / Issue #385 DTCG→ResolvedTheme)

作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf384-385-gui-v2`
branch: `feature/gui-driver-neutral-design-contract`
head: `d3b211cc09c01d74395dfab3354c895b345fc744`（origin/main `355b7a1` merge 済み・衝突なし）
PR: https://github.com/scarecrow173/ruviform/pull/397
環境: Windows 11 x64 (MINGW64) / rustc 1.99.0 / cargo 1.99.0
実行日時: 2026-11-07 (UTC)

## 実施した検証（全て実走・実値）

| 検証 | コマンド | 結果 | 備考 |
| --- | --- | --- | --- |
| fmt | `mise run fmt` / `mise run fmt-check` | Pass (exit 0) | `cargo fmt --all -- --check`。merge 後に再実走済み |
| workspace check | `mise run check` | Pass (exit 0) | `cargo check --workspace --all-targets`（CARGO_BUILD_JOBS=2） |
| workspace lint | `mise run lint` | Pass (exit 0) | `cargo clippy --workspace --all-targets -- -D warnings` |
| crate test | `mise exec -- cargo test -p ruviform-ui-theme` | Pass | **18 tests + 1 doctest** 全 pass |
| workspace test | `mise run test`（`cargo test --workspace`） | 1 件 flake → 単独 pass | `ruviform-plugin-host` L1 e2e `wedged_child_write_converges_and_teardown_is_bounded` が初回のみ `SpawnFailed{Timeout}`。単独再実行 pass。coordinator 通知（msg_419e373e1f1f）により main でも同系失敗を確認済み・本変更無関係 |
| merge 後再検証 | `mise run fmt-check` + `mise exec -- cargo check -p ruviform-ui-theme --all-targets` | Pass (exit 0) | head `d3b211c` 上で実走 |
| ADR audit | `node scripts/audit_adr.js` | 新 ADR error 0 | `Pros and Cons of the Options` 追記で info 解消 |
| doc index | `node scripts/doc_maintenance.js` | 再生成済み | specs/designs/plans/tasks/ADR README の managed index を再生成。自 doc の orphan/reciprocal finding は解消（repo 全体の既存 manual finding は本 scope 外） |

## 独立 review 記録

runtime subagent 2 名で独立 review を実施（実装者と兼務せず）。

| reviewer | verdict | 主な finding → disposition |
| --- | --- | --- |
| Reviewer A（契約/ADR・docs 整合） | PASS_WITH_NITS | Fake Driver capability 誤記（theme/component/render を Pass にしない）→ **ADOPTED**（`declared` へ修正 + Fake 列 Pass の定義注記追加）。旧 ADR 双方向リンク → **FIXED** 確認済み。`Pros and Cons` 節欠落 → **ADOPTED**（Option A–D 追記） |
| Reviewer B（#385 契約厳密性） | BLOCKED → 修正済み | mode semantics（MissingMode vs optional modes）→ **ADOPTED**（modeless fixture + 選択 policy 明文化）。length rounding → **ADOPTED**（`ResolvedLength.rounding` + `parse_rounding`）。layer 順序の呼出側依存 → **ADOPTED**（merge 前に layer 順を強制）。alias 宛先型 mismatch → **ADOPTED**（`ResolveError::TypeMismatch`）。部分 alias `"{a}-{b}px"` 拒否 → **ADOPTED**（whole-value のみ受理・`{`/`}` 含有非一致は InvalidValue）。duplicate token path → **ADOPTED**。override merge semantics → **ADOPTED**（同一 key の base+mode map wholesale 置換を明文化） |
| Validator+Judge | 実施中 | rate-limit で 1 回目の validator が未取得のため独立 subagent で再実施。最終 verdict は本ファイルまたは coordinator 報告に追記 |

## 契約仕様の要点（下流 worker 向け）

- `rvf-dtcg-profile.v1`: parse → schema/type validation → **layer merge**（primitive < semantic < component < product・同一 key は高 precedence 側が base+mode map を wholesale 置換）→ **alias 解決**（whole-value `{path.to.token}` のみ・cycle/unknown-ref/type-mismatch は typed error）→ mode/consumer override → 型付き不変 `ResolvedTheme`。
- alias 解決は merge 後 token 選択に対して行う（merge 前 cycle 拒否を持つ汎用 validator とは順序が異なる — 本 crate が正本）。
- override 可能 key = 同一 token path・同型一致 key のみ。`{`/`}` を含むが whole-value 形式でない文字列は InvalidValue。
- dependency: serde + serde_json のみ。`ResolvedTheme`/公開 API に GPUI/DOM/wry/toolkit/OS handle 型を露出しない。GPUI Theme mapper は `ResolvedTheme` 消費側（one-way）で意味論を定義しない。
- fixture 実例: `crates/ruviform-ui-theme/tests/fixtures/`（primitive/component/product-override/semantic-modeless + err-* 7 件）。

## スコープ外 / 残 Gate

- GPUI/Native Dock の最終採否は未判定（#262/#388/#389 実機証拠後に ADR 逆反映）。本 PR は契約設計の確立であり技術採用の最終承認ではない。
- RF386/RF388/RF389/RF387/RF294/RF390 は他 worker 所有。`ruviform-ui` API 移動・旧 frontend 削除は含まない。
- workspace test の plugin-host L1 timeout は既知環境 flake（main でも発生）。
- `doc_maintenance.js` の repo 全体 manual finding は本 scope 外（既存起因）。

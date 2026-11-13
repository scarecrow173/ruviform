# evidence/rf465 — Issue #465 WebView 移行台帳（現行 main 基準）

Issue #465 の有限残件「現行 main 基準の WebView 台帳化・分類検証」の
証拠成果物。P0/P1（PR #475,
[`docs/discovery/issue-465-legacy-webview-inventory.md`](../../docs/discovery/issue-465-legacy-webview-inventory.md)）
の棚卸しを現行 main で再検証し、4 分類（`remove_candidate` / `reuse` /
`provisional` / `exception`）の機械可読台帳として確定する。

**本スライスは分類・記録のみ。物理撤去（P2）は #419 採用判定 +
AL #189 / SV #232 consumer 受入が未達のため一切実施しない。**

## ファイル

| ファイル | 内容 |
| --- | --- |
| `inventory.json` | 台帳正本（機械可読）。`entries[]` に資産単位の分類・根拠・Owner・退役 Trigger・P0/P1 差分を記録 |

## 対象 SHA

| repo | SHA | 備考 |
| --- | --- | --- |
| scarecrow173/ruviform | `fe69f8187587affca2f904e284bcb2bda47974cc` | 本 worktree HEAD（2026-11-14） |
| scarecrow173/audio-lens | `0e2bfee57ead58510ead95212a2540540efc430b` | origin/main HEAD・clean checkout |
| scarecrow173/synvelis | `8119d3e82abd11e68ec708c6dcbb9519d4aa7bb5` | origin/main HEAD・clean checkout |

両 consumer の ruviform pin は `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38`
（2026-11-11、現行 HEAD の ancestor・P0/P1 merge `4e59494a` を含む）。

## 収集方法

- Ruviform: `rg` 相当の全文検索（`webview|wry|tao|rvf://`）+
  `Cargo.toml`/`mise.toml`/`.github/workflows/` 精読 + `git log
  4e59494a..HEAD` による P0/P1 以降の差分確認。
- AL/SV: ローカル checkout（両 repo とも origin/main HEAD に fetch
  済み・clean）での file-level 列挙。`gh api search/code` は
  `incomplete_results: true` で 0 件を返したため、contents 単位の
  列挙は checkout を正本とした（read-only、consumer repo への変更なし）。
  Issue 状態（AL #189 / SV #232 / RF #419 / #262）は `gh issue view`
  で確認 — すべて OPEN。

## 分類凡例（4 分類）

| token | 日本語 | 意味 |
| --- | --- | --- |
| `remove_candidate` | 削除対象 | WebView 固有・共有意味論なし。P2 gate 成立後の撤去候補 |
| `reuse` | 再利用対象 | driver-neutral 契約・共有 schema・独立資産。誤削除禁止 |
| `provisional` | 暫定維持 | legacy だが現役 consumer/test/共有 fixture が依存。Owner+退役 Trigger 付きで維持 |
| `exception` | 例外 | 歴史証拠・明示的保持方針の資産。Owner+保持理由を記録 |

P0/P1 の 3 分類（現行/legacy/共有）との対応: 現行→`reuse`、
共有→`reuse`（schema/fixture）または `provisional`（WebView-bound
wire の port）、legacy→`remove_candidate`（純 WebView 固有）または
`provisional`（consumer 依存中）。

## 集計

| repo | remove_candidate | reuse | provisional | exception |
| --- | --- | --- | --- | --- |
| ruviform | 10 | 16 | 11 | 2 |
| audio-lens | 1 | 1 | 5 | 0 |
| synvelis | 2 | 1 | 3 | 0 |

## P0/P1（PR #475）からの主な差分

**追加（P0/P1 後に新設され台帳へ収録）:**

- Issue #464 Consumer-neutral Scenario Contract —
  `ruviform-ui-test/src/scenario*`、`tests/scenario_contract.rs`、
  `ruviform-ui-gpui/tests/scenario_contract.rs`、mise
  `test-ui-scenario-contract{,-fake,-gpui}` → `reuse`（RV-028）
- Issue #314 Pixel Golden / visual capture — `src/golden*`、
  `fixtures/golden`、`test-ui-golden`、real-window/offscreen/real-os
  scenario lanes → `reuse`（RV-029）
- Issue #313 windowed TestIo/`rvf.uitest.v1` GPUI backend —
  `windowed_test_app`、`test-ui-windowed-uitest` → `reuse`（RV-030）
- Issue #243 multiwindow smoke、#552 SendInput preflight、#598
  `ui-gpui-verify-macos` + `windowed_test_app` build/link、#418 theme
  codegen、#429/#436/#442 consumer observation lane → `reuse`
  （RV-025/RV-033/RV-034 に集約）
- `crates/ruviform-viz`（独立可視化 crate、Issue #157）→ `reuse`
  （RV-031）
- `ci/` contract tests の webview lane 参照 → `provisional`（RV-022）

**消失:** WebView 関連の削除・rename は検出されず（`crates/
ruviform-ui-webview`、`ui/*`、`poc/`、webview mise/CI lane は不変）。

**分類変更:**

- `ui/guitest/fixtures` を package から分離し `reuse` へ
  （`crates/ruviform-ui/tests/ipc_conformance.rs` が直接読む共有
  corpus — package ごと削除すると Rust conformance が壊れる）。
- `ui/{core,components,viz,binding,guitest}` を `legacy`→`provisional`
  へ精緻化 — 両 consumer の vendored ui/* が現役利用（AL headless
  layer・SV WebView frontend）。P0/P1 後も #377/#378/#490 の契約修正が
  投入されており保守継続中。
- `crates/ruviform-ui` 旧 API 面を `legacy`→`provisional` へ —
  FakeHost・synvelis-workspace・al-gui desktop.rs・同 crate tests が
  現役依存。
- `poc/` をコード（`remove_candidate`）と証拠文書（`exception`）に分割。

**consumer 差分:**

- 両 repo の ruviform pin が `fd1d4d5f`（2026-11-11）へ更新。
  SV の webview bin は旧 `HostConfig`/`run` から `SessionConfig`/
  `run_multi` 系へ移行済みで pin 非互換問題は解消。
- AL に `al-desktop` bin（gpui）が新設され webview/gpui lane が CI
  で完全並存（`webview-verify{,-macos}` + `gpui-verify{,-macos}`）。
- SV の `ci.yml`/`daily-ci.yml` に webview 参照ゼロ — webview feature
  lane は mise task のみ残存（local 開発用）。

## 保護対象の検証（誤削除防止）

| 資産 | 台帳上の分類 | 検証 |
| --- | --- | --- |
| `rvf.ipc.v1` schema（`crates/ruviform-ui/src/ipc.rs`） | `reuse`（RV-023） | driver.rs `Action(Intent)`・ui/binding・fixtures・両 consumer が依存。名前で削除禁止（Issue 明記） |
| Semantic/headless Scenario（#464） | `reuse`（RV-028） | `test-ui-scenario-contract` が Fake+GPUI 両 adapter で契約検証。Issue 検証方法が canonical lane を明示 |
| 独立可視化（`crates/ruviform-viz`） | `reuse`（RV-031） | driver/DOM 非依存・media-core/numeric/exec のみ依存。`ui/viz` とは別系統 |
| `ui/guitest/fixtures` 共有 corpus | `reuse`（RV-016） | Rust `ipc_conformance.rs` が直接読み、TS lane と単一正本 |
| headless/CLI（非 UI crate・demo_cli/demo_dock） | `reuse`（RV-032/RV-037） | driver_boundary guard で webview 依存なしを強制 |

## 検証コマンド（本スライスで実行）

- `node -e "JSON.parse(...)"` 相当で `inventory.json` の構文・件数を検証
- `gh issue view` で AL #189 / SV #232 / RF #419 / #262 の OPEN 状態を確認
- `git log 4e59494a..HEAD -- <paths>` で P0/P1 以降の差分を確認
- `mise run fmt-check` / `mise run check ruviform-ui-webview` /
  `lint` / `test`（crate doc 注記を入れた場合）

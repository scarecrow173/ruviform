---
id: "TSPEC-034udQraULNWJKR6z8GYjq"
type: "test-spec"
status: "proposed"
title: "Issue #127 Linux 実配布方式 設計文書の検証仕様 — Matrix 完全性・判定語彙・分離・Owner 境界"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/designs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/plans/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/tasks/issue-127-linux-dist-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-127-linux-dist-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-127-linux-dist-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/designs/issue-127-linux-dist-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 Linux 実配布方式 設計文書の検証仕様 — Matrix 完全性・判定語彙・分離・Owner 境界

## Purpose

Issue #127 の docs-only 設計チェーン（spec/design/test-spec/plan/task）が、「実測/未実測を区別した Matrix」「未実測形式を配布可能と断定しない」「旧 WebView 分離」「Owner 境界」「判定語彙と証拠リンク」の各要件を**文書として正しく満たす**ことを検証する。本 test-spec は文書の構造・内容・語彙の契約を固定する。実機検証そのもの（GPUI 起動・Dock 等）は本 spec の対象外であり、別 Issue の実機 lane が担う。

## Feature

Linux 実配布方式の設計文書チェーン（`docs/{specs,designs,test-specs,plans,tasks}/issue-127-linux-dist-design.md`）。

## Rules

- **R1 AC 完全写像**: spec の Acceptance Criteria が Issue #127 本文の受入 5 行を全て含み、design の §9（または同等の対応表）が各 AC を設計節へ写像している。
- **R2 判定語彙の固定**: design が `Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked` の語彙を定義し、capability matrix・パッケージ判断・distro/Backend の各判定に適用している。語彙の定義と使用が一貫している。
- **R3 実測/未実測の区別**: capability matrix と runtime 依存が「実測（Pass/Fail）」「宣言（Declared）」「未実測（NotRun）」「非対応（Unsupported）」「実行不能（Blocked）」を区別している。`Declared` を `Pass` と混同しない。
- **R4 Linux 実機 Pass ゼロの正直性**: Linux 実機の GPUI capability で `Pass` を主張する記述がない（全て `NotRun`/`Unsupported`/`Declared`）。fake/headless/旧 WebView/他 OS の成功を GPUI Linux 成功に転用しない。
- **R5 パッケージ形式の保留**: deb/AppImage/Flatpak の全形式が `保留`（または同等の非断定）であり、「配布可能」「採用済み」「サポート対象」と断定する記述がない。各形式に 7 観点（同梱/システム依存・sandbox/portal・更新/インストール・OS/Driver 制約・Native Dock・失敗時 fallback・採否根拠）が記録されている。
- **R6 旧 WebView 分離**: 旧 WebView/WebKitGTK 4.1 固有の runtime/Browser 検証が GPUI 配布の現行必須条件から除外されている。残存レガシー依存が #465 撤去計画と分離して明示され、「削除済み」「撤去完了」と主張しない。
- **R7 Owner 境界**: Ruviform（再利用 capability matrix・runtime/ABI/packaging 条件・共通 GUI/NativeSurface 制約）と AudioLens/Synvelis（製品固有の配布形式・署名・同梱・対応 OS・リリース workflow・製品受入）が区別され、Ruviform の判断が両 Consumer に暗黙の必須依存を生じさせない条件が記述されている。
- **R8 確認手順と証拠リンク**: 実行可能な確認手順（コンテナ lane・実機 matrix・判定記録要件）が定義され、実機・Native Plugin Dock 受入が #419/#262、Consumer 非回帰が #294 の独立証拠へリンクされている。
- **R9 docs-only スコープ**: 変更が `docs/` 配下のみで、`crates/`・`Cargo.*`・`mise.toml`・`.github/` を含まない。PR diff が docs のみである。
- **R10 front matter 関係の整合**: 5 文書の `relations` が相互リンクし（spec→design→test-spec→plan→task）、`implements`/`verified-by`/`derives-from`/`verifies` が正しい方向を向く。全 `relations` の path が実在する。
- **R11 索引・監査**: `docs/{specs,designs,test-specs,plans,tasks}/README.md` が再生成済みで、新規 5 文書が索引に載り、`audit_docs` で新規文書の error が 0（既存の無関係 findings は対象外）。
- **R12 未実施検証の非主張**: `mise run fmt-check`・index 再生成・audit を実際に実行していない限り「pass」と報告しない。`NotRun` を正直に記録する。

## Examples

- Given design §3 capability matrix、When 「X11 ウィンドウ生成」行を確認、Then 判定が `NotRun`（または `Declared`+`NotRun`）であり `Pass` ではない。
- Given design §4 形式比較、When 「Flatpak の採否」行を確認、Then `保留` であり「配布可能」「採用」と断定していない。
- Given design §6、When 「WebKitGTK 4.1」言及を確認、Then 「歴史的参照」「#465 撤去」「削除済みではない」の文脈であり、GPUI 必須条件として扱われていない。
- Given spec Acceptance Criteria、When Issue #127 本文の受入 5 行と照合、Then 5 行全てが spec/design に写像されている。
- Given PR diff、When `git diff --name-only origin/main...HEAD` を確認、Then `docs/` 以外のファイルが含まれない。

## Verification

- 文書レビュー: R1–R8・R10 を人手/レビューで確認。
- 構造検証: `node scripts/doc_maintenance.js apply --type <type> --dir <path>` で索引再生成 → `audit_docs` で新規文書 error 0 を確認。
- ビルド検証: `mise run fmt-check`（docs のみ変更だが正規入口として実行）。
- スコープ検証: `git diff --name-only origin/main...HEAD` が `docs/` のみであること。

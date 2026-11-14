# 内部 Plan — Issue #275 OBS-01〜11 受入照合台帳と Consumer Gate 割当

- 文書種別: 内部 plan（`obs-acceptance-audit-design.md` の実施計画相当）
- 日付: 2026-11-14
- 対象 Revision（監査基準）: `20d8e7c23bc365e26060a4ef82d463bb7eed38bc`

## 実施手順

1. **証拠収集（実測）**
   - `gh issue view 275/281`（ruviform）・`gh issue view 127`
     （audio-lens）・`gh issue view 28`（synvelis）で現行
     受入条件・gate 構成・Case 組を取得。
   - `gh pr view 521/547` と merge commit で正本・契約
     Fixture の所在を確認。
   - `gh run list`/`gh run view --job --log` で直近 CI・
     consumer-verify の run・job・test 件数を取得。
   - `git diff <sha>..<sha> -- <crates>` で SHA 間の
     crate 別同一性を確認。
   - `crates/ruviform-observability`（`Cargo.toml`・`src/`・
     `tests/`）・`crates/ruviform-plugin-host` の module/
     feature/test target を棚卸し。
2. **ローカル補助証拠**
   - `mise run test ruviform-observability --all-features`
     （Windows・dev toolchain）— 全 feature の test target
     実行。
   - `mise run test ruviform-plugin-host`（Windows）—
     OBS-04 fixture 実行。runner bin 不足の環境起因失敗は
     `cargo build -p ruviform-plugin-export --bins` で補い
     再実行して切り分ける。
3. **台帳作成**: `evidence/rf275/obs-acceptance-ledger.md`
   に §1 照合表・§2 証拠行列・§3 Gate 割当・§4 SHA 引継・
   §5 判定メモを記述。
4. **canonical doc 追記**: 統合設計正本へ台帳参照 +
   Gate 割当要約の追記型セクションを追加（既存記述は不変）。
5. **検証**: `mise run fmt-check`、`git diff` で変更範囲・
   追記のみを確認、review-protocol レビュー。
6. **統合**: `git fetch origin main && git merge origin/main`
   → 生成物確認 → commit（Conventional Commits）→ push →
   `gh pr create`。

## ファイルマップ

| ファイル | 操作 |
| --- | --- |
| `evidence/rf275/obs-acceptance-ledger.md` | 新規（台帳正本） |
| `evidence/rf275/obs-acceptance-audit-{spec,design,plan,task}.md` | 新規（内部 doc chain） |
| `docs/designs/issue-275-observability-integration-design.md` | 追記型差分のみ |

## 依存・順序

- 手順 1〜2 は並行可。台帳は証拠確定後に一括作成。
- canonical doc 追記は台帳の Gate 割当表と整合させる。

## 検証マトリクス

| 検証 | コマンド | 期待 |
| --- | --- | --- |
| 書式 | `mise run fmt-check` | exit 0 |
| 変更範囲 | `git status`/`git diff --stat` | evidence/rf275 + 設計正本のみ |
| 追記のみ | `git diff docs/designs/issue-275-observability-integration-design.md` | 追加行のみ・削除/変更行なし（追記セクション内を除く） |
| 台帳突合 | 証拠ファイル・gh 出力との照合 | 全参照が実在 |

## Rollback

文書のみの変更。問題時は当該 commit の revert で戻せる。
コード・CI・manifest への影響なし。

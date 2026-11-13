---
id: "EXP-issue-306-fault-seam-fuzz"
title: "Issue #306 G07 fault seam 実現可能性の検討と clippy 修正経緯"
created: "2026-11-14"
updated: "2026-11-14"
---

# Issue #306 G07 fault seam 実現可能性の検討と clippy 修正経緯(2026-11-14)

- 計測契約: `docs/specs/issue-306-fault-seam-fuzz.md` +
  `docs/designs/issue-306-fault-seam-fuzz-design.md`（approved）
- 正本証拠: `evidence/rf306/fault-seam-fuzz-report.md`

## 1. G07 seam の実現可能性検討

### 問い

前 slice（`evidence/rf306/fault-fuzz-report.md`）で実 OS から
誘発不能と記録された `Indeterminate`・publish 後 DirSync 失敗・
`TempState::Unknown` を、公開 API を変えずに決定的に検証する
seam を `ruviform-io-file` に安全に設けられるか。

### 調査した選択肢

| 選択肢 | 判定 | 根拠 |
| --- | --- | --- |
| A. 実 FS fixture で誘発を試みる | 不可 | OS が publish 原子性・dir sync 失敗・cleanup 結果を制御不能に返す場面を決定的に作れない（前 slice の NotRun 記録どおり） |
| B. `ruviform-io-e2e` 側に mock を複製 | 棄却 | `MockFileOps`/`MockScript`/pipeline 内部型を fork すると oracle が二重化し、pipeline 本体との乖離を検出できない |
| C. 公開 API に fault 注入点を追加 | 棄却 | 本番署名の変更は Issue の「公開 API を変えない」制約に反する |
| D. 既存 `#[cfg(test)]` mock を opt-in feature で外部公開 | **採用** | `PlatformFileOps` が private trait で全 OS 操作の境界であり、mock はその in-memory 実装。`cfg(any(test, feature))` + `#[doc(hidden)]` + std-only facade で漏洩を封じ込められる |

### 採用形状（D）の安全性確認

- `PlatformFileOps`/`MockFileOps`/`MockScript`/`MockPublish`/
  `StageOwner`/`RecoveryInfo` は `pub(crate)` または private の
  まま — facade（`FaultScript`/`PublishScript`/`PublishReport`/
  `TempReport`/`staged_file`/`MockFs`）の公開 signature は std 型と
  公開 result 型のみで、private 型を出さない（`private_interfaces`
  を構造的に回避）。
- `StagedFile::with_ops` は `pub(crate)` のまま cfg のみ拡張。
- feature は非 default・additive。io-e2e の dev-dep 経由でのみ
  有効化され、既定 build・公開 API・RT 面は不変
  （feature 無し `check-io-file`/`lint-io-file`/`test-io-file`
  全 pass で確認）。
- `MockPublish.physically_applied` を report から独立した軸に
  維持し、「Unknown 報告 + 物理適用済み」の正直なモデルを可能に
  した（`Indeterminate` 契約試験で使用）。

## 2. `clippy::assigning_clones` 修正の経緯

`lint-io-file`（`clippy --all-targets -- -D warnings`）で
`mock.rs` の publish 実装 `fs.dest = fs.temp.clone()` が
`assigning_clones` に検出された（新しい clippy で導入された
lint。対象コードは本 slice で facade 追加のために触れた既存行）。

試行と結果:

1. `fs.dest.clone_from(&fs.temp)` — clippy 提案どおりに適用したが
   `error[E0502]`: `fs` が `MutexGuard` であり、`fs.dest` の
   mutable borrow と `fs.temp` の immutable borrow が guard の
   `DerefMut` 経由で disjoint と認識されず borrow check 失敗。
2. local へ clone してから代入:
   `let published = fs.temp.clone(); fs.dest = published;`
   — borrow は逐次で競合せず、lint も `assigning_clones` の
   対象外（clone は代入式ではなく事前束縛）。`#[allow]` 不使用・
   挙動不変。`lint-io-file`/`test-io-file` で再検証済み。

## 3. G08 corpus replay の設計検討

- PRNG: 依存追加を避けるため splitmix64 を test file 内に実装
  （外部 crate・clock・環境依存なし）。
- 変異: byte flip（1–4 箇所）/ truncation / PRNG byte 拡張 /
  byte splice の 4 系統を case ごとに決定的に選択。
- panic 方針: reject と panic を区別するため `catch_unwind` で
  計数し、合計 0 を assert。panic hook は計数中のみ無音化して
  必ず復元（resume_unwind で再送出）。
- 除外: deflate（io-e2e の feature 集合外）・WAV（#305 lane）・
  secure extraction/publish FS 経路（FS 副作用を持つため
  `fault_boundary`/`fault_seam` が正本）・sustained fuzz
  （外部 gate・Deferred）。

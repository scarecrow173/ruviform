# 内部 Task — Issue #625 main tip deterministic compile error 2 件の修正

- 文書種別: 内部 task（`ci-compile-fix-plan.md` の作業分解相当）
- 日付: 2026-11-16
- 対象 Revision: `cbcc83437616fdd7418da37183c6be66b6cfe225`

## Tasks

- [x] T1: Full CI run `43783872777` の失敗 job 一覧と一次
      ログを `gh run view` で取得し、E0433 ×9（output_ports
      test）と `unnecessary_cast` ×5（events.rs）を確認する。
- [x] T2: 原因を code path で特定する — output_ports.rs の
      cfg-gated `ruviform_media_core` import と非 gated
      `real_device_send_smoke` の不整合、events.rs の
      `StatesAndFlags`（`DefaultEnumType` = `c_int`/`c_uint`
      platform 揺れ）への `as u32` no-op cast。既存 idiom
      （`sdk_enum_i32`・`enum_to_*`）と file-level `#![allow]`
      先例（component.rs）の存在を確認。
- [x] T3: `output_ports.rs` の media_core import から
      `#[cfg(windows)]` を除去する（honest-exit 設計を維持）。
- [x] T4: `events.rs` に `flag_to_u32` helper
      （`impl Into<i64>` + `try_from`）を追加し、5 箇所の
      `as u32` を置換する。
- [x] T5: ローカル検証 — `mise run fmt-check`、
      `mise run check|lint|test ruviform-midi-device`、
      `mise run check|lint|test ruviform-plugin-test-vst3`。
- [x] T6: 補助検証 — `x86_64-unknown-linux-gnu` /
      `aarch64-apple-darwin` target の cargo check/clippy で
      失敗 cfg 環境を再現検証。
- [x] T7: `evidence/rf625/` に doc chain と検証記録を作成し、
      review-protocol レビューで blocker を解消する。
- [ ] T8: `git fetch origin main && git merge origin/main` →
      commit → `fix/625-ci-lanes` へ push → `gh pr create`
      （`Refs #625`）→ worker_done 報告。

## 判断メモ

- doc chain を canonical `docs/` ではなく `evidence/rf625/`
  に置くのは、編集範囲が指定 2 file + `evidence/rf625/` に
  限定されているため。内部文書として spec→design→plan→task
  の連鎖を維持する（rf275 slice と同じ配置先例）。
- `evidence/rf625/` には前 slice（e2e handshake 分類修正、
  PR #626 で merge 済み）の `root-cause.md`/`verification.md`
  が既存 — 本 slice の記録は `ci-compile-fix-` prefix で
  併存させ、既存 file は変更しない。
- output_ports.rs は「test を `#[cfg(windows)]` に寄せる」
  代替案を検討したが、non-Windows honest-exit カバレッジを
  失うため import の gate 除去を採用（design 参照）。
- events.rs は `component.rs` の file-level
  `#![allow(clippy::unnecessary_cast)]` 先例があるが、本 slice
  要件が `#[allow]` 禁止のため `flag_to_u32` helper 方式を
  採用。`#[expect]` は Windows で unfulfilled になるため不可。

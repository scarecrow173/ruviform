# fixtures/golden — Pixel Golden baseline store

Issue #314 slice 1 の baseline store 規約。実 baseline PNG は
consumer 側（各製品 repo / `target/` 配下の staging）が生成し、
本ディレクトリは **layout・承認手順・監査規約の正本**として
README のみを置く（合成画像は unit/integration test が
`temp_dir` 上で生成し、repo には commit しない）。

## layout

```
<baseline_root>/<story_id>/<theme>/<state>/<file_stem>.png
<baseline_root>/<story_id>/<theme>/<state>/<file_stem>.json
```

- `<file_stem>` は `GoldenKey::file_stem()` が返す fs 安全文字列
  （`:` `/` 等は `_` に sanitize 済み）。canonical key 全体は
  sidecar の `key` フィールドに JSON で保持される。
- sidecar schema は `rvf.golden.baseline/v1`。key・environment・
  policy（channel tolerance + mask）・image meta（寸法/byte 長/
  fingerprint）・approval を持つ。
- candidate は staging root（慣例 `target/golden/`）配下に同一
  layout で書かれ、sidecar の `approval` は `null`。

## 更新手順（candidate → 人間承認 → promote）

1. `BaselineStore::write_candidate(staging, key, image, env, policy)`
   で staging に candidate を書く。**この時点で baseline は
   一切変更されない**。
2. diff 画像・stats・evidence（`RUVIFORM_GOLDEN_EVIDENCE` 指定時）
   を人間がレビューする。
3. `BaselineStore::promote(staging, key, BaselineApproval{..})` で
   staging candidate を baseline path へ atomic に移す。promote
   時に candidate の存在・sidecar 整合・key 一致を再検証する。
   `BaselineApproval` は承認者識別と note を必須とし、sidecar の
   `approval` に記録される（audit trail）。

baseline の直接上書き API は存在しない。stale/破損 sidecar・
PNG/sidecar の key 不一致は `StoreError`（verdict ではなく tool
error）として失敗する。

## 語彙・evidence

- verdict は driver-lab と同一の `pass/fail/not_run/unsupported/
  blocked`。`compared_pixels == 0`・size mismatch・baseline 不在は
  `pass` にしない（`blocked`/`fail`）。
- evidence は `RUVIFORM_GOLDEN_EVIDENCE` 設定時のみ JSONL
  （schema `rvf.golden.evidence/v1`）を append。driver-lab の
  `RUVIFORM_DRIVER_LAB_EVIDENCE` と同系の env-gated 方式で、
  通常 test 実行は副作用を持たない。

## 対象外（slice 3 以降）

実 pixel capture（offscreen 実測・`TestIo::screenshot` 実
backend・OS window capture）は本 slice の対象外。capture 層は
`unsupported`/`not_run` を正直に返す契約のみ定義済み。
semantic oracle・DrawPlan・story fixture 接続も対象外
（#464/#426/#313 側の責務）。

# rf#1005 consumer-oracle doc chain stale 修正 — 照合記録

- Issue: `scarecrow173/ruviform#1005`
- 対象: `docs/designs/issue-630-consumer-oracle.md`・
  `docs/test-specs/issue-630-consumer-oracle.md` の事実記述更新
- 実測日: 2026-11-25
- OS: Windows (OS-L1)
- 作業 HEAD: ruviform `ca5f352ba8ba0f401404c67990e147426590761c`
  （PR #1004 merge 後の main。`git fetch origin main` 後に
  `docs/1005-consumer-oracle-docchain` を作成）
- 変更種別: docs+evidence のみ。`.rs`・`tests/`・`Cargo.toml`・
  `Cargo.lock`・`mise.toml`・`.github/**`・docs index・consumer
  repository は編集していない。

## 事実の再実測

consumer repository は read-only で参照した。Orca worktree からは
兄弟パス `../synvelis`・`../audio-lens` ではなく、実体パス
`/g/repositry/audio-products/{synvelis,audio-lens}` を対象にした。
両 consumer repository の `git status --porcelain` は 0 行
（clean）だった。

| 対象 | 測定 SHA / 値 | 結果 |
| --- | --- | --- |
| ruviform HEAD | `ca5f352ba8ba0f401404c67990e147426590761c` | PR #1004 merge 後の main と一致 |
| synvelis HEAD | `8585cda756231f4e33a50370b4e1f3cf42c53059` | 前提値と一致 |
| audio-lens HEAD | `7fd87add13e24858f6677d9c2dea135bf581ebe6` | 前提値と一致 |
| synvelis `Cargo.lock` sha256 | `303e2c856c1333639ee689a7bda1c9636ccdf230261793609d7bdf6328cc8b8a` | 補助記録 |
| audio-lens `Cargo.lock` sha256 | `4bcf6b149c91175537da1312e109b6046e236bd6619bb2913020dfae6279e44c` | 補助記録 |
| synvelis lock 内 ruviform resolved rev | `20dc41f71c47674b3e51d1eb0921124b94f8bfaf` | fixture 3 crate entry が同 rev を指す |
| audio-lens lock 内 ruviform resolved rev | `e4c9f6f0715da3ac479f8c437d21a021327e51da` | fixture crate entry なし |
| `scarecrow173/ruviform#532` | `state=CLOSED`・`closedAt=2026-11-21T11:35:15Z` | open gate ではなく resolved 相当 |

`#532` の確認コマンド:

```sh
gh issue view 532 -R scarecrow173/ruviform --json number,state,closedAt,title
# exit 0 / state=CLOSED / closedAt=2026-11-21T11:35:15Z
```

## dep 非汚染の grep 実測

対象名は lock パッケージ名の
`ruviform-plugin-dock-test`・`ruviform-plugin-test-vst3`・
`ruviform-plugin-test-clap`。`test_instrument` は fixture class
名であり独立 crate 名ではないため、grep 対象の crate 名には
含めない。

| コマンド | exit | 観測結果 |
| --- | --- | --- |
| `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test" /g/repositry/audio-products/audio-lens/Cargo.lock` | 1 | fixture crate 名非含有。現時点の audio-lens 側の観測は従前どおり |
| `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test" /g/repositry/audio-products/synvelis/Cargo.lock` | 0 | 6 行に match。`ruviform-plugin-dock-test`（L6532）・`ruviform-plugin-test-clap`（L6576）・`ruviform-plugin-test-vst3`（L6590）の package entry と、`synvelis-app` dependencies 側の参照 3 行（L7712/L7714/L7715） |
| `sed -n '/^\[dependencies\]/,/^\[/p' /g/repositry/audio-products/synvelis/crates/synvelis-app/Cargo.toml \| grep -nE '^(ruviform-plugin-test\|ruviform-plugin-dock-test)'` | 1 | production `[dependencies]` には fixture crate 非含有 |
| `sed -n '/^\[dev-dependencies\]/,/^\[/p' /g/repositry/audio-products/synvelis/crates/synvelis-app/Cargo.toml \| grep -nE '^(ruviform-plugin-test\|ruviform-plugin-dock-test)'` | 0 | `[dev-dependencies]` の直接依存行 3 行に match。`ruviform-plugin-dock-test.workspace = true`・`ruviform-plugin-test-vst3.workspace = true`・`ruviform-plugin-test-clap.workspace = true`。依存行に anchor したためコメント中の名前は数えない |

synvelis 側 `crates/synvelis-app/Cargo.toml` の現行配置も確認した
（read-only）。`[dependencies]` は 11-39 行、`[dev-dependencies]`
は 41-53 行で、fixture 3 crate は後者にのみある。したがって
synvelis の含有は sv#176 S1 による consumer の dev-dep 自主採用
であり、production DAG 汚染ではない。audio-lens は fixture crate
を依然含有しない。combined grep はどちらか一方で match すれば
exit 0 になるため、本記録では repo ごとの grep を採用した。

## 文書ごとの矛盾箇所と修正方針

| 文書・箇所 | 旧記述の矛盾 | 採用した修正 |
| --- | --- | --- |
| `docs/designs/issue-630-consumer-oracle.md` verdict 語彙 | manifest の語彙を 3 種とし、`gate` を `#532` の着地待ちとして説明していた。現行 manifest の `policyVocabulary.dispositions` は `headless-contract-match`・`external-owner`・`gate`・`resolved` の 4 種で、`ac4-real-window-dcomp` は `resolved` | 語彙数を 4 種へ修正。`gate` は外部修正 Issue の着地を待つ一般語として残し、`#532` CLOSED に伴う ac4-real-window-dcomp の resolved 相当を事実として分離した。`resolved` の定義は現行 manifest に合わせた |
| `docs/designs/issue-630-consumer-oracle.md` dep 非汚染の証明形 | 「両 consumer `Cargo.lock` 非含有・exit 1 期待値」という旧事実を残していた | 構造半分（ruviform production DAG 非依存）と実測半分（consumer lock/依存経路）の証明構造は維持し、2026-11-25 測定時点の repo ごとの結果へ修正。synvelis dev-dep 自主採用と audio-lens 非含有を分離し、dev-dep 採用は強制や production 汚染ではないと明記した |
| `docs/test-specs/issue-630-consumer-oracle.md` dep 非汚染 Rule/Example | combined grep が両 consumer とも exit 1 になることを期待していた。現行では synvelis が match するため成立しない | repo ごとの grep、synvelis production `[dependencies]` 非含有、`[dev-dependencies]` 経路、evidence 記録要件を分けた Rule/Example へ修正。combined grep を repo ごとの結論に使わないことを明記した |

両文書の frontmatter `updated` は 2026-11-25 へ更新した。
`status`・fixture contract・manifest schema・production 非汚染
ルール・consumer 実走 legs の責務境界は変更していない。

## `docs/specs` との整合確認

`docs/specs/issue-630-consumer-oracle.md` は read-only で突合した
（編集なし）。少なくとも次の語彙・事実は矛盾なく読める:

- 要件 4・受入条件 4: audio-lens 非含有、synvelis dev-dep 経路
  含有、production DAG 非含有、2 半分の分離記録。
- 非目標: `scarecrow173/ruviform#532` は CLOSED（2026-11-21）、
  ac4-real-window-dcomp は resolved として追跡。
- dev-dep 自主採用は「強制しない」の否定にならない。

本 slice は spec 側の語彙へ寄せた事実修正であり、spec 側との
意味上の矛盾は確認されなかった。

## 規範の意味を変えていない理由

修正は観測対象・測定時点・語彙の現行数値を正本（manifest・spec）
へ合わせるものに限定した。`gate` 語彙の削除や schema 変更は
していない。production DAG 非汚染の規範は維持し、consumer が
dev-dep で fixture を自主採用することを違反扱いする新規範も
追加していない。consumer 実走 legs と headless contract match の
責務境界も従前どおりである。

## 正規検証

正規入口は `mise.toml` のタスク。変更は Markdown のみのため、
`POLICIES.md` §5 の文書軽量チェックとして次を実施する。cargo
`check`/`lint`/`test`/`build` は本 slice の必須検証に含めない。

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Passed — `.rs` 非回帰の非破壊確認（本 slice は `.rs` を触らない） |
| `mise exec -- prek run --all-files` | 0 | Passed — trailing whitespace・EOF・yaml・toml・line ending・BOM・private key・illegal names・actionlint 全緑 |

事実照合の grep/SHA 実測は evidence 記録用の read-only 補助
コマンドであり、lane 化していない。

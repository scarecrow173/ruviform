# rf#1014 verification — ruviform slice

## 対象

- Repository: `scarecrow173/ruviform`
- Branch: `scarecrow173/rf1014-rf-policy`（Orca worktree 既定名。rename していない）
- Base / `origin/main`: `32612b574c6ee4ef02a4ed1b9bbf8739f5dc5f4f`
- 作業開始時 HEAD: `32612b574c6ee4ef02a4ed1b9bbf8739f5dc5f4f`
- 作業 SHA（policy / skill / spec 実装 commit）: `0429320c1548e1e43049fa664403c65485b7b487`
- OS/Arch: Windows 11 / AMD64
- 実行日: 2026-11-25（UTC）

## 正本の取得と transcription fidelity

正本は次のコマンドで取得した live Issue 本文である。

```sh
gh issue view 1014 -R scarecrow173/ruviform --json body --jq .body
```

Dispatch snapshot と live Issue の方法論節に意味上の差異はなかった。`scarecrow173/ruviform#1015` も同じ方法で取得し、未移行の個別検証の owner として参照した。

`POLICIES.md` §6 への写し方は、見出しの追加と節番号の付与だけを編集上の変更とし、rule 本文は live Issue の記述を保持した。確認には Python で live Issue を再取得し、各節の本文が `POLICIES.md` に含まれるかを照合した。

| 対象 | 結果 |
| --- | --- |
| 共通原則 10 件 | 10/10 が `POLICIES.md` §6.1 に verbatim で存在 |
| 検証粒度・境界 | 本文 403 chars が verbatim で存在 |
| 手法選定・契約分解 | 本文 398 chars が verbatim で存在 |
| 健全性・検証可能性 | 本文 488 chars が verbatim で存在 |
| コスト・証跡・同期 | 本文 567 chars が verbatim で存在 |
| 開発工程・管理場所・責務 | 本文 571 chars が verbatim で存在 |

照合結果は `principles 10 of 10`、5 節とも `verbatim`、`common heading True`、`renumbered heading True`、`effective unchanged True` だった。共通原則と 5 つの方法論節を合わせた 6 つの方法論節の coverage を満たす。rule の追加・省略・緩和・強化はしていない。語彙の言い換えはしていない。

共通節には `test-formal`、`test-formal-kani`、wire/archive/structured/WAV、rf#806、rf#1015 への言及を入れていない。これらは §7、verify skill、formal spec にだけ配置した。`RF`/`AL`/`SV` の責務文は rf#1014 本文にある 3 repository 共通の責務なので共通節へ含めた。

## 変更内容

### `POLICIES.md`

- 現行 §5 と旧 §6 のあいだに `## 6. 形式的検証の原則・検証粒度・検証境界` を挿入した。
- 旧 `## 6. Ruviform固有の適用` を `## 7. Ruviform固有の適用` に renumber した。§1〜§5 は変更していない。
- §3 の `Required`/`Deferred`/`NotApplicable` と `Pass`/`Fail`/`NotRun`/`Unsupported` は rename せず、保証種別・対象・同期状態・失敗原因を独立軸として扱う cross-reference だけを追加した。
- §7 に、wire codec、archive、structured documents、PCM WAV の既存個別検証が `scarecrow173/ruviform#1015` で Property 起点へ移行する対象であること、本ポリシー反映が個別 proof/Stub/Bound/Unwinding Assertions/`cfg(kani)`/既存 lane の承認または再設計を意味しないことを追加した。
- `Effective` 行は変更していない。3 repository 同期改訂で合意済みの date/version が Coordinator から示されていないため、推測で新しい日付を追加しなかった。

### `.agents/skills/verify-ruviform/SKILL.md`

- `## Formal` を書き直し、`test-formal`/`test-formal-kani` を普遍的な昇格階層ではなく、既存の個別 Property に対して選ばれた complementary lane として説明した。
- 方法論の正本を `POLICIES.md` §6、個別 Property/Bound/証跡仕様を `docs/specs/formal-verification.md`、この skill を対象固有の実行・保守手順とする責務分離を追加した。POLICIES の rule 本文は複製していない。
- 製品・モデル・harness・依存変更時に、影響する Property を特定し、最小 lane/test を再検証し、結果・assurance kind・target・sync state・failure cause を evidence へ同期し、再検証完了まで verified sync commit を進めない手順を追加した。
- `--no-unwinding-checks` と WAV の `cfg(kani)` 診断 payload 置換を、`scarecrow173/ruviform#1015` が再評価する未移行 legacy exception と明示した。proof placement の `#[cfg(kani)] mod proofs` と区別した。
- honest verdict、環境欠如を pass にしない運用、local/integration の区別、mutation-fail evidence、Kani/CBMC version 記録、proptest regression seed replay、formal spec との同期を保持した。

### `docs/specs/formal-verification.md`

- 冒頭に consistency note を追加し、この文書が rf#806 で導入された個別 verification lane・Bound・証跡の inventory/spec であり、現行方法論の正本は `POLICIES.md` §6 であることを明示した。
- `D 層` と `proptest → Kani → TLA+/Lean` を固定昇格路ではなく rf#806 当時の導入経緯または個別 lane の説明として読む note を追加した。front matter の `title`/`status` は歴史的識別子と lifecycle 範囲外として変更せず、`updated` だけを 2026-11-25 に更新した。
- `test-formal` を第一層、`test-formal-kani` を第二層とする説明を、既存 Property に対する complementary lane の説明へ置き換えた。
- `## 昇格経路（本 slice 外）` を `## 手法適合候補（固定昇格路ではない）` に置き換え、採用義務・順序・昇格経路ではないことを明示した。
- `kani::assume` と Bound に、正当な事前条件/限定領域、充足可能性、到達可能性、保証差の記録という評価軸を追加した。個別 harness の再審査は `scarecrow173/ruviform#1015` に委ねることを明示した。
- `--no-unwinding-checks` と `cfg(kani)` 診断 payload 置換を legacy exception として分類し、通常の Proof Pass の根拠や新規 copy 可能な pattern としないことを明示した。
- Harness/Bound/unproven-area/complement table、resource-caused `not_run`、manual/opt-in、mutation-fail evidence、honest verdict の記述は削除していない。

## AGENTS.md と docs index の判定

`AGENTS.md` は編集していない。lines 5-7 が Issue 起票・設計・実装・テスト・レビュー・受入判定で `POLICIES.md` を読むことを必須にしており、詳細を重複記載しない rule もある。形式的検証だけがこの導線では辿れない客観的な gap は確認できなかった。`POLICIES.md` §6 はこの導線で到達できるため、size budget を持つ `AGENTS.md` へ詳細や個別 signpost を複製しない判断とした。

`docs/specs/README.md` は編集していない。`formal-verification.md` の row が line 260 に存在することを確認した。本 slice は新規 spec doc を追加せず、front matter の title/status も変更しないため、index 変更は不要と判断した。

## renumber reference grep

実施コマンド:

```sh
rg --hidden -n "POLICIES(\.md)?.*(§6|6節)|(§6|6節).*POLICIES(\.md)?" . --glob '!target/**' --glob '!apm_modules/**' --glob '!.git/**'
```

分類結果:

- `.agents/skills/verify-ruviform/SKILL.md` と `docs/specs/formal-verification.md` の hit は、本 slice で追加した新 §6 への正しい参照。
- `docs/designs/issue-627-policies-revision-proposal.md` line 72 と `evidence/rf627/policies-baseline-comparison.md` line 25 は、当時の §6 が repo 固有節だったことを示す歴史的記録。renumber に合わせて書き換えない。
- `docs/specs/issue-627-virtual-first-verification-contract.md` line 114 は、別途確認した歴史的 spec であり、当時の §6 を指す記録なので書き換えない。
- active guidance のうち、旧 §6 を Ruviform 固有節として参照し続けるものは見つからなかった。

## 固定昇格 framing grep

実施コマンド:

```sh
rg -n "property-based testing → bounded model checking|proptest → Kani → TLA\+ → Lean|proptest.*Kani.*TLA|昇格経路|再昇格|第一層|第二層|D 層|D-layer" POLICIES.md AGENTS.md .agents/skills/verify-ruviform/SKILL.md docs/specs/formal-verification.md
```

残存 hit と判定:

- `POLICIES.md` line 97: 固定昇格路を廃止する rule 自体。正当な残存。
- `docs/specs/formal-verification.md` line 5: front matter title。rf#806 の実装 spec を識別する歴史的名称であり、consistency note で現行方法論と区別した。
- 同 line 87: consistency note が `D 層` と `proptest → Kani → TLA+/Lean` を固定昇格路として読まないための説明。正当な残存。
- 同 line 399: 「採用義務・順序・昇格経路ではない」という否定形。正当な残存。
- `AGENTS.md` と `.agents/skills/verify-ruviform/SKILL.md` には、旧昇格路を現行方法論として示す hit はなかった。

## 正規検証

実装 commit `0429320c1548e1e43049fa664403c65485b7b487` に対して実行した。

| コマンド | exit code | 結果 |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass。Rust 差分なし。書き換える `mise run fmt` は使っていない |
| `mise exec -- prek run --all-files` | 0 | Pass。auto-fix による差分なし |
| `mise exec -- prek run --all-files`（evidence 追加後） | 0 | Pass。auto-fix による差分なし |

`git commit` 時の pre-commit hook も全項目 Pass だった。lease 外ファイルへの auto-fix はなかった。evidence 追加後の最終結果は、この行を加えた最終本文でも同じコマンドを再実行し、PR 本文に記録する。

## 実行しなかった検証

| 検証 | 判定 | 理由 |
| --- | --- | --- |
| `mise run test-formal` / `mise run test-formal-kani` | NotRun | 本 slice は docs/policy-only であり、既存 lane の移行や健全性審査は `scarecrow173/ruviform#1015` の scope。任意実行は acceptance の根拠にしない |
| Workspace full lane、Feature Matrix、MSRV、3OS 横断 | NotApplicable | 製品コード・依存・公開 API を変更していない docs-only slice。POLICIES §5 の比例原則に従う |
| GUI-L2 / EXT-L3 / 物理機材 | NotApplicable | ポリシー変更自体に新たな GUI/物理ゲートは要求されない |
| merge 後 main SHA の必須 CI / 3 repository semantic comparison | NotRun | ローカル slice では実行できない。Coordinator が統合レビューで確認する |

## formal spec の通読確認

`docs/specs/formal-verification.md` を通読し、Harness/Bound/unproven-area/complement table、resource-caused `not_run`、manual/opt-in lane、mutation-fail evidence、honest verdict を保持したまま、固定昇格路だけを現行方法論から外したことを確認した。長い歴史記録は削除していない。

## cross-repo comparison に必要な情報

ruviform slice 単独では 3 repository semantic equality を完了できない。比較対象の共通節は `POLICIES.md` `## 6. 形式的検証の原則・検証粒度・検証境界` であり、ruviform 側の実装 commit は `0429320c1548e1e43049fa664403c65485b7b487`、base/sync 元の `origin/main` は `32612b574c6ee4ef02a4ed1b9bbf8739f5dc5f4f` である。最終的な PR head SHA、audio-lens/synvelis との意味比較、merge 後 main SHA の CI 証拠は Coordinator の統合作業に残る。

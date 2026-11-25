# rf#1011 ubuntu clippy `unused_self` 修正の検証記録

## 失敗分析

失敗の正本は GitHub Actions run `45545064602`、SHA `ca5f352ba8ba0f401404c67990e147426590761c` である。失敗した job は `verify (ubuntu-latest)`（job `138972590139`、step `Lint workspace (Linux; mise run lint-full)`）と `l1-real-verify (ubuntu-latest)`（job `138972589998`、step `Lint host feature (mise run lint-l1-real-plugin)`）である。

当初の clippy diagnostic は、対象 run の両 ubuntu job で共通だった。対象は `crates/ruviform-plugin-vst3/src/host.rs:2289` の `fn focus_host_window(&self, _view: &ComPtr<IPlugView>)` に対する `clippy::unused_self` である。この関数は `#[cfg(not(target_os = "windows"))]` 側の defined refusal で、Windows ではコンパイルされない。一方、ubuntu の lint lane は `-D warnings` で実行され、workspace lint の pedantic 警告が error になるため、ubuntu job でだけ失敗として現れた。

最初の branch CI では、VST3 側の修正後に compile cutoff で隠れていた追加指摘が `crates/ruviform-plugin-clap/src/host.rs:1260` の `#[cfg(not(target_os = "windows"))] fn focus_host_window(&mut self)` に残っていることが分かった。こちらも non-Windows 側だけが `self` を使わず、同じ lint が ubuntu の両 lint lane で発火した。

## 修正方針の判断

VST3 側の `focus_host_window` は public API でも trait 実装でもなく、`Vst3Plugin` の private inherent method である。共有 callsite は `set_gui_focus_target` 内の `GuiFocusTarget::HostWindow` だけであり、Windows 側が必要とする instance state は Copy な `gui_parent` だけである。このため、`#[allow(clippy::unused_self)]` や `#[expect(clippy::unused_self)]` で lint を隠さず、clippy の助言どおり associated function 化した。実装では両 cfg variant のシグネチャを `Option<(ParentLease, vst3::Steinberg::FIDString)>` と `&ComPtr<IPlugView>` に揃え、callsite から `Self::focus_host_window(self.gui_parent, view)` として明示的に渡す。non-Windows 側は引き続き `Err(HostError::Unsupported)` を返し、Windows 側の `SetFocus` と `onFocus(false)` の振る舞い、エラー写像、doc の意味は変えていない。

CLAP 側も `focus_host_window` は private inherent method だが、Windows 側は `self.gui`、`self.gui_parent`、`self.editor_ops()` を必要とする。このため Copy state だけを引き渡す VST3 側と同じ形にはできない。Coordinator に lease 拡張と方針を確認し、両 cfg variant を private associated function `fn focus_host_window(this: &mut Self)` / `fn focus_host_window(_this: &mut Self)` に揃え、callsite を `Self::focus_host_window(self)` にする最小修正が承認された。non-Windows 側は引き続き `Err(HostError::Unsupported)` を返し、Windows 側は GUI-open gate、container lease、`IsWindow`、best-effort `SetFocus`、`apply_focus(false, Editor)` の順序とエラー写像を変えていない。callsite へ cfg を移して helper を削除する案は、cfg による分岐を呼び出し側へ広げるため採用しなかった。

## ローカル検証

実施 OS は Windows x86_64（Orca worker）である。結果はいずれも canonical な mise タスクによる。VST3 側の個別検証は VST3 修正後に実施し、CLAP 側の個別検証と L1 lane 再検証は CLAP 修正後に実施した。

| コマンド | exit code | 結果 |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass（VST3 修正後、CLAP 修正後のいずれも） |
| `mise run check ruviform-plugin-vst3 --features host` | 0 | Pass |
| `mise run lint ruviform-plugin-vst3 --features host` | 0 | Pass |
| `mise run test ruviform-plugin-vst3 --features host` | 0 | Pass（99 lib tests + 2 runner bin tests。L1 E2E integration tests は通常実行では ignored） |
| `mise run check ruviform-plugin-clap --features host` | 0 | Pass |
| `mise run lint ruviform-plugin-clap --features host` | 0 | Pass |
| `mise run test ruviform-plugin-clap --features host` | 0 | Pass（82 lib tests + 2 runner bin tests + 3 doctests。L1 E2E integration tests は通常実行では ignored） |
| `mise run lint-l1-real-plugin` | 0 | Pass（CLAP 修正後に再実施） |
| `mise run test-l1-real-plugin` | 0 | Pass（CLAP 修正後に再実施。CLAP/VST3 host feature build、lib/bin tests、real plugin/fault/stateful L1 E2E を含む） |
| `mise exec -- prek run --all-files` | 0 | Pass（VST3/CLAP 修正後と evidence 更新後に実施） |

## branch CI

branch は `scarecrow173/rf1011-ubuntu-clippy` である。ruviform の `ci.yml` は `workflow_dispatch` only なので、各 push 後に対象 SHA を指定して手動 dispatch した。

### 反復 1: VST3 修正のみ

| 項目 | 値 |
| --- | --- |
| HEAD SHA | `e27eae46729e5f3acc10d96f8d91e00958bc7a6c` |
| run | `45630346317` — https://github.com/scarecrow173/ruviform/actions/runs/45630346317 |
| run の `headSha` | `e27eae46729e5f3acc10d96f8d91e00958bc7a6c` |
| 全体 conclusion | `failure` |
| `verify (ubuntu-latest)` | `failure`（job `139261523025`） |
| `l1-real-verify (ubuntu-latest)` | `failure`（job `139261523483`） |
| 失敗内容 | いずれも `crates/ruviform-plugin-clap/src/host.rs:1260` の `clippy::unused_self`。VST3 側の当初指摘は再発しなかった |

### 反復 2: CLAP 修正を追加

| 項目 | 値 |
| --- | --- |
| コード検証対象 SHA | `156b25c1693701c0a30ac27bcd8459d44dc7c11d` |
| run | `45633428519` — https://github.com/scarecrow173/ruviform/actions/runs/45633428519 |
| run の `headSha` | `156b25c1693701c0a30ac27bcd8459d44dc7c11d` |
| 全体 conclusion | `failure` |
| `verify (ubuntu-latest)` | `success`（job `139272452070`） |
| `l1-real-verify (ubuntu-latest)` | `success`（job `139272452679`） |
| ubuntu lane 以外の失敗 | `verify-macos`（job `139272452223`）と `ui-verify-macos`（job `139272452342`）が `cancelled`。GitHub-hosted macOS runner を複数 attempt で acquire できなかったインフラ失敗で、今回のコード差分に起因する compile/test/lint failure ではない。`ci-summary`（job `139282922868`）はこれらを集約して `failure` になった |

run `45633428519` は SHA `156b25c1693701c0a30ac27bcd8459d44dc7c11d` のコードを検証し、Worker の責務である ubuntu 2 job はいずれも `success` だった。この結果を記録する後続 commit は `evidence/rf1011/verification.md` だけの evidence-only commit であり、Rust コードの振る舞いを変えないため、Coordinator の承認により再 dispatch は行わない。したがって、最終 branch HEAD そのものを指す CI run ではなく、最終コード内容を検証した run として読む必要がある。

## lease 外の追加 clippy 指摘と Coordinator 確認

lease 外の `crates/ruviform-plugin-clap/src/host.rs` に追加指摘があったため、編集前に Coordinator へファイル名・lint・最小修正方針を確認した。Coordinator は同ファイルを rf#1011 の lease に追加し、両 cfg variant を private associated function に揃える方針を承認した。callsite inline + helper 削除の別案は、cfg が呼び出し側へ移って構造が劣るとして不採用になった。

branch CI の結果を evidence に記録すると、その記録 commit 自体が branch HEAD を進める固定点矛盾についても Coordinator に確認した。Coordinator は run `45633428519` をコード内容の最終検証とし、結果を記録する evidence-only commit では再 dispatch を不要と判断した。ただし、この commit が evidence/docs のみであることを diff で検証し、最終 HEAD の CI pass と誤読されないよう SHA 対応を明記する条件が付いた。

## 統合検証の境界

merge 後 main CI は Worker 完了時点では NotRun/Pending である。対象 main SHA の CI 証拠、rf#1011 の tracker close、および rf#253 の bookkeeping は Coordinator が確認する。本記録は branch CI とローカル検証の証拠であり、統合受入の完了を主張しない。

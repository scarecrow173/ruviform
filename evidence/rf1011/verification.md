# rf#1011 ubuntu clippy `unused_self` 修正の検証記録

## 失敗分析

失敗の正本は GitHub Actions run `45545064602`、SHA `ca5f352ba8ba0f401404c67990e147426590761c` である。失敗した job は `verify (ubuntu-latest)`（job `138972590139`、step `Lint workspace (Linux; mise run lint-full)`）と `l1-real-verify (ubuntu-latest)`（job `138972589998`、step `Lint host feature (mise run lint-l1-real-plugin)`）である。

両 job の clippy diagnostic は共通で、対象は `crates/ruviform-plugin-vst3/src/host.rs:2289` の `fn focus_host_window(&self, _view: &ComPtr<IPlugView>)` に対する `clippy::unused_self` である。この関数は `#[cfg(not(target_os = "windows"))]` 側の defined refusal で、Windows ではコンパイルされない。一方、ubuntu の lint lane は `-D warnings` で実行され、workspace lint の pedantic 警告が error になるため、ubuntu job でだけ失敗として現れた。

## 修正方針の判断

`focus_host_window` は public API でも trait 実装でもなく、`Vst3Plugin` の private inherent method である。共有 callsite は `set_gui_focus_target` 内の `GuiFocusTarget::HostWindow` だけであり、Windows 側が必要とする instance state は Copy な `gui_parent` だけである。

このため、`#[allow(clippy::unused_self)]` や `#[expect(clippy::unused_self)]` で lint を隠さず、clippy の助言どおり associated function 化した。実装では両 cfg variant のシグネチャを `Option<(ParentLease, vst3::Steinberg::FIDString)>` と `&ComPtr<IPlugView>` に揃え、callsite から `Self::focus_host_window(self.gui_parent, view)` として明示的に渡す。non-Windows 側は引き続き `Err(HostError::Unsupported)` を返し、Windows 側の `SetFocus` と `onFocus(false)` の振る舞い、エラー写像、doc の意味は変えていない。

## ローカル検証

実施 OS は Windows x86_64（Orca worker）である。結果はいずれも canonical な mise タスクによる。

| コマンド | exit code | 結果 |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-plugin-vst3 --features host` | 0 | Pass |
| `mise run lint ruviform-plugin-vst3 --features host` | 0 | Pass |
| `mise run test ruviform-plugin-vst3 --features host` | 0 | Pass（99 lib tests + 2 runner bin tests。L1 E2E integration tests は通常実行では ignored） |
| `mise run lint-l1-real-plugin` | 0 | Pass |
| `mise run test-l1-real-plugin` | 0 | Pass（CLAP/VST3 host feature build、lib/bin tests、real plugin/fault/stateful L1 E2E を含む） |
| `mise exec -- prek run --all-files` | 0 | Pass |

## branch CI

branch CI は最終 branch HEAD に対して実行する。初回 dispatch 時点の情報は未確定であり、結果確認後にこの節を更新する。

| 項目 | 値 |
| --- | --- |
| branch | `scarecrow173/rf1011-ubuntu-clippy` |
| 最終 HEAD SHA | Pending |
| run ID / URL | Pending |
| run の `headSha` | Pending |
| 全体 conclusion | Pending |
| `verify (ubuntu-latest)` | Pending |
| `l1-real-verify (ubuntu-latest)` | Pending |

## lease 外の追加 clippy 指摘

現時点で lease 外ファイルへの追加 clippy 指摘は確認していない。ubuntu CI が lease 外ファイルを指摘した場合は、編集前にファイル名・lint・最小修正方針を Coordinator へ確認する。

## 統合検証の境界

merge 後 main CI は Worker 完了時点では NotRun/Pending である。対象 main SHA の CI 証拠、rf#1011 の tracker close、および rf#253 の bookkeeping は Coordinator が確認する。本記録は branch CI とローカル検証の証拠であり、統合受入の完了を主張しない。

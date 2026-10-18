# Project Template

言語・フレームワーク非依存の、mise + APM + prek + Git LFS による最小開発基盤です。
このリポジトリにはアプリケーションコード、言語別設定、CI、文書ディレクトリを含めません。

## 新規プロジェクトでの使い方

1. GitHub の **Use this template** から新規リポジトリを作成する。
2. `README.md` と `apm.yml` の名前・説明をプロジェクトに合わせて変更する。
3. mise をインストールし、リポジトリで次を実行する。

```sh
mise install
mise run setup
```

`mise run setup` は APM 依存関係を復元し、`prek install` でコミットフックを、`git lfs install --local` でこのリポジトリの LFS フィルターと Git フックを有効化します（グローバルGit設定は変更しません）。
全ファイルのフックを手動実行する場合は `mise exec -- prek run --all-files` を使用してください。
Git LFS が必要な Git 操作では、mise を有効化したシェルを使うか、`mise exec -- git push` のように実行してください。初回clone時にLFSオブジェクトが未取得なら、セットアップ後に `mise exec -- git lfs pull` を実行してください。

4. 生成された `mise.lock` と `apm.lock.yaml` を確認してコミットする。
5. 使用言語・フレームワークが確定してから `mise.toml` にツール・検証タスクを追加する。

> 初回作成時はツールによる解決がまだ行われていないため、lockfile は含めていません。
> 生成後の通常運用では `mise install --locked` を使用してください。

## 管理対象

| ファイル | 役割 |
| --- | --- |
| `.gitattributes` | OS間の改行・テキスト属性、Git LFS管理対象 |
| `.gitignore` | 秘密情報、ローカルファイル、APM生成物の除外 |
| `AGENTS.md` | エージェント共通作業規約 |
| `apm.yml` | 共通のAPMパッケージとMCP設定 |
| `.pre-commit-config.yaml` | prekによる共通のGitフック |
| `mise.toml` | APM・prek・Git LFSなどの共通ツールと `setup` タスク |
| `apm.lock.yaml` / `mise.lock` | 初期化後に生成・追跡するlockfile |

言語に依存するソース配置、Lint、テスト、ビルドは各プロジェクトが決定します。

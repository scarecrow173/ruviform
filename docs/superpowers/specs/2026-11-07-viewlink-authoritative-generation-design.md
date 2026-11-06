# ViewLinkへのbackend generation採用設計

日付: 2026-11-07  
状態: ユーザー方針選択済み・設計承認待ち  
対象: Ruviform `ui/binding` と Synvelis PR #171

## 意図

Synvelisのview remount後に、frontendがlink-localなgeneration 1を送る一方、backendは同じ`viewId`のattach epoch 2以上を要求し、操作をreceiptなしで破棄するP1を修正する。view generationのauthoritative sourceであるattach応答の値をRuviform `ViewLink`へ渡し、全bindingとsubscriptionのcontextを同期させる。

## 目標

- backend attach応答のgenerationをlinkと既存binding/query/subscriptionの共通epochとして採用する。
- attach前に作ったbindingとattach後に作ったbindingのdispatchが同じauthoritative generationを使う。
- detach、remount、pending attach応答の破棄後に、新しい有効なattach応答で世代を再同期する。
- 応答競合時はSynvelisのattach tokenが古い応答をlinkへ適用しない。
- linkの既存利用者とのsource compatibilityを保ち、authoritative値を持たない利用者は従来のlocal counter動作を保つ。
- Ruviform SDKとSynvelis consumerの正規mise検証および恒久回帰テストで、wire generationとbackend stale guardの両側を検証する。

## 非目標

- backend stale-generation guardを緩める、または古いintentにreceiptを発行する。
- consumerごとにgeneration counterを複製する。
- commandIdのview間一意化、detach窓receiptの扱い、detached中に生成するbindingの契約を本変更へ含める。それらはValidatorで確認された別候補として切り分け、必要なIssueへ追跡する。
- 実OS WebView、GPU、DSP、plugin、multi-window実機の未実施laneを合格扱いする。

## 検討した方法

1. **attach応答generationをViewLinkへ採用（選択）** — `ViewLink.attach(windowId, generation?)`にauthoritative値を渡せる形を追加し、linkがbinding・query・subscriptionへ同じ値を伝える。引数を省略した既存callerは従来のlocal counterを使う。Synvelisはattach tokenを検証した後に`res.generation`を渡す。根本のepoch所有境界を一つにでき、pending応答破棄後も次の成功応答で同期できる。
2. **LinkRegistryでlinkをmount間に保持** — 正常なremountではcounterを保てるが、frontendがattach応答を破棄してlinkがattachを知らないケースではbackendだけgenerationが進むため、今回の不変条件を保証しない。単独修正には採用しない。
3. **backend stale guardを緩和** — 古いepochのdispatchを許し、遅延操作の誤適用リスクを増やす。既存のfail-closed契約を損ねるため採用しない。

## 契約と変更範囲

- Ruviform `ViewLink.attach` は任意のauthoritative generationを受け取れる。値が指定された場合、viewのgeneration getter、CommandBinding/QueryBindingのattach epoch、subscription contextはその値に一致する。省略時は既存のlocal incrementを維持する。
- authoritative値が現在値と異なるattachは新しいepochへの遷移として扱い、旧subscription/listenerが新epochへprojectionを漏らさず、必要なcancel・再購読・binding再attachを行う。同windowの通常idempotent attachで値も同じ場合は既存どおりno-op。
- generation入力は非負の整数に限定し、無効値を受理しない。
- Synvelisはattach応答をtokenで検証してから`link.attach(windowId, res.generation)`を呼び、古い応答がgenerationを巻き戻さない。
- 実装者は既存のpublic API規約、`LinkDetachReason`、QueryBinding lifecycle、外部作成binding registrationへの影響を確認し、上記契約から外れる必要があれば実装前に設計を更新する。

## 検証

- Ruviform `ui/binding`のテストを先にRED化し、authoritative generation採用、attach前/後のbinding、同window再同期時のlistener数・stale projection、local fallbackを確認する。
- Synvelis consumer testでviewId固定のunmount/remount後に、attach応答generation 2が`ViewLink`と実`IpcBindingPort`のwire `ctx.generation`に届くことを確認する。
- pending attachを破棄し、次の有効なattach応答でbackend generationへ再同期するケースを確認する。
- `mise run`の対象SDK/frontend検証とSynvelis gateを実行し、候補SHAに対するFull CIを確認する。
- headless/UI fake hostのgreenはnative実機laneの受入とは分離して記録する。

## 受入条件

- remount後にdispatchされるwire generationがbackend attach応答generationと一致する。
- backend `UiRuntime`は同じgenerationのintentに通常のreceiptを発行し、古いgenerationは引き続きdropする。
- binding lifecycle、projection subscription、receipt/test routingに重複・stale漏出がない。
- RuviformとSynvelis双方の変更が独立レビュー/Validator/Judgeを通過する。Synvelis PR #171はP1修正を含む候補headで再CIし、native NotRunを明記した状態を保つ。

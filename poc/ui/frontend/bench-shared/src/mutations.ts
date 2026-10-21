// DOM mutation 計装（AC5 更新粒度の FW 比較・FW 非依存）。
// MutationObserver でベンチ期間中の MutationRecord を種別集計する。
// 比較の判定材料:
// - 手書き DOM（vanilla）や fine-grained 更新（Solid）は attributes /
//   characterData の部分更新に留まり childList churn（add/remove）を出さない。
// - VDOM 全面再描画は既存ノードを破棄して childList の added/removed を
//   発生させるため、更新粒度の違いが counts に可視化される。

/** DOM mutation の種別集計。`observed_frames`/`meter_channels` は runner が埋める。 */
export interface DomMutationCounts {
  /** attribute 変化レコード数（メータの style.transform 書き込み等）。 */
  attributes: number;
  /** childList 変化レコード数（ノード追加/削除が起きた回数）。 */
  child_list: number;
  /** characterData 変化レコード数（HUD テキスト書き換え等）。 */
  character_data: number;
  /** childList 変化で追加されたノード総数。 */
  added_nodes: number;
  /** childList 変化で削除されたノード総数。 */
  removed_nodes: number;
  /** attribute 名ごとの変化回数（"style" 等）。 */
  attributes_by_name: Record<string, number>;
  /** 観察窓内で実行された rAF フレーム数（warmup 込み）。runner が設定。 */
  observed_frames?: number;
  /** メータチャンネル数（mutations-per-meter-update 正規化用）。runner が設定。 */
  meter_channels?: number;
}

function freshCounts(): DomMutationCounts {
  return {
    attributes: 0,
    child_list: 0,
    character_data: 0,
    added_nodes: 0,
    removed_nodes: 0,
    attributes_by_name: {},
  };
}

/**
 * `observe(root, {subtree:true, all mutation types})` の薄いラッパ。
 * `start` で計数リセット＋観察開始、`stop` が `takeRecords()` で滞留分を
 * 回収してから `disconnect()` し、集計済み counts を返す。
 */
export class MutationMeter {
  private obs: MutationObserver | null = null;
  private counts: DomMutationCounts = freshCounts();

  /** 観察開始（既に観察中なら一度止めてリセット）。 */
  start(root: Element): void {
    this.stop();
    this.counts = freshCounts();
    this.obs = new MutationObserver((records) => this.consume(records));
    this.obs.observe(root, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
    });
  }

  private consume(records: MutationRecord[]): void {
    for (const r of records) {
      switch (r.type) {
        case "attributes": {
          this.counts.attributes++;
          const name = r.attributeName ?? "(unknown)";
          this.counts.attributes_by_name[name] =
            (this.counts.attributes_by_name[name] ?? 0) + 1;
          break;
        }
        case "childList": {
          this.counts.child_list++;
          this.counts.added_nodes += r.addedNodes.length;
          this.counts.removed_nodes += r.removedNodes.length;
          break;
        }
        case "characterData": {
          this.counts.character_data++;
          break;
        }
      }
    }
  }

  /** 観察終了。滞留レコードを回収してから切断し、集計を返す。未 start なら null。 */
  stop(): DomMutationCounts | null {
    if (!this.obs) return null;
    this.consume(this.obs.takeRecords());
    this.obs.disconnect();
    this.obs = null;
    return this.counts;
  }
}

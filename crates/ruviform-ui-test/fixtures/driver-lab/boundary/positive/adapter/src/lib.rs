//! Positive fixture: allowlisted adapter。gpui への依存は allowlist
//! が許可するが、公開 signature の漏出検査は adapter にも適用される
//! — private 利用のみ許され、backend 型の public surface 露出は
//! finding になる。

use common::WindowSpec;

#[cfg(feature = "gpui")]
use gpui::Entity as _GpuiEntity;

/// private root view。consumer が名付けられないため backend trait
/// impl（`impl gpui::Render for Root`）は public API でない —
/// adapter が backend 契約を private に実装する典型形。
#[cfg(feature = "gpui")]
struct Root;

#[cfg(feature = "gpui")]
impl gpui::Render for Root {}

/// adapter の公開 API — backend 型を露出しない。
pub fn open(spec: WindowSpec) -> bool {
    spec.width > 0
}

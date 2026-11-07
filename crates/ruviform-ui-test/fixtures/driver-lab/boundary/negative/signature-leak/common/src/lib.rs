//! Negative: 公開 signature に forbidden path root。

/// pub fn の戻り型が `gpui::Entity`。
pub fn make() -> gpui::Entity { todo!() }

/// pub struct の pub field が `tao::Size`。
pub struct S {
    /// leaked field。
    pub size: tao::Size,
}

mod internal {
    /// private mod 内の pub trait — re-export で公開。
    pub trait Leak {
        /// leaked assoc type。
        type Out: wry::Into;
    }
}

/// re-export で private mod の pub item が公開される。
pub use internal::Leak;

/// public type への impl — method signature が漏出。
pub struct Pub;
impl Pub {
    /// leaked method arg。
    pub fn take(&self, _e: gpui::Entity) {}
}

/// private type への impl — 検出対象外（public API でない）。
struct Priv;
impl Priv {
    /// private type 上なので finding にならない。
    pub fn take(&self, _e: gpui::Entity) {}
}

//! Positive fixture: driver-neutral common crate。toolkit 参照なし。

/// 契約型のみを使う public API。
pub struct WindowSpec {
    /// logical width。
    pub width: u32,
}

/// private mod 内の pub item は public path chain を持たないが、
/// guard は保守的に全 mod を走査する — ここは clean であるべき。
mod internal {
    /// crate 内部用の helper（forbidden root 参照なし）。
    pub fn helper(v: usize) -> usize {
        v + 1
    }
}

pub use internal::helper as internal_helper;

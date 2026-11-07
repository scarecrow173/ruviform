//! Negative fixture: protected crate が forbidden dep を直接宣言。

/// clean signature — 違反は dep 宣言側。
pub fn spec() -> u32 {
    0
}

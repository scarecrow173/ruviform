//! Minimal member crate for the ruviform workspace (仮置き).
//!
//! The crate name and public API are placeholders to keep the workspace
//! non-empty until the detailed design in issue #1 re-evaluates the final
//! crate split. Only device-independent pure functions belong here.

/// Returns the crate version string.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_core::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }
}

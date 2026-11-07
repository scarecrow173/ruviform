//! Positive fixture: allowlisted adapter。gpui を private に利用して
//! よい（guard は allowlist package の signature を走査しない）。

use common::WindowSpec;

#[cfg(feature = "gpui")]
use gpui::Entity as _GpuiEntity;

/// adapter の公開 API — backend 型を露出しない。
pub fn open(spec: WindowSpec) -> bool {
    spec.width > 0
}

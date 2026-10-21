//! Fixed-size host string conversion helpers.
//!
//! VST3 metadata travels in fixed-size `char8` (`PFactoryInfo`,
//! `PClassInfo`, `PClassInfo2`, plus the `category`/`subCategories`
//! fields of `PClassInfoW`) and UTF-16 `TChar` (`BusInfo`,
//! `ParameterInfo`, `String128`, the remaining `PClassInfoW` fields)
//! arrays. These helpers write a Rust `&str` into such a buffer with a
//! guaranteed NUL terminator.
//!
//! [`copy_cstring`] allocates once for the `CString` round trip — its
//! call sites (factory info, class info) are all outside the
//! processing path.

use std::ffi::{CString, c_char};

use vst3::Steinberg::Vst::TChar;

/// Copies `src` into `dst` as a NUL-terminated `char8` string,
/// truncating to the buffer capacity.
///
/// **RT contract:** non-RT only — the `CString` conversion allocates;
/// call sites are factory/class-info callbacks only.
pub(crate) fn copy_cstring(src: &str, dst: &mut [c_char]) {
    let c_string = CString::new(src).unwrap_or_default();
    let bytes = c_string.as_bytes_with_nul();
    let len = bytes.len().min(dst.len());
    for (src, dst) in bytes.iter().zip(dst.iter_mut()).take(len) {
        // `u8` → `c_char` reinterprets the byte: values ≥ 0x80 wrap on
        // signed-`char` platforms, which is the intended raw-byte copy
        // (`c_char` is `i8` on Windows, `u8` elsewhere — `as` is the
        // only form valid on both).
        #[allow(clippy::cast_possible_wrap)]
        {
            *dst = *src as c_char;
        }
    }
    if len < bytes.len() {
        if let Some(last) = dst.last_mut() {
            *last = 0;
        }
    }
}

/// Copies `src` into `dst` as a NUL-terminated UTF-16 `TChar` string,
/// truncating to the buffer capacity.
///
/// **RT contract:** RT-suitable — bounded `O(dst.len())` scan,
/// allocation-free.
pub(crate) fn copy_wstring(src: &str, dst: &mut [TChar]) {
    let mut len = 0;
    for (src, dst) in src.encode_utf16().zip(dst.iter_mut()) {
        *dst = src;
        len += 1;
    }
    if len < dst.len() {
        dst[len] = 0;
    } else if let Some(last) = dst.last_mut() {
        *last = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `&str` as `c_char` bytes for assertions — scoped cast
    /// allowances keep the platform-ambiguous `c_char` signedness out
    /// of the production code.
    #[allow(clippy::cast_possible_wrap)]
    fn c_chars(s: &str) -> Vec<c_char> {
        s.bytes().map(|b| b as c_char).collect()
    }

    #[test]
    fn copy_cstring_terminates_and_truncates() {
        let mut buf = [0; 8];
        copy_cstring("hi", &mut buf);
        let expected = c_chars("hi\0");
        assert_eq!(&buf[..3], &expected[..]);

        let mut small = [1; 4];
        copy_cstring("abcdef", &mut small);
        assert_eq!(small[3], 0);
        let expected = c_chars("abc");
        assert_eq!(&small[..3], &expected[..]);
    }

    #[test]
    fn copy_wstring_terminates_and_truncates() {
        let mut buf = [0; 8];
        copy_wstring("Gain", &mut buf);
        assert_eq!(
            &buf[..5],
            &[
                u16::from(b'G'),
                u16::from(b'a'),
                u16::from(b'i'),
                u16::from(b'n'),
                0,
            ]
        );

        let mut small = [1; 3];
        copy_wstring("abcdef", &mut small);
        assert_eq!(small[2], 0);
    }
}

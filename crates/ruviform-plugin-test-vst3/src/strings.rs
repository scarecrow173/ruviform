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
    if len < bytes.len()
        && let Some(last) = dst.last_mut()
    {
        *last = 0;
    }
}

/// Compares a NUL-terminated `char8` string (`FIDString` — view-type
/// names like `"editor"`, platform-type names like `"HWND"`) against
/// `expected`. `null` compares false; otherwise the bytes up to the
/// terminator must equal `expected` exactly.
///
/// The scan is bounded at 1024 bytes — a hostile non-terminated
/// input cannot run away past the bound (the design's stated
/// defense; `CStr::from_ptr` would scan unboundedly).
///
/// # Safety
///
/// `string` must be null or point to a valid NUL-terminated `c_char`
/// sequence — the VST3 string contract (the same trust level
/// `createInstance` places on `cid`/`iid`). The bound limits how far
/// past a contract violation the read may run, it does not make a
/// non-terminated pointer safe.
#[expect(unsafe_code)]
pub(crate) unsafe fn cstring_eq(string: *const c_char, expected: &str) -> bool {
    /// Upper bound on the scanned length — generous for every VST3
    /// name literal while capping runaway reads on hostile input.
    const FIDSTRING_MAX_LEN: usize = 1024;

    if string.is_null() {
        return false;
    }
    let expected = expected.as_bytes();
    let mut i = 0usize;
    loop {
        if i >= FIDSTRING_MAX_LEN {
            return false;
        }
        // SAFETY: `i` is bounded; each byte read is within the caller's
        // string-or-bounded-region contract (see fn docs). The `u8`
        // pointer view keeps the comparison in the byte domain —
        // `c_char` signedness is platform-dependent.
        let byte = unsafe { *string.add(i).cast::<u8>() };
        if byte == 0 {
            // Every preceding byte already matched `expected` —
            // equality reduces to equal length.
            return i == expected.len();
        }
        if i >= expected.len() || byte != expected[i] {
            return false;
        }
        i += 1;
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

    #[expect(unsafe_code)]
    #[test]
    fn cstring_eq_compares_content_exactly() {
        let hwnd = CString::new("HWND").unwrap();
        let editor = CString::new("editor").unwrap();
        let longer = CString::new("HWNDx").unwrap();
        let shorter = CString::new("HWD").unwrap();
        // SAFETY: all pointers reference live NUL-terminated CStrings.
        unsafe {
            assert!(cstring_eq(hwnd.as_ptr(), "HWND"));
            assert!(cstring_eq(editor.as_ptr(), "editor"));
            assert!(!cstring_eq(hwnd.as_ptr(), "hwnd"));
            assert!(!cstring_eq(hwnd.as_ptr(), "editor"));
            assert!(!cstring_eq(longer.as_ptr(), "HWND"));
            assert!(!cstring_eq(shorter.as_ptr(), "HWND"));
            assert!(!cstring_eq(std::ptr::null(), "HWND"));
        }
    }

    /// A non-terminated buffer compares false at the 1024-byte bound —
    /// the scan cannot run away (the buffer is valid for 2048 bytes,
    /// so reading up to the bound stays in bounds).
    #[expect(unsafe_code)]
    #[test]
    fn cstring_eq_bounds_the_scan() {
        let mut buf = [b'A'; 2048];
        // SAFETY: `buf` is a live 2048-byte array; the scan reads at
        // most 1024 bytes before giving up.
        unsafe {
            assert!(!cstring_eq(buf.as_ptr().cast::<c_char>(), "HWND"));
            assert!(!cstring_eq(
                buf.as_ptr().cast::<c_char>(),
                &"A".repeat(2000)
            ));
        }
        buf[1023] = 0;
        // SAFETY: same live buffer; now NUL-terminated at the bound.
        unsafe {
            assert!(cstring_eq(buf.as_ptr().cast::<c_char>(), &"A".repeat(1023)));
            assert!(!cstring_eq(
                buf.as_ptr().cast::<c_char>(),
                &"A".repeat(1024)
            ));
        }
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

//! Real OS backend for the shared vfs layer — `WindowsVfsOps`.
//!
//! Uses the NT system calls instead of the Win32 path APIs so every
//! operation is handle-relative and no-follow:
//!
//! - `NtOpenFile` on an absolute `\\??\\` path opens the trusted
//!   directory handle (the caller's trust boundary).
//! - `NtCreateFile` / `NtOpenFile` with
//!   `OBJECT_ATTRIBUTES.RootDirectory = <open dir handle>` and a
//!   single-component `ObjectName` traverse one level at a time —
//!   path strings are never re-resolved below the trusted handle.
//! - `OBJ_DONT_REPARSE | FILE_OPEN_REPARSE_POINT` means a symlink or
//!   junction is never followed during traversal: opens either fail
//!   or address the reparse point itself (then the post-open
//!   attribute check refuses it).
//! - `FILE_CREATE` disposition is atomic create-new: it refuses any
//!   existing name (including a planted reparse point) — no
//!   exists()-then-create window.
//! - `NtDeleteFile` relative to a verified dir handle deletes the
//!   named object itself (`OBJ_DONT_REPARSE`), including a swapped-in
//!   link object.
//! - `probe`/`read_link` open the leaf with
//!   `FILE_READ_ATTRIBUTES|SYNCHRONIZE` + `FILE_OPEN_REPARSE_POINT`
//!   (no `OBJ_DONT_REPARSE` — the reparse object itself must be
//!   visible) and classify via `FileBasicInfo`/`FileIdInfo`/
//!   `FileStandardInfo`/`FileAttributeTagInfo` + the reparse buffer.
//! - Listings use `NtQueryDirectoryFile` with `RestartScan` so every
//!   call holds independent enumeration state.
//! - Identity is `GetFileInformationByHandleEx(FileIdInfo)` — it
//!   names the object the handle addresses.
//!
//! `OBJ_DONT_REPARSE` requires Windows 10 RS1 (build 14393)+; the
//! [`supports_dont_reparse`] gate installs the Unsupported backend on
//! older builds rather than silently ignoring reparse points.
//!
//! **RT contract:** non-RT only.

#![cfg(windows)]

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::path::{Path, PathBuf};

use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_BOTH_DIR_INFORMATION, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_INFORMATION_CLASS,
    FILE_NON_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
    FileBothDirectoryInformation, NtCreateFile, NtDeleteFile, NtOpenFile, NtQueryDirectoryFile,
    REPARSE_DATA_BUFFER, RtlNtStatusToDosErrorNoTeb,
};
use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
use windows_sys::Win32::Foundation::{
    HANDLE, NTSTATUS, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, UNICODE_STRING,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ACCESS_RIGHTS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_BASIC_INFO, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FILE_ID_128, FILE_ID_INFO, FILE_INFO_BY_HANDLE_CLASS, FILE_LIST_DIRECTORY,
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FILE_STANDARD_INFO, FILE_TRAVERSE, FileAttributeTagInfo, FileBasicInfo, FileIdInfo,
    FileStandardInfo, GetFileInformationByHandleEx, GetFinalPathNameByHandleW,
    GetVolumeInformationByHandleW, SYNCHRONIZE,
};
use windows_sys::Win32::System::IO::{DeviceIoControl, IO_STATUS_BLOCK};
use windows_sys::Win32::System::Ioctl::FSCTL_GET_REPARSE_POINT;
use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows_sys::Win32::System::SystemServices::{
    IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK,
};

use super::{NodeIdentity, NodeTypeTag, VfsDir, VfsEntry, VfsKind, VfsOps, VfsProbe};
use crate::rooted::IdentityStrength;

const SHARE_ALL: FILE_SHARE_MODE = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
const DIR_ACCESS: FILE_ACCESS_RIGHTS =
    FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE;
const ATTRS_ACCESS: FILE_ACCESS_RIGHTS = FILE_READ_ATTRIBUTES | SYNCHRONIZE;
const READ_ACCESS: FILE_ACCESS_RIGHTS = FILE_GENERIC_READ;
const FILE_ACCESS: FILE_ACCESS_RIGHTS = FILE_GENERIC_WRITE | FILE_READ_ATTRIBUTES;
/// Attributes for traversal opens (dirs, create, delete): refuse to
/// follow a reparse point at the named component.
const OBJ_ATTRS: u32 = OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE;
/// Attributes for leaf-object opens (`probe`/`read_link`/
/// `open_regular_leaf`): open the reparse object itself so its
/// tag/target can be read — the resolve layer decides whether to
/// follow.
const OBJ_ATTRS_LEAF: u32 = OBJ_CASE_INSENSITIVE;

/// Whether this Windows build honors `OBJ_DONT_REPARSE` (RS1 /
/// build 14393+). Older builds silently ignore it, so the no-follow
/// guarantee cannot be claimed there.
#[expect(unsafe_code, reason = "RtlGetVersion is ntdll FFI; see SAFETY comment")]
pub(crate) fn supports_dont_reparse() -> bool {
    const RS1_BUILD: u32 = 14393;
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: u32::try_from(size_of::<OSVERSIONINFOW>()).unwrap_or(u32::MAX),
        ..OSVERSIONINFOW::default()
    };
    // SAFETY: `info` is a valid version buffer for the declared size.
    let status = unsafe { RtlGetVersion(&raw mut info) };
    if status < 0 {
        return false;
    }
    (info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber) >= (10, 0, RS1_BUILD)
}

/// The Windows [`VfsOps`] implementation.
#[derive(Debug)]
pub(crate) struct WindowsVfsOps;

impl VfsOps for WindowsVfsOps {
    fn open_dir_path(&self, path: &Path) -> io::Result<Box<dyn VfsDir>> {
        let handle = open_path_dir(path)?;
        Ok(Box::new(WinVfsDir {
            handle,
            path: path.to_path_buf(),
        }))
    }

    fn stat_file(&self, file: &File) -> io::Result<VfsProbe> {
        probe_handle(file.as_raw_handle())
    }
}

/// An open directory (HANDLE + diagnostic path).
#[derive(Debug)]
struct WinVfsDir {
    handle: OwnedHandle,
    path: PathBuf,
}

impl VfsDir for WinVfsDir {
    fn path(&self) -> &Path {
        &self.path
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        Ok(probe_handle(self.handle.as_raw_handle())?.identity)
    }

    fn stat_self(&self) -> io::Result<VfsProbe> {
        probe_handle(self.handle.as_raw_handle())
    }

    fn canonicalize_handle(&self) -> io::Result<PathBuf> {
        final_path_name(self.handle.as_raw_handle())
    }

    fn probe(&self, name: &OsStr) -> io::Result<VfsProbe> {
        // `OBJ_DONT_REPARSE` is deliberately absent: the leaf reparse
        // object itself must open so its attributes/tag are visible.
        let handle = open_any_leaf(self.handle.as_raw_handle(), name)?;
        probe_handle(handle.as_raw_handle())
    }

    fn open_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        let handle = open_dir_rel(self.handle.as_raw_handle(), name)?;
        // Post-open attribute check: a reparse point at this name
        // must never be treated as a real directory.
        let basic = basic_info(handle.as_raw_handle())?;
        if basic.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "ruviform-io-file: component is a reparse point, not a directory",
            ));
        }
        Ok(Box::new(WinVfsDir {
            handle,
            path: self.path.join(name),
        }))
    }

    #[expect(
        unsafe_code,
        reason = "raw HANDLE → File ownership transfer; see SAFETY comment"
    )]
    fn open_regular_leaf(&self, name: &OsStr) -> io::Result<(File, VfsProbe)> {
        let handle = open_leaf_rel(self.handle.as_raw_handle(), name)?;
        let probe = probe_handle(handle)?;
        // SAFETY: `handle` is a freshly opened file HANDLE opened
        // for synchronous read; `File` takes unique ownership.
        let file = unsafe { File::from_raw_handle(handle) };
        Ok((file, probe))
    }

    fn read_link(&self, name: &OsStr) -> io::Result<OsString> {
        let handle = open_any_leaf(self.handle.as_raw_handle(), name)?;
        reparse_target(handle.as_raw_handle())
    }

    fn list(&self) -> io::Result<Vec<VfsEntry>> {
        list_dir(self)
    }

    fn create_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        let handle = create_dir_rel(self.handle.as_raw_handle(), name)?;
        Ok(Box::new(WinVfsDir {
            handle,
            path: self.path.join(name),
        }))
    }

    #[expect(
        unsafe_code,
        reason = "raw HANDLE → File ownership transfer; see SAFETY comment"
    )]
    fn create_file(&self, name: &OsStr) -> io::Result<File> {
        let handle = create_file_rel(self.handle.as_raw_handle(), name)?;
        // SAFETY: `handle` is a freshly opened file HANDLE opened for
        // synchronous write access; `File` takes unique ownership.
        Ok(unsafe { File::from_raw_handle(handle) })
    }

    fn remove_file(&self, name: &OsStr) -> io::Result<()> {
        delete_rel(self.handle.as_raw_handle(), name)
    }

    fn remove_dir(&self, name: &OsStr) -> io::Result<()> {
        delete_rel(self.handle.as_raw_handle(), name)
    }
}

// ------------------------------------------------------------------
// NT syscall helpers. `unsafe` is justified per-call: every
// OBJECT_ATTRIBUTES is scoped to the call, UNICODE_STRING buffers
// outlive it, and returned HANDLEs are immediately wrapped in
// OwnedHandle.
// ------------------------------------------------------------------

/// Converts a Win32 absolute path to its NT `\\??\\` form.
fn nt_path(path: &Path) -> Vec<u16> {
    let text = path.as_os_str();
    let wide: Vec<u16> = text.encode_wide().collect();
    let s = String::from_utf16_lossy(&wide);
    let nt = if let Some(rest) = s.strip_prefix(r"\\?\") {
        format!(r"\??\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\.\") {
        format!(r"\??\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\") {
        format!(r"\??\UNC\{rest}")
    } else {
        format!(r"\??\{s}")
    };
    nt.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_component(name: &OsStr) -> Vec<u16> {
    name.encode_wide().chain(std::iter::once(0)).collect()
}

fn unicode_string(buf: &[u16]) -> UNICODE_STRING {
    UNICODE_STRING {
        Length: u16::try_from((buf.len() - 1) * 2).unwrap_or(u16::MAX),
        MaximumLength: u16::try_from(buf.len() * 2).unwrap_or(u16::MAX),
        Buffer: buf.as_ptr().cast_mut(),
    }
}

#[expect(unsafe_code, reason = "RtlNtStatusToDosErrorNoTeb is ntdll FFI")]
fn nt_error(status: NTSTATUS) -> io::Error {
    // SAFETY: pure status→code mapping function in ntdll.
    let code = unsafe { RtlNtStatusToDosErrorNoTeb(status) };
    io::Error::from_raw_os_error(i32::try_from(code).unwrap_or(i32::MAX))
}

fn object_attributes(root: RawHandle, name: &UNICODE_STRING, attributes: u32) -> OBJECT_ATTRIBUTES {
    OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).unwrap_or(u32::MAX),
        RootDirectory: root.cast(),
        ObjectName: &raw const *name,
        Attributes: attributes,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    }
}

/// `NtOpenFile` on an absolute NT path (trusted path only — the
/// caller's trust boundary). `FILE_DIRECTORY_FILE` refuses
/// non-directories.
#[expect(
    unsafe_code,
    reason = "NtOpenFile is NT syscall FFI; see SAFETY comments"
)]
fn open_path_dir(path: &Path) -> io::Result<OwnedHandle> {
    let name_buf = nt_path(path);
    let name = unicode_string(&name_buf);
    let attrs = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).unwrap_or(u32::MAX),
        RootDirectory: std::ptr::null_mut(),
        ObjectName: &raw const name,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    };
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut sb = IO_STATUS_BLOCK::default();
    // SAFETY: `attrs`/`name`/`sb` are valid for the duration of the
    // call; `name_buf` outlives them.
    let status = unsafe {
        NtOpenFile(
            &raw mut handle,
            DIR_ACCESS,
            &raw const attrs,
            &raw mut sb,
            SHARE_ALL,
            FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
        )
    };
    if status < 0 {
        return Err(nt_error(status));
    }
    // SAFETY: `handle` is a freshly opened HANDLE owned by us.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// `NtOpenFile` relative to `dir` opening an existing child
/// *directory* — `OBJ_DONT_REPARSE | FILE_OPEN_REPARSE_POINT` refuses
/// to follow links (a junction/symlink-dir opens the reparse object,
/// which the caller's post-open attribute check then rejects).
fn open_dir_rel(dir: RawHandle, name: &OsStr) -> io::Result<OwnedHandle> {
    open_rel_impl(
        dir,
        name,
        DIR_ACCESS,
        SHARE_ALL,
        FILE_DIRECTORY_FILE,
        OBJ_ATTRS,
    )
}

/// `NtOpenFile` relative to `dir` opening the leaf object itself —
/// attributes-only access, `FILE_OPEN_REPARSE_POINT`, **without**
/// `OBJ_DONT_REPARSE` so a link opens as a link. Tries as file, then
/// as directory (a junction's target kind is unknown until opened).
fn open_any_leaf(dir: RawHandle, name: &OsStr) -> io::Result<OwnedHandle> {
    match open_rel_impl(
        dir,
        name,
        ATTRS_ACCESS,
        SHARE_ALL,
        FILE_NON_DIRECTORY_FILE,
        OBJ_ATTRS_LEAF,
    ) {
        Ok(h) => Ok(h),
        Err(_) => open_rel_impl(
            dir,
            name,
            ATTRS_ACCESS,
            SHARE_ALL,
            FILE_DIRECTORY_FILE,
            OBJ_ATTRS_LEAF,
        ),
    }
}

/// `NtOpenFile` relative to `dir` for the verified read leaf open —
/// `FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT |
/// FILE_SYNCHRONOUS_IO_NONALERT`, `FILE_GENERIC_READ`, `SHARE_ALL`.
/// Synchronous non-alert open produces no async pend; reparse
/// objects open as themselves (the post-open probe classifies).
fn open_leaf_rel(dir: RawHandle, name: &OsStr) -> io::Result<RawHandle> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = object_attributes(dir.cast(), &name, OBJ_ATTRS_LEAF);
    open_nt(
        &attrs,
        READ_ACCESS,
        SHARE_ALL,
        FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
    )
}

#[expect(
    unsafe_code,
    reason = "NtOpenFile is NT syscall FFI; see SAFETY comments"
)]
fn open_rel_impl(
    dir: RawHandle,
    name: &OsStr,
    access: FILE_ACCESS_RIGHTS,
    share: FILE_SHARE_MODE,
    options_kind: u32,
    attributes: u32,
) -> io::Result<OwnedHandle> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = object_attributes(dir, &name, attributes);
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut sb = IO_STATUS_BLOCK::default();
    // SAFETY: `dir` is a valid open directory HANDLE; all out-params
    // valid for the call; `name_buf` outlives them.
    let status = unsafe {
        NtOpenFile(
            &raw mut handle,
            access,
            &raw const attrs,
            &raw mut sb,
            share,
            options_kind | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )
    };
    if status < 0 {
        return Err(nt_error(status));
    }
    // SAFETY: `handle` is a freshly opened HANDLE owned by us.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

#[expect(
    unsafe_code,
    reason = "NtOpenFile is NT syscall FFI; see SAFETY comments"
)]
fn open_nt(
    attrs: &OBJECT_ATTRIBUTES,
    access: FILE_ACCESS_RIGHTS,
    share: FILE_SHARE_MODE,
    options: u32,
) -> io::Result<RawHandle> {
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut sb = IO_STATUS_BLOCK::default();
    // SAFETY: `attrs`/`sb` are valid for the call; the ObjectName
    // buffer outlives them at the caller.
    let status = unsafe { NtOpenFile(&raw mut handle, access, attrs, &raw mut sb, share, options) };
    if status < 0 {
        return Err(nt_error(status));
    }
    Ok(handle)
}

/// `NtCreateFile(FILE_CREATE)` relative to `dir`: exclusive create of
/// a child directory — refuses any existing name including reparse
/// points.
#[expect(
    unsafe_code,
    reason = "NtCreateFile is NT syscall FFI; see SAFETY comments"
)]
fn create_dir_rel(dir: RawHandle, name: &OsStr) -> io::Result<OwnedHandle> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = object_attributes(dir, &name, OBJ_ATTRS);
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut sb = IO_STATUS_BLOCK::default();
    let alloc: i64 = 0;
    // SAFETY: `dir` is a valid open directory HANDLE; all out-params
    // are valid for the call; `name_buf` outlives them.
    let status = unsafe {
        NtCreateFile(
            &raw mut handle,
            DIR_ACCESS,
            &raw const attrs,
            &raw mut sb,
            &raw const alloc,
            FILE_ATTRIBUTE_DIRECTORY,
            SHARE_ALL,
            FILE_CREATE,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            std::ptr::null(),
            0,
        )
    };
    if status < 0 {
        return Err(nt_error(status));
    }
    // SAFETY: `handle` is a freshly opened HANDLE owned by us.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// `NtCreateFile(FILE_CREATE)` relative to `dir`: exclusive create of
/// a regular file — refuses any existing name (including reparse
/// points); `ShareAccess=0` keeps the new file exclusively ours while
/// open.
#[expect(
    unsafe_code,
    reason = "NtCreateFile is NT syscall FFI; see SAFETY comments"
)]
fn create_file_rel(dir: RawHandle, name: &OsStr) -> io::Result<RawHandle> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = object_attributes(dir, &name, OBJ_ATTRS);
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut sb = IO_STATUS_BLOCK::default();
    let alloc: i64 = 0;
    // SAFETY: `dir` is a valid open directory HANDLE; all out-params
    // valid for the call; `name_buf` outlives them.
    let status = unsafe {
        NtCreateFile(
            &raw mut handle,
            FILE_ACCESS,
            &raw const attrs,
            &raw mut sb,
            &raw const alloc,
            FILE_ATTRIBUTE_NORMAL,
            0,
            FILE_CREATE,
            FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            std::ptr::null(),
            0,
        )
    };
    if status < 0 {
        return Err(nt_error(status));
    }
    Ok(handle)
}

/// `NtDeleteFile` relative to `dir` — deletes the named object itself
/// (`OBJ_DONT_REPARSE` → a link is unlinked, never followed).
#[expect(
    unsafe_code,
    reason = "NtDeleteFile is NT syscall FFI; see SAFETY comments"
)]
fn delete_rel(dir: RawHandle, name: &OsStr) -> io::Result<()> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = object_attributes(dir, &name, OBJ_ATTRS);
    // SAFETY: `dir` is a valid open directory HANDLE; `attrs`/`name`
    // valid for the call; `name_buf` outlives them.
    let status = unsafe { NtDeleteFile(&raw const attrs) };
    if status < 0 {
        return Err(nt_error(status));
    }
    Ok(())
}

// ------------------------------------------------------------------
// Handle queries — identity, kind, size, times, reparse tag.
// ------------------------------------------------------------------

/// One `GetFileInformationByHandleEx` query, typed on `T`.
#[expect(
    unsafe_code,
    reason = "GetFileInformationByHandleEx is Win32 FFI; see SAFETY comment"
)]
fn query_info<T: Default>(handle: RawHandle, class: FILE_INFO_BY_HANDLE_CLASS) -> io::Result<T> {
    let mut info = T::default();
    // SAFETY: `handle` is a valid open HANDLE; `info` is valid for
    // `size_of::<T>()` bytes of write.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle.cast(),
            class,
            (&raw mut info).cast(),
            u32::try_from(size_of::<T>()).unwrap_or(u32::MAX),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}

fn basic_info(handle: RawHandle) -> io::Result<FILE_BASIC_INFO> {
    query_info::<FILE_BASIC_INFO>(handle, FileBasicInfo)
}

/// FILETIME (100-ns ticks since 1601) → `SystemTime`. Far-future
/// timestamps are clamped honestly to `None` — `ticks * 100` would
/// overflow `u64` for dates beyond ~year 30828, so the conversion
/// splits into seconds/nanos before scaling and `checked_add`
/// bounds the result to `SystemTime`'s representable range.
fn filetime_to_system(t: i64) -> Option<std::time::SystemTime> {
    const EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000;
    let ticks = u64::try_from(t).ok()?.checked_sub(EPOCH_DIFF_100NS)?;
    let secs = ticks / 10_000_000;
    let nanos = u32::try_from(ticks % 10_000_000).ok()? * 100;
    std::time::UNIX_EPOCH.checked_add(std::time::Duration::new(secs, nanos))
}

/// Whether the volume hosting `handle` reports strong file identity
/// (NTFS/ReFS — the 128-bit `FileId` embeds the MFT sequence number,
/// discriminating delete+recreate). FAT/exFAT/unknown/foreign FS
/// names land on `Weak`, honestly.
#[expect(
    unsafe_code,
    reason = "GetVolumeInformationByHandleW is Win32 FFI; see SAFETY comment"
)]
fn volume_strength(handle: RawHandle) -> IdentityStrength {
    let mut name_buf = [0u16; 64];
    // SAFETY: `handle` is a valid open HANDLE; all buffers are
    // valid for the declared sizes and outlive the call.
    let ok = unsafe {
        GetVolumeInformationByHandleW(
            handle.cast(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            name_buf.as_mut_ptr(),
            u32::try_from(name_buf.len()).unwrap_or(u32::MAX),
        )
    };
    if ok == 0 {
        return IdentityStrength::Weak;
    }
    let end = name_buf
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(name_buf.len());
    let name = String::from_utf16_lossy(&name_buf[..end]);
    if name.eq_ignore_ascii_case("NTFS") || name.eq_ignore_ascii_case("ReFS") {
        IdentityStrength::Strong
    } else {
        IdentityStrength::Weak
    }
}

/// Reparse-tag → kind: name-surrogate tags (`SYMLINK`,
/// `MOUNT_POINT`/junction) are `Symlink` (target-resolution
/// candidates); every other tag is `Other` — never followed.
fn reparse_kind(tag: u32) -> VfsKind {
    match tag {
        IO_REPARSE_TAG_SYMLINK | IO_REPARSE_TAG_MOUNT_POINT => VfsKind::Symlink,
        _ => VfsKind::Other,
    }
}

/// Full probe of the object an open handle addresses:
/// `FileBasicInfo` + `FileIdInfo` + `FileStandardInfo` +
/// `FileAttributeTagInfo` (reparse classification) + volume FS name
/// (identity strength).
fn probe_handle(handle: RawHandle) -> io::Result<VfsProbe> {
    let basic = basic_info(handle)?;
    let standard = query_info::<FILE_STANDARD_INFO>(handle, FileStandardInfo)?;
    let attrs = basic.FileAttributes;
    // A reparse point is classified before the directory bit — a
    // junction is DIRECTORY|REPARSE_POINT and must report `Symlink`,
    // never `Dir` (treating it as a real dir would follow it).
    let kind = if attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        let tag = query_info::<FILE_ATTRIBUTE_TAG_INFO>(handle, FileAttributeTagInfo)
            .map_or(0, |t| t.ReparseTag);
        reparse_kind(tag)
    } else if attrs & FILE_ATTRIBUTE_DIRECTORY != 0 {
        VfsKind::Dir
    } else {
        VfsKind::File
    };
    let identity = identity_parts(handle, &basic, kind)?;
    Ok(VfsProbe {
        identity,
        kind,
        byte_len: (kind == VfsKind::File)
            .then(|| u64::try_from(standard.EndOfFile).unwrap_or(u64::MAX)),
        modified: filetime_to_system(basic.LastWriteTime),
        strength: volume_strength(handle),
    })
}

/// `(volume, 128-bit file id, tag, discriminator)` of the object the
/// handle addresses. A failed `FileIdInfo` query propagates — a
/// fabricated `(0,0)` volume/index would falsely match unrelated
/// objects and let archive identity-gated cleanup or `open_pinned`
/// claim a guarantee the filesystem never provided.
fn identity_parts(
    handle: RawHandle,
    basic: &FILE_BASIC_INFO,
    kind: VfsKind,
) -> io::Result<NodeIdentity> {
    let info = query_info::<FILE_ID_INFO>(handle, FileIdInfo)?;
    let FILE_ID_128 { Identifier } = info.FileId;
    let (volume, index) = (info.VolumeSerialNumber, u128::from_le_bytes(Identifier));
    let (tag, discriminator) = match kind {
        VfsKind::Dir => {
            // Directory ChangeTime mutates when children are added, so
            // only CreationTime can discriminate. It is caller-settable
            // in principle, but colliding with us additionally requires
            // FileId reuse — out of reach for a racing adversary.
            (NodeTypeTag::Dir, basic.CreationTime)
        }
        VfsKind::File => {
            // `ChangeTime` cannot be set by a caller — a
            // delete+recreate at a recycled FileId always produces a
            // different value.
            (NodeTypeTag::File, basic.ChangeTime)
        }
        _ => (NodeTypeTag::Other, basic.ChangeTime),
    };
    Ok(NodeIdentity {
        volume,
        index,
        tag,
        discriminator: u64::try_from(discriminator).unwrap_or(0),
        // The 128-bit FileId already embeds the MFT sequence number,
        // so the index itself discriminates recreation.
        handle: None,
    })
}

/// `GetFinalPathNameByHandleW` — the handle-derived canonical path
/// (`\\?\X:\…` verbatim form, same shape std `canonicalize`
/// produces).
#[expect(
    unsafe_code,
    reason = "GetFinalPathNameByHandleW is Win32 FFI; see SAFETY comment"
)]
fn final_path_name(handle: RawHandle) -> io::Result<PathBuf> {
    let mut buf = vec![0u16; 1024];
    // SAFETY: `handle` is a valid open HANDLE; `buf` is valid for
    // `buf.len()` UTF-16 units of write. 0 flags = VOLUME_NAME_DOS |
    // FILE_NAME_NORMALIZED.
    let n = unsafe {
        GetFinalPathNameByHandleW(
            handle.cast(),
            buf.as_mut_ptr(),
            u32::try_from(buf.len()).unwrap_or(u32::MAX),
            0,
        )
    };
    if n == 0 {
        return Err(io::Error::last_os_error());
    }
    let n = usize::try_from(n).unwrap_or(0);
    if n >= buf.len() {
        buf.resize(n + 1, 0);
        // SAFETY: as above with the larger buffer.
        let m = unsafe {
            GetFinalPathNameByHandleW(
                handle.cast(),
                buf.as_mut_ptr(),
                u32::try_from(buf.len()).unwrap_or(u32::MAX),
                0,
            )
        };
        if m == 0 {
            return Err(io::Error::last_os_error());
        }
        let m = usize::try_from(m).unwrap_or(0);
        buf.truncate(m);
    } else {
        buf.truncate(n);
    }
    Ok(PathBuf::from(OsString::from_wide(&buf)))
}

/// `DeviceIoControl(FSCTL_GET_REPARSE_POINT)` — the reparse point's
/// stored substitute name in raw NT-namespace form (`\??\`-style;
/// namespace bridging is the resolve layer's job). A non-reparse
/// object fails `InvalidInput`.
#[expect(
    unsafe_code,
    reason = "DeviceIoControl/reparse-buffer read is Win32 FFI; see SAFETY comments"
)]
fn reparse_target(handle: RawHandle) -> io::Result<OsString> {
    const BUF_CAP: usize = 16 * 1024;
    // `u64` elements keep the buffer 8-aligned so the raw
    // REPARSE_DATA_BUFFER pointer cast is within alignment.
    let mut buf = vec![0u64; BUF_CAP / 8];
    let mut returned: u32 = 0;
    // SAFETY: `handle` is a valid open HANDLE; `buf` is valid for
    // BUF_CAP bytes of write; `returned` is valid for write.
    let ok = unsafe {
        DeviceIoControl(
            handle.cast(),
            FSCTL_GET_REPARSE_POINT,
            std::ptr::null(),
            0,
            buf.as_mut_ptr().cast(),
            u32::try_from(BUF_CAP).unwrap_or(u32::MAX),
            &raw mut returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        let e = io::Error::last_os_error();
        // ERROR_NOT_A_REPARSE_POINT (4390): the name is not a link.
        if e.raw_os_error() == Some(4390) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ruviform-io-file: not a reparse point",
            ));
        }
        return Err(e);
    }
    // SAFETY: `buf` holds a REPARSE_DATA_BUFFER returned by the
    // kernel (8-aligned allocation ≥ the struct's alignment); the
    // header fields are within the returned length.
    let reparse = unsafe { &*buf.as_ptr().cast::<REPARSE_DATA_BUFFER>() };
    let (sub_off, sub_len, path_off) = match reparse.ReparseTag {
        IO_REPARSE_TAG_SYMLINK => {
            // SAFETY: the tag selects the SymbolicLinkReparseBuffer
            // union arm.
            let b = unsafe { &reparse.Anonymous.SymbolicLinkReparseBuffer };
            (
                usize::from(b.SubstituteNameOffset),
                usize::from(b.SubstituteNameLength),
                (&raw const b.PathBuffer).cast::<u8>() as usize - buf.as_ptr() as usize,
            )
        }
        IO_REPARSE_TAG_MOUNT_POINT => {
            // SAFETY: the tag selects the MountPointReparseBuffer
            // union arm.
            let b = unsafe { &reparse.Anonymous.MountPointReparseBuffer };
            (
                usize::from(b.SubstituteNameOffset),
                usize::from(b.SubstituteNameLength),
                (&raw const b.PathBuffer).cast::<u8>() as usize - buf.as_ptr() as usize,
            )
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ruviform-io-file: reparse point is not a name surrogate",
            ));
        }
    };
    let start = path_off.saturating_add(sub_off);
    let byte_len = sub_len;
    if start.saturating_add(byte_len) > usize::try_from(returned).unwrap_or(BUF_CAP) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ruviform-io-file: malformed reparse buffer",
        ));
    }
    // SAFETY: `[start, start+byte_len)` lies inside the kernel-filled
    // region of the 8-aligned buffer; `start` is even (the
    // PathBuffer field is WCHAR-aligned), so the `u16` view is
    // aligned.
    let wide = unsafe {
        std::slice::from_raw_parts(buf.as_ptr().cast::<u16>().add(start / 2), byte_len / 2)
    };
    Ok(OsString::from_wide(wide))
}

// ------------------------------------------------------------------
// Directory listing — NtQueryDirectoryFile with RestartScan so every
// call owns independent enumeration state.
// ------------------------------------------------------------------

#[expect(
    unsafe_code,
    reason = "NtQueryDirectoryFile is NT syscall FFI; see SAFETY comments"
)]
fn list_dir(dir: &WinVfsDir) -> io::Result<Vec<VfsEntry>> {
    const BUF_CAP: usize = 64 * 1024;
    const STATUS_NO_MORE_FILES: NTSTATUS = 0x8000_0006u32.cast_signed();
    // `u64` elements keep the buffer 8-aligned — matching
    // FILE_BOTH_DIR_INFORMATION record alignment (NextEntryOffset
    // values are LONGLONG-aligned, so `offset` stays a multiple of 8).
    let mut buf = vec![0u64; BUF_CAP / 8];
    let mut out = Vec::new();
    let mut restart = true;
    loop {
        let mut sb = IO_STATUS_BLOCK::default();
        // SAFETY: `dir.handle` is a valid open directory HANDLE;
        // `buf`/`sb` are valid for the call.
        let status = unsafe {
            NtQueryDirectoryFile(
                dir.handle.as_raw_handle().cast(),
                std::ptr::null_mut(),
                None,
                std::ptr::null(),
                &raw mut sb,
                buf.as_mut_ptr().cast(),
                u32::try_from(BUF_CAP).unwrap_or(u32::MAX),
                FileBothDirectoryInformation as FILE_INFORMATION_CLASS,
                false,
                std::ptr::null_mut(),
                restart,
            )
        };
        restart = false;
        // STATUS_NO_MORE_FILES — enumeration finished.
        if status == STATUS_NO_MORE_FILES {
            break;
        }
        if status < 0 {
            return Err(nt_error(status));
        }
        let mut offset = 0usize;
        loop {
            // SAFETY: `buf` holds FILE_BOTH_DIR_INFORMATION records
            // returned by the kernel; `offset` walks via
            // NextEntryOffset inside the returned region, and the
            // buffer is 8-aligned.
            let info = unsafe {
                &*buf
                    .as_ptr()
                    .add(offset / 8)
                    .cast::<FILE_BOTH_DIR_INFORMATION>()
            };
            let name_len = usize::try_from(info.FileNameLength).unwrap_or(0) / 2;
            // `info` already points at `offset` inside `buf` — the
            // field's buffer position is the pointer difference.
            let name_start =
                (&raw const info.FileName).cast::<u8>() as usize - buf.as_ptr() as usize;
            if name_start + name_len * 2 <= BUF_CAP {
                // SAFETY: `[name_start, +name_len*2)` is inside the
                // kernel-filled region; the FileName field is
                // WCHAR-aligned, so the `u16` view is aligned.
                let wide = unsafe {
                    std::slice::from_raw_parts(
                        buf.as_ptr().cast::<u16>().add(name_start / 2),
                        name_len,
                    )
                };
                let name = OsString::from_wide(wide);
                if name != "." && name != ".." {
                    let attrs = info.FileAttributes;
                    // Reparse before directory — a junction is
                    // DIRECTORY|REPARSE_POINT and must be `Symlink`.
                    let kind = if attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                        // For reparse points the EaSize field carries
                        // the reparse tag.
                        reparse_kind(info.EaSize)
                    } else if attrs & FILE_ATTRIBUTE_DIRECTORY != 0 {
                        VfsKind::Dir
                    } else {
                        VfsKind::File
                    };
                    out.push(VfsEntry { name, kind });
                }
            }
            if info.NextEntryOffset == 0 {
                break;
            }
            offset += usize::try_from(info.NextEntryOffset).unwrap_or(usize::MAX);
            if offset >= buf.len() * 8 {
                break;
            }
        }
        // sb.Information == 0 → no entries returned this call.
        if sb.Information == 0 {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    /// FILETIME for ~year 3000 (ticks past 1601 epoch) — pre-fix this
    /// overflowed `ticks * 100` in `u64` and panicked in debug builds.
    #[test]
    fn filetime_far_future_does_not_overflow() {
        // ~year 3000: (3000-1601) years ≈ 4.4e19 100ns ticks incl. epoch
        // offset; with offset subtraction ≈ 3.3e19 ticks → *100 would
        // exceed u64::MAX (≈1.8e19). Must clamp to `None`, never wrap.
        let year3000_filetime: i64 = 441_919_392_000_000_000;
        let t = filetime_to_system(year3000_filetime);
        assert!(
            t.is_none_or(|t| t >= std::time::UNIX_EPOCH),
            "far-future time must not wrap to the past: {t:?}"
        );
        // i64::MAX may clamp to `SystemTime::MAX` or `None` — both
        // honest; it must never panic or wrap into the past.
        assert!(filetime_to_system(i64::MAX).is_none_or(|t| t >= std::time::UNIX_EPOCH));
        assert_eq!(filetime_to_system(-1), None);
        // Just before the 1601 FILETIME epoch → `None`, not a wrap.
        assert_eq!(filetime_to_system(0), None);
        assert_eq!(filetime_to_system(116_444_735_999_999_999), None);
    }

    #[test]
    fn filetime_normal_value_converts() {
        // 2024-01-01 UTC ≈ 133_485_120_000_000_000 ticks since 1601.
        let t = filetime_to_system(133_485_120_000_000_000).unwrap();
        let epoch = t.duration_since(std::time::UNIX_EPOCH).unwrap();
        // ticks/1e7 - 11644473600s (1601->1970 epoch delta).
        assert_eq!(epoch.as_secs(), 1_704_038_400);
    }

    /// `FileIdInfo` on a non-filesystem handle (the `NUL` device)
    /// deterministically fails — pre-fix the failure was replaced with
    /// a fabricated `(0,0)` identity that could match unrelated objects.
    /// Now the query error must propagate: no object may report a
    /// success identity the filesystem did not supply.
    #[test]
    fn file_id_failure_propagates_instead_of_fabricating_identity() {
        let nul = File::open("NUL").expect("NUL device must open on Windows");
        assert!(
            query_info::<FILE_ID_INFO>(nul.as_raw_handle(), FileIdInfo).is_err(),
            "FileIdInfo is expected to fail on a device handle"
        );
        assert!(
            identity_parts(
                nul.as_raw_handle(),
                &FILE_BASIC_INFO::default(),
                VfsKind::File,
            )
            .is_err(),
            "FileIdInfo failure must not produce a fabricated identity"
        );
        // The whole probe must fail-closed — archive cleanup and
        // `open_pinned` both consume this boundary.
        assert!(probe_handle(nul.as_raw_handle()).is_err());
    }
}

//! Real OS backend for staging trees — `WindowsExtractOps`
//! (IO-IMP-10).
//!
//! Uses the NT system calls instead of the Win32 path APIs so every
//! operation is handle-relative and no-follow:
//!
//! - `NtOpenFile` on an absolute `\\??\\` path opens the trusted
//!   parent / staging-root handle.
//! - `NtCreateFile` / `NtOpenFile` with
//!   `OBJECT_ATTRIBUTES.RootDirectory = <open dir handle>` and a
//!   single-component `ObjectName` traverse one level at a time —
//!   path strings are never re-resolved below the trusted handle.
//! - `OBJ_DONT_REPARSE | FILE_OPEN_REPARSE_POINT` means a symlink or
//!   junction is never followed: opens either fail or address the
//!   reparse point itself.
//! - `FILE_CREATE` disposition is atomic create-new: it refuses any
//!   existing name (including a planted reparse point) — no
//!   exists()-then-create window.
//! - `NtDeleteFile` relative to a verified dir handle deletes the
//!   named object itself (`OBJ_DONT_REPARSE`), including a swapped-in
//!   link object — but only after an explicit
//!   `probe_identity`/`identity` equality check, so a foreign object
//!   at a recorded name is retained rather than deleted.
//! - Identity is `GetFileInformationByHandleEx(FileIdInfo)` on the
//!   open handle — it names the object the handle addresses.
//!
//! Caveat documented in the design: `OBJ_DONT_REPARSE` requires
//! Windows 10 RS1+; on filesystems that cannot honor reparse
//! semantics the calls fail honestly (`Io`) rather than silently
//! following links.
//!
//! **RT contract:** non-RT only.

#![cfg(windows)]

use std::fs::File;
use std::io::{self, Write};
use std::mem::{MaybeUninit, size_of};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::path::{Path, PathBuf};

use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT,
    FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile, NtDeleteFile, NtOpenFile,
    RtlNtStatusToDosErrorNoTeb,
};
use windows_sys::Win32::Foundation::{
    HANDLE, NTSTATUS, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, UNICODE_STRING,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ACCESS_RIGHTS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_BASIC_INFO, FILE_GENERIC_WRITE, FILE_ID_128, FILE_ID_INFO,
    FILE_INFO_BY_HANDLE_CLASS, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
    FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, FileBasicInfo, FileIdInfo,
    GetFileInformationByHandleEx, SYNCHRONIZE,
};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

use super::fs::{
    MAX_NAME_ATTEMPTS, NodeIdentity, NodeTypeTag, PlatformExtractOps, StagingDir, StagingFile,
    StagingRoot, staging_name,
};

const OBJ_ATTRS: u32 = OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE;
const SHARE_ALL: FILE_SHARE_MODE = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
const DIR_ACCESS: FILE_ACCESS_RIGHTS =
    FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE;
const FILE_ACCESS: FILE_ACCESS_RIGHTS = FILE_GENERIC_WRITE | FILE_READ_ATTRIBUTES;

/// The Windows [`PlatformExtractOps`] implementation.
#[derive(Debug)]
pub(crate) struct WindowsExtractOps;

impl PlatformExtractOps for WindowsExtractOps {
    /// Creates the staging root: open trusted parent → `NtCreateFile
    /// FILE_CREATE` unique directory name → verify it is a real
    /// directory.
    fn create_staging_root(&self, parent: &Path) -> io::Result<Box<dyn StagingRoot>> {
        let parent = std::path::absolute(parent)?;
        let parent_handle = open_path_dir(&parent)?;
        for _ in 0..MAX_NAME_ATTEMPTS {
            let name = staging_name("rvf-staging");
            match create_dir_rel(parent_handle.as_raw_handle(), &name) {
                Ok(dir) => {
                    return Ok(Box::new(WinDir {
                        handle: dir,
                        path: parent.join(&name),
                        parent: Some(ParentRef {
                            handle: parent_handle,
                            name,
                        }),
                    }));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "ruviform-io-archive: staging name space exhausted",
        ))
    }

    /// Re-opens an existing staging root by path (final component
    /// opened as a directory with `OBJ_DONT_REPARSE` → a swapped link
    /// fails or opens the reparse object, which the identity check
    /// then rejects).
    fn open_staging_root(&self, path: &Path) -> io::Result<Box<dyn StagingRoot>> {
        let path = std::path::absolute(path)?;
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?
            .to_string_lossy()
            .into_owned();
        let parent_path = path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let parent_handle = open_path_dir(&parent_path)?;
        let handle = open_dir_rel(parent_handle.as_raw_handle(), &name)?;
        Ok(Box::new(WinDir {
            handle,
            path,
            parent: Some(ParentRef {
                handle: parent_handle,
                name,
            }),
        }))
    }
}

/// An open staging directory (`HANDLE` + diagnostic path). `parent`
/// is set only on the root so it can remove itself.
#[derive(Debug)]
struct WinDir {
    handle: OwnedHandle,
    path: PathBuf,
    parent: Option<ParentRef>,
}

#[derive(Debug)]
struct ParentRef {
    handle: OwnedHandle,
    name: String,
}

impl StagingDir for WinDir {
    fn path(&self) -> &Path {
        &self.path
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        identity_of(self.handle.as_raw_handle())
    }

    fn probe_identity(&self, name: &str) -> io::Result<NodeIdentity> {
        let handle = open_any_rel(self.handle.as_raw_handle(), name)?;
        identity_of(handle.as_raw_handle())
    }

    fn create_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
        let handle = create_dir_rel(self.handle.as_raw_handle(), name)?;
        Ok(Box::new(WinDir {
            handle,
            path: self.path.join(name),
            parent: None,
        }))
    }

    fn open_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
        let handle = open_dir_rel(self.handle.as_raw_handle(), name)?;
        Ok(Box::new(WinDir {
            handle,
            path: self.path.join(name),
            parent: None,
        }))
    }

    #[expect(
        unsafe_code,
        reason = "HANDLE → File ownership wrap; see SAFETY comment"
    )]
    fn create_file(&self, name: &str) -> io::Result<Box<dyn StagingFile>> {
        let handle = create_file_rel(self.handle.as_raw_handle(), name)?;
        // SAFETY: `handle` is a freshly opened file HANDLE opened for
        // synchronous write access; `File` takes unique ownership.
        let file = unsafe { File::from_raw_handle(handle) };
        Ok(Box::new(WinFile(file)))
    }

    fn remove_file(&self, name: &str) -> io::Result<()> {
        delete_rel(self.handle.as_raw_handle(), name)
    }

    fn remove_dir(&self, name: &str) -> io::Result<()> {
        delete_rel(self.handle.as_raw_handle(), name)
    }
}

impl StagingRoot for WinDir {
    /// Deletes the staging root via its parent handle. Re-checks the
    /// recorded name's identity against this handle's own `FileId`
    /// first: a swapped-in link or foreign directory reports an error
    /// instead of being deleted.
    fn remove_root(self: Box<Self>) -> io::Result<()> {
        let Some(parent) = &self.parent else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ruviform-io-archive: remove_root on a non-root directory",
            ));
        };
        let expected = identity_of(self.handle.as_raw_handle())?;
        let observed = {
            let handle = open_any_rel(parent.handle.as_raw_handle(), &parent.name)?;
            identity_of(handle.as_raw_handle())?
        };
        if observed != expected {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "ruviform-io-archive: staging root identity changed; refusing removal",
            ));
        }
        delete_rel(parent.handle.as_raw_handle(), &parent.name)
    }
}

/// An open staging file.
#[derive(Debug)]
struct WinFile(File);

impl Write for WinFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StagingFile for WinFile {
    fn identity(&self) -> io::Result<NodeIdentity> {
        identity_of(self.0.as_raw_handle())
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
    use std::os::windows::ffi::OsStrExt;
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

fn wide_component(name: &str) -> Vec<u16> {
    name.encode_utf16().chain(std::iter::once(0)).collect()
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

/// `NtOpenFile` on an absolute NT path (trusted parent only).
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

/// `NtCreateFile(FILE_CREATE)` relative to `dir`: exclusive create of
/// a child directory — refuses any existing name including reparse
/// points.
#[expect(
    unsafe_code,
    reason = "NtCreateFile is NT syscall FFI; see SAFETY comments"
)]
fn create_dir_rel(dir: RawHandle, name: &str) -> io::Result<OwnedHandle> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).unwrap_or(u32::MAX),
        RootDirectory: dir.cast(),
        ObjectName: &raw const name,
        Attributes: OBJ_ATTRS,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    };
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

/// `NtOpenFile` relative to `dir` opening an existing child
/// *directory* — `OBJ_DONT_REPARSE | FILE_OPEN_REPARSE_POINT` refuses
/// to follow links (a junction/symlink-dir opens the reparse object,
/// whose identity then mismatches).
fn open_dir_rel(dir: RawHandle, name: &str) -> io::Result<OwnedHandle> {
    open_rel_impl(dir, name, DIR_ACCESS, SHARE_ALL, FILE_DIRECTORY_FILE)
}

/// `NtOpenFile` relative to `dir` opening a child object of *either*
/// kind (for identity probes): tries as file, then as directory.
fn open_any_rel(dir: RawHandle, name: &str) -> io::Result<OwnedHandle> {
    const ATTRS_ACCESS: FILE_ACCESS_RIGHTS = FILE_READ_ATTRIBUTES | SYNCHRONIZE;
    match open_rel_impl(dir, name, ATTRS_ACCESS, SHARE_ALL, FILE_NON_DIRECTORY_FILE) {
        Ok(h) => Ok(h),
        Err(_) => open_rel_impl(dir, name, ATTRS_ACCESS, SHARE_ALL, FILE_DIRECTORY_FILE),
    }
}

#[expect(
    unsafe_code,
    reason = "NtOpenFile is NT syscall FFI; see SAFETY comments"
)]
fn open_rel_impl(
    dir: RawHandle,
    name: &str,
    access: FILE_ACCESS_RIGHTS,
    share: FILE_SHARE_MODE,
    options_kind: u32,
) -> io::Result<OwnedHandle> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).unwrap_or(u32::MAX),
        RootDirectory: dir.cast(),
        ObjectName: &raw const name,
        Attributes: OBJ_ATTRS,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    };
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

/// `NtCreateFile(FILE_CREATE)` relative to `dir`: exclusive create of
/// a regular file — refuses any existing name (including reparse
/// points); `OBJ_DONT_REPARSE | FILE_OPEN_REPARSE_POINT` never
/// follows.
#[expect(
    unsafe_code,
    reason = "NtCreateFile is NT syscall FFI; see SAFETY comments"
)]
fn create_file_rel(dir: RawHandle, name: &str) -> io::Result<RawHandle> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).unwrap_or(u32::MAX),
        RootDirectory: dir.cast(),
        ObjectName: &raw const name,
        Attributes: OBJ_ATTRS,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    };
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut sb = IO_STATUS_BLOCK::default();
    let alloc: i64 = 0;
    // SAFETY: `dir` is a valid open directory HANDLE; all out-params
    // valid for the call; `name_buf` outlives them. ShareAccess = 0
    // keeps the new file exclusively ours while open.
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
fn delete_rel(dir: RawHandle, name: &str) -> io::Result<()> {
    let name_buf = wide_component(name);
    let name = unicode_string(&name_buf);
    let attrs = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).unwrap_or(u32::MAX),
        RootDirectory: dir.cast(),
        ObjectName: &raw const name,
        Attributes: OBJ_ATTRS,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    };
    // SAFETY: `dir` is a valid open directory HANDLE; `attrs`/`name`
    // valid for the call; `name_buf` outlives them.
    let status = unsafe { NtDeleteFile(&raw const attrs) };
    if status < 0 {
        return Err(nt_error(status));
    }
    Ok(())
}

/// `GetFileInformationByHandleEx(FileIdInfo)` — the (volume, 128-bit
/// file id) of the object the handle addresses.
#[expect(
    unsafe_code,
    reason = "GetFileInformationByHandleEx is Win32 FFI; see SAFETY comment"
)]
fn identity_of(handle: RawHandle) -> io::Result<NodeIdentity> {
    let mut info = MaybeUninit::<FILE_ID_INFO>::uninit();
    // SAFETY: `handle` is a valid open HANDLE; `info` is valid for
    // `size_of::<FILE_ID_INFO>()` bytes of write.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle.cast(),
            FileIdInfo as FILE_INFO_BY_HANDLE_CLASS,
            info.as_mut_ptr().cast(),
            u32::try_from(size_of::<FILE_ID_INFO>()).unwrap_or(u32::MAX),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `info` was fully initialized by the successful call.
    let info = unsafe { info.assume_init() };
    let FILE_ID_128 { Identifier } = info.FileId;
    let mut basic = MaybeUninit::<FILE_BASIC_INFO>::uninit();
    // SAFETY: `handle` is a valid open HANDLE; `basic` is valid for
    // `size_of::<FILE_BASIC_INFO>()` bytes of write.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle.cast(),
            FileBasicInfo as FILE_INFO_BY_HANDLE_CLASS,
            basic.as_mut_ptr().cast(),
            u32::try_from(size_of::<FILE_BASIC_INFO>()).unwrap_or(u32::MAX),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `basic` was fully initialized by the successful call.
    let basic = unsafe { basic.assume_init() };
    let attrs = basic.FileAttributes;
    let (tag, discriminator) = if attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        (NodeTypeTag::Other, basic.ChangeTime)
    } else if attrs & FILE_ATTRIBUTE_DIRECTORY != 0 {
        // Directory ChangeTime mutates when children are added, so
        // only CreationTime can discriminate. It is caller-settable
        // in principle, but colliding with us additionally requires
        // FileId reuse — out of reach for a racing adversary.
        (NodeTypeTag::Dir, basic.CreationTime)
    } else {
        // `ChangeTime` cannot be set by a caller — a delete+recreate
        // at a recycled FileId always produces a different value.
        // The session refreshes file identities post-write so the
        // recorded value reflects the final write.
        (NodeTypeTag::File, basic.ChangeTime)
    };
    Ok(NodeIdentity {
        volume: info.VolumeSerialNumber,
        index: u128::from_le_bytes(Identifier),
        tag,
        discriminator: u64::try_from(discriminator).unwrap_or(0),
        // The 128-bit FileId already embeds the MFT sequence number,
        // so the index itself discriminates recreation.
        handle: None,
    })
}

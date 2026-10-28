//! Who may open the pipe (architecture §11, §13).
//!
//! Windows gives a new named pipe a default DACL that lets *Everyone* and *Anonymous* read it.
//! The server therefore creates every instance with an explicit, protected DACL that allows
//! only the current user: `D:P(A;;GA;;;<user SID>)`. Remote clients are refused separately
//! (`PIPE_REJECT_REMOTE_CLIENTS`, set by the server options).
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW, GetSecurityInfo,
    SDDL_REVISION_1, SE_KERNEL_OBJECT,
};
use windows_sys::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetTokenInformation, PSECURITY_DESCRIPTOR, PSID,
    SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// The current user's SID as a string, e.g. `S-1-5-21-…-1001`.
pub fn current_user_sid() -> io::Result<String> {
    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: the pseudo-handle from GetCurrentProcess needs no closing; `token` is an out
    // parameter closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = token_user_sid(token);
    // SAFETY: `token` was opened above and is closed exactly once.
    unsafe { CloseHandle(token) };
    result
}

fn token_user_sid(token: HANDLE) -> io::Result<String> {
    let mut len = 0u32;
    // SAFETY: a size query with a null buffer; fails with ERROR_INSUFFICIENT_BUFFER and sets `len`.
    unsafe { GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &raw mut len) };
    if len == 0 {
        return Err(io::Error::last_os_error());
    }
    // u64 elements keep the buffer aligned for TOKEN_USER.
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: `buf` holds at least `len` writable bytes.
    if unsafe { GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), len, &raw mut len) }
        == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: on success the buffer starts with a TOKEN_USER whose SID points into `buf`.
    let sid = unsafe { (*buf.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut wide: *mut u16 = ptr::null_mut();
    // SAFETY: `sid` is valid while `buf` lives; `wide` is allocated by the call and freed below.
    if unsafe { ConvertSidToStringSidW(sid, &raw mut wide) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `wide` is a NUL-terminated string allocated with LocalAlloc.
    let s = unsafe { take_local_wide(wide) };
    Ok(s)
}

/// `SECURITY_ATTRIBUTES` with a self-relative descriptor built from SDDL; freed on drop.
pub struct SecurityAttributes {
    attrs: SECURITY_ATTRIBUTES,
}

impl SecurityAttributes {
    /// Only `sid` may open the object; inherited ACEs are blocked (`P`).
    pub fn only_user(sid: &str) -> io::Result<Self> {
        Self::from_sddl(&user_only_sddl(sid))
    }

    pub fn from_sddl(sddl: &str) -> io::Result<Self> {
        let wide: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
        let mut sd: PSECURITY_DESCRIPTOR = ptr::null_mut();
        // SAFETY: `wide` is NUL-terminated; `sd` receives a LocalAlloc'd descriptor freed in Drop.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                SDDL_REVISION_1,
                &raw mut sd,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            attrs: SECURITY_ATTRIBUTES {
                nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
                lpSecurityDescriptor: sd,
                bInheritHandle: 0,
            },
        })
    }

    /// For `CreateNamedPipeW`; valid while `self` lives.
    pub fn as_ptr(&self) -> *mut c_void {
        (&raw const self.attrs).cast_mut().cast()
    }
}

impl Drop for SecurityAttributes {
    fn drop(&mut self) {
        // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW, freed once.
        unsafe { LocalFree(self.attrs.lpSecurityDescriptor as HLOCAL) };
    }
}

// SAFETY: the descriptor is immutable after creation and owned by this value alone.
unsafe impl Send for SecurityAttributes {}
// SAFETY: as above; shared access only reads it.
unsafe impl Sync for SecurityAttributes {}

pub fn user_only_sddl(sid: &str) -> String {
    format!("D:P(A;;GA;;;{sid})")
}

/// The DACL of an open kernel object as SDDL, for tests and diagnostics.
pub fn dacl_sddl(object: &impl std::os::windows::io::AsRawHandle) -> io::Result<String> {
    let handle: HANDLE = object.as_raw_handle();
    let mut sd: PSECURITY_DESCRIPTOR = ptr::null_mut();
    // SAFETY: `object` keeps `handle` open for the call; `sd` is LocalAlloc'd by the call and freed below.
    let err = unsafe {
        GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &raw mut sd,
        )
    };
    if err != 0 {
        return Err(io::Error::from_raw_os_error(
            i32::try_from(err).unwrap_or(-1),
        ));
    }
    let mut wide: *mut u16 = ptr::null_mut();
    // SAFETY: `sd` is valid until freed below; `wide` is LocalAlloc'd and freed in take_local_wide.
    let ok = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            sd,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &raw mut wide,
            ptr::null_mut(),
        )
    };
    let result = if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: NUL-terminated string allocated by the call.
        Ok(unsafe { take_local_wide(wide) })
    };
    // SAFETY: allocated by GetSecurityInfo, freed once.
    unsafe { LocalFree(sd as HLOCAL) };
    result
}

/// A SID string in `S-1-…` form. SDDL aliases are resolved: `dacl_sddl` prints well-known
/// accounts that way, e.g. `LA` for the built-in Administrator that CI runners use.
pub fn canonical_sid(sid: &str) -> io::Result<String> {
    let wide: Vec<u16> = sid.encode_utf16().chain(Some(0)).collect();
    let mut psid: PSID = ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated; `psid` is LocalAlloc'd by the call and freed below.
    if unsafe { ConvertStringSidToSidW(wide.as_ptr(), &raw mut psid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut out: *mut u16 = ptr::null_mut();
    // SAFETY: `psid` is valid until freed below; `out` is LocalAlloc'd by the call.
    let result = if unsafe { ConvertSidToStringSidW(psid, &raw mut out) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: NUL-terminated string allocated by the call.
        Ok(unsafe { take_local_wide(out) })
    };
    // SAFETY: allocated by ConvertStringSidToSidW, freed once.
    unsafe { LocalFree(psid as HLOCAL) };
    result
}

/// Copies a NUL-terminated UTF-16 string allocated with `LocalAlloc` and frees it.
///
/// # Safety
/// `p` must be a valid NUL-terminated string allocated with `LocalAlloc`.
unsafe fn take_local_wide(p: *mut u16) -> String {
    let mut len = 0;
    // SAFETY: the caller guarantees a terminator.
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` elements were just read.
    let s = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) });
    // SAFETY: the caller guarantees LocalAlloc ownership.
    unsafe { LocalFree(p as HLOCAL) };
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_user_has_a_sid() {
        let sid = current_user_sid().expect("sid");
        assert!(sid.starts_with("S-1-"), "{sid}");
    }

    #[test]
    fn sddl_parses_into_a_descriptor() {
        let sid = current_user_sid().expect("sid");
        SecurityAttributes::only_user(&sid).expect("descriptor");
        assert!(SecurityAttributes::from_sddl("D:P(A;;GA;;;nonsense)").is_err());
    }

    #[test]
    fn sddl_aliases_resolve_to_full_sids() {
        let sid = current_user_sid().expect("sid");
        assert_eq!(canonical_sid(&sid).expect("own sid"), sid);
        let admin = canonical_sid("LA").expect("built-in Administrator");
        assert!(
            admin.starts_with("S-1-5-21-") && admin.ends_with("-500"),
            "{admin}"
        );
        assert_eq!(canonical_sid("WD").expect("Everyone"), "S-1-1-0");
        assert!(canonical_sid("nonsense").is_err());
    }
}

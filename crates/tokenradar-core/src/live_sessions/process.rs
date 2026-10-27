//! Is a pid still the process that wrote a session file? Guards against crashed sessions and
//! pid reuse by comparing the process creation time with the file's `procStart`.

/// What the platform could tell about a pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// Running; `created` is its creation time as a FILETIME (100 ns since 1601-01-01 UTC).
    Alive { created: u64 },
    /// No such process, or it has exited.
    Dead,
    /// Exists but could not be queried (e.g. another user's elevated process), or the platform
    /// has no check yet.
    Unknown,
}

/// FILETIME ticks between 1601-01-01 and 1970-01-01.
const UNIX_EPOCH_FILETIME: u64 = 116_444_736_000_000_000;

/// Unix milliseconds of a FILETIME.
pub fn filetime_to_unix_ms(ft: u64) -> i64 {
    i64::try_from(ft.saturating_sub(UNIX_EPOCH_FILETIME) / 10_000).unwrap_or(i64::MAX)
}

#[cfg(windows)]
#[allow(unsafe_code)]
pub fn probe(pid: u32) -> Liveness {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, FILETIME, GetLastError, STILL_ACTIVE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // SAFETY: plain Win32 calls. The handle is checked for null, only read through, and closed
    // on every path; the out-parameters are valid, writable locals.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return if GetLastError() == ERROR_ACCESS_DENIED {
                Liveness::Unknown
            } else {
                Liveness::Dead
            };
        }
        // An exited process stays openable while anyone holds a handle to it.
        let mut code = 0u32;
        let exited =
            GetExitCodeProcess(handle, &raw mut code) != 0 && code != STILL_ACTIVE.cast_unsigned();
        let zero = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut created, mut exit, mut kernel, mut user) = (zero, zero, zero, zero);
        let ok = GetProcessTimes(
            handle,
            &raw mut created,
            &raw mut exit,
            &raw mut kernel,
            &raw mut user,
        ) != 0;
        CloseHandle(handle);
        if exited {
            Liveness::Dead
        } else if ok {
            Liveness::Alive {
                created: (u64::from(created.dwHighDateTime) << 32)
                    | u64::from(created.dwLowDateTime),
            }
        } else {
            Liveness::Unknown
        }
    }
}

/// macOS and Linux arrive with Step 21 (`kill(pid, 0)` plus the start time from the OS).
#[cfg(not(windows))]
pub fn probe(_pid: u32) -> Liveness {
    Liveness::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_filetime_to_unix_millis() {
        // A `procStart` observed in a real session file and its process start time.
        assert_eq!(
            filetime_to_unix_ms(134_375_564_581_785_791),
            1_793_082_858_178
        );
        assert_eq!(filetime_to_unix_ms(UNIX_EPOCH_FILETIME), 0);
        assert_eq!(filetime_to_unix_ms(0), 0);
    }

    #[cfg(windows)]
    #[test]
    fn the_current_process_is_alive_and_a_bogus_pid_is_not() {
        match probe(std::process::id()) {
            Liveness::Alive { created } => {
                let ms = filetime_to_unix_ms(created);
                let now = crate::domain::Timestamp::now().as_millis();
                assert!(ms <= now && now - ms < 3_600_000, "created {ms}, now {now}");
            }
            other => panic!("expected alive, got {other:?}"),
        }
        // Pids are multiples of 4 on Windows; this one is never handed out.
        assert_eq!(probe(0xFFFF_FFFD), Liveness::Dead);
    }

    #[cfg(windows)]
    #[test]
    fn an_exited_child_is_dead_even_while_its_handle_is_held() {
        let mut child = std::process::Command::new("cmd")
            .args(["/C", "exit 0"])
            .spawn()
            .expect("spawn");
        let pid = child.id();
        child.wait().expect("wait");
        // `child` still owns a process handle here, so the pid cannot be reused yet.
        assert_eq!(probe(pid), Liveness::Dead);
    }
}

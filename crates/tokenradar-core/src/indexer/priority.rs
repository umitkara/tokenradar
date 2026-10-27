//! Background CPU and I/O priority for indexer workers (architecture §2, ADR-024).

/// Lowers the calling thread's CPU *and* I/O priority so indexing never competes with the
/// user's foreground work. Returns whether the platform supports it and the call succeeded.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn enter_background_mode() -> bool {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
    };
    // SAFETY: `GetCurrentThread` returns a pseudo handle that always refers to the calling
    // thread and needs no cleanup; `SetThreadPriority` only reads it.
    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) != 0 }
}

/// No-op until macOS/Linux support (Step 21), where `setpriority`/`ioprio_set` apply.
#[cfg(not(windows))]
pub fn enter_background_mode() -> bool {
    false
}

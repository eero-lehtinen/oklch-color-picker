use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE,
        STD_OUTPUT_HANDLE, SetStdHandle,
    },
};

pub fn attach_parent() {
    // SAFETY: Called during startup, before any application threads are started.
    // These handles are borrowed from the process and are never closed here.
    unsafe {
        let handles = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE]
            .map(|kind| (kind, GetStdHandle(kind)));

        // Attaching can replace standard handles. Keep inherited pipes and files so
        // shell redirection and callers capturing the selected color still work.
        // Failure is expected when launched without a console, e.g. from Explorer.
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }

        for (kind, handle) in handles {
            if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
                SetStdHandle(kind, handle);
            }
        }
    }
}

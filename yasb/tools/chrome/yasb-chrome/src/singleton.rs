//! One instance of a panel at a time.
//!
//! The bar launches these panels on every click. Without a guard, a triple
//! click stacks three copies of the same window on top of each other, and
//! because each one repaints every second the stack flickers and the bottom
//! copy keeps the acrylic backdrop captured from behind the top two.
//!
//! The guard is a named mutex, which is the cheapest thing that survives the
//! "user clicks again while the first is still up" case correctly:
//!
//! * first launch creates the mutex and owns it, and the window opens;
//! * later launches find the mutex already exists, so they do *not* open a
//!   window — they find the live one by class name and raise it, then exit.
//!
//! Raising rather than exiting silently matters: clicking the palette button
//! while the picker is open should bring it back to you, not appear to do
//! nothing at all. That "it does nothing" behaviour was the original symptom
//! being chased here.

use windows::Win32::Foundation::{BOOL, ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, IsWindowVisible, PostMessageW, SetForegroundWindow, SetWindowPos,
    HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
};
use windows::core::PCWSTR;

/// An owned named mutex. Dropping it releases the name, so the next launch
/// gets a fresh window — which is what closing the panel should do.
pub struct Instance {
    _handle: windows::Win32::Foundation::HANDLE,
}

/// Try to become the sole owner of `name`.
///
/// `Ok(Some(_))` means "you own it, go ahead and open the window".
/// `Ok(None)` means another instance already holds it.
///
/// # Panics
/// Never: a mutex that cannot be created is reported as `Err` so the caller
/// can decide whether to open anyway. Refusing to show a panel because the
/// guard failed would be worse than showing a second one.
pub fn acquire(name: &str) -> Result<Option<Instance>, String> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // `Local\` scopes the name to this logon session, so a second user
    // session on the same machine does not collide with this one.
    let handle = match unsafe { CreateMutexW(None, true, PCWSTR(wide.as_ptr())) } {
        Ok(h) => h,
        Err(e) => return Err(format!("CreateMutexW: {e}")),
    };
    // This must be read immediately after the call and before anything else
    // can call into Win32, because `GetLastError` is only meaningful for the
    // most recent call on this thread. Reading it any later — or ignoring it,
    // as an earlier version of this function did — means every launch claims
    // to be the first and the guard never fires.
    let err = unsafe { GetLastError() };
    if err == ERROR_ALREADY_EXISTS {
        // Someone else owns the name. We still got a handle, so release it
        // rather than leaking one per click.
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(handle) };
        Ok(None)
    } else {
        Ok(Some(Instance { _handle: handle }))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // `HANDLE` is `Copy`, so dropping the struct does *not* close the
        // underlying mutex. Without this the name stays owned for the life of
        // the process and every later launch is treated as a duplicate — the
        // panel would open once and then never again.
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(self._handle) };
    }
}

/// The window a duplicate launch found and acted on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// No window of that class was on screen.
    None,
    /// The window was raised.
    Raised,
    /// The message was posted to the window.
    Messaged,
}

fn find_topmost_of_class_inner(class: &str, msg: u32, action: Action) -> Action {
    thread_local! {
        static RESULT: std::cell::Cell<Action> = const { std::cell::Cell::new(Action::None) };
        /// What to do to the window once found, and the action to report.
        static ACT: std::cell::RefCell<Option<(u32, Action)>> =
            const { std::cell::RefCell::new(None) };
    }
    RESULT.with(|r| r.set(Action::None));
    ACT.with(|a| *a.borrow_mut() = Some((msg, action)));
    let mut needle = class.to_string();

    unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        let needle = unsafe { &*(lp.0 as *const String) };
        let mut buf = [0u16; 128];
        let n = unsafe { GetClassNameW(hwnd, &mut buf) };
        if n > 0 {
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let name = String::from_utf16_lossy(&buf[..end]);
            // `EnumWindows` walks in z-order, so the first hit is the topmost
            // copy. With duplicates no longer possible there is at most one,
            // but taking the first keeps this correct if the guard is ever
            // bypassed.
            if name == *needle && unsafe { IsWindowVisible(hwnd) }.as_bool() {
                let (msg, action) = ACT
                    .with(|a| a.borrow().unwrap_or((0, Action::None)));
                match msg {
                    0 => {
                        let _ = unsafe {
                            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE)
                        };
                        let _ = unsafe { SetForegroundWindow(hwnd) };
                    }
                    _ => {
                        let _ = unsafe { PostMessageW(hwnd, msg, WPARAM(0), LPARAM(0)) };
                    }
                }
                RESULT.with(|r| r.set(action));
                // Stop at the first match: the rest are behind it.
                return BOOL(0);
            }
        }
        BOOL(1)
    }

    let _ = unsafe { EnumWindows(Some(cb), LPARAM(&mut needle as *mut String as isize)) };
    RESULT.with(|r| r.get())
}

/// Raise the topmost visible window whose class matches, if any.
///
/// Returns whether one was found. Used when [`acquire`] reports a duplicate:
/// the window is still there, it just lost focus.
pub fn raise_window_of_class(class: &str) -> bool {
    find_topmost_of_class_inner(class, 0, Action::Raised) == Action::Raised
}

/// Post `msg` to the topmost visible window of `class`.
///
/// This is how a panel is toggled closed. A duplicate launch owns no window of
/// its own, so it cannot destroy the first one's — it can only ask. The
/// receiving panel handles the message in its own window procedure, which is
/// the only place where destroying the window is safe.
///
/// Returns whether a window was found. `None` means the instance that held the
/// name has already exited and there is nothing left to close, so the caller
/// should open a fresh window rather than treating the click as consumed.
pub fn notify_window_of_class(class: &str, msg: u32) -> Action {
    find_topmost_of_class_inner(class, msg, Action::Messaged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_name_is_owned_by_the_first_caller() {
        // A name unique to this test run; a second acquire of the same name is
        // the duplicate case, which is exactly what the guard has to catch.
        let name = format!("Local\\yasb-chrome-test-{}", std::process::id());
        let first = acquire(&name).expect("first acquire");
        assert!(first.is_some(), "the first caller must own the name");
        let second = acquire(&name).expect("second acquire");
        assert!(second.is_none(), "a duplicate must not get the name");
        // Dropping releases it, which is what closing the panel does.
        drop(first);
        let third = acquire(&name).expect("third acquire");
        assert!(third.is_some(), "the name must be reusable after release");
    }

    #[test]
    fn raising_a_class_that_is_not_running_reports_false() {
        assert!(!raise_window_of_class("NoSuchWindowClassForThisTest"));
    }

    #[test]
    fn notifying_a_class_that_is_not_running_does_nothing() {
        // Nothing to message: the caller must be able to tell the difference
        // between "closed the open panel" and "there was nothing there", or a
        // click that lands during a close would be swallowed.
        assert_eq!(
            notify_window_of_class("NoSuchWindowClassForThisTest", 0x8001),
            Action::None
        );
    }
}

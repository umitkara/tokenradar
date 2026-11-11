//! Hiding the widget while a full-screen app runs: a topmost window over an exclusive
//! full-screen game or a presentation is in the way, and can upset the game (Step 18).
//!
//! The shell's own notion of "busy" is asked every few seconds while the widget is on; the
//! [`Gate`] turns the answers into hide and show decisions.

/// What the shell says about the user right now.
pub trait UserState {
    /// A full-screen app, a Direct3D full-screen game or presentation mode is running.
    fn busy(&self) -> bool;
}

/// Remembers whether the widget is hidden for a full-screen app.
#[derive(Debug, Default)]
pub struct Gate {
    hidden: bool,
}

impl Gate {
    /// Asks `state` (when `enabled`) and returns the new hidden state if it changed. Turning
    /// the setting off brings a hidden widget back at once.
    pub fn poll(&mut self, state: &dyn UserState, enabled: bool) -> Option<bool> {
        let hide = enabled && state.busy();
        (hide != self.hidden).then(|| {
            self.hidden = hide;
            hide
        })
    }

    pub const fn hidden(&self) -> bool {
        self.hidden
    }
}

/// `SHQueryUserNotificationState`: busy for a full-screen app (`QUNS_BUSY`), a Direct3D
/// full-screen game (`QUNS_RUNNING_D3D_FULL_SCREEN`) and presentation mode
/// (`QUNS_PRESENTATION_MODE`).
#[cfg(windows)]
pub struct Shell;

#[cfg(windows)]
impl UserState for Shell {
    #[allow(unsafe_code)]
    fn busy(&self) -> bool {
        use windows_sys::Win32::UI::Shell::{
            QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
            SHQueryUserNotificationState,
        };
        let mut state = 0;
        // SAFETY: a writable local out parameter.
        let ok = unsafe { SHQueryUserNotificationState(&raw mut state) } == 0;
        ok && matches!(
            state,
            QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct Fake(Cell<bool>);

    impl UserState for Fake {
        fn busy(&self) -> bool {
            self.0.get()
        }
    }

    #[test]
    fn hides_while_busy_and_comes_back_after() {
        let shell = Fake(Cell::new(false));
        let mut gate = Gate::default();
        assert_eq!(gate.poll(&shell, true), None);
        shell.0.set(true);
        assert_eq!(gate.poll(&shell, true), Some(true));
        // Still busy: no new decision, no redraw.
        assert_eq!(gate.poll(&shell, true), None);
        assert!(gate.hidden());
        shell.0.set(false);
        assert_eq!(gate.poll(&shell, true), Some(false));
    }

    #[test]
    fn turning_the_setting_off_shows_the_widget_again() {
        let shell = Fake(Cell::new(true));
        let mut gate = Gate::default();
        assert_eq!(gate.poll(&shell, true), Some(true));
        assert_eq!(gate.poll(&shell, false), Some(false));
        assert_eq!(gate.poll(&shell, false), None);
    }
}

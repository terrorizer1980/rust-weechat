//! Weechat Hook module.
//!
//! Weechat hooks are used for many different things, to create commands, to
//! listen to events on a file descriptor, add completions to weechat, etc.
//! This module contains hook creation methods for the `Weechat` object.

#[cfg(feature = "unstable")]
mod signal;

mod commands;
mod timer;
mod fd;

#[cfg(feature = "unstable")]
pub use signal::{SignalHook, SignalHookValue};
pub use fd::{FdHook, FdHookMode};
pub use commands::{CommandDescription, CommandHook, CommandRunHook};
pub use timer::TimerHook;

use weechat_sys::{t_hook, t_weechat_plugin};
use crate::Weechat;

/// Weechat Hook type. The hook is unhooked automatically when the object is
/// dropped.
pub(crate) struct Hook {
    pub(crate) ptr: *mut t_hook,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl Drop for Hook {
    fn drop(&mut self) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let unhook = weechat.get().unhook.unwrap();
        unsafe { unhook(self.ptr) };
    }
}

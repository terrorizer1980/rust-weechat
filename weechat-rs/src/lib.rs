#![warn(missing_docs)]

use std::ffi::CString;
use weechat_sys::t_weechat_plugin;

#[cfg(feature = "async-executor")]
mod executor;

pub mod bar;
pub mod buffer;
pub mod completion;
pub mod config;
pub mod hooks;
pub mod weechat;

pub use weechat_macro::weechat_plugin;

/// Test
///
/// # Safety
pub trait WeechatPlugin: Sized {
    /// Initialize
    fn init(weechat: &Weechat, args: ArgsWeechat) -> Result<Self, ()>;
}

pub use crate::buffer::Buffer;

/// Main Weechat struct that encapsulates common weechat API functions.
/// It has a similar API as the weechat script API.
pub struct Weechat {
    pub(crate) ptr: *mut t_weechat_plugin,
}

#[cfg(feature = "async-executor")]
pub use executor::JoinHandle;
pub use weechat::ArgsWeechat;

/// Status values for Weechat callbacks
pub enum ReturnCode {
    /// The callback returned successfully.
    Ok = weechat_sys::WEECHAT_RC_OK as isize,
    /// The callback returned successfully and the command will not be executed
    /// after the callback.
    OkEat = weechat_sys::WEECHAT_RC_OK_EAT as isize,
    /// The callback returned with an error.
    Error = weechat_sys::WEECHAT_RC_ERROR as isize,
}

pub(crate) struct LossyCString;

impl LossyCString {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new<T: AsRef<str>>(t: T) -> CString {
        match CString::new(t.as_ref()) {
            Ok(cstr) => cstr,
            Err(_) => CString::new(t.as_ref().replace('\0', ""))
                .expect("string has no nulls"),
        }
    }
}

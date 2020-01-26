#![warn(missing_docs)]

use libc::c_int;

#[cfg(feature = "async-executor")]
mod executor;

pub mod bar;
pub mod buffer;
pub mod completion;
pub mod config;
pub mod hooks;
pub mod weechat;

pub use weechat_macro::weechat_plugin;

#[cfg(feature = "async-executor")]
pub use executor::JoinHandle;
pub use weechat::{ArgsWeechat, Weechat};

pub use bar::{BarItem, LightBarItem};
pub use buffer::{Buffer, BufferSettings, Nick, NickArgs};

pub use hooks::{
    CommandDescription, CommandHook, CommandRunHook, FdHook, FdHookMode,
    SignalHook, SignalHookValue, TimerHook,
};

pub use completion::{Completion, CompletionHook, CompletionPosition};

use std::ffi::CString;

pub trait WeechatPlugin: Sized {
    fn init(weechat: &Weechat, args: ArgsWeechat) -> WeechatResult<Self>;
}

pub struct Error(pub c_int);
pub type WeechatResult<T> = Result<T, Error>;

/// Status values for Weechat callbacks
pub enum ReturnCode {
    Ok = weechat_sys::WEECHAT_RC_OK as isize,
    OkEat = weechat_sys::WEECHAT_RC_OK_EAT as isize,
    Error = weechat_sys::WEECHAT_RC_ERROR as isize,
}

pub(crate) struct LossyCString;

impl LossyCString {
    pub(crate) fn new<T: AsRef<str>>(t: T) -> CString {
        match CString::new(t.as_ref()) {
            Ok(cstr) => cstr,
            Err(_) => CString::new(t.as_ref().replace('\0', ""))
                .expect("string has no nulls"),
        }
    }
}

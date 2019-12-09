#![warn(missing_docs)]

use libc::c_int;

pub mod bar;
pub mod buffer;
pub mod completion;
pub mod config;
pub mod hooks;
pub mod weechat;

pub use weechat_macro::weechat_plugin;

pub use weechat::{ArgsWeechat, OptionChanged, Weechat};

pub use bar::{BarItem, LightBarItem};
pub use buffer::{Buffer, Nick, NickArgs};

pub use config::{
    BaseConfigOption, BooleanOpt, BooleanOption, BooleanOptionSettings,
    ColorOption, ConfigOption, IntegerOption,
};
pub use config::{Config, ConfigSection, ConfigSectionSettings};

pub use hooks::{
    CommandDescription, CommandHook, CommandRunHook, FdHook, FdHookMode,
    SignalHook, SignalHookValue, TimerHook,
};

pub use completion::{Completion, CompletionHook, CompletionPosition};

use std::ffi::CString;

pub trait WeechatPlugin: Sized {
    fn init(weechat: &Weechat, args: ArgsWeechat) -> WeechatResult<Self>;
}

pub struct Error(c_int);
pub type WeechatResult<T> = Result<T, Error>;

/// Status values for weechat callbacks
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

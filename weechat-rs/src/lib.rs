//! # `rust-weechat`
//!
//! rust-weechat implements high level bindings for the Weechat plugin API.
//!
//! The bindings make it possible to create powerful Weechat plugins using rust.
//!
//! ```no_run
//! use std::borrow::Cow;
//! use weechat::buffer::{Buffer, BufferSettings, NickSettings};
//! use weechat::hooks::{CommandSettings, Command};
//! use weechat::{weechat_plugin, ArgsWeechat, Weechat, WeechatPlugin};
//!
//! struct SamplePlugin {
//!     _command: Command,
//! }
//!
//! impl SamplePlugin {
//!     fn input_cb(
//!         _weechat: &Weechat,
//!         buffer: &Buffer,
//!         input: Cow<str>,
//!     ) -> Result<(), ()> {
//!         buffer.print(&input);
//!         Ok(())
//!     }
//!
//!     fn close_cb(_weechat: &Weechat, _buffer: &Buffer) -> Result<(), ()> {
//!         Weechat::print("Closing buffer");
//!         Ok(())
//!     }
//!
//!     fn rust_command_cb(_weechat: &Weechat, buffer: &Buffer, args: ArgsWeechat) {
//!        buffer.print("Hello world");
//!
//!        for arg in args {
//!            buffer.print(&arg)
//!        }
//!    }
//! }
//!
//! impl WeechatPlugin for SamplePlugin {
//!     fn init(weechat: &Weechat, _args: ArgsWeechat) -> Result<Self, ()> {
//!         Weechat::print("Hello Rust!");
//!
//!         let buffer_settings = BufferSettings::new("Test buffer")
//!             .input_callback(SamplePlugin::input_cb)
//!             .close_callback(SamplePlugin::close_cb);
//!
//!         let buffer_handle =
//!             Weechat::buffer_new(buffer_settings).expect("Can't create buffer");
//!
//!         let buffer = buffer_handle.upgrade().expect("Buffer already closed?");
//!
//!         let op_group = buffer
//!             .add_nicklist_group("operators", "blue", true, None)
//!             .expect("Can't create nick group");
//!         let emma = op_group
//!             .add_nick(
//!                 NickSettings::new("Emma")
//!                     .set_color("magenta")
//!                     .set_prefix("&")
//!                     .set_prefix_color("green"),
//!             )
//!             .expect("Can't add nick to group");
//!
//!         let sample_command = CommandSettings::new("rustcommand");
//!
//!         let command = weechat.hook_command(
//!             sample_command,
//!             SamplePlugin::rust_command_cb,
//!         );
//!
//!         Ok(SamplePlugin {
//!             _command: command,
//!         })
//!     }
//! }
//!
//! impl Drop for SamplePlugin {
//!     fn drop(&mut self) {
//!         Weechat::print("Bye rust");
//!     }
//! }
//! ```
//!
//! The above plugin implementation still needs to be registered as a Weechat
//! plugin:
//!
//! ```ignore
//! weechat_plugin!(
//!     SamplePlugin,
//!     name: "rust_sample",
//!     author: "poljar",
//!     description: "",
//!     version: "0.1.0",
//!     license: "MIT"
//! );
//! ```

#![warn(missing_docs)]

use std::ffi::CString;

#[cfg(feature = "async-executor")]
mod executor;
mod weechat;

pub mod buffer;
pub mod config;
pub mod hooks;
pub mod infolist;

pub use crate::weechat::{ArgsWeechat, Weechat};

pub use weechat_macro::weechat_plugin;

/// Weechat plugin trait.
///
/// Implement this trait over your struct to implement a Weechat plugin. The
/// init method will get called when Weechat loads the plugin, while the
/// Drop method will be called when Weechat unloads the plugin.
pub trait WeechatPlugin: Sized {
    /// Initialize the plugin.
    ///
    /// # Arguments
    ///
    /// * `weechat` - A borrow to a Weechat object that will be valid during the
    ///     duration of the init callback.
    ///
    /// * `args` - Arguments passed to the plugin when it is loaded.
    fn init(weechat: &Weechat, args: ArgsWeechat) -> Result<Self, ()>;
}

#[cfg(feature = "async-executor")]
pub use executor::JoinHandle;

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

use libc::{c_char, c_int};
use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;

use weechat_sys::{t_gui_buffer, t_weechat_plugin, WEECHAT_RC_OK};

use crate::buffer::Buffer;
use crate::{ArgsWeechat, LossyCString, ReturnCode, Weechat};

use super::Hook;

/// Hook for a weechat command, the command is removed when the object is
/// dropped.
pub struct CommandHook<T> {
    _hook: Hook,
    _hook_data: Box<CommandHookData<T>>,
}

#[derive(Default)]
/// Description for a weechat command that should will be hooked.
/// The fields of this struct accept the same string formats that are described
/// in the weechat API documentation.
pub struct CommandDescription<'a> {
    /// Name of the command.
    pub name: &'a str,
    /// Description for the command (displayed with `/help command`)
    pub description: &'a str,
    /// Arguments for the command (displayed with `/help command`)
    pub args: &'a str,
    /// Description for the command arguments (displayed with `/help command`)
    pub args_description: &'a str,
    /// Completion template for the command.
    pub completion: &'a str,
}

struct CommandHookData<T> {
    callback: fn(&T, Buffer, ArgsWeechat),
    callback_data: T,
    weechat_ptr: *mut t_weechat_plugin,
}

/// Hook for a weechat command, the hook is removed when the object is dropped.
pub struct CommandRunHook<T> {
    _hook: Hook,
    _hook_data: Box<CommandRunHookData<T>>,
}

struct CommandRunHookData<T> {
    callback: fn(&T, Buffer, Cow<str>) -> ReturnCode,
    callback_data: T,
    weechat_ptr: *mut t_weechat_plugin,
}

impl Weechat {
    /// Create a new weechat command.
    ///
    /// # Arguments
    ///
    /// * `command_info`
    ///
    /// Returns the hook of the command. The command is unhooked if the hook is
    /// dropped.
    pub fn hook_command<T>(
        &self,
        command_info: CommandDescription,
        callback: fn(data: &T, buffer: Buffer, args: ArgsWeechat),
        callback_data: Option<T>,
    ) -> CommandHook<T>
    where
        T: Default,
    {
        unsafe extern "C" fn c_hook_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
            argc: i32,
            argv: *mut *mut c_char,
            _argv_eol: *mut *mut c_char,
        ) -> c_int {
            let hook_data: &mut CommandHookData<T> =
                { &mut *(pointer as *mut CommandHookData<T>) };
            let weechat = Weechat::from_ptr(hook_data.weechat_ptr);
            let buffer = weechat.buffer_from_ptr(buffer);
            let callback = hook_data.callback;
            let callback_data = &hook_data.callback_data;
            let args = ArgsWeechat::new(argc, argv);

            callback(callback_data, buffer, args);

            WEECHAT_RC_OK
        }

        let name = LossyCString::new(command_info.name);
        let description = LossyCString::new(command_info.description);
        let args = LossyCString::new(command_info.args);
        let args_description = LossyCString::new(command_info.args_description);
        let completion = LossyCString::new(command_info.completion);

        let data = Box::new(CommandHookData {
            callback,
            callback_data: callback_data.unwrap_or_default(),
            weechat_ptr: self.ptr,
        });

        let data_ref = Box::leak(data);

        let hook_command = self.get().hook_command.unwrap();
        let hook_ptr = unsafe {
            hook_command(
                self.ptr,
                name.as_ptr(),
                description.as_ptr(),
                args.as_ptr(),
                args_description.as_ptr(),
                completion.as_ptr(),
                Some(c_hook_cb::<T>),
                data_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };
        let hook_data = unsafe { Box::from_raw(data_ref) };
        let hook = Hook {
            ptr: hook_ptr,
            weechat_ptr: self.ptr,
        };

        CommandHook::<T> {
            _hook: hook,
            _hook_data: hook_data,
        }
    }

    /// Override an existing Weechat command.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to hook (wildcard `*` is allowed).
    ///
    /// * `callback` - A function that will be called when the command is run.
    ///
    /// * `callback_data` - Data that will be passed to the callback every time
    ///     the callback runs. This data will be freed when the hook is unhooked.
    pub fn hook_command_run<T>(
        &self,
        command: &str,
        callback: fn(data: &T, buffer: Buffer, command: Cow<str>) -> ReturnCode,
        callback_data: Option<T>,
    ) -> CommandRunHook<T>
    where
        T: Default,
    {
        unsafe extern "C" fn c_hook_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
            command: *const std::os::raw::c_char,
        ) -> c_int {
            let hook_data: &mut CommandRunHookData<T> =
                { &mut *(pointer as *mut CommandRunHookData<T>) };
            let callback = hook_data.callback;
            let callback_data = &hook_data.callback_data;

            let weechat = Weechat::from_ptr(hook_data.weechat_ptr);
            let buffer = weechat.buffer_from_ptr(buffer);
            let command = CStr::from_ptr(command).to_string_lossy();

            callback(callback_data, buffer, command) as isize as i32
        }

        let data = Box::new(CommandRunHookData {
            callback,
            callback_data: callback_data.unwrap_or_default(),
            weechat_ptr: self.ptr,
        });

        let data_ref = Box::leak(data);
        let hook_timer = self.get().hook_command_run.unwrap();

        let command = LossyCString::new(command);

        let hook_ptr = unsafe {
            hook_timer(
                self.ptr,
                command.as_ptr(),
                Some(c_hook_cb::<T>),
                data_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };
        let hook_data = unsafe { Box::from_raw(data_ref) };
        let hook = Hook {
            ptr: hook_ptr,
            weechat_ptr: self.ptr,
        };

        CommandRunHook::<T> {
            _hook: hook,
            _hook_data: hook_data,
        }
    }
}

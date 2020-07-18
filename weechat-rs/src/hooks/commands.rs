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
pub struct Command {
    _hook: Hook,
    _hook_data: Box<CommandHookData>,
}

/// Trait for the command callback
///
/// A blanket implementation for pure `FnMut` functions exists, if data needs to
/// be passed to the callback implement this over your struct.
pub trait CommandCallback {
    /// Callback that will be called when the command is executed.
    ///
    /// # Arguments
    ///
    /// * `weechat` - A Weechat context.
    ///
    /// * `buffer` - The buffer that received the command.
    ///
    /// * `arguments` - The arguments that were passed to the command, this will
    ///     include the command as the first argument.
    fn callback(
        &mut self,
        weechat: &Weechat,
        buffer: &Buffer,
        arguments: ArgsWeechat,
    );
}

impl<T: FnMut(&Weechat, &Buffer, ArgsWeechat) + 'static> CommandCallback for T {
    fn callback(
        &mut self,
        weechat: &Weechat,
        buffer: &Buffer,
        arguments: ArgsWeechat,
    ) {
        self(weechat, buffer, arguments)
    }
}

#[derive(Default)]
/// Description for a weechat command that should will be hooked.
/// The fields of this struct accept the same string formats that are described
/// in the weechat API documentation.
pub struct CommandSettings {
    /// Name of the command.
    name: String,
    /// Description for the command (displayed with `/help command`)
    description: String,
    /// Arguments for the command (displayed with `/help command`)
    arguments: Vec<String>,
    /// Description for the command arguments (displayed with `/help command`)
    argument_descriptoin: String,
    /// Completion template for the command.
    completion: Vec<String>,
}

impl CommandSettings {
    /// Create new command settings.
    ///
    /// This describes how a command will be created.
    ///
    /// #Arguments
    ///
    /// * `name` - The name that the section should get.
    pub fn new<P: Into<String>>(name: P) -> Self {
        CommandSettings {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the description of the command.
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the command.
    pub fn description<D: Into<String>>(mut self, descritpion: D) -> Self {
        self.description = descritpion.into();
        self
    }

    /// Set the
    ///
    /// # Arguments
    ///
    /// * `description` - The description of the command.
    pub fn add_argument<T: Into<String>>(mut self, argument: T) -> Self {
        self.arguments.push(argument.into());
        self
    }

    pub fn arguments_description<T: Into<String>>(
        mut self,
        descritpion: T,
    ) -> Self {
        self.argument_descriptoin = descritpion.into();
        self
    }

    pub fn add_completion<T: Into<String>>(mut self, completion: T) -> Self {
        self.completion.push(completion.into());
        self
    }
}

struct CommandHookData {
    callback: Box<dyn CommandCallback>,
    weechat_ptr: *mut t_weechat_plugin,
}

/// Hook for a weechat command, the hook is removed when the object is dropped.
pub struct CommandRun {
    _hook: Hook,
    _hook_data: Box<CommandRunHookData>,
}

pub trait CommandRunCallback {
    fn callback(
        &mut self,
        weechat: &Weechat,
        buffer: &Buffer,
        command: Cow<str>,
    ) -> ReturnCode;
}

impl<T: FnMut(&Weechat, &Buffer, Cow<str>) -> ReturnCode + 'static>
    CommandRunCallback for T
{
    fn callback(
        &mut self,
        weechat: &Weechat,
        buffer: &Buffer,
        command: Cow<str>,
    ) -> ReturnCode {
        self(weechat, buffer, command)
    }
}

struct CommandRunHookData {
    callback: Box<dyn CommandRunCallback>,
    weechat_ptr: *mut t_weechat_plugin,
}

impl Weechat {
    /// Create a new Weechat command.
    ///
    /// Returns the hook of the command. The command is unhooked if the hook is
    /// dropped.
    ///
    /// # Arguments
    ///
    /// * `command_settings` - Settings for the new command.
    ///
    /// * `callback` - The callback that will be called if the command is run.
    pub fn hook_command(
        &self,
        command_settings: CommandSettings,
        callback: impl CommandCallback + 'static,
    ) -> Command {
        unsafe extern "C" fn c_hook_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
            argc: i32,
            argv: *mut *mut c_char,
            _argv_eol: *mut *mut c_char,
        ) -> c_int {
            let hook_data: &mut CommandHookData =
                { &mut *(pointer as *mut CommandHookData) };
            let weechat = Weechat::from_ptr(hook_data.weechat_ptr);
            let buffer = weechat.buffer_from_ptr(buffer);
            let cb = &mut hook_data.callback;
            let args = ArgsWeechat::new(argc, argv);

            cb.callback(&weechat, &buffer, args);

            WEECHAT_RC_OK
        }

        let name = LossyCString::new(command_settings.name);
        let description = LossyCString::new(command_settings.description);
        let args = LossyCString::new(command_settings.arguments.join("||"));
        let args_description =
            LossyCString::new(command_settings.argument_descriptoin);
        let completion =
            LossyCString::new(command_settings.completion.join("||"));

        let data = Box::new(CommandHookData {
            callback: Box::new(callback),
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
                Some(c_hook_cb),
                data_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };
        let hook_data = unsafe { Box::from_raw(data_ref) };
        let hook = Hook {
            ptr: hook_ptr,
            weechat_ptr: self.ptr,
        };

        Command {
            _hook: hook,
            _hook_data: hook_data,
        }
    }

    /// Override an existing Weechat command.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to override (wildcard `*` is allowed).
    ///
    /// * `callback` - The function that will be called when the command is run.
    pub fn hook_command_run(
        &self,
        command: &str,
        callback: impl CommandRunCallback + 'static,
    ) -> CommandRun {
        unsafe extern "C" fn c_hook_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
            command: *const std::os::raw::c_char,
        ) -> c_int {
            let hook_data: &mut CommandRunHookData =
                { &mut *(pointer as *mut CommandRunHookData) };
            let cb = &mut hook_data.callback;

            let weechat = Weechat::from_ptr(hook_data.weechat_ptr);
            let buffer = weechat.buffer_from_ptr(buffer);
            let command = CStr::from_ptr(command).to_string_lossy();

            cb.callback(&weechat, &buffer, command) as isize as i32
        }

        let data = Box::new(CommandRunHookData {
            callback: Box::new(callback),
            weechat_ptr: self.ptr,
        });

        let data_ref = Box::leak(data);
        let hook_command_run = self.get().hook_command_run.unwrap();

        let command = LossyCString::new(command);

        let hook_ptr = unsafe {
            hook_command_run(
                self.ptr,
                command.as_ptr(),
                Some(c_hook_cb),
                data_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };
        let hook_data = unsafe { Box::from_raw(data_ref) };
        let hook = Hook {
            ptr: hook_ptr,
            weechat_ptr: self.ptr,
        };

        CommandRun {
            _hook: hook,
            _hook_data: hook_data,
        }
    }
}

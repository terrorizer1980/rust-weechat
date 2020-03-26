use libc::{c_char, c_int};
use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;

use weechat_sys::{t_gui_buffer, t_gui_completion, t_weechat_plugin};

use crate::buffer::Buffer;
use crate::hooks::Hook;
use crate::{LossyCString, ReturnCode, Weechat};

/// A handle to a completion item.
pub struct Completion {
    weechat_ptr: *mut t_weechat_plugin,
    ptr: *mut t_gui_completion,
}

/// The positions an entry can be added to a completion list.
#[derive(Clone, Copy)]
pub enum CompletionPosition {
    /// Insert the item in a way that keeps the list sorted.
    Sorted,
    /// Insert the item at the beginning of the list.
    Beginning,
    /// Insert the item at the end of the list.
    End,
}

impl CompletionPosition {
    pub(crate) fn value(&self) -> &str {
        match self {
            CompletionPosition::Sorted => "sort",
            CompletionPosition::Beginning => "beginning",
            CompletionPosition::End => "end",
        }
    }
}

impl Completion {
    pub(crate) fn from_raw(
        weechat: *mut t_weechat_plugin,
        completion: *mut t_gui_completion,
    ) -> Completion {
        Completion {
            weechat_ptr: weechat,
            ptr: completion,
        }
    }

    /// Add a word for completion, keeping the list sorted.
    pub fn add(&self, word: &str) {
        self.add_with_options(word, false, CompletionPosition::Sorted)
    }

    /// Add a word for completion in a specific position specific if the word is a nick name
    pub fn add_with_options(
        &self,
        word: &str,
        is_nick: bool,
        position: CompletionPosition,
    ) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let hook_completion_list_add =
            weechat.get().hook_completion_list_add.unwrap();

        let word = LossyCString::new(word);
        let method = LossyCString::new(position.value());

        unsafe {
            hook_completion_list_add(
                self.ptr,
                word.as_ptr(),
                is_nick as i32,
                method.as_ptr(),
            );
        }
    }
}

/// Hook for a completion item, the hook is removed when the object is dropped.
pub struct CompletionHook {
    _hook: Hook,
    _hook_data: Box<CompletionHookData>,
}

struct CompletionHookData {
    #[allow(clippy::type_complexity)]
    callback: Box<dyn FnMut(&Weechat, &Buffer, Cow<str>, Completion) -> ReturnCode + 'static>,
    weechat_ptr: *mut t_weechat_plugin,
}

impl Weechat {
    /// Hook a completion.
    ///
    /// * `completion_item` - The name of the completion item
    /// * `description` - The description of the completion item
    /// * `callback` - A function that will be called when the completion is used, the callback must
    ///     populate the words for the completion
    /// * `callback_data` - Data that will be passed to the callback every time
    ///     the callback runs. This data will be freed when the hook is unhooked.
    pub fn hook_completion<T>(
        &self,
        completion_item: &str,
        description: &str,
        callback: impl FnMut(
            &Weechat,
            &Buffer,
            Cow<str>,
            Completion,
        ) -> ReturnCode + 'static,
    ) -> CompletionHook
    {
        unsafe extern "C" fn c_hook_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            completion_item: *const c_char,
            buffer: *mut t_gui_buffer,
            completion: *mut t_gui_completion,
        ) -> c_int {
            let hook_data: &mut CompletionHookData =
                { &mut *(pointer as *mut CompletionHookData) };
            let callback = &mut hook_data.callback;
            let weechat = Weechat::from_ptr(hook_data.weechat_ptr);
            let buffer = weechat.buffer_from_ptr(buffer);

            let completion_item =
                CStr::from_ptr(completion_item).to_string_lossy();

            callback(
                &weechat,
                &buffer,
                completion_item,
                Completion::from_raw(hook_data.weechat_ptr, completion),
            ) as i32
        }

        let data = Box::new(CompletionHookData {
            callback: Box::new(callback),
            weechat_ptr: self.ptr,
        });

        let data_ref = Box::leak(data);
        let hook_completion = self.get().hook_completion.unwrap();

        let completion_item = LossyCString::new(completion_item);
        let description = LossyCString::new(description);

        let hook_ptr = unsafe {
            hook_completion(
                self.ptr,
                completion_item.as_ptr(),
                description.as_ptr(),
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

        CompletionHook {
            _hook: hook,
            _hook_data: hook_data,
        }
    }
}

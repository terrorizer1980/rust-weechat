use libc::{c_char, c_int};
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;

use weechat_sys::{t_weechat_plugin, WEECHAT_RC_OK};

use super::Hook;
use crate::{LossyCString, ReturnCode, Weechat};

/// Hook for a signal, the hook is removed when the object is dropped.
pub struct SignalHook<T> {
    _hook: Hook,
    _hook_data: Box<SignalHookData<T>>,
}

struct SignalHookData<T> {
    callback: fn(&T, &Weechat, SignalHookValue) -> ReturnCode,
    callback_data: T,
    weechat_ptr: *mut t_weechat_plugin,
}

/// The type of data returned by a signal
#[derive(Debug)]
pub enum SignalHookValue {
    /// String data
    String(String),
    /// Integer data
    Integer(i32),
    /// Pointer data
    Pointer(*mut c_void),
}

impl SignalHookValue {
    pub(crate) fn from_raw_with_type(
        data_type: &str,
        data: *mut c_void,
    ) -> Option<SignalHookValue> {
        match data_type {
            "string" => unsafe {
                Some(SignalHookValue::String(
                    CStr::from_ptr(data as *const c_char)
                        .to_string_lossy()
                        .into_owned(),
                ))
            },
            "integer" => {
                let data = data as *const c_int;
                if data.is_null() {
                    None
                } else {
                    unsafe { Some(SignalHookValue::Integer(*(data))) }
                }
            }
            "pointer" => Some(SignalHookValue::Pointer(data)),
            _ => None,
        }
    }
}

pub trait SignalCallback {
    fn callback() {
    }
}

impl Weechat {
    /// Hook a signal.
    ///
    /// # Arguments
    ///
    /// * `signal_name` - The signal to hook (wildcard `*` is allowed).
    ///
    /// * `callback` - A function that will be called when the signal is received.
    ///
    /// * `callback_data` - Data that will be passed to the callback every time
    ///     the callback runs. This data will be freed when the hook is unhooked.
    pub fn hook_signal<T>(
        &self,
        signal_name: &str,
        callback: fn(
            data: &T,
            weechat: &Weechat,
            signal_value: SignalHookValue,
        ) -> ReturnCode,
        callback_data: Option<T>,
    ) -> SignalHook<T>
    where
        T: Default,
    {
        unsafe extern "C" fn c_hook_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            _signal: *const c_char,
            data_type: *const c_char,
            signal_data: *mut c_void,
        ) -> c_int {
            let hook_data: &mut SignalHookData<T> =
                { &mut *(pointer as *mut SignalHookData<T>) };
            let callback = hook_data.callback;
            let callback_data = &hook_data.callback_data;

            // this cannot contain invalid utf
            let data_type =
                CStr::from_ptr(data_type).to_str().unwrap_or_default();
            if let Some(value) =
                SignalHookValue::from_raw_with_type(data_type, signal_data)
            {
                callback(
                    callback_data,
                    &Weechat::from_ptr(hook_data.weechat_ptr),
                    value,
                ) as i32
            } else {
                WEECHAT_RC_OK
            }
        }

        let data = Box::new(SignalHookData {
            callback,
            callback_data: callback_data.unwrap_or_default(),
            weechat_ptr: self.ptr,
        });

        let data_ref = Box::leak(data);
        let hook_signal = self.get().hook_signal.unwrap();

        let signal_name = LossyCString::new(signal_name);

        let hook_ptr = unsafe {
            hook_signal(
                self.ptr,
                signal_name.as_ptr(),
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

        SignalHook::<T> {
            _hook: hook,
            _hook_data: hook_data,
        }
    }
}

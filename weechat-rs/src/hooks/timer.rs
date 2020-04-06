use libc::c_int;
use std::os::raw::c_void;
use std::time::Duration;
use std::ptr;

use weechat_sys::{t_weechat_plugin, WEECHAT_RC_OK};

use crate::Weechat;
use super::Hook;

/// A hook for a timer, the hook will be removed when the object is dropped.
pub struct TimerHook<T> {
    _hook: Hook,
    _hook_data: Box<TimerHookData<T>>,
}

struct TimerHookData<T> {
    callback: fn(&T, &Weechat, i32),
    callback_data: T,
    weechat_ptr: *mut t_weechat_plugin,
}

impl Weechat {
    /// Create a timer that will repeatedly fire.
    ///
    /// # Arguments
    ///
    /// * `interval` - The delay between calls in milliseconds.
    ///
    /// * `align_second` - The alignment on a second. For example, if the
    ///     current time is 09:00, if the interval = 60000 (60 seconds), and
    ///     align_second = 60, then timer is called each minute on the 0th
    ///     second.
    ///
    /// * `max_calls` - The number of times the callback should be called, 0
    ///     means it's called forever.
    ///
    /// * `callback` - A function that will be called when the timer fires, the
    ///     `remaining` argument will be -1 if the timer has no end.
    ///
    /// * `callback_data` - Data that will be passed to the callback every time
    ///     the callback runs. This data will be freed when the hook is
    ///     unhooked.
    pub fn hook_timer<T>(
        &self,
        interval: Duration,
        align_second: i32,
        max_calls: i32,
        callback: fn(data: &T, weechat: &Weechat, remaining: i32),
        callback_data: Option<T>,
    ) -> TimerHook<T>
    where
        T: Default,
    {
        unsafe extern "C" fn c_hook_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            remaining: i32,
        ) -> c_int {
            let hook_data: &mut TimerHookData<T> =
                { &mut *(pointer as *mut TimerHookData<T>) };
            let callback = &hook_data.callback;
            let callback_data = &hook_data.callback_data;

            callback(
                callback_data,
                &Weechat::from_ptr(hook_data.weechat_ptr),
                remaining,
            );

            WEECHAT_RC_OK
        }

        let data = Box::new(TimerHookData::<T> {
            callback,
            callback_data: callback_data.unwrap_or_default(),
            weechat_ptr: self.ptr,
        });

        let data_ref = Box::leak(data);
        let hook_timer = self.get().hook_timer.unwrap();

        let hook_ptr = unsafe {
            hook_timer(
                self.ptr,
                interval.as_millis() as i64,
                align_second,
                max_calls,
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

        TimerHook {
            _hook: hook,
            _hook_data: hook_data,
        }
    }
}

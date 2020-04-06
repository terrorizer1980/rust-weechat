use libc::c_int;
use std::os::raw::c_void;
use std::ptr;
use std::os::unix::io::AsRawFd;

use weechat_sys::WEECHAT_RC_OK;

use crate::Weechat;
use super::Hook;

/// Setting for the FdHook.
pub enum FdHookMode {
    /// Catch read events.
    Read,
    /// Catch write events.
    Write,
    /// Catch read and write events.
    ReadWrite,
}

impl FdHookMode {
    pub(crate) fn as_tuple(&self) -> (i32, i32) {
        let read = match self {
            FdHookMode::Read => 1,
            FdHookMode::ReadWrite => 1,
            FdHookMode::Write => 0,
        };

        let write = match self {
            FdHookMode::Read => 0,
            FdHookMode::ReadWrite => 1,
            FdHookMode::Write => 1,
        };
        (read, write)
    }
}

/// Hook for a file descriptor, the hook is removed when the object is dropped.
pub struct FdHook<T, F> {
    _hook: Hook,
    _hook_data: Box<FdHookData<T, F>>,
}

struct FdHookData<T, F> {
    callback: fn(&T, fd_object: &mut F),
    callback_data: T,
    fd_object: F,
}

impl Weechat {
    /// Hook an object that can be turned into a raw file descriptor.
    /// Returns the hook object.
    ///
    /// # Arguments
    ///
    /// * `fd_object` - An object for wich the file descriptor will be watched
    ///     and the callback called when read or write operations can happen
    ///     on it.
    ///
    /// * `mode` - Configure the hook to watch for writes, reads or both on the
    ///     file descriptor.
    ///
    /// * `callback` - A function that will be called if a watched event on the
    ///     file descriptor happends.
    ///
    /// * `callback_data` - Data that will be passed to the callback every time
    ///     the callback runs. This data will be freed when the hook is
    ///     unhooked.
    pub fn hook_fd<T, F>(
        &self,
        fd_object: F,
        mode: FdHookMode,
        callback: fn(data: &T, fd_object: &mut F),
        callback_data: Option<T>,
    ) -> Result<FdHook<T, F>, ()>
    where
        T: Default,
        F: AsRawFd,
    {
        unsafe extern "C" fn c_hook_cb<T, F>(
            pointer: *const c_void,
            _data: *mut c_void,
            _fd: i32,
        ) -> c_int {
            let hook_data: &mut FdHookData<T, F> =
                { &mut *(pointer as *mut FdHookData<T, F>) };
            let callback = hook_data.callback;
            let callback_data = &hook_data.callback_data;
            let fd_object = &mut hook_data.fd_object;

            callback(callback_data, fd_object);

            WEECHAT_RC_OK
        }

        let fd = fd_object.as_raw_fd();

        let data = Box::new(FdHookData {
            callback,
            callback_data: callback_data.unwrap_or_default(),
            fd_object,
        });

        let data_ref = Box::leak(data);
        let hook_fd = self.get().hook_fd.unwrap();
        let (read, write) = mode.as_tuple();

        let hook_ptr = unsafe {
            hook_fd(
                self.ptr,
                fd,
                read,
                write,
                0,
                Some(c_hook_cb::<T, F>),
                data_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };

        if hook_ptr.is_null() {
            unsafe { Box::from_raw(data_ref) };
            return Err(());
        };

        let hook_data = unsafe { Box::from_raw(data_ref) };
        let hook = Hook {
            ptr: hook_ptr,
            weechat_ptr: self.ptr,
        };

        Ok(FdHook::<T, F> {
            _hook: hook,
            _hook_data: hook_data,
        })
    }
}

//! Bar items are used to display status information in Weechat.
use core::ptr;
use libc::c_char;
use std::os::raw::c_void;
use weechat_sys::{
    t_gui_bar_item, t_gui_buffer, t_gui_window, t_hashtable, t_weechat_plugin,
};

use crate::buffer::Buffer;
use crate::{LossyCString, Weechat};

struct BarItemCbData {
    callback: Box<dyn FnMut(&Weechat, &Buffer) -> String>,
    weechat_ptr: *mut t_weechat_plugin,
}

/// A handle to a bar item. The bar item is automatically removed when the object is
/// dropped.
pub struct BarItemHandle {
    name: String,
    ptr: *mut t_gui_bar_item,
    weechat: *mut t_weechat_plugin,
    _data: Box<BarItemCbData>,
}

impl Drop for BarItemHandle {
    fn drop(&mut self) {
        let weechat = Weechat::from_ptr(self.weechat);
        let bar_item_remove = weechat.get().bar_item_remove.unwrap();
        unsafe { bar_item_remove(self.ptr) };
    }
}

impl BarItemHandle {
    /// Update the content of the bar item, by calling its build callback.
    pub fn update(&self) {
        Weechat::bar_item_update(&self.name);
    }
}

impl Weechat {
    /// Create a new bar item that can be added by a user.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the new bar item.
    ///
    /// * `callback` - The callback that should be called after the bar items
    /// is marked to be updated.
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    ///
    /// # Example
    /// ```
    /// let item = Weechat::new_bar_item("buffer_plugin", |_, _| {
    ///     "rust/sample".to_owned()
    /// });
    /// ```
    ///
    // TODO: Provide window object, the callback should accept a Window object wrapping a t_gui_window
    // TODO: If we're going to allow bar items to be searched for like we do for
    // buffers, we need to do something about the multiple ownership that may
    // come from this.
    pub fn new_bar_item(
        name: &str,
        callback: impl FnMut(&Weechat, &Buffer) -> String + 'static,
    ) -> Result<BarItemHandle, ()> {
        unsafe extern "C" fn c_item_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            _bar_item: *mut t_gui_bar_item,
            _window: *mut t_gui_window,
            buffer: *mut t_gui_buffer,
            _extra_info: *mut t_hashtable,
        ) -> *mut c_char {
            let data: &mut BarItemCbData =
                { &mut *(pointer as *mut BarItemCbData) };
            let weechat = Weechat::from_ptr(data.weechat_ptr);
            let buffer = weechat.buffer_from_ptr(buffer);

            let callback = &mut data.callback;

            let ret = callback(&weechat, &buffer);

            // Weechat wants a malloc'ed string
            libc::strdup(LossyCString::new(ret).as_ptr())
        }
        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };

        let data = Box::new(BarItemCbData {
            callback: Box::new(callback),
            weechat_ptr: weechat.ptr,
        });

        let data_ref = Box::leak(data);
        let bar_item_new = weechat.get().bar_item_new.unwrap();

        let bar_item_name = LossyCString::new(name);

        let bar_item_ptr = unsafe {
            bar_item_new(
                weechat.ptr,
                bar_item_name.as_ptr(),
                Some(c_item_cb),
                data_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };

        let cb_data = unsafe { Box::from_raw(data_ref) };

        if bar_item_ptr.is_null() {
            return Err(());
        }

        Ok(BarItemHandle {
            name: name.to_owned(),
            ptr: bar_item_ptr,
            weechat: weechat.ptr,
            _data: cb_data,
        })
    }

    /// Update the content of a bar item, by calling its build callback.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the bar item that should be updated.
    pub fn bar_item_update(name: &str) {
        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };

        let bar_item_update = weechat.get().bar_item_update.unwrap();

        let name = LossyCString::new(name);

        unsafe { bar_item_update(name.as_ptr()) }
    }
}

//! Weechat Buffer module containing Buffer and Nick types.
use std::borrow::Cow;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;

#[cfg(feature = "async-executor")]
use futures::future::{Future, FutureExt, LocalBoxFuture};

use crate::{LossyCString, Weechat};
use libc::{c_char, c_int};
use weechat_sys::{
    t_gui_buffer, t_gui_nick, t_gui_nick_group, t_weechat_plugin,
    WEECHAT_RC_ERROR, WEECHAT_RC_OK,
};

/// A high level Buffer type encapsulating weechats C buffer pointer.
/// The buffer won't be closed if the object is destroyed.
#[derive(Eq)]
pub struct Buffer<'a> {
    pub(crate) weechat: *mut t_weechat_plugin,
    pub(crate) ptr: *mut t_gui_buffer,
    weechat_phantom: PhantomData<&'a Weechat>,
}

impl PartialEq for Buffer<'_> {
    fn eq(&self, other: &Buffer) -> bool {
        self.ptr == other.ptr
    }
}

#[cfg(feature = "async-executor")]
pub(crate) struct BufferPointers<T: Clone> {
    pub(crate) weechat: *mut t_weechat_plugin,
    pub(crate) input_cb: Option<BufferInputCallback<T>>,
    pub(crate) close_cb: Option<BufferCloseCallback>,
    pub(crate) input_data: Option<T>,
}

#[cfg(not(feature = "async-executor"))]
pub(crate) struct BufferPointers {
    pub(crate) weechat: *mut t_weechat_plugin,
    pub(crate) input_cb: Option<BufferInputCallback>,
    pub(crate) close_cb: Option<BufferCloseCallback>,
}

#[cfg(not(feature = "async-executor"))]
pub type BufferInputCallback =
    Box<dyn FnMut(&Weechat, &Buffer, Cow<str>) -> Result<(), ()>>;

#[cfg(feature = "async-executor")]
pub type BufferInputCallback<T> =
    Box<dyn FnMut(Option<T>, String) -> LocalBoxFuture<'static, ()>>;

pub type BufferCloseCallback =
    Box<dyn FnMut(&Weechat, &Buffer) -> Result<(), ()>>;

#[cfg(feature = "async-executor")]
pub struct BufferSettings<T: Clone> {
    pub(crate) name: String,
    pub(crate) input_callback: Option<BufferInputCallback<T>>,
    pub(crate) input_data: Option<T>,
    pub(crate) close_callback: Option<BufferCloseCallback>,
}

#[cfg(not(feature = "async-executor"))]
pub struct BufferSettings {
    pub(crate) name: String,
    pub(crate) input_callback: Option<BufferInputCallback>,
    pub(crate) close_callback: Option<BufferCloseCallback>,
}

#[cfg(feature = "async-executor")]
impl<T: Clone> BufferSettings<T> {
    pub fn new(name: &str) -> Self {
        BufferSettings {
            name: name.to_owned(),
            input_callback: None,
            input_data: None,
            close_callback: None,
        }
    }

    pub fn input_callback<C: 'static>(
        mut self,
        mut callback: impl FnMut(Option<T>, String) -> C + 'static,
    ) -> Self
    where
        C: Future<Output = ()>,
    {
        let future = move |data, input| callback(data, input).boxed_local();
        self.input_callback = Some(Box::new(future));
        self
    }

    pub fn input_data(mut self, data: T) -> Self {
        self.input_data = Some(data);
        self
    }

    pub fn close_callback(
        mut self,
        callback: impl FnMut(&Weechat, &Buffer) -> Result<(), ()> + 'static,
    ) -> Self {
        self.close_callback = Some(Box::new(callback));
        self
    }
}

#[cfg(not(feature = "async-executor"))]
impl BufferSettings {
    pub fn new(name: &str) -> Self {
        BufferSettings {
            name: name.to_owned(),
            input_callback: None,
            close_callback: None,
        }
    }

    pub fn input_callback(
        mut self,
        callback: impl FnMut(&Weechat, &Buffer, Cow<str>) -> Result<(), ()>
            + 'static,
    ) -> Self {
        self.input_callback = Some(Box::new(callback));
        self
    }

    pub fn close_callback(
        mut self,
        callback: impl FnMut(&Weechat, &Buffer) -> Result<(), ()> + 'static,
    ) -> Self {
        self.close_callback = Some(Box::new(callback));
        self
    }
}

impl Weechat {
    /// Search a buffer by plugin and/or name.
    /// * `plugin_name` - name of a plugin, the following special value is
    ///     allowed: "==", the buffer name used is the buffers full name.
    /// * `buffer_name` - name of a buffer, if this is an empty string,
    ///     the current buffer is returned (buffer displayed by current
    ///     window); if the name starts with (?i), the search is case
    ///     insensitive.
    /// Returns a Buffer if one is found, otherwise None.
    pub fn buffer_search(
        &self,
        plugin_name: &str,
        buffer_name: &str,
    ) -> Option<Buffer> {
        let buffer_search = self.get().buffer_search.unwrap();

        let plugin_name = LossyCString::new(plugin_name);
        let buffer_name = LossyCString::new(buffer_name);

        let buf_ptr = unsafe {
            buffer_search(plugin_name.as_ptr(), buffer_name.as_ptr())
        };
        if buf_ptr.is_null() {
            None
        } else {
            Some(self.buffer_from_ptr(buf_ptr))
        }
    }

    pub(crate) fn buffer_from_ptr(
        &self,
        buffer_ptr: *mut t_gui_buffer,
    ) -> Buffer {
        Buffer {
            weechat: self.ptr,
            ptr: buffer_ptr,
            weechat_phantom: PhantomData,
        }
    }

    /// Get the currently open buffer
    pub fn current(&self) -> Option<Buffer> {
        let buffer_search = self.get().buffer_search.unwrap();

        let buf_ptr =
            unsafe { buffer_search(ptr::null_mut(), ptr::null_mut()) };
        if buf_ptr.is_null() {
            None
        } else {
            Some(self.buffer_from_ptr(buf_ptr))
        }
    }

    /// Create a new Weechat buffer
    ///
    /// * `settings` - Settings for the new buffer.
    ///
    /// Returns a Buffer if one has been created, otherwise an empty Error.
    #[cfg(feature = "async-executor")]
    pub fn buffer_new<T: Clone>(
        &self,
        settings: BufferSettings<T>,
    ) -> Result<Buffer, ()> {
        unsafe extern "C" fn c_input_cb<T: Clone>(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
            input_data: *const c_char,
        ) -> c_int {
            let input_data = CStr::from_ptr(input_data).to_string_lossy();

            let pointers: &mut BufferPointers<T> =
                { &mut *(pointer as *mut BufferPointers<T>) };

            let weechat = Weechat::from_ptr(pointers.weechat);
            let buffer = weechat.buffer_from_ptr(buffer);
            let data = pointers.input_data.clone();

            if let Some(callback) = pointers.input_cb.as_mut() {
                let future = callback(data, input_data.to_string());
                Weechat::spawn_buffer_cb(&buffer.get_full_name(), future);
            }

            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_close_cb<T: Clone>(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
        ) -> c_int {
            // We use from_raw() here so that the box get's freed at the end
            // of this scope.
            let pointers = Box::from_raw(pointer as *mut BufferPointers<T>);
            let weechat = Weechat::from_ptr(pointers.weechat);
            let buffer = weechat.buffer_from_ptr(buffer);

            let ret = if let Some(mut callback) = pointers.close_cb {
                callback(&weechat, &buffer).is_ok()
            } else {
                true
            };

            if ret {
                WEECHAT_RC_OK
            } else {
                WEECHAT_RC_ERROR
            }
        }

        let c_input_cb: Option<WeechatInputCbT> = match settings.input_callback
        {
            Some(_) => Some(c_input_cb::<T>),
            None => None,
        };

        // We create a box and use leak to stop rust from freeing our data,
        // we are giving weechat ownership over the data and will free it in
        // the buffer close callback.
        let buffer_pointers = Box::new(BufferPointers::<T> {
            weechat: self.ptr,
            input_cb: settings.input_callback,
            input_data: settings.input_data,
            close_cb: settings.close_callback,
        });
        let buffer_pointers_ref: &BufferPointers<T> =
            Box::leak(buffer_pointers);

        let buf_new = self.get().buffer_new.unwrap();
        let c_name = LossyCString::new(settings.name);

        // TODO this can fail, return a Option type
        let buf_ptr = unsafe {
            buf_new(
                self.ptr,
                c_name.as_ptr(),
                c_input_cb,
                buffer_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
                Some(c_close_cb::<T>),
                buffer_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };

        if buf_ptr.is_null() {
            return Err(());
        }

        Ok(Buffer {
            weechat: self.ptr,
            ptr: buf_ptr,
            weechat_phantom: PhantomData,
        })
    }

    /// Create a new Weechat buffer
    ///
    /// * `settings` - Settings for the new buffer.
    ///
    /// Returns a Buffer if one has been created, otherwise an empty Error.
    #[cfg(not(feature = "async-executor"))]
    pub fn buffer_new(&self, settings: BufferSettings) -> Result<Buffer, ()> {
        unsafe extern "C" fn c_input_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
            input_data: *const c_char,
        ) -> c_int {
            let input_data = CStr::from_ptr(input_data).to_string_lossy();

            let pointers: &mut BufferPointers =
                { &mut *(pointer as *mut BufferPointers) };

            let weechat = Weechat::from_ptr(pointers.weechat);
            let buffer = weechat.buffer_from_ptr(buffer);

            let ret = if let Some(callback) = pointers.input_cb.as_mut() {
                callback(&weechat, &buffer, input_data).is_ok()
            } else {
                true
            };

            if ret {
                WEECHAT_RC_OK
            } else {
                WEECHAT_RC_ERROR
            }
        }

        unsafe extern "C" fn c_close_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
        ) -> c_int {
            // We use from_raw() here so that the box get's freed at the end
            // of this scope.
            let pointers = Box::from_raw(pointer as *mut BufferPointers);
            let weechat = Weechat::from_ptr(pointers.weechat);
            let buffer = weechat.buffer_from_ptr(buffer);

            let ret = if let Some(mut callback) = pointers.close_cb {
                callback(&weechat, &buffer).is_ok()
            } else {
                true
            };

            if ret {
                WEECHAT_RC_OK
            } else {
                WEECHAT_RC_ERROR
            }
        }

        let c_input_cb: Option<WeechatInputCbT> = match settings.input_callback
        {
            Some(_) => Some(c_input_cb),
            None => None,
        };

        // We create a box and use leak to stop rust from freeing our data,
        // we are giving weechat ownership over the data and will free it in
        // the buffer close callback.
        let buffer_pointers = Box::new(BufferPointers {
            weechat: self.ptr,
            input_cb: settings.input_callback,
            close_cb: settings.close_callback,
        });
        let buffer_pointers_ref: &BufferPointers = Box::leak(buffer_pointers);

        let buf_new = self.get().buffer_new.unwrap();
        let c_name = LossyCString::new(settings.name);

        // TODO this can fail, return a Option type
        let buf_ptr = unsafe {
            buf_new(
                self.ptr,
                c_name.as_ptr(),
                c_input_cb,
                buffer_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
                Some(c_close_cb),
                buffer_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };

        if buf_ptr.is_null() {
            return Err(());
        }

        Ok(Buffer {
            weechat: self.ptr,
            ptr: buf_ptr,
            weechat_phantom: PhantomData,
        })
    }
}

pub(crate) type WeechatInputCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    data: *mut c_void,
    buffer: *mut t_gui_buffer,
    input_data: *const c_char,
) -> c_int;

/// Nick creation arguments
pub struct NickArgs<'a> {
    /// Name of the new nick.
    pub name: &'a str,
    /// Color for the nick.
    pub color: &'a str,
    /// Prefix that will be shown before the name.
    pub prefix: &'a str,
    /// Color of the prefix.
    pub prefix_color: &'a str,
    /// Should the nick be visible in the nicklist.
    pub visible: bool,
}

/// Weechat Nick type
pub struct Nick {
    ptr: *mut t_gui_nick,
    buf_ptr: *mut t_gui_buffer,
    weechat_ptr: *mut t_weechat_plugin,
}

impl Nick {
    /// Create a high level Nick object from C nick and buffer pointers.
    pub(crate) fn from_ptr(
        ptr: *mut t_gui_nick,
        buf_ptr: *mut t_gui_buffer,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Nick {
        Nick {
            ptr,
            buf_ptr,
            weechat_ptr,
        }
    }

    /// Get a Weechat object out of the nick.
    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }

    /// Get a string property of the nick.
    /// * `property` - The name of the property to get the value for, this can
    ///     be one of name, color, prefix or prefix_color. If a unknown
    ///     property is requested an empty string is returned.
    pub fn get_string(&self, property: &str) -> Option<Cow<str>> {
        let weechat = self.get_weechat();
        let get_string = weechat.get().nicklist_nick_get_string.unwrap();
        let c_property = LossyCString::new(property);
        unsafe {
            let ret = get_string(self.buf_ptr, self.ptr, c_property.as_ptr());

            if ret.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ret).to_string_lossy())
            }
        }
    }

    /// Get the name property of the nick.
    pub fn get_name(&self) -> Cow<str> {
        self.get_string("name").unwrap()
    }

    /// Removes the nick from it's nicklist
    pub fn remove(&self) {
        let weechat = self.get_weechat();

        let nicklist_remove_nick = weechat.get().nicklist_remove_nick.unwrap();

        unsafe {
            nicklist_remove_nick(self.buf_ptr, self.ptr);
        }
    }
}

/// Weechat nicklist Group type.
pub struct NickGroup {
    pub(crate) ptr: *mut t_gui_nick_group,
    _buf_ptr: *mut t_gui_buffer,
}

impl<'a> Default for NickArgs<'a> {
    fn default() -> NickArgs<'a> {
        NickArgs {
            name: "",
            color: "",
            prefix: "",
            prefix_color: "",
            visible: true,
        }
    }
}

impl Buffer<'_> {
    /// Display a message on the buffer.
    pub fn print(&self, message: &str) {
        let weechat = Weechat::from_ptr(self.weechat);
        let printf_date_tags = weechat.get().printf_date_tags.unwrap();

        let fmt_str = LossyCString::new("%s");
        let c_message = LossyCString::new(message);

        unsafe {
            printf_date_tags(
                self.ptr,
                0,
                ptr::null(),
                fmt_str.as_ptr(),
                c_message.as_ptr(),
            )
        }
    }

    /// Display a message on the buffer with attached date and tags
    pub fn print_date_tags(&self, date: i64, tags: &[&str], message: &str) {
        let weechat = Weechat::from_ptr(self.weechat);
        let printf_date_tags = weechat.get().printf_date_tags.unwrap();

        let fmt_str = LossyCString::new("%s");
        let tags = tags.join(",");
        let tags = LossyCString::new(tags);
        let message = LossyCString::new(message);

        unsafe {
            printf_date_tags(
                self.ptr,
                date,
                tags.as_ptr(),
                fmt_str.as_ptr(),
                message.as_ptr(),
            )
        }
    }

    /// Search for a nicklist group by name
    pub fn search_nicklist_group(&self, name: &str) -> Option<NickGroup> {
        let weechat = Weechat::from_ptr(self.weechat);

        let nicklist_search_group =
            weechat.get().nicklist_search_group.unwrap();

        let name = LossyCString::new(name);

        unsafe {
            let group =
                nicklist_search_group(self.ptr, ptr::null_mut(), name.as_ptr());

            if group.is_null() {
                None
            } else {
                Some(NickGroup {
                    ptr: group,
                    _buf_ptr: self.ptr,
                })
            }
        }
    }

    /// Search for a nick in a group
    pub fn search_nick(
        &self,
        nick: &str,
        group: Option<&NickGroup>,
    ) -> Option<Nick> {
        let weechat = Weechat::from_ptr(self.weechat);

        let nicklist_search_nick = weechat.get().nicklist_search_nick.unwrap();

        let nick = LossyCString::new(nick);
        let group_ptr = group.map(|g| g.ptr).unwrap_or(ptr::null_mut());

        unsafe {
            let nick = nicklist_search_nick(self.ptr, group_ptr, nick.as_ptr());

            if nick.is_null() {
                None
            } else {
                Some(Nick::from_ptr(nick, self.ptr, self.weechat))
            }
        }
    }

    /// Create and add a new nick to the buffer nicklist. Returns the newly
    /// created nick.
    /// The nick won't be removed from the nicklist if the returned nick is
    /// dropped.
    /// * `nick` - Nick arguments struct for the nick that should be added.
    /// * `group` - Nicklist group that the nick should be added to. If no
    ///     group is provided the nick is added to the root group.
    pub fn add_nick(&self, nick: NickArgs, group: Option<&NickGroup>) -> Nick {
        let weechat = Weechat::from_ptr(self.weechat);

        // TODO this conversions can fail if any of those strings contain a
        // null byte.
        let c_nick = LossyCString::new(nick.name);
        let color = LossyCString::new(nick.color);
        let prefix = LossyCString::new(nick.prefix);
        let prefix_color = LossyCString::new(nick.prefix_color);
        let add_nick = weechat.get().nicklist_add_nick.unwrap();

        let group_ptr = match group {
            Some(g) => g.ptr,
            None => ptr::null_mut(),
        };

        let nick_ptr = unsafe {
            add_nick(
                self.ptr,
                group_ptr,
                c_nick.as_ptr(),
                color.as_ptr(),
                prefix.as_ptr(),
                prefix_color.as_ptr(),
                nick.visible as i32,
            )
        };

        Nick::from_ptr(nick_ptr, self.ptr, self.weechat)
    }

    /// Create and add a new nicklist group to the buffers nicklist.
    /// * `name` - Name of the new group.
    /// * `color` - Color of the new group.
    /// * `visible` - Should the group be visible in the nicklist.
    /// * `parent_group` - Parent group that the group should be added to.
    ///     If no group is provided the group is added to the root group.
    /// Returns the new nicklist group. The group is not removed if the object
    /// is dropped.
    pub fn add_group(
        &self,
        name: &str,
        color: &str,
        visible: bool,
        parent_group: Option<&NickGroup>,
    ) -> NickGroup {
        let weechat = Weechat::from_ptr(self.weechat);
        let add_group = weechat.get().nicklist_add_group.unwrap();

        let c_name = LossyCString::new(name);
        let c_color = LossyCString::new(color);

        let group_ptr = match parent_group {
            Some(g) => g.ptr,
            None => ptr::null_mut(),
        };

        let group_ptr = unsafe {
            add_group(
                self.ptr,
                group_ptr,
                c_name.as_ptr(),
                c_color.as_ptr(),
                visible as i32,
            )
        };

        NickGroup {
            ptr: group_ptr,
            _buf_ptr: self.ptr,
        }
    }

    fn set(&self, property: &str, value: &str) {
        let weechat = Weechat::from_ptr(self.weechat);

        let buffer_set = weechat.get().buffer_set.unwrap();
        let option = LossyCString::new(property);
        let value = LossyCString::new(value);

        unsafe { buffer_set(self.ptr, option.as_ptr(), value.as_ptr()) };
    }

    fn get_string(&self, property: &str) -> Option<Cow<str>> {
        let weechat = Weechat::from_ptr(self.weechat);

        let buffer_get = weechat.get().buffer_get_string.unwrap();
        let property = LossyCString::new(property);

        unsafe {
            let value = buffer_get(self.ptr, property.as_ptr());
            if value.is_null() {
                None
            } else {
                Some(CStr::from_ptr(value).to_string_lossy())
            }
        }
    }

    /// Get the value of a buffer localvar
    pub fn get_localvar(&self, property: &str) -> Option<Cow<str>> {
        self.get_string(&format!("localvar_{}", property))
    }

    /// Set the value of a buffer localvar
    pub fn set_localvar(&self, property: &str, value: &str) {
        self.set(&format!("localvar_set_{}", property), value)
    }

    /// Get the full name of the buffer.
    pub fn get_full_name(&self) -> Cow<str> {
        self.get_string("full_name").unwrap()
    }

    /// Set the full name of the buffer
    pub fn set_full_name(&self, name: &str) {
        self.set("full_name", name);
    }

    /// Get the name of the buffer.
    pub fn get_name(&self) -> Cow<str> {
        self.get_string("name").unwrap()
    }

    /// Set the name of the buffer.
    pub fn set_name(&self, name: &str) {
        self.set("name", name);
    }

    /// Get the short_name of the buffer.
    pub fn get_short_name(&self) -> Cow<str> {
        self.get_string("short_name").unwrap()
    }

    /// Set the short_name of the buffer.
    pub fn set_short_name(&self, name: &str) {
        self.set("short_name", name);
    }

    /// Get the plugin name of the plugin that owns this buffer.
    pub fn plugin_name(&self) -> Cow<str> {
        self.get_string("plugin").unwrap()
    }

    /// Hide time for all lines in the buffer.
    pub fn disable_time_for_each_line(&self) {
        self.set("time_for_each_line", "0");
    }

    /// Disable the nicklist for this buffer.
    pub fn disable_nicklist(&self) {
        self.set("nicklist", "0")
    }

    /// Enable the nicklist for this buffer.
    pub fn enable_nicklist(&self) {
        self.set("nicklist", "1")
    }

    /// Set the title of the buffer.
    /// * `title` - The new title that will be set.
    pub fn set_title(&self, title: &str) {
        self.set("title", title);
    }

    /// Disable logging for this buffer.
    pub fn disable_log(&self) {
        self.set("localvar_set_no_log", "1");
    }

    /// Clear buffer contents
    pub fn clear(&self) {
        let weechat = Weechat::from_ptr(self.weechat);

        let buffer_clear = weechat.get().buffer_clear.unwrap();
        unsafe { buffer_clear(self.ptr) }
    }

    /// Get the contents of the input
    pub fn input(&self) -> Cow<str> {
        self.get_string("input").unwrap()
    }

    /// Switch to the buffer
    pub fn switch_to(&self) {
        self.set("display", "1");
    }
}

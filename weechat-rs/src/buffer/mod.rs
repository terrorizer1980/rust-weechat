//! Weechat Buffer module containing Buffer and Nick types.

mod nick;
mod nickgroup;

use std::borrow::Cow;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;

use std::cell::Cell;
use std::rc::Rc;

#[cfg(feature = "async-executor")]
use async_trait::async_trait;
#[cfg(feature = "async-executor")]
use futures::future::LocalBoxFuture;

use crate::{LossyCString, Weechat};
use libc::{c_char, c_int};
use weechat_sys::{
    t_gui_buffer, t_gui_nick, t_weechat_plugin, WEECHAT_RC_ERROR, WEECHAT_RC_OK,
};

pub use crate::buffer::nick::{Nick, NickSettings};
pub use crate::buffer::nickgroup::NickGroup;

/// A Weechat buffer.
///
/// A buffer contains the data displayed on the screen.
pub struct Buffer<'a> {
    pub(crate) inner: InnerBuffers<'a>,
}

pub(crate) enum InnerBuffers<'a> {
    BorrowedBuffer(InnerBuffer<'a>),
    OwnedBuffer(InnerOwnedBuffer<'a>),
}

pub(crate) struct InnerOwnedBuffer<'a> {
    pub(crate) weechat: *mut t_weechat_plugin,
    pub(crate) buffer_handle: &'a BufferHandle,
}

pub(crate) struct InnerBuffer<'a> {
    pub(crate) weechat: *mut t_weechat_plugin,
    pub(crate) ptr: *mut t_gui_buffer,
    pub(crate) weechat_phantom: PhantomData<&'a Weechat>,
}

impl PartialEq for Buffer<'_> {
    fn eq(&self, other: &Buffer) -> bool {
        self.ptr() == other.ptr()
    }
}

/// A handle to a buffer that was created in the current plugin.
///
/// This means that the plugin owns this buffer. Nevertheless Weechat can
/// invalidate the buffer between callbacks at any point in time.
///
/// The buffer handle can be upgraded to a buffer which can then manipulate the
/// buffer state using the `upgrade()` method.
pub struct BufferHandle {
    buffer_name: Rc<String>,
    weechat: *mut t_weechat_plugin,
    buffer_ptr: Rc<Cell<*mut t_gui_buffer>>,
}

impl BufferHandle {
    /// Upgrade the buffer handle into a `Buffer`.
    ///
    /// This is necessary to do because the handle can be invalidated by Weechat
    /// between callbacks.
    pub fn upgrade(&self) -> Result<Buffer<'_>, ()> {
        let ptr = self.buffer_ptr.get();

        if ptr.is_null() {
            Err(())
        } else {
            let buffer = Buffer {
                inner: InnerBuffers::OwnedBuffer(InnerOwnedBuffer {
                    weechat: self.weechat,
                    buffer_handle: self.clone(),
                }),
            };
            Ok(buffer)
        }
    }
}

#[cfg(feature = "async-executor")]
pub(crate) struct BufferPointersAsync {
    pub(crate) weechat: *mut t_weechat_plugin,
    pub(crate) input_cb: Option<Box<dyn BufferInputCallbackAsync>>,
    pub(crate) close_cb: Option<BufferCloseCallback>,
    pub(crate) buffer_cell: Option<Rc<Cell<*mut t_gui_buffer>>>,
}

pub(crate) struct BufferPointers {
    pub(crate) weechat: *mut t_weechat_plugin,
    pub(crate) input_cb: Option<Box<dyn BufferInputCallback>>,
    pub(crate) close_cb: Option<BufferCloseCallback>,
    pub(crate) buffer_cell: Option<Rc<Cell<*mut t_gui_buffer>>>,
}

/// Callback that will be called if the user inputs something into the buffer
/// input field. This is the non-async version of the callback.
pub trait BufferInputCallback: 'static {
    fn callback(
        &mut self,
        weechat: &Weechat,
        buffer: &Buffer,
        input: Cow<str>,
    ) -> Result<(), ()>;
}

#[cfg(feature = "async-executor")]
#[cfg_attr(docsrs, doc(cfg(feature = "async-executor")))]
#[async_trait(?Send)]
/// Trait for the buffer input callback.
/// This is the async version of the callback.
pub trait BufferInputCallbackAsync: 'static {
    /// Callback that will be called if the user inputs something into the buffer
    /// input field.
    async fn callback(&mut self, buffer: BufferHandle, input: String);
}

#[cfg(feature = "async-executor")]
#[async_trait(?Send)]
impl<
        T: FnMut(BufferHandle, String) -> LocalBoxFuture<'static, ()> + 'static,
    > BufferInputCallbackAsync for T
{
    async fn callback(&mut self, buffer: BufferHandle, input: String) {
        self(buffer, input).await
    }
}

/// Callback that will be called if the buffer gets closed.
pub type BufferCloseCallback =
    Box<dyn FnMut(&Weechat, &Buffer) -> Result<(), ()>>;

#[cfg(feature = "async-executor")]
#[cfg_attr(docsrs, doc(cfg(feature = "async-executor")))]
/// Settings for the creation of a buffer.
pub struct BufferSettingsAsync {
    pub(crate) name: String,
    pub(crate) input_callback: Option<Box<dyn BufferInputCallbackAsync>>,
    pub(crate) close_callback: Option<BufferCloseCallback>,
}

/// Settings for the creation of a buffer.
pub struct BufferSettings {
    pub(crate) name: String,
    pub(crate) input_callback: Option<Box<dyn BufferInputCallback>>,
    pub(crate) close_callback: Option<BufferCloseCallback>,
}

#[cfg(feature = "async-executor")]
impl BufferSettingsAsync {
    /// Create new default buffer creation settings.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the new buffer. Needs to be unique across a
    /// plugin, otherwise the buffer creation will fail.
    pub fn new(name: &str) -> Self {
        BufferSettingsAsync {
            name: name.to_owned(),
            input_callback: None,
            close_callback: None,
        }
    }

    /// Set the buffer input callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - An async function that will be called once a user inputs
    ///     data into the buffer input line.
    pub fn input_callback(
        mut self,
        callback: impl BufferInputCallbackAsync,
    ) -> Self {
        self.input_callback = Some(Box::new(callback));
        self
    }

    /// Set the close callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - The callback that should be called before a buffer is
    ///     closed.
    pub fn close_callback(
        mut self,
        callback: impl FnMut(&Weechat, &Buffer) -> Result<(), ()> + 'static,
    ) -> Self {
        self.close_callback = Some(Box::new(callback));
        self
    }
}

impl BufferSettings {
    /// Create new default buffer creation settings.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the new buffer. Needs to be unique across a
    /// plugin, otherwise the buffer creation will fail.
    pub fn new(name: &str) -> Self {
        BufferSettings {
            name: name.to_owned(),
            input_callback: None,
            close_callback: None,
        }
    }

    /// Set the buffer input callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - A function or a struct that implements the
    /// BufferCloseCallback trait.
    pub fn input_callback(
        mut self,
        callback: impl BufferInputCallback + 'static,
    ) -> Self {
        self.input_callback = Some(Box::new(callback));
        self
    }

    /// Set the close callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - The callback that should be called before a buffer is
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
    ///
    /// Returns a Buffer if one is found, otherwise None.
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - name of a plugin, the following special value is
    ///     allowed: "==", the buffer name used is the buffers full name.
    ///
    /// * `buffer_name` - name of a buffer, if this is an empty string,
    ///     the current buffer is returned (buffer displayed by current
    ///     window); if the name starts with (?i), the search is case
    ///     insensitive.
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
            inner: InnerBuffers::BorrowedBuffer(InnerBuffer {
                weechat: self.ptr,
                ptr: buffer_ptr,
                weechat_phantom: PhantomData,
            }),
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

    /// Create a new Weechat buffer with an async input callback.
    ///
    /// * `settings` - Settings for the new buffer.
    ///
    /// Returns a Buffer if one has been created, otherwise an empty Error.
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    ///
    /// # Example
    /// ```no_execute
    /// # use futures::future::{FutureExt, LocalBoxFuture};
    /// # use weechat::Weechat;
    /// # use weechat::buffer::{BufferHandle, BufferSettingsAsync};
    /// fn input_cb(buffer: BufferHandle, input: String) -> LocalBoxFuture<'static, ()> {
    ///     async move {
    ///         let buffer = buffer.upgrade().unwrap();
    ///         buffer.print(&input);
    ///     }.boxed_local()
    /// }
    ///
    /// let buffer_settings = BufferSettingsAsync::new("test_buffer")
    ///     .input_callback(input_cb)
    ///     .close_callback(|weechat, buffer| {
    ///         Ok(())
    /// });
    ///
    /// let buffer_handle = Weechat::buffer_new_with_async(buffer_settings)
    ///     .expect("Can't create new room buffer");
    ///
    /// let buffer = buffer_handle
    ///     .upgrade()
    ///     .expect("Can't upgrade newly created buffer");
    ///
    /// buffer.enable_nicklist();
    /// buffer.print("Hello world");
    /// ```
    #[cfg(feature = "async-executor")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async-executor")))]
    pub fn buffer_new_with_async(
        settings: BufferSettingsAsync,
    ) -> Result<BufferHandle, ()> {
        unsafe extern "C" fn c_input_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
            input_data: *const c_char,
        ) -> c_int {
            let input_data = CStr::from_ptr(input_data).to_string_lossy();

            let pointers: &mut BufferPointersAsync =
                { &mut *(pointer as *mut BufferPointersAsync) };

            let weechat = Weechat::from_ptr(pointers.weechat);
            let buffer = weechat.buffer_from_ptr(buffer);
            let buffer_cell = pointers
                .buffer_cell
                .as_ref()
                .expect("Buffer cell wasn't initialized properly")
                .clone();

            let buffer_handle = BufferHandle {
                buffer_name: Rc::new(buffer.full_name().to_string()),
                weechat: pointers.weechat,
                buffer_ptr: buffer_cell,
            };
            if let Some(cb) = pointers.input_cb.as_mut() {
                let future = cb.callback(buffer_handle, input_data.to_string());
                Weechat::spawn_buffer_cb(
                    buffer.full_name().to_string(),
                    future,
                );
            }

            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_close_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            buffer: *mut t_gui_buffer,
        ) -> c_int {
            // We use from_raw() here so that the box gets deallocated at the
            // end of this scope.
            let pointers = Box::from_raw(pointer as *mut BufferPointersAsync);
            let weechat = Weechat::from_ptr(pointers.weechat);
            let buffer = weechat.buffer_from_ptr(buffer);

            let ret = if let Some(mut callback) = pointers.close_cb {
                callback(&weechat, &buffer).is_ok()
            } else {
                true
            };

            // Invalidate the buffer pointer now.
            let mut cell = pointers
                .buffer_cell
                .as_ref()
                .expect("Buffer cell wasn't initialized properly")
                .replace(ptr::null_mut());

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

        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };

        // We create a box and use leak to stop rust from freeing our data,
        // we are giving Weechat ownership over the data and will free it in
        // the buffer close callback.
        let buffer_pointers = Box::new(BufferPointersAsync {
            weechat: weechat.ptr,
            input_cb: settings.input_callback,
            close_cb: settings.close_callback,
            buffer_cell: None,
        });

        let buffer_pointers_ref = Box::leak(buffer_pointers);

        let buf_new = weechat.get().buffer_new.unwrap();
        let c_name = LossyCString::new(settings.name);

        let buf_ptr = unsafe {
            buf_new(
                weechat.ptr,
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
            unsafe { Box::from_raw(buffer_pointers_ref) };
            return Err(());
        }

        let pointers: &mut BufferPointersAsync =
            unsafe { &mut *(buffer_pointers_ref as *mut BufferPointersAsync) };

        let buffer = weechat.buffer_from_ptr(buf_ptr);
        let buffer_cell = Rc::new(Cell::new(buf_ptr));

        pointers.buffer_cell = Some(buffer_cell.clone());

        Ok(BufferHandle {
            buffer_name: Rc::new(buffer.full_name().to_string()),
            weechat: weechat.ptr,
            buffer_ptr: buffer_cell,
        })
    }

    /// Create a new Weechat buffer
    ///
    /// * `settings` - Settings for the new buffer.
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    ///
    /// Returns a Buffer if one has been created, otherwise an empty Error.
    /// # Example
    /// ```no_run
    /// # use std::borrow::Cow;
    /// # use weechat::Weechat;
    /// # use weechat::buffer::{Buffer, BufferHandle, BufferSettings};
    /// fn input_cb(weechat: &Weechat, buffer: &Buffer, input: Cow<str>) -> Result<(), ()> {
    ///     buffer.print(&input);
    ///     Ok(())
    /// }
    ///
    /// let buffer_settings = BufferSettings::new("test_buffer")
    ///     .input_callback(input_cb)
    ///     .close_callback(|weechat, buffer| {
    ///         Ok(())
    /// });
    ///
    /// let buffer_handle = Weechat::buffer_new(buffer_settings)
    ///     .expect("Can't create new room buffer");
    ///
    /// let buffer = buffer_handle
    ///     .upgrade()
    ///     .expect("Can't upgrade newly created buffer");
    ///
    /// buffer.enable_nicklist();
    /// buffer.print("Hello world");
    /// ```
    pub fn buffer_new(settings: BufferSettings) -> Result<BufferHandle, ()> {
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

            let ret = if let Some(ref mut cb) = pointers.input_cb.as_mut() {
                cb.callback(&weechat, &buffer, input_data).is_ok()
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
            // We use from_raw() here so that the box gets freed at the end
            // of this scope.
            let pointers = Box::from_raw(pointer as *mut BufferPointers);
            let weechat = Weechat::from_ptr(pointers.weechat);
            let buffer = weechat.buffer_from_ptr(buffer);

            let ret = if let Some(mut callback) = pointers.close_cb {
                callback(&weechat, &buffer).is_ok()
            } else {
                true
            };

            // Invalidate the buffer pointer now.
            pointers
                .buffer_cell
                .as_ref()
                .expect("Buffer cell wasn't initialized properly")
                .replace(ptr::null_mut());

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

        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };

        // We create a box and use leak to stop rust from freeing our data,
        // we are giving weechat ownership over the data and will free it in
        // the buffer close callback.
        let buffer_pointers = Box::new(BufferPointers {
            weechat: weechat.ptr,
            input_cb: settings.input_callback,
            close_cb: settings.close_callback,
            buffer_cell: None,
        });
        let buffer_pointers_ref = Box::leak(buffer_pointers);

        let buf_new = weechat.get().buffer_new.unwrap();
        let c_name = LossyCString::new(settings.name);

        let buf_ptr = unsafe {
            buf_new(
                weechat.ptr,
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
            unsafe { Box::from_raw(buffer_pointers_ref) };
            return Err(());
        }

        let pointers: &mut BufferPointers =
            unsafe { &mut *(buffer_pointers_ref as *mut BufferPointers) };

        let buffer = weechat.buffer_from_ptr(buf_ptr);
        let buffer_cell = Rc::new(Cell::new(buf_ptr));

        pointers.buffer_cell = Some(buffer_cell.clone());

        Ok(BufferHandle {
            buffer_name: Rc::new(buffer.full_name().to_string()),
            weechat: weechat.ptr,
            buffer_ptr: buffer_cell,
        })
    }
}

pub(crate) type WeechatInputCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    data: *mut c_void,
    buffer: *mut t_gui_buffer,
    input_data: *const c_char,
) -> c_int;

impl Buffer<'_> {
    fn weechat(&self) -> Weechat {
        let ptr = match &self.inner {
            InnerBuffers::BorrowedBuffer(b) => b.weechat,
            InnerBuffers::OwnedBuffer(b) => b.weechat,
        };

        Weechat::from_ptr(ptr)
    }

    fn ptr(&self) -> *mut t_gui_buffer {
        match &self.inner {
            InnerBuffers::BorrowedBuffer(b) => b.ptr,
            InnerBuffers::OwnedBuffer(b) => {
                let ptr = b.buffer_handle.buffer_ptr.get();

                if ptr.is_null() {
                    panic!(
                        "Buffer {} has been closed.",
                        b.buffer_handle.buffer_name
                    )
                } else {
                    ptr
                }
            }
        }
    }

    /// Display a message on the buffer.
    pub fn print(&self, message: &str) {
        let weechat = self.weechat();
        let printf_date_tags = weechat.get().printf_date_tags.unwrap();

        let fmt_str = LossyCString::new("%s");
        let c_message = LossyCString::new(message);

        unsafe {
            printf_date_tags(
                self.ptr(),
                0,
                ptr::null(),
                fmt_str.as_ptr(),
                c_message.as_ptr(),
            )
        }
    }

    /// Display a message on the buffer with attached date and tags
    ///
    /// # Arguments
    ///
    /// * `date` - A unix time-stamp representing the date of the message, 0
    ///     means now.
    /// * `tags` - A list of tags that will be applied to the printed line.
    /// * `message` - The message that will be displayed.
    pub fn print_date_tags(&self, date: i64, tags: &[&str], message: &str) {
        let weechat = self.weechat();
        let printf_date_tags = weechat.get().printf_date_tags.unwrap();

        let fmt_str = LossyCString::new("%s");
        let tags = tags.join(",");
        let tags = LossyCString::new(tags);
        let message = LossyCString::new(message);

        unsafe {
            printf_date_tags(
                self.ptr(),
                date,
                tags.as_ptr(),
                fmt_str.as_ptr(),
                message.as_ptr(),
            )
        }
    }

    /// Search for a nicklist group by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the nicklist that should be searched for.
    ///
    /// Returns a NickGroup if one is found, None otherwise.
    pub fn search_nicklist_group(&self, name: &str) -> Option<NickGroup> {
        let weechat = self.weechat();

        let nicklist_search_group =
            weechat.get().nicklist_search_group.unwrap();

        let name = LossyCString::new(name);

        let group = unsafe {
            nicklist_search_group(self.ptr(), ptr::null_mut(), name.as_ptr())
        };

        if group.is_null() {
            None
        } else {
            Some(NickGroup {
                ptr: group,
                buf_ptr: self.ptr(),
                weechat_ptr: self.weechat().ptr,
                buffer: PhantomData,
            })
        }
    }

    /// Search for a nick in the whole nicklist.
    ///
    /// # Arguments
    ///
    /// * `nick` - The name of the nick that should be found.
    ///
    /// Returns a `Nick` if one is found, None otherwise.
    pub fn search_nick(&self, nick: &str) -> Option<Nick> {
        let weechat = self.weechat();
        let nick = Buffer::search_nick_helper(&weechat, self.ptr(), nick, None);

        if nick.is_null() {
            None
        } else {
            Some(Nick {
                ptr: nick,
                buf_ptr: self.ptr(),
                weechat_ptr: weechat.ptr,
                buffer: PhantomData,
            })
        }
    }

    fn search_nick_helper(
        weechat: &Weechat,
        buffer_ptr: *mut t_gui_buffer,
        nick: &str,
        group: Option<&NickGroup>,
    ) -> *mut t_gui_nick {
        let nicklist_search_nick = weechat.get().nicklist_search_nick.unwrap();

        let nick = LossyCString::new(nick);
        let group_ptr = group.map(|g| g.ptr).unwrap_or(ptr::null_mut());

        unsafe { nicklist_search_nick(buffer_ptr, group_ptr, nick.as_ptr()) }
    }

    /// Create and add a new nick to the buffer nicklist.
    ///
    /// This will add the nick to the root nick group.
    ///
    /// # Arguments
    ///
    /// * `nick_settings` - Nick arguments struct for the nick that should be
    ///     added.
    ///
    /// Returns the newly created nick if one is created successfully, an empty
    /// error otherwise.
    pub fn add_nick(&self, nick_settings: NickSettings) -> Result<Nick, ()> {
        let weechat = self.weechat();
        let nick_ptr =
            Buffer::add_nick_helper(&weechat, self.ptr(), nick_settings, None);

        if nick_ptr.is_null() {
            return Err(());
        }

        Ok(Nick {
            ptr: nick_ptr,
            buf_ptr: self.ptr(),
            weechat_ptr: self.weechat().ptr,
            buffer: PhantomData,
        })
    }

    /// Removes a group from the nicklist.
    ///
    /// # Arguments
    ///
    /// * `group_name` - The name of the group that should be removed.
    ///
    /// Returns `true` if a group was found and removed, `false` otherwise.
    pub fn remove_nicklist_group(&mut self, group_name: &str) -> bool {
        let weechat = self.weechat();

        let group = self.search_nicklist_group(group_name);

        match group {
            Some(group) => {
                let nicklist_remove_group =
                    weechat.get().nicklist_remove_group.unwrap();

                unsafe {
                    nicklist_remove_group(self.ptr(), group.ptr);
                }
                true
            }
            None => false,
        }
    }

    /// Removes a nick from the nicklist.
    ///
    /// # Arguments
    ///
    /// * `nick` - The name of the nick that should be removed.
    ///
    /// Returns `true` if a nick was found and removed, `false` otherwise.
    pub fn remove_nick(&mut self, nick: &str) -> bool {
        let weechat = self.weechat();

        let nick = self.search_nick(nick);

        match nick {
            Some(nick) => {
                let nicklist_remove_nick =
                    weechat.get().nicklist_remove_nick.unwrap();

                unsafe {
                    nicklist_remove_nick(self.ptr(), nick.ptr);
                }
                true
            }
            None => false,
        }
    }

    fn add_nick_helper(
        weechat: &Weechat,
        buffer_ptr: *mut t_gui_buffer,
        nick_settings: NickSettings,
        group: Option<&NickGroup>,
    ) -> *mut t_gui_nick {
        let c_nick = LossyCString::new(nick_settings.name);
        let color = LossyCString::new(nick_settings.color);
        let prefix = LossyCString::new(nick_settings.prefix);
        let prefix_color = LossyCString::new(nick_settings.prefix_color);

        let add_nick = weechat.get().nicklist_add_nick.unwrap();

        let group_ptr = match group {
            Some(g) => g.ptr,
            None => ptr::null_mut(),
        };

        unsafe {
            add_nick(
                buffer_ptr,
                group_ptr,
                c_nick.as_ptr(),
                color.as_ptr(),
                prefix.as_ptr(),
                prefix_color.as_ptr(),
                nick_settings.visible as i32,
            )
        }
    }

    /// Create and add a new nicklist group to the buffers nicklist.
    /// * `name` - Name of the new group.
    /// * `color` - Color of the new group.
    /// * `visible` - Should the group be visible in the nicklist.
    /// * `parent_group` - Parent group that the group should be added to.
    ///     If no group is provided the group is added to the root group.
    /// Returns the new nicklist group. The group is not removed if the object
    /// is dropped.
    pub fn add_nicklist_group(
        &self,
        name: &str,
        color: &str,
        visible: bool,
        parent_group: Option<&NickGroup>,
    ) -> Result<NickGroup, ()> {
        let weechat = self.weechat();
        let add_group = weechat.get().nicklist_add_group.unwrap();

        let c_name = LossyCString::new(name);
        let c_color = LossyCString::new(color);

        let group_ptr = match parent_group {
            Some(g) => g.ptr,
            None => ptr::null_mut(),
        };

        let group_ptr = unsafe {
            add_group(
                self.ptr(),
                group_ptr,
                c_name.as_ptr(),
                c_color.as_ptr(),
                visible as i32,
            )
        };

        if group_ptr.is_null() {
            return Err(());
        }

        Ok(NickGroup {
            ptr: group_ptr,
            buf_ptr: self.ptr(),
            weechat_ptr: self.weechat().ptr,
            buffer: PhantomData,
        })
    }

    fn set(&self, property: &str, value: &str) {
        let weechat = self.weechat();

        let buffer_set = weechat.get().buffer_set.unwrap();
        let option = LossyCString::new(property);
        let value = LossyCString::new(value);

        unsafe { buffer_set(self.ptr(), option.as_ptr(), value.as_ptr()) };
    }

    fn get_string(&self, property: &str) -> Option<Cow<str>> {
        let weechat = self.weechat();

        let buffer_get = weechat.get().buffer_get_string.unwrap();
        let property = LossyCString::new(property);

        unsafe {
            let value = buffer_get(self.ptr(), property.as_ptr());
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
    pub fn full_name(&self) -> Cow<str> {
        self.get_string("full_name").unwrap()
    }

    /// Set the full name of the buffer
    pub fn set_full_name(&self, name: &str) {
        self.set("full_name", name);
    }

    /// Get the name of the buffer.
    pub fn name(&self) -> Cow<str> {
        self.get_string("name").unwrap()
    }

    /// Set the name of the buffer.
    pub fn set_name(&self, name: &str) {
        self.set("name", name);
    }

    /// Get the short_name of the buffer.
    pub fn short_name(&self) -> Cow<str> {
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

    /// Get the title of the buffer
    pub fn title(&self) {
        self.get_string("title");
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
        let weechat = self.weechat();

        let buffer_clear = weechat.get().buffer_clear.unwrap();
        unsafe { buffer_clear(self.ptr()) }
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

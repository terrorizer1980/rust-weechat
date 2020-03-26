//! Main weechat module

use weechat_sys::t_weechat_plugin;

use crate::LossyCString;
use libc::{c_char, c_int};
use std::borrow::Cow;
use std::ffi::CStr;
use std::{ptr, vec};

#[cfg(feature = "async-executor")]
use crate::executor::WeechatExecutor;
#[cfg(feature = "async-executor")]
pub use async_task::JoinHandle;
#[cfg(feature = "async-executor")]
use std::future::Future;

/// An iterator over the arguments of a Weechat command, yielding a String value
/// for each argument.
pub struct ArgsWeechat {
    iter: vec::IntoIter<String>,
}

impl ArgsWeechat {
    /// Create an ArgsWeechat object from the underlying weechat C types.
    /// Expects the strings in argv to be valid utf8, if not invalid UTF-8
    /// sequences are replaced with the replacement character.
    ///
    /// # Safety
    ///
    /// This should never be called by the user, this is called internally but
    /// needs to be public because it's used in the macro expansion of the
    /// plugin init method.
    #[doc(hidden)]
    pub fn new(argc: c_int, argv: *mut *mut c_char) -> ArgsWeechat {
        let argc = argc as isize;
        let args: Vec<String> = (0..argc)
            .map(|i| {
                let cstr = unsafe {
                    CStr::from_ptr(*argv.offset(i) as *const libc::c_char)
                };

                String::from_utf8_lossy(&cstr.to_bytes().to_vec()).to_string()
            })
            .collect();
        ArgsWeechat {
            iter: args.into_iter(),
        }
    }
}

impl Iterator for ArgsWeechat {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl ExactSizeIterator for ArgsWeechat {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl DoubleEndedIterator for ArgsWeechat {
    fn next_back(&mut self) -> Option<String> {
        self.iter.next_back()
    }
}

static mut WEECHAT: Option<Weechat> = None;
static mut WEECHAT_THREAD_ID: Option<std::thread::ThreadId> = None;

/// Main Weechat struct that encapsulates common weechat API functions.
/// It has a similar API as the weechat script API.
pub struct Weechat {
    pub(crate) ptr: *mut t_weechat_plugin,
}

impl Weechat {
    /// Create a Weechat object from a C t_weechat_plugin pointer.
    ///
    /// # Arguments
    ///
    /// * `ptr` - C pointer of the weechat plugin.
    ///
    /// # Safety
    ///
    /// This should never be called by the user. This is called internally.
    #[doc(hidden)]
    pub unsafe fn init_from_ptr(ptr: *mut t_weechat_plugin) -> Weechat {
        assert!(!ptr.is_null());

        WEECHAT = Some(Weechat { ptr });
        WEECHAT_THREAD_ID = Some(std::thread::current().id());

        #[cfg(feature = "async-executor")]
        WeechatExecutor::start();
        Weechat { ptr }
    }

    /// Free internal plugin data.
    /// # Safety
    ///
    /// This should never be called by the user. This is called internally.
    #[doc(hidden)]
    pub unsafe fn free() {
        #[cfg(feature = "async-executor")]
        WeechatExecutor::free();
    }

    pub(crate) fn from_ptr(ptr: *mut t_weechat_plugin) -> Weechat {
        assert!(!ptr.is_null());
        Weechat { ptr }
    }

    /// Get the Weechat plugin.
    ///
    /// # Safety
    ///
    /// It is generally safe to call this method, the plugin pointer is valid
    /// for the durration of the plugin lifetime. The problem is that many
    /// Weechat objects need to have a lifetime bound to a Weechat context
    /// object that is only valid for the duration of a callback.
    ///
    /// Since this one will have a static lifetime, objects that are fetched
    /// from this object may have a longer lifetime than they should.
    pub unsafe fn weechat() -> &'static mut Weechat {
        match WEECHAT {
            Some(ref mut w) => w,
            None => panic!("Plugin wasn't initialized correctly"),
        }
    }

    #[inline]
    pub(crate) fn get(&self) -> &t_weechat_plugin {
        unsafe { &*self.ptr }
    }

    /// Write a message in WeeChat log file (weechat.log).
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    pub fn log(msg: &str) {
        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };
        let log_printf = weechat.get().log_printf.unwrap();

        let fmt = LossyCString::new("%s");
        let msg = LossyCString::new(msg);

        unsafe {
            log_printf(fmt.as_ptr(), msg.as_ptr());
        }
    }

    /// Display a message on the core weechat buffer.
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    pub fn print(msg: &str) {
        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };

        let printf_date_tags = weechat.get().printf_date_tags.unwrap();

        let fmt = LossyCString::new("%s");
        let msg = LossyCString::new(msg);

        unsafe {
            printf_date_tags(
                ptr::null_mut(),
                0,
                ptr::null(),
                fmt.as_ptr(),
                msg.as_ptr(),
            );
        }
    }

    pub(crate) fn check_thread() {
        let weechat_thread_id = unsafe {
            WEECHAT_THREAD_ID.as_ref().expect(
                "Weechat main thread ID wasn't found, plugin \
                 wasn't correctly initialized",
            )
        };

        if std::thread::current().id() != *weechat_thread_id {
            panic!(
                "Weechat methods can be only called from the main Weechat \
                 thread."
            )
        }
    }

    /// Return a string color code for display.
    ///
    /// # Arguments
    ///
    /// `color_name` - name of the color
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    pub fn color(color_name: &str) -> &str {
        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };
        let weechat_color = weechat.get().color.unwrap();

        let color_name = LossyCString::new(color_name);
        unsafe {
            let color = weechat_color(color_name.as_ptr());
            CStr::from_ptr(color)
                .to_str()
                .expect("Weechat returned a non UTF-8 string")
        }
    }

    /// Retrieve a prefix value
    ///
    /// # Arguments:
    ///
    /// `prefix` - The name of the prefix.
    ///
    /// Valid prefixes are:
    /// * error
    /// * network
    /// * action
    /// * join
    /// * quit
    ///
    /// An empty string will be returned if the prefix is not found
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    pub fn prefix(prefix: &str) -> &str {
        Weechat::check_thread();
        let weechat = unsafe { Weechat::weechat() };
        let prefix_fn = weechat.get().prefix.unwrap();
        let prefix = LossyCString::new(prefix);

        unsafe {
            CStr::from_ptr(prefix_fn(prefix.as_ptr()))
                .to_str()
                .expect("Weechat returned a non UTF-8 string")
        }
    }

    /// Get some info from Weechat or a plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - name the info
    ///
    /// * `arguments` - arguments for the info
    pub fn info_get(&self, name: &str, arguments: &str) -> Option<Cow<str>> {
        let info_get = self.get().info_get.unwrap();

        let info_name = LossyCString::new(name);
        let arguments = LossyCString::new(arguments);

        unsafe {
            let info =
                info_get(self.ptr, info_name.as_ptr(), arguments.as_ptr());
            if info.is_null() {
                None
            } else {
                Some(CStr::from_ptr(info).to_string_lossy())
            }
        }
    }

    /// Evaluate a weechat expression and return the result
    //
    // TODO: Add hashtable options
    // TODO: This needs better docs and examples.
    pub fn eval_string_expression(&self, expr: &str) -> Option<Cow<str>> {
        let string_eval_expression = self.get().string_eval_expression.unwrap();

        let expr = LossyCString::new(expr);

        unsafe {
            let result = string_eval_expression(
                expr.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );

            if result.is_null() {
                None
            } else {
                Some(CStr::from_ptr(result).to_string_lossy())
            }
        }
    }

    #[cfg(feature = "async-executor")]
    /// Spawn a new `Future` on the main Weechat thread.
    ///
    /// # Panics
    ///
    /// Panics if the method is not called from the main Weechat thread.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use weechat::Weechat;
    /// use async_std::sync::{channel, Receiver};
    /// use futures::executor::block_on;
    ///
    /// pub async fn task(receiver: Receiver<String>) {
    ///     loop {
    ///         match receiver.recv().await {
    ///             Some(m) => {
    ///                 Weechat::print(&format!("Received message: {}", m));
    ///             },
    ///             None => {
    ///                 Weechat::print("Error channel closed");
    ///                 return;
    ///             }
    ///         }
    ///     }
    /// }
    ///
    /// let (tx, rx) = channel(1000);
    ///
    /// Weechat::spawn(task(rx));
    /// block_on(tx.send("Hello wordl".to_string()));
    /// ```
    pub fn spawn<F, R>(future: F) -> JoinHandle<R, ()>
    where
        F: Future<Output = R> + 'static,
        R: 'static,
    {
        WeechatExecutor::spawn(future)
    }

    #[cfg(feature = "async-executor")]
    pub(crate) fn spawn_buffer_cb<F, R>(
        buffer_name: String,
        future: F,
    ) -> JoinHandle<R, String>
    where
        F: Future<Output = R> + 'static,
        R: 'static,
    {
        WeechatExecutor::spawn_buffer_cb(buffer_name, future)
    }
}

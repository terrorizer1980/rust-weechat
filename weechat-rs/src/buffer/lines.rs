use std::borrow::Cow;
use std::ffi::c_void;
use std::marker::PhantomData;

use crate::buffer::Buffer;
use crate::Weechat;
use weechat_sys::{t_hdata, t_weechat_plugin};

/// An iterator that steps over the lines of the buffer.
pub struct BufferLines<'a> {
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
    pub(crate) first_line: *mut c_void,
    pub(crate) last_line: *mut c_void,
    pub(crate) buffer: PhantomData<&'a Buffer<'a>>,
}

impl<'a> Iterator for BufferLines<'a> {
    type Item = BufferLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let line_hdata = unsafe { weechat.hdata_get("line") };

        let line_data_pointer = unsafe {
            let line_hdata = weechat.hdata_get("line");
            weechat.hdata_pointer(line_hdata, self.first_line, "data")
        };

        if line_data_pointer.is_null() {
            return None;
        }

        self.first_line =
            unsafe { weechat.hdata_move(line_hdata, self.first_line, 1) };

        Some(BufferLine {
            weechat,
            line_data_pointer,
            buffer: PhantomData,
        })
    }
}

impl<'a> DoubleEndedIterator for BufferLines<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let line_hdata = unsafe { weechat.hdata_get("line") };

        let line_data_pointer = unsafe {
            let line_hdata = weechat.hdata_get("line");
            weechat.hdata_pointer(line_hdata, self.last_line, "data")
        };

        if line_data_pointer.is_null() {
            return None;
        }

        self.last_line =
            unsafe { weechat.hdata_move(line_hdata, self.last_line, -1) };

        Some(BufferLine {
            weechat,
            line_data_pointer,
            buffer: PhantomData,
        })
    }
}

/// The buffer line, makes it possible to modify the printed message and other
/// line data.
pub struct BufferLine<'a> {
    weechat: Weechat,
    line_data_pointer: *mut c_void,
    buffer: PhantomData<&'a Buffer<'a>>,
}

impl<'a> BufferLine<'a> {
    fn hdata(&self) -> *mut t_hdata {
        unsafe { self.weechat.hdata_get("line_data") }
    }

    /// Get the prefix of the line, everything left of the message separator
    /// (usually `|`) is considered the prefix.
    pub fn prefix(&self) -> Cow<str> {
        unsafe {
            self.weechat.hdata_string(
                self.hdata(),
                self.line_data_pointer,
                "prefix",
            )
        }
    }

    /// Get the message of the line.
    pub fn message(&self) -> Cow<str> {
        unsafe {
            self.weechat.hdata_string(
                self.hdata(),
                self.line_data_pointer,
                "message",
            )
        }
    }

    /// Get the date of the line.
    pub fn date(&self) -> i64 {
        unsafe {
            self.weechat.hdata_time(
                self.hdata(),
                self.line_data_pointer,
                "date",
            )
        }
    }

    /// Get the date the line was printed.
    pub fn date_printed(&self) -> i64 {
        unsafe {
            self.weechat.hdata_time(
                self.hdata(),
                self.line_data_pointer,
                "date_printed",
            )
        }
    }

    /// Is the line highlighted.
    pub fn highlighted(&self) -> bool {
        unsafe {
            self.weechat.hdata_char(
                self.hdata(),
                self.line_data_pointer,
                "highlight",
            ) != 0
        }
    }

    /// Get the list of tags of the line.
    pub fn tags(&self) -> Vec<Cow<str>> {
        unsafe {
            let count = self.weechat.hdata_var_array_size(
                self.hdata(),
                self.line_data_pointer,
                "tags_array",
            );

            let mut tags = Vec::with_capacity(count as usize);

            for i in 0..count {
                let tag = self.weechat.hdata_string(
                    self.hdata(),
                    self.line_data_pointer,
                    &format!("{}|tags_array", i),
                );
                tags.push(tag);
            }

            tags
        }
    }
}

use std::borrow::Cow;
use std::collections::{HashMap, hash_map::Keys};
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr;
use std::time::{SystemTime, Duration};

use weechat_sys::{t_infolist, t_weechat_plugin, t_gui_buffer};

use crate::{LossyCString, Weechat};
use crate::buffer::{Buffer, InnerBuffers,  InnerBuffer};

pub struct Infolist<'a> {
    ptr: *mut t_infolist,
    infolist_name: String,
    weechat_ptr: *mut t_weechat_plugin,
    phantom_weechat: PhantomData<&'a Weechat>,
}

#[derive(Eq, Hash, PartialEq)]
pub enum InfolistType {
    Integer,
    String,
    Time,
    Buffer,
}

impl From<&str> for InfolistType {
    fn from(value: &str) -> Self {
        match value {
            "i" => InfolistType::Integer,
            "s" => InfolistType::String,
            "t" => InfolistType::Time,
            "p" => InfolistType::Buffer,
            v => panic!("Got unexprected value {}", v)
        }
    }
}

pub struct InfolistItem<'a> {
    ptr: *mut t_infolist,
    weechat_ptr: *mut t_weechat_plugin,
    fields: HashMap<String, InfolistType>,
    infolist: PhantomData<&'a Infolist<'a>>,
}

impl<'a> InfolistItem<'a> {
    fn integer(&self, name: &str) -> i32 {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let name = LossyCString::new(name);

        let infolist_integer = weechat.get().infolist_integer.unwrap();

        unsafe {
            infolist_integer(self.ptr, name.as_ptr())
        }
    }

    fn string(&self, name: &str) -> Option<Cow<str>> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let name = LossyCString::new(name);

        let infolist_string = weechat.get().infolist_string.unwrap();

        unsafe {
            let ptr = infolist_string(self.ptr, name.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy())
            }
        }
    }

    fn buffer(&self, name: &str) -> Option<Buffer> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let name = LossyCString::new(name);

        let infolist_pointer = weechat.get().infolist_pointer.unwrap();

        let ptr = unsafe {
            infolist_pointer(self.ptr, name.as_ptr()) as *mut t_gui_buffer
        };

        if ptr.is_null() {
            return None;
        }

        Some(Buffer {
            inner: InnerBuffers::BorrowedBuffer(InnerBuffer {
                weechat: self.weechat_ptr,
                ptr,
                weechat_phantom: PhantomData,
            }),
        })
    }

    fn time(&self, name: &str) -> SystemTime {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let name = LossyCString::new(name);

        let infolist_time = weechat.get().infolist_time.unwrap();

        let time = unsafe {
            infolist_time(self.ptr, name.as_ptr())
        };

        let unix = SystemTime::UNIX_EPOCH;
        let duration = Duration::from_secs(time as u64);

        unix.checked_add(duration).unwrap()
    }

    pub fn get(&self, key: &str) -> Option<InfolistVariable> {
        let infolist_type = self.fields.get(key)?;

        let variable = match infolist_type {
            InfolistType::Integer => InfolistVariable::Integer(self.integer(key)),
            InfolistType::String => InfolistVariable::String(self.string(key)?),
            InfolistType::Time => InfolistVariable::Time(self.time(key)),
            InfolistType::Buffer => InfolistVariable::Buffer(self.buffer(key)?),
        };

        Some(variable)
    }

    pub fn keys(&self) -> Keys<'_, String, InfolistType> {
        self.fields.keys()
    }
}

pub enum InfolistVariable<'a> {
    Integer(i32),
    String(Cow<'a, str>),
    Time(SystemTime),
    Buffer(Buffer<'a>),
}

impl<'a> Infolist<'a> {
    fn get_fields(&self) -> HashMap<String, InfolistType> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let infolist_fields = weechat.get().infolist_fields.unwrap();
        let mut fields: HashMap<String, InfolistType> = HashMap::new();

        let fields_string = unsafe {
            let ptr = infolist_fields(self.ptr);
            CStr::from_ptr(ptr).to_string_lossy()
        };

        for field in fields_string.split(',') {
            let split: Vec<&str> = field.split(':').collect();

            let infolist_type = split[0];
            let name = split[1];

            // Skip the buffer, we can't safely expose them
            // without knowing the size of the buffer. (Note the buffer here 
            // isn't a GUI buffer but a vector like thing.
            if infolist_type == "b" {
                continue;
            }

            let field = if infolist_type == "p" {
                // TODO this should be in a static hashmap, there are more
                // infolists that contain buffer pointers.
                if self.infolist_name == "logger_buffer" && name == "buffer" {
                    InfolistType::Buffer
                } else {
                    continue
                }
            } else {
                InfolistType::from(infolist_type)
            };

            fields.insert(name.to_owned(), field);
        }

        fields
    }
}

impl<'a> Drop for Infolist<'a> {
    fn drop(&mut self) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let infolist_free = weechat.get().infolist_free.unwrap();

        unsafe {
            infolist_free(self.ptr)
        }
    }
}

impl Weechat {
    pub fn get_infolist(
        &self,
        infolist_name: &str,
        arguments: Option<&str>,
    ) -> Result<Infolist, ()> {
        let infolist_get = self.get().infolist_get.unwrap();

        let name = LossyCString::new(infolist_name);
        let arguments = if let Some(args) = arguments {
            Some(LossyCString::new(args))
        } else {
            None
        };

        let infolist_ptr = unsafe {
            infolist_get(
                self.ptr,
                name.as_ptr(),
                ptr::null_mut(),
                arguments.map_or(ptr::null_mut(), |a| a.as_ptr()),
            )
        };

        if infolist_ptr.is_null() {
            Err(())
        } else {
            Ok(Infolist {
                ptr: infolist_ptr,
                infolist_name: infolist_name.to_owned(),
                weechat_ptr: self.ptr,
                phantom_weechat: PhantomData,
            })
        }
    }
}

impl<'a> Iterator for Infolist<'a> {
    type Item = InfolistItem<'a>;

    fn next(&mut self) -> Option<InfolistItem<'a>> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let infolist_next = weechat.get().infolist_next.unwrap();

        let ret = unsafe { infolist_next(self.ptr) };

        if ret == 1 {
            let fields = self.get_fields();

            Some(InfolistItem {
                ptr: self.ptr,
                weechat_ptr: self.weechat_ptr,
                fields,
                infolist: PhantomData,
            })
        } else {
            None
        }
    }
}

impl<'a> DoubleEndedIterator for Infolist<'a> {
    fn next_back(&mut self) -> Option<InfolistItem<'a>> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let infolist_prev = weechat.get().infolist_prev.unwrap();

        let ret = unsafe { infolist_prev(self.ptr) };

        if ret == 1 {
            let fields = self.get_fields();

            Some(InfolistItem {
                ptr: self.ptr,
                weechat_ptr: self.weechat_ptr,
                fields,
                infolist: PhantomData,
            })
        } else {
            None
        }
    }
}

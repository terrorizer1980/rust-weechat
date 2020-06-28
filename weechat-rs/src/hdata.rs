use weechat_sys::t_hdata;
use std::ffi::c_void;

use crate::{LossyCString, Weechat};

impl Weechat {
    pub(crate) unsafe fn hdata_get(&self, name: &str) -> *mut t_hdata {
        let hdata_get = self.get().hdata_get.unwrap();

        let name = LossyCString::new(name);

        hdata_get(self.ptr, name.as_ptr())
    }

    pub(crate) unsafe fn hdata_pointer(&self, hdata: *mut t_hdata, pointer: *mut c_void, name: &str) -> *mut c_void {
        let hdata_pointer = self.get().hdata_pointer.unwrap();
        let name = LossyCString::new(name);

        hdata_pointer(hdata, pointer, name.as_ptr())
    }

    pub(crate) unsafe fn hdata_integer(&self, hdata: *mut t_hdata, pointer: *mut c_void, name: &str) -> i32 {
        let hdata_integer = self.get().hdata_integer.unwrap();
        let name = LossyCString::new(name);

        hdata_integer(hdata, pointer, name.as_ptr())
    }
}

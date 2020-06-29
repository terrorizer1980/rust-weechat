use std::borrow::Cow;
use std::ffi::c_void;
use std::ffi::CStr;
use std::collections::HashMap;

use weechat_sys::{t_hdata, t_hashtable};

use crate::{LossyCString, Weechat};

impl Weechat {
    fn hashmap_to_weechat(&self, hashmap: HashMap<String, String>) -> *mut t_hashtable {
        let hashtable_new = self.get().hashtable_new.unwrap();

        for (key, value) in hashmap {
            let key = LossyCString::new(key);
            let value = LossyCString::new(value);
        }
        todo!()
    }
}

//! A module providing a typed api for Weechat configuration files

use crate::config::{BaseConfigOption, BorrowedOption, ConfigOption, HidenConfigOptionT};
use crate::ConfigSection;
use crate::Weechat;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ffi::CStr;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Represents the settings for a new boolean config option.
#[derive(Default)]
pub struct StringOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: String,

    pub(crate) value: String,

    pub(crate) null_allowed: bool,

    pub(crate) change_cb: Option<Box<dyn FnMut(&StringOpt)>>,

    pub(crate) check_cb: Option<Box<dyn FnMut(&StringOpt, Cow<str>)>>,

    pub(crate) delete_cb: Option<Box<dyn FnMut(&StringOpt)>>,
}

impl StringOptionSettings {
    pub fn new<N: Into<String>>(name: N) -> Self {
        StringOptionSettings {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn description<D: Into<String>>(mut self, descritpion: D) -> Self {
        self.description = descritpion.into();
        self
    }

    pub fn default_value<V: Into<String>>(mut self, value: V) -> Self {
        self.default_value = value.into();
        self
    }

    pub fn value<V: Into<String>>(mut self, value: V) -> Self {
        self.value = value.into();
        self
    }

    pub fn null_allowed(mut self, value: bool) -> Self {
        self.null_allowed = value;
        self
    }

    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&StringOpt) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
    pub fn set_check_callback(
        mut self,
        callback: impl FnMut(&StringOpt, Cow<str>) + 'static,
    ) -> Self {
        self.check_cb = Some(Box::new(callback));
        self
    }
    pub fn set_delete_callback(
        mut self,
        callback: impl FnMut(&StringOpt) + 'static,
    ) -> Self {
        self.delete_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a boolean value.
pub struct StringOption<'a> {
    pub(crate) inner: StringOpt,
    pub(crate) section: PhantomData<&'a ConfigSection>,
}

pub struct StringOpt {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl BorrowedOption for StringOpt {
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self {
        StringOpt {
            ptr: option_ptr,
            weechat_ptr,
        }
    }
}

impl<'a> Deref for StringOption<'a> {
    type Target = StringOpt;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl HidenConfigOptionT for StringOpt {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> HidenConfigOptionT for StringOption<'a> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> BaseConfigOption for StringOption<'a> {}
impl BaseConfigOption for StringOpt {}

impl<'a> ConfigOption<'a> for StringOpt {
    type R = Cow<'a, str>;

    fn value(&self) -> Self::R {
        let weechat = self.get_weechat();
        let config_string = weechat.get().config_string.unwrap();
        unsafe {
            let string = config_string(self.get_ptr());
            CStr::from_ptr(string).to_string_lossy()
        }
    }
}

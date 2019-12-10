use crate::config::{
    BaseConfigOption, BorrowedOption, ConfigOption, ConfigSection,
    HiddenBorrowedOption, HidenConfigOptionT,
};
use crate::Weechat;
use std::marker::PhantomData;
use std::ops::Deref;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Represents the settings for a new integer config option.
#[derive(Default)]
pub struct IntegerOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: i32,

    pub(crate) value: i32,

    pub(crate) min: i32,

    pub(crate) max: i32,

    pub(crate) string_values: String,

    pub(crate) change_cb: Option<Box<dyn FnMut(&IntegerOpt)>>,
}

impl IntegerOptionSettings {
    pub fn new<N: Into<String>>(name: N) -> Self {
        IntegerOptionSettings {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn description<D: Into<String>>(mut self, descritpion: D) -> Self {
        self.description = descritpion.into();
        self
    }

    pub fn default_value<V: Into<i32>>(mut self, value: V) -> Self {
        self.default_value = value.into();
        self
    }

    pub fn value<V: Into<i32>>(mut self, value: V) -> Self {
        self.value = value.into();
        self
    }

    pub fn string_values<V: Into<String>>(mut self, value: V) -> Self {
        self.string_values = value.into();
        self
    }

    pub fn min(mut self, value: i32) -> Self {
        self.min = value;
        self
    }

    pub fn max(mut self, value: i32) -> Self {
        self.max = value;
        self
    }

    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&IntegerOpt) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a boolean value.
pub struct IntegerOption<'a> {
    pub(crate) inner: IntegerOpt,
    pub(crate) section: PhantomData<&'a ConfigSection>,
}

pub struct IntegerOpt {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl HiddenBorrowedOption for IntegerOpt {
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self {
        IntegerOpt {
            ptr: option_ptr,
            weechat_ptr,
        }
    }
}

impl BorrowedOption for IntegerOpt {}

impl<'a> Deref for IntegerOption<'a> {
    type Target = IntegerOpt;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl HidenConfigOptionT for IntegerOpt {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> HidenConfigOptionT for IntegerOption<'a> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> BaseConfigOption for IntegerOption<'a> {}
impl BaseConfigOption for IntegerOpt {}

impl<'a> ConfigOption<'a> for IntegerOpt {
    type R = i32;

    fn value(&self) -> Self::R {
        let weechat = self.get_weechat();
        let config_integer = weechat.get().config_integer.unwrap();
        unsafe { config_integer(self.get_ptr()) }
    }
}

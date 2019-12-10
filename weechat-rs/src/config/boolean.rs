use crate::config::{
    BaseConfigOption, BorrowedOption, ConfigOption, HidenConfigOptionT,
};
use crate::ConfigSection;
use crate::Weechat;
use std::marker::PhantomData;
use std::ops::Deref;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Represents the settings for a new boolean config option.
#[derive(Default)]
pub struct BooleanOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: bool,

    pub(crate) value: bool,

    pub(crate) change_cb: Option<Box<dyn FnMut(&BooleanOpt)>>,
}

impl BooleanOptionSettings {
    pub fn new<N: Into<String>>(name: N) -> Self {
        BooleanOptionSettings {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn description<D: Into<String>>(mut self, descritpion: D) -> Self {
        self.description = descritpion.into();
        self
    }

    pub fn default_value(mut self, value: bool) -> Self {
        self.default_value = value;
        self
    }

    pub fn value(mut self, value: bool) -> Self {
        self.value = value;
        self
    }

    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&BooleanOpt) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a boolean value.
pub struct BooleanOption<'a> {
    pub(crate) inner: BooleanOpt,
    pub(crate) section: PhantomData<&'a ConfigSection>,
}

pub struct BooleanOpt {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl BorrowedOption for BooleanOpt {
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self {
        BooleanOpt {
            ptr: option_ptr,
            weechat_ptr,
        }
    }
}

impl<'a> Deref for BooleanOption<'a> {
    type Target = BooleanOpt;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl HidenConfigOptionT for BooleanOpt {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> HidenConfigOptionT for BooleanOption<'a> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> BaseConfigOption for BooleanOption<'a> {}
impl BaseConfigOption for BooleanOpt {}

impl<'a> ConfigOption<'a> for BooleanOpt {
    type R = bool;

    fn value(&self) -> Self::R {
        let weechat = self.get_weechat();
        let config_boolean = weechat.get().config_boolean.unwrap();
        let ret = unsafe { config_boolean(self.get_ptr()) };
        ret != 0
    }
}

impl PartialEq<bool> for BooleanOpt {
    fn eq(&self, other: &bool) -> bool {
        self.value() == *other
    }
}

use crate::config::{
    BaseConfigOption, BorrowedOption, ConfigOption, ConfigSection,
    HiddenBorrowedOption, HidenConfigOptionT,
};
use crate::Weechat;
use std::borrow::Cow;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ops::Deref;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Represents the settings for a new color config option.
#[derive(Default)]
pub struct ColorOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: String,

    pub(crate) value: String,

    pub(crate) change_cb: Option<Box<dyn FnMut(&Weechat, &ColorOpt)>>,
}

impl ColorOptionSettings {
    pub fn new<N: Into<String>>(name: N) -> Self {
        ColorOptionSettings {
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

    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&Weechat, &ColorOpt) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a color value.
pub struct ColorOption<'a> {
    pub(crate) inner: ColorOpt,
    pub(crate) section: PhantomData<&'a ConfigSection>,
}

pub struct ColorOpt {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl HiddenBorrowedOption for ColorOpt {
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self {
        ColorOpt {
            ptr: option_ptr,
            weechat_ptr,
        }
    }
}

impl BorrowedOption for ColorOpt {}

impl<'a> Deref for ColorOption<'a> {
    type Target = ColorOpt;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl HidenConfigOptionT for ColorOpt {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> HidenConfigOptionT for ColorOption<'a> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl<'a> BaseConfigOption for ColorOption<'a> {}
impl BaseConfigOption for ColorOpt {}

impl<'a> ConfigOption<'a> for ColorOpt {
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

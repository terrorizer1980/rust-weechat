use crate::config::{
    BaseConfigOption, ConfigOptions, FromPtrs, HidenConfigOptionT,
};
use crate::Weechat;
use std::borrow::Cow;
use std::ffi::CStr;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Represents the settings for a new color config option.
#[derive(Default)]
pub struct ColorOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: String,

    pub(crate) change_cb: Option<Box<dyn FnMut(&Weechat, &ColorOption)>>,
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

    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&Weechat, &ColorOption) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a color value.
#[derive(Debug)]
pub struct ColorOption {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl FromPtrs for ColorOption {
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self {
        ColorOption {
            ptr: option_ptr,
            weechat_ptr,
        }
    }
}

impl HidenConfigOptionT for ColorOption {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl BaseConfigOption for ColorOption {}

impl<'a> ConfigOptions<'a> for ColorOption {
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

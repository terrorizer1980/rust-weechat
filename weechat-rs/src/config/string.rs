use crate::config::{
    BaseConfigOption, ConfigOptions, FromPtrs, HidenConfigOptionT,
};
use crate::Weechat;
use std::borrow::Cow;
use std::ffi::CStr;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Settings for a new string option.
#[derive(Default)]
pub struct StringOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: String,

    pub(crate) change_cb: Option<Box<dyn FnMut(&Weechat, &StringOption)>>,

    pub(crate) check_cb:
        Option<Box<dyn FnMut(&Weechat, &StringOption, Cow<str>) -> bool>>,
}

impl StringOptionSettings {
    /// Create new settings that can be used to create a new string option.
    ///
    /// # Arguments
    /// `name` - The name of the new option.
    pub fn new<N: Into<String>>(name: N) -> Self {
        StringOptionSettings {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the description of the option.
    ///
    /// # Arguments
    /// `description` - The description of the new option.
    pub fn description<D: Into<String>>(mut self, descritpion: D) -> Self {
        self.description = descritpion.into();
        self
    }

    /// Set the default value of the option.
    ///
    /// This is the value the option will have if it isn't set by the user. If
    /// the option is reset, the option will take this value.
    ///
    /// # Arguments
    /// `value` - The value that should act as the default value.
    pub fn default_value<V: Into<String>>(mut self, value: V) -> Self {
        self.default_value = value.into();
        self
    }

    /// Set the callback that will run when the value of the option changes.
    ///
    /// # Arguments
    /// `callback` - The callback that will be run.
    ///
    /// # Examples
    /// ```
    /// let settings = StringOptionSettings::new("address")
    ///     .set_change_callback(|weechat, option| {
    ///         weechat.print("Option changed");
    ///     });
    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&Weechat, &StringOption) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }

    /// Set a callback to check the validity of the string option.
    ///
    /// # Arguments
    /// `callback` - The callback that will be run.
    ///
    /// # Examples
    /// ```
    /// let settings = StringOptionSettings::new("address")
    ///     .set_change_callback(|weechat, option| {
    ///         if option.value()
    ///     });
    pub fn set_check_callback(
        mut self,
        callback: impl FnMut(&Weechat, &StringOption, Cow<str>) -> bool + 'static,
    ) -> Self {
        self.check_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a string value.
#[derive(Debug)]
pub struct StringOption {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl FromPtrs for StringOption {
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self {
        StringOption {
            ptr: option_ptr,
            weechat_ptr,
        }
    }
}

impl HidenConfigOptionT for StringOption {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl BaseConfigOption for StringOption {}

impl<'a> ConfigOptions<'a> for StringOption {
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

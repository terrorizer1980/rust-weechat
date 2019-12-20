use crate::config::{
    BaseConfigOption, ConfigOptions, FromPtrs, HidenConfigOptionT,
};
use crate::Weechat;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Settings for a new boolean option.
#[derive(Default)]
pub struct BooleanOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: bool,

    pub(crate) change_cb: Option<Box<dyn FnMut(&Weechat, &BooleanOption)>>,
}

impl BooleanOptionSettings {
    /// Create new settings that can be used to create a new boolean option.
    ///
    /// # Arguments
    /// `name` - The name of the new option.
    pub fn new<N: Into<String>>(name: N) -> Self {
        BooleanOptionSettings {
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
    pub fn default_value(mut self, value: bool) -> Self {
        self.default_value = value;
        self
    }

    /// Set the callback that will run when the value of the option changes.
    ///
    /// # Arguments
    /// `callback` - The callback that will be run.
    ///
    /// # Examples
    /// ```
    /// let settings = BooleanOptionSettings::new("autoconnect")
    ///     .set_change_callback(|weechat, option| {
    ///         weechat.print("Option changed");
    ///     });
    /// ```
    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&Weechat, &BooleanOption) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a boolean value.
#[derive(Debug)]
pub struct BooleanOption {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl FromPtrs for BooleanOption {
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self {
        BooleanOption {
            ptr: option_ptr,
            weechat_ptr,
        }
    }
}

impl HidenConfigOptionT for BooleanOption {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl BaseConfigOption for BooleanOption {}

impl<'a> ConfigOptions<'a> for BooleanOption {
    type R = bool;

    fn value(&self) -> Self::R {
        let weechat = self.get_weechat();
        let config_boolean = weechat.get().config_boolean.unwrap();
        let ret = unsafe { config_boolean(self.get_ptr()) };
        ret != 0
    }
}

impl PartialEq<bool> for BooleanOption {
    fn eq(&self, other: &bool) -> bool {
        self.value() == *other
    }
}

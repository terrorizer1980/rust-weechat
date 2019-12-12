use crate::config::{
    BaseConfigOption, BorrowedOption, ConfigOption, ConfigSection,
    HiddenBorrowedOption, HidenConfigOptionT,
};
use crate::Weechat;
use std::marker::PhantomData;
use std::ops::Deref;
use weechat_sys::{t_config_option, t_weechat_plugin};

/// Settings for building a new integer option.
#[derive(Default)]
pub struct IntegerOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: i32,

    pub(crate) min: i32,

    pub(crate) max: i32,

    pub(crate) string_values: String,

    pub(crate) change_cb: Option<Box<dyn FnMut(&Weechat, &IntegerOpt)>>,
}

impl IntegerOptionSettings {
    /// Create new settings that will create a new option with the given name.
    ///
    /// # Arguments
    ///
    /// `name` - The name of the new option.
    pub fn new<N: Into<String>>(name: N) -> Self {
        IntegerOptionSettings {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the description of the option.
    ///
    /// # Arguments
    ///
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
    ///
    /// `value` - The value that should act as the default value.
    pub fn default_value<V: Into<i32>>(mut self, value: V) -> Self {
        self.default_value = value.into();
        self
    }

    /// Set the string values of the option.
    ///
    /// This setting decides if the integer option should act as an enum taking
    /// symbolic values.
    ///
    /// # Arguments
    ///
    /// `values` - The values that should act as the symbolic values.
    ///
    /// # Examples
    /// ```
    /// let settings = IntegerOptionSettings::new("server_buffer")
    ///     .string_values(vec!["independent", "merged"]);
    ///
    /// let option = section.new_integer_option(settings).expect("Can't create option");
    ///
    /// ```
    pub fn string_values<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let vec: Vec<String> = values.into_iter().map(Into::into).collect();
        self.string_values = vec.join("|");
        self
    }

    /// Set minimal value of the integer option.
    ///
    /// # Arguments
    ///
    /// `value` - The values that should act as minimal valid value.
    pub fn min(mut self, value: i32) -> Self {
        self.min = value;
        self
    }

    /// Set maximum value of the integer option.
    ///
    /// # Arguments
    ///
    /// `value` - The values that should act as maximal valid value.
    pub fn max(mut self, value: i32) -> Self {
        self.max = value;
        self
    }

    /// Set the callback that will run when the value of the option changes.
    ///
    /// # Arguments
    ///
    /// `callback` - The callback that will be run.
    /// # Examples
    /// ```
    /// let settings = IntegerOptionSettings::new("server_buffer")
    ///     .string_values(vec!["independent", "merged"]);
    ///     .set_change_callback(|weechat, option| {
    ///         weechat.print("Option changed");
    ///     });
    /// ```
    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&Weechat, &IntegerOpt) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
}

/// A config option with a integer value.
pub struct IntegerOption<'a> {
    pub(crate) inner: IntegerOpt,
    pub(crate) section: PhantomData<&'a ConfigSection>,
}

/// The borrowed equivalent of the IntegerOption
///
/// This option type will be present in callbacks.
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

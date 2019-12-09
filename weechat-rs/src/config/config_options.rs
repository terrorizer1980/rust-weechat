//! A module providing a typed api for Weechat configuration files

use crate::ConfigSection;
use crate::Weechat;
use std::borrow::Cow;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ops::Deref;
use weechat_sys::{t_config_option, t_weechat_plugin};

#[derive(Default)]
pub(crate) struct OptionDescription<'a> {
    pub name: &'a str,
    pub option_type: OptionType,
    pub description: &'a str,
    pub string_values: &'a str,
    pub min: i32,
    pub max: i32,
    pub default_value: &'a str,
    pub value: &'a str,
    pub null_allowed: bool,
}

pub(crate) enum OptionType {
    Boolean,
    Integer,
    String,
    Color,
}

impl OptionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OptionType::Boolean => "boolean",
            OptionType::Integer => "integer",
            OptionType::String => "string",
            OptionType::Color => "color",
        }
    }
}

impl Default for OptionType {
    fn default() -> Self {
        OptionType::String
    }
}

/// Represents the settings for a new boolean config option.
#[derive(Default)]
pub struct BooleanOptionSettings {
    pub(crate) name: String,

    pub(crate) description: String,

    pub(crate) default_value: bool,

    pub(crate) value: bool,

    pub(crate) null_allowed: bool,

    pub(crate) change_cb: Option<Box<dyn FnMut(&BooleanOpt)>>,

    pub(crate) check_cb: Option<Box<dyn FnMut(&BooleanOpt, Cow<str>)>>,

    pub(crate) delete_cb: Option<Box<dyn FnMut(&BooleanOpt)>>,
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

    pub fn null_allowed(mut self, value: bool) -> Self {
        self.null_allowed = value;
        self
    }

    pub fn set_change_callback(
        mut self,
        callback: impl FnMut(&BooleanOpt) + 'static,
    ) -> Self {
        self.change_cb = Some(Box::new(callback));
        self
    }
    pub fn set_check_callback(
        mut self,
        callback: impl FnMut(&BooleanOpt, Cow<str>) + 'static,
    ) -> Self {
        self.check_cb = Some(Box::new(callback));
        self
    }
    pub fn set_delete_callback(
        mut self,
        callback: impl FnMut(&BooleanOpt) + 'static,
    ) -> Self {
        self.delete_cb = Some(Box::new(callback));
        self
    }
}

pub trait HidenConfigOptionT {
    /// Returns the raw pointer to the config option.
    fn get_ptr(&self) -> *mut t_config_option;
}

pub trait BaseConfigOption: HidenConfigOptionT {
    /// Returns the weechat object that this config option was created with.
    fn get_weechat(&self) -> Weechat;
}

/// A trait that defines common behavior for the different data types of config options.
pub trait ConfigOption: BaseConfigOption {
    type R;

    /// Get the value of the option.
    fn value(&self) -> Self::R;

    /// Resets the option to its default value.
    fn reset(&self, run_callback: bool) -> crate::OptionChanged {
        let weechat = self.get_weechat();
        let option_reset = weechat.get().config_option_reset.unwrap();

        let ret = unsafe { option_reset(self.get_ptr(), run_callback as i32) };

        crate::OptionChanged::from_int(ret)
    }
}

pub(crate) struct OptionPointers<T> {
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
    pub(crate) check_cb: Option<Box<dyn FnMut(&T, Cow<str>)>>,
    pub(crate) change_cb: Option<Box<dyn FnMut(&T)>>,
    pub(crate) delete_cb: Option<Box<dyn FnMut(&T)>>,
}

/// A config option with a string value.
pub struct StringOption<'a> {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
    pub(crate) section: PhantomData<&'a ConfigSection>,
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

pub trait BorrowedOption {
    /// Returns the raw pointer to the config option.
    fn from_ptrs(
        option_ptr: *mut t_config_option,
        weechat_ptr: *mut t_weechat_plugin,
    ) -> Self;
}

/// A config option with a integer value.
pub struct IntegerOption<'a> {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
    pub(crate) section: PhantomData<&'a ConfigSection>,
}

/// A config option with a color value.
pub struct ColorOption<'a> {
    pub(crate) ptr: *mut t_config_option,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
    pub(crate) section: PhantomData<&'a ConfigSection>,
}

impl HidenConfigOptionT for StringOption<'_> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }
}

impl HidenConfigOptionT for BooleanOpt {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }
}

impl HidenConfigOptionT for ColorOption<'_> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }
}

impl HidenConfigOptionT for IntegerOption<'_> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }
}

impl BaseConfigOption for StringOption<'_> {
    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl BaseConfigOption for BooleanOpt {
    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl BaseConfigOption for ColorOption<'_> {
    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl BaseConfigOption for IntegerOption<'_> {
    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

// impl<'a> ConfigOption<'a> for StringOption<'a> {
//     type R = Cow<'a, str>;

//     fn value(&self) -> Self::R {
//         let weechat = self.get_weechat();
//         let config_string = weechat.get().config_string.unwrap();
//         unsafe {
//             let string = config_string(self.get_ptr());
//             CStr::from_ptr(string).to_string_lossy()
//         }
//     }
// }

impl ConfigOption for BooleanOpt {
    type R = bool;

    fn value(&self) -> Self::R {
        let weechat = self.get_weechat();
        let config_boolean = weechat.get().config_boolean.unwrap();
        let ret = unsafe { config_boolean(self.get_ptr()) };
        ret != 0
    }
}

// impl ConfigOption for IntegerOption {
//     type R = i32;

//     fn value(&self) -> Self::R {
//         let weechat = self.get_weechat();
//         let config_integer = weechat.get().config_integer.unwrap();
//         unsafe { config_integer(self.get_ptr()) }
//     }
// }

impl<'a> ConfigOption for ColorOption<'a> {
    type R = Cow<'a, str>;

    fn value(&self) -> Self::R {
        let weechat = self.get_weechat();
        let config_color = weechat.get().config_color.unwrap();
        unsafe {
            let string = config_color(self.get_ptr());
            CStr::from_ptr(string).to_string_lossy()
        }
    }
}

impl PartialEq<bool> for BooleanOpt {
    fn eq(&self, other: &bool) -> bool {
        self.value() == *other
    }
}

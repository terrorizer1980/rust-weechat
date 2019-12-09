//! A module providing a typed api for Weechat configuration files

use crate::ConfigSection;
use crate::Weechat;
use std::borrow::Cow;
use std::marker::PhantomData;
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

pub trait HidenConfigOptionT {
    /// Returns the raw pointer to the config option.
    fn get_ptr(&self) -> *mut t_config_option;
    fn get_weechat(&self) -> Weechat;
}

pub trait BaseConfigOption: HidenConfigOptionT {}

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

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl HidenConfigOptionT for ColorOption<'_> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

    fn get_weechat(&self) -> Weechat {
        Weechat::from_ptr(self.weechat_ptr)
    }
}

impl HidenConfigOptionT for IntegerOption<'_> {
    fn get_ptr(&self) -> *mut t_config_option {
        self.ptr
    }

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

// impl ConfigOption for IntegerOption {
//     type R = i32;

//     fn value(&self) -> Self::R {
//         let weechat = self.get_weechat();
//         let config_integer = weechat.get().config_integer.unwrap();
//         unsafe { config_integer(self.get_ptr()) }
//     }
// }

// impl<'a> ConfigOption for ColorOption<'a> {
//     type R = Cow<'a, str>;

//     fn value(&self) -> Self::R {
//         let weechat = self.get_weechat();
//         let config_color = weechat.get().config_color.unwrap();
//         unsafe {
//             let string = config_color(self.get_ptr());
//             CStr::from_ptr(string).to_string_lossy()
//         }
//     }
// }

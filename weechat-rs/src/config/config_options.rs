//! A module providing a typed api for Weechat configuration files

use crate::ConfigSection;
use crate::Weechat;
use crate::LossyCString;
use std::borrow::Cow;
use std::ffi::CStr;
use std::convert::TryFrom;
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

pub enum OptionType {
    Boolean,
    Integer,
    String,
    Color,
}

impl TryFrom<&str> for OptionType {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let ret = match value {
            "boolean" => OptionType::Boolean,
            "integer" => OptionType::Integer,
            "string" => OptionType::String,
            "color" => OptionType::Color,
            _ => return Err("Invalid option type"),
        };

        Ok(ret)
    }
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

    fn get_string(&self, property: &str) -> Option<Cow<str>> {
        let weechat = self.get_weechat();
        let get_string = weechat.get().config_option_get_string.unwrap();
        let property = LossyCString::new(property);

        unsafe {
            let string = get_string(self.get_ptr(), property.as_ptr());
            if string.is_null() {
                None
            } else {
                Some(CStr::from_ptr(string).to_string_lossy())
            }
        }
    }
}

/// Base configuration option methods.
///
/// These methods are implemented for every option and don't depend on the
/// option type.
pub trait BaseConfigOption: HidenConfigOptionT {
    /// Get the name of the option.
    fn name(&self) -> Cow<str> {
        self.get_string("name").expect("Can't get the name of the option")
    }

    /// Get the description of the option.
    fn description(&self) -> Cow<str> {
        self.get_string("description").expect("Can't get the description of the option")
    }

    /// Get the section name of the section the option belongs to.
    fn section_name(&self) -> Cow<str> {
        self.get_string("section_name").expect("Can't get the section name of the option")
    }

    /// Get the config name the option belongs to.
    fn config_name(&self) -> Cow<str> {
        self.get_string("config_name").expect("Can't get the config name of the option")
    }

    /// Get the type of the config option
    fn option_type(&self) -> OptionType {
        let option_type = self.get_string("type").expect("Can't get the config name of the option");
        OptionType::try_from(option_type.as_ref()).unwrap()
    }

    /// Resets the option to its default value.
    fn reset(&self, run_callback: bool) -> crate::OptionChanged {
        let weechat = self.get_weechat();
        let option_reset = weechat.get().config_option_reset.unwrap();

        let ret = unsafe { option_reset(self.get_ptr(), run_callback as i32) };

        crate::OptionChanged::from_int(ret)
    }

    /// Is the option undefined/null.
    fn is_null(&self) -> bool {
        let weechat = self.get_weechat();
        let is_null = weechat.get().config_option_is_null.unwrap();

        let ret = unsafe { is_null(self.get_ptr()) };

        ret != 0
    }

}

/// A trait that defines common behavior for the different data types of config options.
pub trait ConfigOption<'a>: BaseConfigOption {
    /// The return type of the config option.
    type R;

    /// Get the value of the option.
    fn value(&self) -> Self::R;
}

pub(crate) struct OptionPointers<T> {
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
    pub(crate) check_cb: Option<Box<dyn FnMut(&T, Cow<str>)>>,
    pub(crate) change_cb: Option<Box<dyn FnMut(&T)>>,
    pub(crate) delete_cb: Option<Box<dyn FnMut(&T)>>,
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

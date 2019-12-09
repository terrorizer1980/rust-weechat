//! Weechat Configuration module

use libc::{c_char, c_int};
use std::collections::HashMap;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;

use crate::config_options::{
    BaseConfigOption, BooleanOption, BooleanOptionSettings, BorrowedOption,
    ColorOption, ConfigOption, IntegerOption, OptionDescription,
    OptionPointers, OptionType, StringOption, BooleanOpt
};
use crate::{LossyCString, Weechat};
use std::borrow::Cow;
use weechat_sys::{
    t_config_file, t_config_option, t_config_section, t_weechat_plugin,
    WEECHAT_RC_OK,
};

/// Weechat configuration file
pub struct Config {
    ptr: *mut t_config_file,
    weechat_ptr: *mut t_weechat_plugin,
    _config_data: Box<ConfigPointers>,
    sections: HashMap<String, ConfigSection>,
}

struct ConfigPointers {
    reload_cb: Box<dyn FnMut()>,
}

struct ConfigSectionPointers {
    read_cb: Option<Box<dyn FnMut(&str, &str)>>,
    write_cb: Option<Box<dyn FnMut(&str)>>,
}

/// Weechat Configuration section
pub struct ConfigSection {
    ptr: *mut t_config_section,
    config_ptr: *mut t_config_file,
    weechat_ptr: *mut t_weechat_plugin,
    section_data: *const c_void,
}

/// Represents the options when creating a new config section.
#[derive(Default)]
pub struct ConfigSectionSettings {
    name: String,

    read_callback: Option<Box<dyn FnMut(&str, &str)>>,

    /// A function called when the section is written to the disk
    write_callback: Option<Box<dyn FnMut(&str)>>,

    /// A function called when default values for the section must be written to the disk
    write_default_callback: Option<Box<dyn FnMut()>>,
}

impl ConfigSectionSettings {
    /// Create a new config section info.
    /// This can be passed to a config which will create a new ConfigSection.
    /// #Arguments
    /// `name` - The name that the section should get.
    pub fn new<P: Into<String>>(name: P) -> Self {
        ConfigSectionSettings {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the function that will be called when an option from the section is
    /// read from the disk.
    ///
    /// #Arguments
    /// `callback` - The callback for a section read operation.
    pub fn set_read_callback(
        mut self,
        callback: impl FnMut(&str, &str) + 'static,
    ) -> Self {
        self.read_callback = Some(Box::new(callback));
        self
    }
    pub fn set_write_callback(
        mut self,
        callback: impl FnMut(&str) + 'static,
    ) -> Self {
        self.write_callback = Some(Box::new(callback));
        self
    }
    pub fn set_write_default_callback(
        mut self,
        callback: impl FnMut() + 'static,
    ) -> Self {
        self.write_default_callback = Some(Box::new(callback));
        self
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let config_free = weechat.get().config_free.unwrap();

        // Drop the sections first.
        self.sections.clear();

        unsafe {
            // Now drop the config.
            config_free(self.ptr)
        };
    }
}

impl Drop for ConfigSection {
    fn drop(&mut self) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let options_free = weechat.get().config_section_free_options.unwrap();
        let section_free = weechat.get().config_section_free.unwrap();

        unsafe {
            Box::from_raw(self.section_data as *mut ConfigSectionPointers);
            options_free(self.ptr);
            section_free(self.ptr);
        };
    }
}

type SectionReadCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    _data: *mut c_void,
    _config: *mut t_config_file,
    _section: *mut t_config_section,
    option_name: *const i8,
    value: *const i8,
) -> c_int;

type SectionWriteCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    _data: *mut c_void,
    _config: *mut t_config_file,
    section_name: *const c_char,
) -> c_int;

impl Config {
    /// Create a new section in the configuration file.
    pub fn new_section(
        &mut self,
        section_info: ConfigSectionSettings,
    ) -> &ConfigSection {
        unsafe extern "C" fn c_read_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            _config: *mut t_config_file,
            _section: *mut t_config_section,
            option_name: *const c_char,
            value: *const c_char,
        ) -> c_int {
            let option_name = CStr::from_ptr(option_name).to_string_lossy();
            let value = CStr::from_ptr(value).to_string_lossy();
            let pointers: &mut ConfigSectionPointers =
                { &mut *(pointer as *mut ConfigSectionPointers) };

            if let Some(ref mut callback) = pointers.read_cb {
                callback(option_name.as_ref(), value.as_ref())
            }
            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_write_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            _config: *mut t_config_file,
            section_name: *const c_char,
        ) -> c_int {
            let section_name = CStr::from_ptr(section_name).to_string_lossy();

            let pointers: &mut ConfigSectionPointers =
                { &mut *(pointer as *mut ConfigSectionPointers) };

            if let Some(ref mut callback) = pointers.write_cb {
                callback(section_name.as_ref())
            }
            WEECHAT_RC_OK
        }

        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let new_section = weechat.get().config_new_section.unwrap();

        let name = LossyCString::new(&section_info.name);

        let (c_read_cb, read_cb) = match section_info.read_callback {
            Some(cb) => (Some(c_read_cb as SectionReadCbT), Some(cb)),
            None => (None, None),
        };

        let (c_write_cb, write_cb) = match section_info.write_callback {
            Some(cb) => (Some(c_write_cb as SectionWriteCbT), Some(cb)),
            None => (None, None),
        };

        let section_data =
            Box::new(ConfigSectionPointers { read_cb, write_cb });
        let section_data_ptr = Box::leak(section_data);

        let ptr = unsafe {
            new_section(
                self.ptr,
                name.as_ptr(),
                0,
                0,
                c_read_cb,
                section_data_ptr as *const _ as *const c_void,
                ptr::null_mut(),
                c_write_cb,
                section_data_ptr as *const _ as *const c_void,
                ptr::null_mut(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        let section = ConfigSection {
            ptr,
            config_ptr: self.ptr,
            weechat_ptr: weechat.ptr,
            section_data: section_data_ptr as *const _ as *const c_void,
        };
        self.sections.insert(section_info.name.clone(), section);
        &self.sections[&section_info.name]
    }

    pub fn search_section(&self, section_name: &str) -> Option<&ConfigSection> {
        self.sections.get(section_name)
    }

    pub fn write_line(&self, option_name: &str, value: Option<&str>) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let write_line = weechat.get().config_write_line.unwrap();

        let option_name = LossyCString::new(option_name);

        let c_value = match value {
            Some(v) => LossyCString::new(v).as_ptr(),
            None => ptr::null(),
        };

        unsafe {
            write_line(self.ptr, option_name.as_ptr(), c_value);
        }
    }

    pub fn write_option(&self, option: &dyn BaseConfigOption) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let write_option = weechat.get().config_write_option.unwrap();
        unsafe {
            write_option(self.ptr, option.get_ptr());
        }
    }
}

type WeechatOptChangeCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    _data: *mut c_void,
    option_pointer: *mut t_config_option,
);

type WeechatOptCheckCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    _data: *mut c_void,
    option_pointer: *mut t_config_option,
    value: *const c_char,
) -> c_int;

impl ConfigSection {
    /// Create a new string Weechat configuration option.
    // pub fn new_string_option<D>(
    //     &self,
    //     name: &str,
    //     description: &str,
    //     default_value: &str,
    //     value: &str,
    //     null_allowed: bool,
    //     change_cb: impl FnMut(&mut D, &StringOption),
    // ) -> StringOption
    // where
    //     D: Default,
    // {
    //     let ptr = self.new_option(
    //         OptionDescription {
    //             name,
    //             description,
    //             option_type: OptionType::String,
    //             default_value,
    //             value,
    //             null_allowed,
    //             ..Default::default()
    //         },
    //         None,
    //         None::<String>,
    //         Box::new(change_cb),
    //         None,
    //         None::<String>,
    //     );
    //     StringOption {
    //         ptr,
    //         weechat_ptr: self.weechat_ptr,
    //         section: PhantomData,
    //     }
    // }

    /// Create a new boolean Weechat configuration option.
    pub fn new_boolean_option(
        &self,
        settings: BooleanOptionSettings,
    ) -> BooleanOption {
        let value = if settings.value { "on" } else { "off" };
        let default_value = if settings.default_value { "on" } else { "off" };
        let ptr = self.new_option(
            OptionDescription {
                name: &settings.name,
                description: &settings.description,
                option_type: OptionType::Boolean,
                default_value,
                value,
                null_allowed: settings.null_allowed,
                ..Default::default()
            },
            settings.check_cb,
            settings.change_cb,
            settings.delete_cb,
        );
        BooleanOption {
            inner: BooleanOpt::from_ptrs(ptr, self.weechat_ptr),
            section: PhantomData,
        }
    }

    /// Create a new integer Weechat configuration option.
    // pub fn new_integer_option<D>(
    //     &self,
    //     name: &str,
    //     description: &str,
    //     string_values: &str,
    //     min: i32,
    //     max: i32,
    //     default_value: &str,
    //     value: &str,
    //     null_allowed: bool,
    //     change_cb: Option<fn(&mut D, &IntegerOption)>,
    //     change_cb_data: Option<D>,
    // ) -> IntegerOption
    // where
    //     D: Default,
    // {
    //     let ptr = self.new_option(
    //         OptionDescription {
    //             name,
    //             option_type: OptionType::Integer,
    //             description,
    //             string_values,
    //             min,
    //             max,
    //             default_value,
    //             value,
    //             null_allowed,
    //         },
    //         None,
    //         None::<String>,
    //         change_cb,
    //         change_cb_data,
    //         None,
    //         None::<String>,
    //     );
    //     IntegerOption {
    //         ptr,
    //         weechat_ptr: self.weechat_ptr,
    //         section: PhantomData,
    //     }
    // }

    // /// Create a new color Weechat configuration option.
    // pub fn new_color_option<D>(
    //     &self,
    //     name: &str,
    //     description: &str,
    //     default_value: &str,
    //     value: &str,
    //     null_allowed: bool,
    //     change_cb: Option<fn(&mut D, &ColorOption)>,
    // ) -> ColorOption
    // where
    //     D: Default,
    // {
    //     let ptr = self.new_option(
    //         OptionDescription {
    //             name,
    //             description,
    //             option_type: OptionType::Color,
    //             default_value,
    //             value,
    //             null_allowed,
    //             ..Default::default()
    //         },
    //         None,
    //         change_cb,
    //         None,
    //     );
    //     ColorOption {
    //         ptr,
    //         weechat_ptr: self.weechat_ptr,
    //         section: PhantomData,
    //     }
    // }

    fn new_option<T>(
        &self,
        option_description: OptionDescription,
        check_cb: Option<Box<dyn FnMut(&T, Cow<str>)>>,
        change_cb: Option<Box<dyn FnMut(&T)>>,
        delete_cb: Option<Box<dyn FnMut(&T)>>,
    ) -> *mut t_config_option
    where
        T: BorrowedOption,
    {
        unsafe extern "C" fn c_check_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            option_pointer: *mut t_config_option,
            value: *const c_char,
        ) -> c_int
        where
            T: BorrowedOption,
        {
            let value = CStr::from_ptr(value).to_string_lossy();
            let pointers: &mut OptionPointers<T> =
                { &mut *(pointer as *mut OptionPointers<T>) };

            let option = T::from_ptrs(option_pointer, pointers.weechat_ptr);

            if let Some(callback) = &mut pointers.check_cb {
                callback(&option, value)
            };

            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_change_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            option_pointer: *mut t_config_option,
        ) where
            T: BorrowedOption
        {
            let pointers: &mut OptionPointers<T> =
                { &mut *(pointer as *mut OptionPointers<T>) };

            let option = T::from_ptrs(option_pointer, pointers.weechat_ptr);

            if let Some(callback) = &mut pointers.change_cb {
                callback(&option)
            };
        }

        unsafe extern "C" fn c_delete_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            option_pointer: *mut t_config_option,
        ) where
            T: BorrowedOption
        {
            let pointers: &mut OptionPointers<T> =
                { &mut *(pointer as *mut OptionPointers<T>) };

            let option = T::from_ptrs(option_pointer, pointers.weechat_ptr);

            if let Some(callback) = &mut pointers.delete_cb {
                callback(&option)
            };
        }

        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let name = LossyCString::new(option_description.name);
        let description = LossyCString::new(option_description.description);
        let option_type =
            LossyCString::new(option_description.option_type.as_str());
        let string_values = LossyCString::new(option_description.string_values);
        let default_value = LossyCString::new(option_description.default_value);
        let value = LossyCString::new(option_description.value);

        let c_check_cb = match check_cb {
            Some(_) => Some(c_check_cb::<T> as WeechatOptCheckCbT),
            None => None,
        };

        let c_change_cb: Option<WeechatOptChangeCbT> = match change_cb {
            Some(_) => Some(c_change_cb::<T>),
            None => None,
        };

        let c_delete_cb: Option<WeechatOptChangeCbT> = match delete_cb {
            Some(_) => Some(c_delete_cb::<T>),
            None => None,
        };

        let option_pointers = Box::new(OptionPointers {
            weechat_ptr: self.weechat_ptr,
            check_cb,
            change_cb,
            delete_cb,
        });

        // TODO this currently leaks.
        let option_pointers_ref: &OptionPointers<T> =
            Box::leak(option_pointers);

        let config_new_option = weechat.get().config_new_option.unwrap();
        unsafe {
            config_new_option(
                self.config_ptr,
                self.ptr,
                name.as_ptr(),
                option_type.as_ptr(),
                description.as_ptr(),
                string_values.as_ptr(),
                option_description.min,
                option_description.max,
                default_value.as_ptr(),
                value.as_ptr(),
                option_description.null_allowed as i32,
                c_check_cb,
                option_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
                c_change_cb,
                option_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
                c_delete_cb,
                option_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        }
    }
}

/// Configuration file part of the weechat API.
impl Weechat {
    /// Create a new Weechat configuration file, returns a `Config` object.
    /// The configuration file is freed when the `Config` object is dropped.
    /// * `name` - Name of the new configuration file
    /// * `reload_callback` - Callback that will be called when the
    /// configuration file is reloaded.
    /// * `reload_data` - Data that will be taken over by weechat and passed
    /// to the reload callback, this data will be freed when the `Config`
    /// object returned by this method is dropped.
    pub fn config_new(
        &self,
        name: &str,
        reload_callback: impl FnMut() + 'static,
    ) -> Option<Config> {
        unsafe extern "C" fn c_reload_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            _config_pointer: *mut t_config_file,
        ) -> c_int {
            let pointers: &mut ConfigPointers =
                { &mut *(pointer as *mut ConfigPointers) };

            let cb = &mut pointers.reload_cb;
            cb();

            WEECHAT_RC_OK
        }

        let c_name = LossyCString::new(name);

        let config_pointers = Box::new(ConfigPointers {
            reload_cb: Box::new(reload_callback),
        });
        let config_pointers_ref = Box::leak(config_pointers);

        let config_new = self.get().config_new.unwrap();
        let config_ptr = unsafe {
            config_new(
                self.ptr,
                c_name.as_ptr(),
                Some(c_reload_cb),
                config_pointers_ref as *const _ as *const c_void,
                ptr::null_mut(),
            )
        };

        if config_ptr.is_null() {
            return None;
        };

        let config_data = unsafe { Box::from_raw(config_pointers_ref) };

        Some(Config {
            ptr: config_ptr,
            weechat_ptr: self.ptr,
            _config_data: config_data,
            sections: HashMap::new(),
        })
    }
}

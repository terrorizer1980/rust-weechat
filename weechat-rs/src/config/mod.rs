//! Weechat Configuration module

mod boolean;
mod color;
mod config_options;
mod integer;
mod section;
mod string;

use libc::{c_char, c_int};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;

pub use crate::config::boolean::{
    BooleanOpt, BooleanOption, BooleanOptionSettings,
};
pub use crate::config::color::{ColorOpt, ColorOption, ColorOptionSettings};
pub use crate::config::config_options::{
    BaseConfigOption, BorrowedOption, ConfigOption, OptionType,
};
pub(crate) use crate::config::config_options::{
    HiddenBorrowedOption, HidenConfigOptionT, OptionDescription, OptionPointers,
};
pub use crate::config::integer::{
    IntegerOpt, IntegerOption, IntegerOptionSettings,
};
pub use crate::config::section::{ConfigSection, ConfigSectionSettings};
use crate::config::section::{
    ConfigSectionPointers, SectionReadCbT, SectionWriteCbT,
};
pub use crate::config::string::{
    StringOpt, StringOption, StringOptionSettings,
};
use crate::{LossyCString, Weechat};
use weechat_sys::{
    t_config_file, t_config_section, t_weechat_plugin, WEECHAT_RC_OK,
};

/// Weechat configuration file
pub struct Config {
    inner: Conf,
    _config_data: Box<ConfigPointers>,
    sections: HashMap<String, ConfigSection>,
}

pub struct Conf {
    ptr: *mut t_config_file,
    weechat_ptr: *mut t_weechat_plugin,
}

struct ConfigPointers {
    reload_cb: Box<dyn FnMut(&Weechat, &Conf)>,
    weechat_ptr: *mut t_weechat_plugin,
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
        reload_callback: impl FnMut(&Weechat, &Conf) + 'static,
    ) -> Option<Config> {
        unsafe extern "C" fn c_reload_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            config_pointer: *mut t_config_file,
        ) -> c_int {
            let pointers: &mut ConfigPointers =
                { &mut *(pointer as *mut ConfigPointers) };

            let cb = &mut pointers.reload_cb;
            let conf = Conf {
                ptr: config_pointer,
                weechat_ptr: pointers.weechat_ptr,
            };

            let weechat = Weechat::from_ptr(pointers.weechat_ptr);

            cb(&weechat, &conf);

            WEECHAT_RC_OK
        }

        let c_name = LossyCString::new(name);

        let config_pointers = Box::new(ConfigPointers {
            reload_cb: Box::new(reload_callback),
            weechat_ptr: self.ptr,
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
            inner: Conf {
                ptr: config_ptr,
                weechat_ptr: self.ptr,
            },
            _config_data: config_data,
            sections: HashMap::new(),
        })
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        let weechat = Weechat::from_ptr(self.inner.weechat_ptr);
        let config_free = weechat.get().config_free.unwrap();

        // Drop the sections first.
        self.sections.clear();

        unsafe {
            // Now drop the config.
            config_free(self.inner.ptr)
        };
    }
}

impl Config {
    /// Create a new section in the configuration file.
    pub fn new_section(
        &mut self,
        section_info: ConfigSectionSettings,
    ) -> &ConfigSection {
        unsafe extern "C" fn c_read_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            config: *mut t_config_file,
            _section: *mut t_config_section,
            option_name: *const c_char,
            value: *const c_char,
        ) -> c_int {
            let option_name = CStr::from_ptr(option_name).to_string_lossy();
            let value = CStr::from_ptr(value).to_string_lossy();
            let pointers: &mut ConfigSectionPointers =
                { &mut *(pointer as *mut ConfigSectionPointers) };

            let conf = Conf {
                ptr: config,
                weechat_ptr: pointers.weechat_ptr,
            };
            let weechat = Weechat::from_ptr(pointers.weechat_ptr);

            if let Some(ref mut callback) = pointers.read_cb {
                callback(&weechat, &conf, option_name.as_ref(), value.as_ref())
            }
            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_write_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            config: *mut t_config_file,
            section_name: *const c_char,
        ) -> c_int {
            let section_name = CStr::from_ptr(section_name).to_string_lossy();

            let pointers: &mut ConfigSectionPointers =
                { &mut *(pointer as *mut ConfigSectionPointers) };

            let conf = Conf {
                ptr: config,
                weechat_ptr: pointers.weechat_ptr,
            };
            let weechat = Weechat::from_ptr(pointers.weechat_ptr);

            if let Some(ref mut callback) = pointers.write_cb {
                callback(&weechat, &conf, section_name.as_ref())
            }
            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_write_default_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            config: *mut t_config_file,
            section_name: *const c_char,
        ) -> c_int {
            let section_name = CStr::from_ptr(section_name).to_string_lossy();

            let pointers: &mut ConfigSectionPointers =
                { &mut *(pointer as *mut ConfigSectionPointers) };

            let conf = Conf {
                ptr: config,
                weechat_ptr: pointers.weechat_ptr,
            };
            let weechat = Weechat::from_ptr(pointers.weechat_ptr);

            if let Some(ref mut callback) = pointers.write_default_cb {
                callback(&weechat, &conf, section_name.as_ref())
            }
            WEECHAT_RC_OK
        }

        let weechat = Weechat::from_ptr(self.inner.weechat_ptr);

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

        let (c_write_default_cb, write_default_cb) = match section_info
            .write_default_callback
        {
            Some(cb) => (Some(c_write_default_cb as SectionWriteCbT), Some(cb)),
            None => (None, None),
        };

        let section_data = Box::new(ConfigSectionPointers {
            read_cb,
            write_cb,
            write_default_cb,
            weechat_ptr: self.inner.weechat_ptr,
        });
        let section_data_ptr = Box::leak(section_data);

        let ptr = unsafe {
            new_section(
                self.inner.ptr,
                name.as_ptr(),
                0,
                0,
                c_read_cb,
                section_data_ptr as *const _ as *const c_void,
                ptr::null_mut(),
                c_write_cb,
                section_data_ptr as *const _ as *const c_void,
                ptr::null_mut(),
                c_write_default_cb,
                section_data_ptr as *const _ as *const c_void,
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
            config_ptr: self.inner.ptr,
            weechat_ptr: weechat.ptr,
            section_data: section_data_ptr as *const _ as *const c_void,
        };
        self.sections.insert(section_info.name.clone(), section);
        &self.sections[&section_info.name]
    }
    pub fn search_section(&self, section_name: &str) -> Option<&ConfigSection> {
        self.sections.get(section_name)
    }
}

impl Conf {
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

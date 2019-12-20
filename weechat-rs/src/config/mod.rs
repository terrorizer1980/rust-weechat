//! Weechat Configuration module

mod boolean;
mod color;
mod config_options;
mod integer;
mod section;
mod string;

use libc::{c_char, c_int};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::os::raw::c_void;
use std::ptr;
use std::rc::Rc;

pub use crate::config::boolean::{BooleanOption, BooleanOptionSettings};
pub use crate::config::color::{ColorOption, ColorOptionSettings};
pub use crate::config::integer::{IntegerOption, IntegerOptionSettings};
pub use crate::config::string::{StringOption, StringOptionSettings};

pub use crate::config::config_options::{
    BaseConfigOption, ConfigOptions, OptionType,
};
pub use crate::config::section::{
    ConfigOption, SectionHandle, SectionHandleMut, ConfigSectionSettings, ConfigSection,
};

pub(crate) use crate::config::config_options::{
    FromPtrs, HidenConfigOptionT, OptionDescription, OptionPointers,
};
use crate::config::section::{
    ConfigSectionPointers, SectionReadCbT, SectionWriteCbT,
};
use crate::{LossyCString, Weechat};

use weechat_sys::{
    t_config_file, t_config_section, t_weechat_plugin, WEECHAT_RC_OK,
};

/// Weechat configuration file
pub struct Config {
    inner: Conf,
    _config_data: Box<ConfigPointers>,
    sections: HashMap<String, Rc<RefCell<ConfigSection>>>,
}

/// The borrowed equivalent of the `Config`. Will be present in callbacks.
pub struct Conf {
    ptr: *mut t_config_file,
    weechat_ptr: *mut t_weechat_plugin,
}

struct ConfigPointers {
    reload_cb: Box<dyn FnMut(&Weechat, &Conf)>,
    weechat_ptr: *mut t_weechat_plugin,
}

impl Weechat {
    /// Create a new Weechat configuration file, returns a `Config` object.
    /// The configuration file is freed when the `Config` object is dropped.
    ///
    /// # Arguments
    /// * `name` - Name of the new configuration file
    /// * `reload_callback` - Callback that will be called when the
    /// configuration file is reloaded.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = weechat::new("server_buffer", |weechat, conf| {
    ///     weechat.print("Config was reloaded")
    /// });
    /// ```
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
    fn return_value_to_error(return_value: c_int) -> std::io::Result<()> {
        match return_value {
            weechat_sys::WEECHAT_CONFIG_READ_OK => Ok(()),
            weechat_sys::WEECHAT_CONFIG_READ_FILE_NOT_FOUND => {
                Err(IoError::new(ErrorKind::NotFound, "File was not found"))
            }
            weechat_sys::WEECHAT_CONFIG_READ_MEMORY_ERROR => {
                Err(IoError::new(ErrorKind::Other, "Not enough memory"))
            }
            _ => unreachable!(),
        }
    }

    /// Read the configuration file from the disk.
    pub fn read(&self) -> std::io::Result<()> {
        let weechat = Weechat::from_ptr(self.inner.weechat_ptr);
        let config_read = weechat.get().config_read.unwrap();

        let ret = unsafe { config_read(self.inner.ptr) };

        Config::return_value_to_error(ret)
    }

    /// Write the configuration file to the disk.
    pub fn write(&self) -> std::io::Result<()> {
        let weechat = Weechat::from_ptr(self.inner.weechat_ptr);
        let config_write = weechat.get().config_write.unwrap();

        let ret = unsafe { config_write(self.inner.ptr) };

        Config::return_value_to_error(ret)
    }

    /// Create a new section in the configuration file.
    ///
    /// # Arguments
    /// `section_settings` - Settings that decide how the section will be
    /// created.
    pub fn new_section(
        &mut self,
        section_settings: ConfigSectionSettings,
    ) -> Option<SectionHandleMut> {
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
            let section = pointers
                .section
                .as_ref()
                .expect("Section reference wasn't set up correctly")
                .upgrade()
                .expect("Config has been destroyed but a read callback run");

            let weechat = Weechat::from_ptr(pointers.weechat_ptr);
            weechat.print(&format!("Hello world {:?}", pointers));

            if let Some(ref mut callback) = pointers.read_cb {
                callback(
                    &weechat,
                    &conf,
                    &mut section.borrow_mut(),
                    option_name.as_ref(),
                    value.as_ref(),
                )
            }
            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_write_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            config: *mut t_config_file,
            section_name: *const c_char,
        ) -> c_int {
            let pointers: &mut ConfigSectionPointers =
                { &mut *(pointer as *mut ConfigSectionPointers) };

            let section = pointers
                .section
                .as_ref()
                .expect("Section reference wasn't set up correctly")
                .upgrade()
                .expect("Config has been destroyed but a read callback run");

            let conf = Conf {
                ptr: config,
                weechat_ptr: pointers.weechat_ptr,
            };
            let weechat = Weechat::from_ptr(pointers.weechat_ptr);

            if let Some(ref mut callback) = pointers.write_cb {
                callback(&weechat, &conf, &mut section.borrow_mut())
            }
            WEECHAT_RC_OK
        }

        unsafe extern "C" fn c_write_default_cb(
            pointer: *const c_void,
            _data: *mut c_void,
            config: *mut t_config_file,
            section_name: *const c_char,
        ) -> c_int {
            let pointers: &mut ConfigSectionPointers =
                { &mut *(pointer as *mut ConfigSectionPointers) };

            let section = pointers
                .section
                .as_ref()
                .expect("Section reference wasn't set up correctly")
                .upgrade()
                .expect("Config has been destroyed but a read callback run");

            let conf = Conf {
                ptr: config,
                weechat_ptr: pointers.weechat_ptr,
            };
            let weechat = Weechat::from_ptr(pointers.weechat_ptr);

            if let Some(ref mut callback) = pointers.write_default_cb {
                callback(&weechat, &conf, &mut section.borrow_mut())
            }
            WEECHAT_RC_OK
        }

        let weechat = Weechat::from_ptr(self.inner.weechat_ptr);

        let new_section = weechat.get().config_new_section.unwrap();

        let name = LossyCString::new(&section_settings.name);

        let (c_read_cb, read_cb) = match section_settings.read_callback {
            Some(cb) => (Some(c_read_cb as SectionReadCbT), Some(cb)),
            None => (None, None),
        };

        let (c_write_cb, write_cb) = match section_settings.write_callback {
            Some(cb) => (Some(c_write_cb as SectionWriteCbT), Some(cb)),
            None => (None, None),
        };

        let (c_write_default_cb, write_default_cb) = match section_settings
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
            section: None,
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

        if ptr.is_null() {
            unsafe { Box::from_raw(section_data_ptr) };
            return None;
        };

        let section = ConfigSection {
            ptr,
            config_ptr: self.inner.ptr,
            weechat_ptr: weechat.ptr,
            section_data: section_data_ptr as *const _ as *const c_void,
            name: section_settings.name.clone(),
            option_pointers: HashMap::new(),
            options: HashMap::new(),
        };

        let section = Rc::new(RefCell::new(section));
        let pointers: &mut ConfigSectionPointers = unsafe { &mut *(section_data_ptr as *mut ConfigSectionPointers) };

        pointers.section = Some(Rc::downgrade(&section));

        self.sections.insert(section_settings.name.clone(), section);
        let section = &self.sections[&section_settings.name];

        Some(SectionHandleMut {
            inner: section.borrow_mut(),
        })
    }

    /// Search the configuration object for a section and borrow it.
    ///
    /// # Arguments
    /// `section_name` - The name of the section that should be retrieved.
    pub fn search_section(&self, section_name: &str) -> Option<SectionHandle> {
        if !self.sections.contains_key(section_name) {
            None
        } else {
            Some(SectionHandle {
                inner: self.sections[section_name].borrow(),
            })
        }
    }

    /// Search the configuration object for a section and borrow it mutably.
    ///
    /// # Arguments
    /// `section_name` - The name of the section that should be retrieved.
    pub fn search_section_mut(
        &mut self,
        section_name: &str,
    ) -> Option<SectionHandleMut> {
        if !self.sections.contains_key(section_name) {
            None
        } else {
            Some(SectionHandleMut {
                inner: self.sections[section_name].borrow_mut(),
            })
        }
    }
}

impl Conf {
    /// Write a line in a configuration file.
    ///
    /// # Arguments
    /// `key` - The key of the option that will be written. Can be a
    /// section name.
    /// `value` - The value of the option that will be written. If `None` a
    /// section will be written instead.
    pub fn write_line(&self, key: &str, value: Option<&str>) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let write_line = weechat.get().config_write_line.unwrap();

        let option_name = LossyCString::new(key);

        let c_value = match value {
            Some(v) => LossyCString::new(v).as_ptr(),
            None => ptr::null(),
        };

        unsafe {
            write_line(self.ptr, option_name.as_ptr(), c_value);
        }
    }

    /// Write a line in a configuration file with option and its value.
    ///
    /// # Arguments
    /// `option` - The option that will be written to the configuration file.
    pub fn write_option<O: AsRef<dyn BaseConfigOption>>(&self, option: O) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let write_option = weechat.get().config_write_option.unwrap();
        unsafe {
            write_option(self.ptr, option.as_ref().get_ptr());
        }
    }
}

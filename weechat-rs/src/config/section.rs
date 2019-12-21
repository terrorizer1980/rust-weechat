use libc::{c_char, c_int};
use std::cell::{RefCell, Ref, RefMut};
use std::collections::HashMap;
use std::ffi::CStr;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_void;
use std::ptr;
use std::rc::{Weak};

use weechat_sys::{
    t_config_file, t_config_option, t_config_section, t_weechat_plugin,
};

use crate::config::config_options::CheckCB;
use crate::config::{
    BaseConfigOption,
    BooleanOption, BooleanOptionSettings, ColorOption, ColorOptionSettings,
    Conf, ConfigOptions, FromPtrs, IntegerOption, IntegerOptionSettings,
    StringOption, StringOptionSettings,
};
use crate::config::{OptionDescription, OptionPointers, OptionType};
use crate::{LossyCString, Weechat};

#[derive(Debug)]
#[allow(missing_docs)]
pub enum ConfigOption {
    Boolean(BooleanOption),
    Integer(IntegerOption),
    String(StringOption),
    Color(ColorOption),
}

impl ConfigOption {
    fn boolean(&self) -> &BooleanOption {
        if let ConfigOption::Boolean(o) = self {
            o
        } else {
            panic!("Invalid option type")
        }
    }

    fn integer(&self) -> &IntegerOption {
        if let ConfigOption::Integer(o) = self {
            o
        } else {
            panic!("Invalid option type")
        }
    }

    fn string(&self) -> &StringOption {
        if let ConfigOption::String(o) = self {
            o
        } else {
            panic!("Invalid option type")
        }
    }

    fn color(&self) -> &ColorOption {
        if let ConfigOption::Color(o) = self {
            o
        } else {
            panic!("Invalid option type")
        }
    }

    fn as_base_config_option(&self) -> &(dyn BaseConfigOption + 'static) {
        match &self {
            ConfigOption::Color(ref o) => o,
            ConfigOption::Boolean(ref o) => o,
            ConfigOption::Integer(ref o) => o,
            ConfigOption::String(ref o) => o,
        }
    }
}

impl AsRef<dyn BaseConfigOption> for ConfigOption {
    fn as_ref(&self) -> &(dyn BaseConfigOption + 'static) {
        self.as_base_config_option()
    }
}

#[derive(Debug)]
pub(crate) enum ConfigOptionPointers {
    Boolean(*const c_void),
    Integer(*const c_void),
    String(*const c_void),
    Color(*const c_void),
}

pub struct SectionHandleMut<'a> {
    pub(crate) inner: RefMut<'a, ConfigSection>,
}

pub struct SectionHandle<'a> {
    pub(crate) inner: Ref<'a, ConfigSection>,
}

impl<'a> Deref for SectionHandle<'a> {
    type Target = ConfigSection;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<'a> Deref for SectionHandleMut<'a> {
    type Target = ConfigSection;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<'a> DerefMut for SectionHandleMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

/// Weechat Configuration section
#[derive(Debug)]
pub struct ConfigSection {
    pub(crate) ptr: *mut t_config_section,
    pub(crate) config_ptr: *mut t_config_file,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
    pub(crate) name: String,
    pub(crate) section_data: *const c_void,
    pub(crate) option_pointers: HashMap<String, ConfigOptionPointers>,
}

type ReadCB = dyn FnMut(&Weechat, &Conf, &mut ConfigSection, &str, &str);
type WriteCB = dyn FnMut(&Weechat, &Conf, &mut ConfigSection);

pub(crate) struct ConfigSectionPointers {
    pub(crate) read_cb: Option<Box<ReadCB>>,
    pub(crate) write_cb: Option<Box<WriteCB>>,
    pub(crate) write_default_cb: Option<Box<WriteCB>>,
    pub(crate) section: Option<Weak<RefCell<ConfigSection>>>,
    pub(crate) weechat_ptr: *mut t_weechat_plugin,
}

impl std::fmt::Debug for ConfigSectionPointers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConfigSectionPointers {{ section_ptr: {:?} weechat_ptr: {:?}}}",
            self.section, self.weechat_ptr
        )
    }
}

/// Represents the options when creating a new config section.
#[derive(Default)]
pub struct ConfigSectionSettings {
    pub(crate) name: String,

    pub(crate) read_callback: Option<Box<ReadCB>>,

    /// A function called when the section is written to the disk
    pub(crate) write_callback: Option<Box<WriteCB>>,

    /// A function called when default values for the section must be written to the disk
    pub(crate) write_default_callback: Option<Box<WriteCB>>,
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
    ///
    /// # Examples
    /// ```
    /// let server_section_options = ConfigSectionSettings::new("server")
    ///     .set_read_callback(|weechat, config, section_name| {
    ///         weechat.print("Writing section");
    /// });
    pub fn set_read_callback(
        mut self,
        callback: impl FnMut(&Weechat, &Conf, &mut ConfigSection, &str, &str)
            + 'static,
    ) -> Self {
        self.read_callback = Some(Box::new(callback));
        self
    }

    /// Set the function that will be called when the section is being written
    /// to the file.
    ///
    /// #Arguments
    /// `callback` - The callback for the section write operation.
    ///
    /// # Examples
    /// ```
    /// let server_section_options = ConfigSectionSettings::new("server")
    ///     .set_write_callback(|weechat, config, section| {
    ///         weechat.print("Writing section");
    /// });
    pub fn set_write_callback(
        mut self,
        callback: impl FnMut(&Weechat, &Conf, &mut ConfigSection) + 'static,
    ) -> Self {
        self.write_callback = Some(Box::new(callback));
        self
    }

    /// Set the function that will be called when default values will need to be
    /// written to to the file.
    ///
    /// #Arguments
    pub fn set_write_default_callback(
        mut self,
        callback: impl FnMut(&Weechat, &Conf, &mut ConfigSection) + 'static,
    ) -> Self {
        self.write_default_callback = Some(Box::new(callback));
        self
    }
}

impl Drop for ConfigSection {
    fn drop(&mut self) {
        let weechat = Weechat::from_ptr(self.weechat_ptr);

        let options_free = weechat.get().config_section_free_options.unwrap();
        let section_free = weechat.get().config_section_free.unwrap();

        for (_, option_ptrs) in self.option_pointers.drain() {
            unsafe {
                match option_ptrs {
                    ConfigOptionPointers::Integer(p) => {
                        Box::from_raw(p as *mut OptionPointers<IntegerOption>);
                    }
                    ConfigOptionPointers::Boolean(p) => {
                        Box::from_raw(p as *mut OptionPointers<BooleanOption>);
                    }
                    ConfigOptionPointers::String(p) => {
                        Box::from_raw(p as *mut OptionPointers<StringOption>);
                    }
                    ConfigOptionPointers::Color(p) => {
                        Box::from_raw(p as *mut OptionPointers<ColorOption>);
                    }
                }
            }
        }

        unsafe {
            Box::from_raw(self.section_data as *mut ConfigSectionPointers);
            options_free(self.ptr);
            section_free(self.ptr);
        };
    }
}

pub(crate) type SectionReadCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    _data: *mut c_void,
    _config: *mut t_config_file,
    _section: *mut t_config_section,
    option_name: *const i8,
    value: *const i8,
) -> c_int;

pub(crate) type SectionWriteCbT = unsafe extern "C" fn(
    pointer: *const c_void,
    _data: *mut c_void,
    _config: *mut t_config_file,
    section_name: *const c_char,
) -> c_int;

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
    /// Get the name of the section.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the config options of this section.
    pub fn options(&self) -> Vec<ConfigOption> {
        self.option_pointers
            .keys()
            .map(|option_name| self.search_option(option_name).unwrap())
            .collect()
    }

    /// Search for an option in this section.
    /// # Arguments
    /// `option_name` - The name of the option to search for.
    pub fn search_option(&self, option_name: &str) -> Option<ConfigOption> {
        let weechat = Weechat::from_ptr(self.weechat_ptr);
        let config_search_option = weechat.get().config_search_option.unwrap();
        let name = LossyCString::new(option_name);

        let ptr = unsafe {
            config_search_option(self.config_ptr, self.ptr, name.as_ptr())
        };

        if ptr.is_null() {
            return None;
        }

        let option_type =
            weechat.config_option_get_string(ptr, "type").unwrap();

        Some(Weechat::option_from_type_and_ptr(self.weechat_ptr, ptr, option_type.as_ref()))
    }

    /// Create a new string Weechat configuration option.
    ///
    /// # Arguments
    /// `settings` - Settings that decide how the option should be created.
    ///
    /// Returns None if the option couldn't be created, e.g. if a option with
    /// the same name already exists.
    pub fn new_string_option(
        &mut self,
        settings: StringOptionSettings,
    ) -> Result<StringOption, ()> {
        let ret = self.new_option(
            OptionDescription {
                name: &settings.name,
                description: &settings.description,
                option_type: OptionType::String,
                default_value: &settings.default_value,
                value: &settings.default_value,
                ..Default::default()
            },
            settings.check_cb,
            settings.change_cb,
            None,
        );

        let (ptr, option_pointers) = if let Some((ptr, ptrs)) = ret {
            (ptr, ptrs)
        } else {
            return Err(());
        };

        let option_ptrs = ConfigOptionPointers::String(option_pointers);
        self.option_pointers
            .insert(settings.name.clone(), option_ptrs);

        let option = StringOption {
            ptr,
            weechat_ptr: self.weechat_ptr,
            _phantom: PhantomData,
        };
        Ok(option)
    }

    /// Create a new boolean Weechat configuration option.
    ///
    /// # Arguments
    /// `settings` - Settings that decide how the option should be created.
    ///
    /// Returns None if the option couldn't be created, e.g. if a option with
    /// the same name already exists.
    pub fn new_boolean_option(
        &mut self,
        settings: BooleanOptionSettings,
    ) -> Result<BooleanOption, ()> {
        let value = if settings.default_value { "on" } else { "off" };
        let default_value = if settings.default_value { "on" } else { "off" };
        let ret = self.new_option(
            OptionDescription {
                name: &settings.name,
                description: &settings.description,
                option_type: OptionType::Boolean,
                default_value,
                value,
                ..Default::default()
            },
            None,
            settings.change_cb,
            None,
        );

        let (ptr, option_pointers) = if let Some((ptr, ptrs)) = ret {
            (ptr, ptrs)
        } else {
            return Err(());
        };

        let option_ptrs = ConfigOptionPointers::Boolean(option_pointers);
        self.option_pointers
            .insert(settings.name.clone(), option_ptrs);

        let option = BooleanOption {
            ptr,
            weechat_ptr: self.weechat_ptr,
            _phantom: PhantomData,
        };

        Ok(option)
    }

    /// Create a new integer Weechat configuration option.
    ///
    /// # Arguments
    /// `settings` - Settings that decide how the option should be created.
    ///
    /// Returns None if the option couldn't be created, e.g. if a option with
    /// the same name already exists.
    pub fn new_integer_option(
        &mut self,
        settings: IntegerOptionSettings,
    ) -> Result<IntegerOption, ()> {
        let ret = self.new_option(
            OptionDescription {
                name: &settings.name,
                option_type: OptionType::Integer,
                description: &settings.description,
                string_values: &settings.string_values,
                min: settings.min,
                max: settings.max,
                default_value: &settings.default_value.to_string(),
                value: &settings.default_value.to_string(),
                ..Default::default()
            },
            None,
            settings.change_cb,
            None,
        );

        let (ptr, option_pointers) = if let Some((ptr, ptrs)) = ret {
            (ptr, ptrs)
        } else {
            return Err(());
        };

        let option_ptrs = ConfigOptionPointers::Integer(option_pointers);
        self.option_pointers
            .insert(settings.name.clone(), option_ptrs);

        let option = IntegerOption {
            ptr,
            weechat_ptr: self.weechat_ptr,
            _phantom: PhantomData,
        };
        Ok(option)
    }

    /// Create a new color Weechat configuration option.
    ///
    /// # Arguments
    /// `settings` - Settings that decide how the option should be created.
    ///
    /// Returns None if the option couldn't be created, e.g. if a option with
    /// the same name already exists.
    pub fn new_color_option(
        &mut self,
        settings: ColorOptionSettings,
    ) -> Result<ColorOption, ()> {
        let ret = self.new_option(
            OptionDescription {
                name: &settings.name,
                description: &settings.description,
                option_type: OptionType::Color,
                default_value: &settings.default_value,
                value: &settings.default_value,
                ..Default::default()
            },
            None,
            settings.change_cb,
            None,
        );

        let (ptr, option_pointers) = if let Some((ptr, ptrs)) = ret {
            (ptr, ptrs)
        } else {
            return Err(());
        };

        let option_ptrs = ConfigOptionPointers::Color(option_pointers);
        self.option_pointers
            .insert(settings.name.clone(), option_ptrs);

        let option = ColorOption {
            ptr,
            weechat_ptr: self.weechat_ptr,
            _phantom: PhantomData,
        };
        Ok(option)
    }

    fn new_option<T>(
        &self,
        option_description: OptionDescription,
        check_cb: Option<Box<CheckCB<T>>>,
        change_cb: Option<Box<dyn FnMut(&Weechat, &T)>>,
        delete_cb: Option<Box<dyn FnMut(&Weechat, &T)>>,
    ) -> Option<(*mut t_config_option, *const c_void)>
    where
        T: ConfigOptions<'static>,
    {
        unsafe extern "C" fn c_check_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            option_pointer: *mut t_config_option,
            value: *const c_char,
        ) -> c_int
        where
            T: ConfigOptions<'static>,
        {
            let value = CStr::from_ptr(value).to_string_lossy();
            let pointers: &mut OptionPointers<T> =
                { &mut *(pointer as *mut OptionPointers<T>) };

            let weechat = Weechat::from_ptr(pointers.weechat_ptr);
            let option = T::from_ptrs(option_pointer, pointers.weechat_ptr);

            let ret = if let Some(callback) = &mut pointers.check_cb {
                callback(&weechat, &option, value)
            } else {
                true
            };

            if ret {
                1
            } else {
                0
            }
        }

        unsafe extern "C" fn c_change_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            option_pointer: *mut t_config_option,
        ) where
            T: ConfigOptions<'static>,
        {
            let pointers: &mut OptionPointers<T> =
                { &mut *(pointer as *mut OptionPointers<T>) };

            let weechat = Weechat::from_ptr(pointers.weechat_ptr);
            let option = T::from_ptrs(option_pointer, pointers.weechat_ptr);

            if let Some(callback) = &mut pointers.change_cb {
                callback(&weechat, &option)
            };
        }

        unsafe extern "C" fn c_delete_cb<T>(
            pointer: *const c_void,
            _data: *mut c_void,
            option_pointer: *mut t_config_option,
        ) where
            T: ConfigOptions<'static>,
        {
            let pointers: &mut OptionPointers<T> =
                { &mut *(pointer as *mut OptionPointers<T>) };

            let weechat = Weechat::from_ptr(pointers.weechat_ptr);
            let option = T::from_ptrs(option_pointer, pointers.weechat_ptr);

            if let Some(callback) = &mut pointers.delete_cb {
                callback(&weechat, &option)
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

        let option_pointers_ref: &OptionPointers<T> =
            Box::leak(option_pointers);

        let config_new_option = weechat.get().config_new_option.unwrap();
        let ptr = unsafe {
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
        };

        if ptr.is_null() {
            None
        } else {
            Some((ptr, option_pointers_ref as *const _ as *const c_void))
        }
    }
}

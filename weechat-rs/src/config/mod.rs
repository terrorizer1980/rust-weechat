//! Weechat configuration for plugins.
//!
//! ##
//!
//! ```
//! let mut config = weechat
//!     .config_new("my_plugin", |weechat, conf| {})
//!     .expect("Can't create new config");
//!
//! let server_section_options = ConfigSectionSettings::new("look")
//! let look_section = config.new_section(server_section_options);
//!
//! let use_colors = BooleanOptionSettings::new("use_colors")
//!     .set_change_callback(move |weechat, option| {});
//!
//! let use_colors = server_section.new_boolean_option(use_colors);
//!
//! config.read().expect("Can't read config");
//!
//! ```

mod boolean;
mod color;
#[allow(clippy::module_inception)]
mod config;
mod config_options;
mod integer;
mod section;
mod string;

pub use crate::config::boolean::{BooleanOption, BooleanOptionSettings};
pub use crate::config::color::{ColorOption, ColorOptionSettings};
pub use crate::config::config::{Conf, Config, OptionChanged};
pub use crate::config::integer::{IntegerOption, IntegerOptionSettings};
pub use crate::config::string::{StringOption, StringOptionSettings};

pub use crate::config::config_options::{
    BaseConfigOption, ConfigOptions, OptionType,
};
pub use crate::config::section::{
    ConfigOption, ConfigSection, ConfigSectionSettings, SectionHandle,
    SectionHandleMut,
};

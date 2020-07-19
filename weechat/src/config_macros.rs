#[doc(hidden)]
#[macro_export]
macro_rules! option_settings {
    ($option_type:ident, $option_name:ident, $description:literal, $default:literal $(,)?) => {
        $crate::paste::expr! {
            [<$option_type OptionSettings>]::new(stringify!($option_name))
                .description($description)
                .default_value($default)
        }
    };
    (Integer, $option_name:ident, $description:literal, $default:literal, $min:literal..$max:literal $(,)?) => {
        IntegerOptionSettings::new(stringify!($option_name))
            .description($description)
            .default_value($default)
            .min($min)
            .max($max)
    };
    (Enum, $option_name:ident, $description:literal, $out_type:ty $(,)?) => {
        IntegerOptionSettings::new(stringify!($option_name))
            .description($description)
            .default_value(<$out_type>::default() as i32)
            .string_values(
                <$out_type>::VARIANTS
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>(),
            );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! option_create {
    ($option_type:ident, $option_weechat_type:ident, $option_name:ident, $($args:tt)*) => {
        $crate::paste::item! {
            fn [<create_option_ $option_name>](section: &mut SectionHandleMut) {
                let option_settings = $crate::option_settings!($option_type, $option_name, $($args)*);
                section.[<new_ $option_weechat_type:lower _option>](option_settings)
                    .expect(&format!("Can't create option {}", stringify!($option_name)));
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! option_getter {
    ($option_type:ident, $option_name:ident, $output_type:ty) => {
        $crate::paste::item! {
            #[allow(dead_code)]
            pub fn [<$option_name>](&self) -> $output_type {
                let option_name = stringify!($option_name);

                if let ConfigOption::[<$option_type>](o) = self.0.search_option(option_name)
                    .expect(&format!("Couldn't find option {} in section {}",
                                     option_name, self.0.name()))
                {
                    $output_type::from(o.value())
                } else {
                    panic!("Incorect option type for option {} in section {}",
                           option_name, self.0.name());
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! option {
    (String, $option_name:ident, $($args:tt)*) => {
        $crate::option_create!(String, String, $option_name, $($args)*);
        $crate::option_getter!(String, $option_name, String);
    };

    (Color, $option_name:ident, $($args:tt)*) => {
        $crate::option_create!(Color, Color, $option_name, $($args)*);
        $crate::option_getter!(Color, $option_name, String);
    };

    (bool, $option_name:ident, $($args:tt)*) => {
        $crate::option_create!(Boolean, Boolean, $option_name, $($args)*);
        $crate::option_getter!(Boolean, $option_name, bool);
    };

    (Integer, $option_name:ident, $($args:tt)*) => {
        $crate::option_create!(Integer, Integer, $option_name, $($args)*);
        $crate::option_getter!(Integer, $option_name, i64);
    };

    (Enum, $option_name:ident, $description:literal, $out_type:ty $(,)?) => {
        $crate::option_create!(Enum, Integer, $option_name, $description, $out_type);
        $crate::option_getter!(Integer, $option_name, $out_type);
    };

    (EvaluatedString, $option_name:ident, $($args:tt)*) => {
        $crate::option_create!(String, String, $option_name, $($args)*);

        $crate::paste::item! {
            pub fn [<$option_name>](&self) -> String {
                let option_name = stringify!($option_name);

                let option = self.0.search_option(option_name)
                    .expect(&format!("Couldn't find option {} in section {}",
                                     option_name, self.0.name()));

                if let ConfigOption::String(o) = option {
                    Weechat::eval_string_expression(&o.value())
                        .expect(&format!(
                            "Can't evaluate string expression for option {} in section {}",
                            option_name,
                            self.0.name())
                        )
                } else {
                    panic!("Incorect option type for option {} in section {}",
                           option_name, self.0.name());
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! section {
    ($section:ident { $($option_name:ident: $option_type:ident {$($option:tt)*}), * $(,)? }) => {
        $crate::paste::item! {
            pub struct [<$section:camel Section>]<'a>(SectionHandle<'a>);

            impl<'a> [<$section:camel Section>]<'a> {
                pub fn create(config: &mut Config) {
                    let section_settings = ConfigSectionSettings::new(stringify!($section));

                    let mut $section = config.new_section(section_settings)
                        .expect(&format!("Can't create config section {}", stringify!($section)));

                    [<$section:camel Section>]::create_options(&mut $section);
                }

                pub fn create_options(section: &mut SectionHandleMut) {
                    $(
                        [<$section:camel Section>]::[<create_option_ $option_name>](section);
                    )*
                }

                $(
                    $crate::option!($option_type, $option_name, $($option)*);
                )*
            }
        }
    }
}

/// Declare a Weechat configuration file.
///
/// This will generate a struct called `Config` which wraps the Weechat struct
/// of the same name. The generated struct will have accessors for every
/// section and option that is declared.
///
/// The generated struct dereferences into the Weechat `Config` struct so
/// additional sections and options can be created the usual way as well.
///
/// The config still needs to be created in the `init()` method of the plugin
/// using `Config::new()`.
///
/// This feature requires Rust nightly.
///
/// # Example
/// ```
/// # use weechat::{Weechat, config};
/// use strum_macros::EnumVariantNames;
///
/// #[derive(EnumVariantNames)]
/// #[strum(serialize_all = "kebab_case")]
/// pub enum ServerBufferMerge {
///     MergeWithCore,
///     MergeWithoutCore,
///     Independent,
/// }
///
/// impl Default for ServerBufferMerge {
///     fn default() -> Self {
///         ServerBufferMerge::MergeWithCore
///     }
/// }
///
/// impl From<i32> for ServerBufferMerge {
///     fn from(value: i32) -> Self {
///         match value {
///             0 => ServerBufferMerge::MergeWithCore,
///             1 => ServerBufferMerge::MergeWithoutCore,
///             2 => ServerBufferMerge::Independent,
///             _ => unreachable!(),
///         }
///     }
/// }
///
/// config!(
///     // The name of the config
///     "my-plugin",
///     Section look {
///         encrypted_room_sign: String {
///             // Description.
///             "A sign that is used to show that the current room is encrypted",
///
///             // Default value.
///             "🔒",
///         },
///
///         server_buffer: Enum {
///             // Description.
///             "Merge server buffers",
///
///             // This is an enum that needs to have the following traits
///             // implemented:
///             //    * Default - To define the default value of the option.
///             //    * From<i32> - To convert the internal Weechat integer option
///             //      to the enum.
///             //    * VariantNames - To get the string representation of the
///             //      enum variants. This is a trait defined in the strum library,
///             //      a simple macro that derives an implementation is provided by
///             //      strum.
///             ServerBufferMerge,
///         },
///
///         quote_fg: Color {
///             // Description.
///             "Foreground color for Matrix style blockquotes",
///
///             // Default value.
///             "lightgreen",
///         },
///     },
///
///     Section network {
///         username: EvaluatedString {
///             // Description.
///             "The username that will be used to log in to the server \
///              (note: content is evaluated, see /help eval)",
///
///             // Default value.
///             "",
///         },
///
///         timeout: Integer {
///             // Description.
///             "A timeout (in seconds) that determines how long we should wait \
///             for a request to finish before aborting.",
///
///             // Default value.
///             30,
///
///             // The range that the value is allowed to have, note that both of
///             // those are inclusive.
///             0..100,
///         },
///
///         autoconnect: bool {
///             // Description.
///             "Automatically connect to the server when Weechat is starting",
///
///             // Default value.
///             false,
///         },
///    }
/// );
/// ```
#[cfg(feature = "config_macro")]
#[cfg_attr(feature = "docs", doc(cfg(config_macro)))]
#[macro_export]
macro_rules! config {
    ($config_name:literal, $(Section $section:ident { $($option:tt)* }), * $(,)?) => {
        #[allow(unused_imports)]
        use weechat::strum::VariantNames;
        use std::ops::{Deref, DerefMut};
        #[allow(unused_imports)]
        use weechat::config::{
            SectionHandle, SectionHandleMut, StringOptionSettings,
            ConfigOption, ConfigSection, ConfigSectionSettings,
            BooleanOptionSettings, IntegerOptionSettings, ColorOptionSettings,
        };
        pub struct Config(weechat::config::Config);

        impl Deref for Config {
            type Target = weechat::config::Config;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for Config {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Config {
            fn new() -> Result<Self, ()> {
                let config = Weechat::config_new($config_name)?;
                let mut config = Config(config);

                config.create_sections();

                Ok(config)
            }

            $crate::paste::item! {
                fn create_sections(&mut self) {
                    $(
                        $crate::paste::expr! { [<$section:camel Section>]::create(self) };
                    )*
                }
            }

            $crate::paste::item! {
                $(
                    #[allow(dead_code)]
                    pub fn $section(&self) -> [<$section:camel Section>] {
                        let section_name = stringify!($section);
                        let section = self.0.search_section(section_name)
                            .expect(&format!("Couldn't find section {}", section_name));

                        $crate::paste::item! { [<$section:camel Section>](section) }
                    }
                )*
            }
        }

        $(
            $crate::section!($section { $($option)* });
        )*
    }
}

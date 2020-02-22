use std::borrow::Cow;
use std::time::Instant;
use weechat::config::{
    BooleanOption, BooleanOptionSettings, Config, ConfigSectionSettings,
};
use weechat::{
    weechat_plugin, ArgsWeechat, Buffer, BufferSettings, CommandDescription,
    CommandHook, Weechat, WeechatPlugin, WeechatResult,
};

use weechat::buffer::NickSettings;
use weechat::{BarItem, LightBarItem};

struct SamplePlugin {
    _rust_hook: CommandHook<String>,
    _rust_config: Config,
    _item: BarItem<String>,
}

impl SamplePlugin {
    fn input_cb(
        weechat: &Weechat,
        buffer: &Buffer,
        input: Cow<str>,
    ) -> Result<(), ()> {
        buffer.print(&input);
        Ok(())
    }

    fn close_cb(weechat: &Weechat, buffer: &Buffer) -> Result<(), ()> {
        Weechat::print("Closing buffer");
        Ok(())
    }

    fn rust_command_cb(data: &String, buffer: Buffer, args: ArgsWeechat) {
        buffer.print(data);
        for arg in args {
            buffer.print(&arg)
        }
    }

    fn option_change_cb(weechat: &Weechat, option: &BooleanOption) {
        Weechat::print("Changing rust option");
    }

    fn bar_cb(
        _data: &String,
        _item: &LightBarItem,
        _buffer: &Buffer,
    ) -> String {
        "rust/sample".to_owned()
    }
}

impl WeechatPlugin for SamplePlugin {
    fn init(weechat: &Weechat, _args: ArgsWeechat) -> WeechatResult<Self> {
        Weechat::print("Hello Rust!");

        let buffer_settings = BufferSettings::new("Test buffer")
            .input_callback(SamplePlugin::input_cb)
            .close_callback(SamplePlugin::close_cb);

        let buffer_handle = weechat
            .buffer_new(buffer_settings)
            .expect("Can't create buffer");

        let buffer = buffer_handle.upgrade().expect("Buffer already closed?");

        buffer.print("Hello test buffer");

        let n = 100;

        let now = Instant::now();

        let op_group = buffer.add_group("operators", "blue", true, None);
        let emma = buffer.add_nick(
            NickSettings::new("Emma")
                .set_color("magenta")
                .set_prefix("&")
                .set_prefix_color("green"),
            Some(&op_group),
        );

        Weechat::print(&format!("Nick name getting test: {}", emma.get_name()));

        for nick_number in 0..n {
            let name = &format!("nick_{}", nick_number);
            let nick = NickSettings::new(name);
            let _ = buffer.add_nick(nick, None);
        }

        buffer.print(&format!(
            "Elapsed time for {} nick additions: {}.{}s.",
            n,
            now.elapsed().as_secs(),
            now.elapsed().subsec_millis()
        ));

        let sample_command = CommandDescription {
            name: "rustcommand",
            ..Default::default()
        };

        let command = weechat.hook_command(
            sample_command,
            SamplePlugin::rust_command_cb,
            Some("Hello rust command".to_owned()),
        );

        let mut config = weechat
            .config_new_with_callback("rust_sample", |weechat, _config| {
                Weechat::print("Reloaded config");
            })
            .expect("Can't create new config");

        {
            let section_info = ConfigSectionSettings::new("sample_section");

            let mut section = config
                .new_section(section_info)
                .expect("Can't create section");

            let option_settings = BooleanOptionSettings::new("test_option")
                .default_value(false)
                .set_change_callback(SamplePlugin::option_change_cb);

            section.new_boolean_option(option_settings).expect("Can't create option");
        }

        let item =
            weechat.new_bar_item("buffer_plugin", SamplePlugin::bar_cb, None);

        Ok(SamplePlugin {
            _rust_hook: command,
            _rust_config: config,
            _item: item,
        })
    }
}

impl Drop for SamplePlugin {
    fn drop(&mut self) {
        Weechat::print("Bye rust");
    }
}

weechat_plugin!(
    SamplePlugin,
    name: "rust_sample",
    author: "poljar",
    description: "",
    version: "0.1.0",
    license: "MIT"
);

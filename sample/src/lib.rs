use std::borrow::Cow;
use std::time::Instant;
use weechat::config::{
    BooleanOption, BooleanOptionSettings, Config, ConfigSectionSettings,
};
use weechat::{
    weechat_plugin, ArgsWeechat, Buffer, CommandDescription, CommandHook,
    NickArgs, Weechat, WeechatPlugin, WeechatResult,
};
use weechat::{BarItem, LightBarItem};

struct SamplePlugin {
    _rust_hook: CommandHook<String>,
    _rust_config: Config,
    _item: BarItem<String>,
}

impl SamplePlugin {
    fn input_cb(data: &mut String, buffer: &Buffer, _input: Cow<str>) {
        buffer.print(data);
        if data == "Hello" {
            data.push_str(" world.");
        }
    }

    fn close_cb(_data: &(), buffer: &Buffer) {
        // weechat.print("Closing buffer")
    }

    fn rust_command_cb(data: &String, buffer: Buffer, args: ArgsWeechat) {
        buffer.print(data);
        for arg in args {
            buffer.print(&arg)
        }
    }

    fn option_change_cb(weechat: &Weechat, option: &BooleanOption) {
        weechat.print("Changing rust option");
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
        weechat.print("Hello Rust!");

        let buffer = weechat.buffer_new(
            "Test buffer",
            Some(SamplePlugin::input_cb),
            Some("Hello".to_owned()),
            Some(SamplePlugin::close_cb),
            None,
        );

        buffer.print("Hello test buffer");

        let n = 100;

        let now = Instant::now();

        let op_group = buffer.add_group("operators", "blue", true, None);
        let emma = buffer.add_nick(
            NickArgs {
                name: "Emma",
                color: "magenta",
                prefix: "&",
                prefix_color: "green",
                ..Default::default()
            },
            Some(&op_group),
        );

        weechat.print(&format!("Nick name getting test: {}", emma.get_name()));

        for nick_number in 0..n {
            let nick = NickArgs {
                name: &format!("nick_{}", nick_number),
                ..Default::default()
            };
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
            .config_new("rust_sample", |weechat, _config| {
                weechat.print("Reloaded config");
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

            section.new_boolean_option(option_settings);
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
    fn drop(&mut self) {}
}

weechat_plugin!(
    SamplePlugin,
    name: "rust_sample",
    author: "poljar",
    description: "",
    version: "0.1.0",
    license: "MIT"
);

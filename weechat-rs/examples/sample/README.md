# sample

Weechat sample plugin.

This is a sample plugin, showcasing how to create a simple Rust plugin and how
to do some common things inside a Weechat plugin.

## Build

To build the plugin
```
make
```

Installation can be done like so

```
make install
```

By default this will install the plugin in your `$HOME/.weechat/plugins` directory.

### Custom `weechat-plugin.h`

Weechat might complain about outdated API versions, in that case the used
`weechat-plugin.h` file that was used to compile the plugin doesn't match the
one that your Weechat installation expects.

By default the system-wide installed `weechat-plugin.h` file will be used, this
can be overridden with an environment variable.

A custom include file can be set with the `WEECHAT_PLUGIN_FILE` environment
variable, this environment variable takes a full path to the custom include
file.

After an adequate `weechat-plugin.h` file is found rebuild the plugin like so

```
WEECHAT_PLUGIN_FILE=/home/example/weechat-plugin.h make install
```

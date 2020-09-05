# hsignal

hsignal sends out hashmaps, hashmaps can contain ints, strings,
pointer, buffers, or time_t values as keys as well as values, but no mixing is
allowed.

We can check the type of the keys/values using `hashtable_get_string()` and map them
to the `SignalData` enum.

# hsignal_send

Turn a rust hashmap into a weechat one. We will need to use generics here since
only one type at a time can be used. Eg Hashmap<&str, &str>, Hashmap<&str, Buffer>,
so the second argument of the hashmap needs to be generic.

# key_bind

Create a builder for keybinds, allow adding multiple `.add_bind()` that takes
two strings. Build a hashmap out of this and convert it into a weechat hashmap.

# macro for commands

?????

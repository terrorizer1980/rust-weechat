
use weechat_sys::{t_gui_buffer, t_gui_nick_group};

/// Weechat nicklist Group type.
pub struct NickGroup {
    pub(crate) ptr: *mut t_gui_nick_group,
    pub(crate) buf_ptr: *mut t_gui_buffer,
}

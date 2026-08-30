#[repr(C)]
pub struct RsactHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct RsactKeyEvent {
    pub kind: u8,  // 0 = none, 1 = char, 2 = special (see RSACT_KEY_* constants)
    pub code: u32, // unicode scalar value for kind=1, key id for kind=2
}

#[unsafe(no_mangle)]
pub extern "C" fn rsact_create(width: u16, height: u16) -> *mut RsactHandle {
    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn rsact_destroy(handle: *mut RsactHandle) {
    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn rsact_set_cell(
    handle: *mut RsactHandle,
    row: u16,
    col: u16,
    ch: u32,
    fg_rgb: u32,
    bg_rgb: u32,
    attrs: u8,
) -> i32 {
    todo!()
} // 0 = ok, (< 0) = error

#[unsafe(no_mangle)]
pub extern "C" fn rsact_render(handle: *mut RsactHandle) -> i32 {
    todo!()
} // diff + flush

#[unsafe(no_mangle)]
pub extern "C" fn rsact_enable_raw_mode(handle: *mut RsactHandle) -> i32 {
    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn rsact_disable_raw_mode(handle: *mut RsactHandle) -> i32 {
    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn rsact_poll_key(handle: *mut RsactHandle, out_event: *mut RsactKeyEvent) -> i32 {
    todo!()
} // 0 = no event, 1 = event written, (< 0) = error

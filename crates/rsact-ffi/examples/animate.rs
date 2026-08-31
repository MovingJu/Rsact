//! Moves a single '#' across row 5, one frame every ~16ms.
//!
//! Run with: `cargo run --example animate -p rsact-ffi`

use rsact_ffi::*;

fn main() {
    unsafe {
        let handle = rsact_create(40, 10);
        assert!(
            !handle.is_null(),
            "rsact_create failed (not a real terminal?)"
        );

        for frame in 0..100u16 {
            rsact_set_cell(handle, 5, frame % 40, '#' as u32, 0xffffff, 0x000000, 0);
            rsact_render(handle);
            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        rsact_destroy(handle);
    }
}

//! Draws a single static frame: a horizontal line across row 0.
//!
//! Run with: `cargo run --example basic -p rsact-ffi`

use rsact_ffi::*;

fn main() {
    unsafe {
        let handle = rsact_create(40, 10);
        assert!(
            !handle.is_null(),
            "rsact_create failed (not a real terminal?)"
        );

        for col in 0..40 {
            rsact_set_cell(handle, 0, col, '-' as u32, 0xffffff, 0x000000, 0);
        }
        assert_eq!(rsact_render(handle), 0);

        std::thread::sleep(std::time::Duration::from_secs(2));
        rsact_destroy(handle);
    }
}

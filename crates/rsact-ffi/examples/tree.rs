//! Builds a small two-counter dashboard purely through the C-style
//! `rsact_element_*`/`rsact_tree_*` API — this is what a C caller does,
//! since there's no `Component` trait to implement on that side. The whole
//! tree is built as one nested expression per frame, HTML-like, instead of
//! naming every child as a separate variable and wiring it in with a
//! setter call.
//!
//! Run with: `cargo run --example tree -p rsact-ffi`

use rsact_ffi::*;
use std::ffi::CString;

/// Builds one `label` / `count` counter as a small vertical container.
fn counter_element(label: &str, count: u32) -> *mut RsactElement {
    unsafe {
        let key = CString::new(label).unwrap();
        let label_key = CString::new("label").unwrap();
        let label_content = CString::new(label).unwrap();
        let value_key = CString::new("value").unwrap();
        let value_content = CString::new(count.to_string()).unwrap();

        let children = [
            rsact_element_text(
                label_key.as_ptr(),
                label_content.as_ptr(),
                20,
                0xffffff,
                0,
                0,
            ),
            rsact_element_text(
                value_key.as_ptr(),
                value_content.as_ptr(),
                20,
                0xffffff,
                0,
                0,
            ),
        ];
        rsact_element_container(
            key.as_ptr(),
            RSACT_LAYOUT_VERTICAL,
            20,
            2,
            children.as_ptr(),
            children.len(),
        )
    }
}

/// Two counters side by side.
fn dashboard_element(left_count: u32, right_count: u32) -> *mut RsactElement {
    unsafe {
        let key = CString::new("dashboard").unwrap();
        let children = [
            counter_element("left", left_count),
            counter_element("right", right_count),
        ];
        rsact_element_container(
            key.as_ptr(),
            RSACT_LAYOUT_HORIZONTAL,
            40,
            2,
            children.as_ptr(),
            children.len(),
        )
    }
}

fn main() {
    unsafe {
        let handle = rsact_tree_create(40, 2);
        assert!(
            !handle.is_null(),
            "rsact_tree_create failed (not a real terminal?)"
        );

        for frame in 0..5u32 {
            let root = dashboard_element(frame, 42);
            assert_eq!(rsact_tree_present(handle, root), 0);
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        rsact_tree_destroy(handle);
    }
}

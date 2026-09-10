//! Builds a small two-counter dashboard purely through the C-style
//! `rsact_element_*`/`rsact_tree_*` API, rebuilding the element tree each
//! frame — this is what a C caller does, since there's no `Component`
//! trait to implement on that side.
//!
//! Run with: `cargo run --example tree -p rsact-ffi`

use rsact_ffi::*;
use std::ffi::CString;

/// Builds one `label` / `count` counter as a small vertical container.
fn counter_element(label: &str, count: u32) -> *mut RsactElement {
    unsafe {
        let key = CString::new(label).unwrap();
        let container = rsact_element_container(key.as_ptr(), 0 /* Vertical */);
        rsact_element_set_width(container, 20);
        rsact_element_set_height(container, 2);

        let label_key = CString::new("label").unwrap();
        let label_content = CString::new(label).unwrap();
        let label_elem = rsact_element_text(label_key.as_ptr(), label_content.as_ptr());
        rsact_element_set_width(label_elem, 20);
        rsact_element_add_child(container, label_elem);

        let value_key = CString::new("value").unwrap();
        let value_content = CString::new(count.to_string()).unwrap();
        let value_elem = rsact_element_text(value_key.as_ptr(), value_content.as_ptr());
        rsact_element_set_width(value_elem, 20);
        rsact_element_add_child(container, value_elem);

        container
    }
}

/// Two counters side by side.
fn dashboard_element(left_count: u32, right_count: u32) -> *mut RsactElement {
    unsafe {
        let key = CString::new("dashboard").unwrap();
        let root = rsact_element_container(key.as_ptr(), 1 /* Horizontal */);
        rsact_element_set_width(root, 40);
        rsact_element_set_height(root, 2);

        rsact_element_add_child(root, counter_element("left", left_count));
        rsact_element_add_child(root, counter_element("right", right_count));

        root
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
            assert_eq!(rsact_tree_set_root(handle, root), 0);
            assert_eq!(rsact_tree_present(handle), 0);
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        rsact_tree_destroy(handle);
    }
}

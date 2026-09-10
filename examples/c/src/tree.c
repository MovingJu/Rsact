#include <stdio.h>
#include "rsact.h"

/* Not yet wired into CMakeLists.txt: examples/c links a *prebuilt*
 * rsact-ffi release binary (see CMakeLists.txt's RSACT_VERSION), and the
 * rsact_element_ and rsact_tree_ functions this file uses were only added
 * after the last tagged release. Add an executable target for this file
 * once RSACT_VERSION points at a release that ships them. */

/* Builds one label/count counter as a small vertical container. */
static RsactElement *counter_element(const char *label, unsigned count) {
    RsactElement *container = rsact_element_container(label, 0 /* Vertical */);
    rsact_element_set_width(container, 20);
    rsact_element_set_height(container, 2);

    RsactElement *label_elem = rsact_element_text("label", label);
    rsact_element_set_width(label_elem, 20);
    rsact_element_add_child(container, label_elem);

    char value[16];
    snprintf(value, sizeof(value), "%u", count);
    RsactElement *value_elem = rsact_element_text("value", value);
    rsact_element_set_width(value_elem, 20);
    rsact_element_add_child(container, value_elem);

    return container;
}

/* Two counters side by side. */
static RsactElement *dashboard_element(unsigned left_count, unsigned right_count) {
    RsactElement *root = rsact_element_container("dashboard", 1 /* Horizontal */);
    rsact_element_set_width(root, 40);
    rsact_element_set_height(root, 2);

    rsact_element_add_child(root, counter_element("left", left_count));
    rsact_element_add_child(root, counter_element("right", right_count));

    return root;
}

/* Builds a small two-counter dashboard purely through the rsact_element_
 * and rsact_tree_ API, rebuilding the element tree each frame and
 * advancing only the left counter's count. */
int main(void) {
    RsactTreeHandle *h = rsact_tree_create(40, 2);
    if (!h) {
        fprintf(stderr, "rsact_tree_create failed (not a real terminal?)\n");
        return 1;
    }

    for (unsigned frame = 0; frame < 5; frame++) {
        RsactElement *root = dashboard_element(frame, 42);
        if (rsact_tree_set_root(h, root) != 0) {
            fprintf(stderr, "rsact_tree_set_root failed\n");
            break;
        }
        if (rsact_tree_present(h) != 0) {
            fprintf(stderr, "rsact_tree_present failed\n");
            break;
        }
    }

    rsact_tree_destroy(h); /* also restores the terminal */
    return 0;
}

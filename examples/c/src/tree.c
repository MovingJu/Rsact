#include <stdio.h>
#include "rsact.h"

/* Builds one label/count counter as a small vertical container, its two
 * text children built inline as a C99 compound-literal array — one nested
 * expression, HTML-like, instead of naming each child and wiring it in
 * with a separate call. */
static RsactElement *counter_element(const char *label, const char *value) {
    return rsact_element_container(
        label, RSACT_LAYOUT_VERTICAL, 20, 2,
        (RsactElement *[]){
            rsact_element_text("label", label, 20, 0xffffff, 0x000000, 0),
            rsact_element_text("value", value, 20, 0xffffff, 0x000000, 0),
        },
        2);
}

/* Two counters side by side, same nested-expression style. */
static RsactElement *dashboard_element(const char *left_value, const char *right_value) {
    return rsact_element_container(
        "dashboard", RSACT_LAYOUT_HORIZONTAL, 40, 2,
        (RsactElement *[]){
            counter_element("left", left_value),
            counter_element("right", right_value),
        },
        2);
}

/* Builds a small two-counter dashboard purely through the rsact_element_
 * and rsact_tree_ API. RsactTreeHandle is a distinct type from RsactHandle
 * on purpose: it can't be passed to rsact_set_cell/rsact_render by
 * accident, since those take a RsactHandle and this is a RsactTreeHandle —
 * the compiler catches that, rather than it silently corrupting a diff
 * baseline at runtime. Rebuilds the element tree each frame, advancing
 * only the left counter's count. */
int main(void) {
    RsactTreeHandle *h = rsact_tree_create(40, 2);
    if (!h) {
        fprintf(stderr, "rsact_tree_create failed (not a real terminal?)\n");
        return 1;
    }

    for (unsigned frame = 0; frame < 5; frame++) {
        char left_value[16];
        snprintf(left_value, sizeof(left_value), "%u", frame);

        RsactElement *root = dashboard_element(left_value, "42");
        if (rsact_tree_present(h, root) != 0) {
            fprintf(stderr, "rsact_tree_present failed\n");
            break;
        }
    }

    rsact_tree_destroy(h); /* also restores the terminal */
    return 0;
}

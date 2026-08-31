#include <stdio.h>
#include "rsact.h"

/* Draws a single static frame: a horizontal line across row 0. */
int main(void) {
    RsactHandle *h = rsact_create(40, 10); /* also enters raw mode */
    if (!h) {
        fprintf(stderr, "rsact_create failed (not a real terminal?)\n");
        return 1;
    }

    for (int col = 0; col < 40; col++) {
        rsact_set_cell(h, 0, col, '-', 0xffffff, 0x000000, 0);
    }

    if (rsact_render(h) != 0) {
        fprintf(stderr, "rsact_render failed\n");
    }

    rsact_destroy(h); /* also restores the terminal */
    return 0;
}

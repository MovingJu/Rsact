#include <stdio.h>
#include "rsact.h"

/* Prints up to 20 decoded key events. rsact_poll_key blocks until a key
 * arrives (v0.1 has no non-blocking mode). */
int main(void) {
    RsactHandle *h = rsact_create(40, 10);
    if (!h) return 1;

    for (int i = 0; i < 20; i++) {
        RsactKeyEvent event;
        int rc = rsact_poll_key(h, &event);
        if (rc < 0) break;  /* error */
        if (rc == 0) break; /* EOF */

        if (event.kind == 1) {
            printf("char: %u\r\n", event.code);
        } else if (event.kind == 2) {
            printf("special key id: %u\r\n", event.code);
        }
    }

    rsact_destroy(h);
    return 0;
}

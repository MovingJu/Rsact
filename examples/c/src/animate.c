#include "rsact.h"

void rectangle(RsactHandle *h, uint16_t row, uint16_t col);

int main(void) {
    uint16_t row, col;
    rsact_terminal_size(&row, &col);
    RsactHandle *h = rsact_create(col, row); /* also enters raw mode */

    rectangle(h, row, col);
    rsact_render(h);
    
    rsact_destroy(h); /* also restores the terminal */
    return 0;
}

void rectangle(RsactHandle *h, uint16_t row, uint16_t col) {
    for (int frame = 0; frame < col; frame++) {
        rsact_set_cell(h, 0, frame, 'o', 0xffffff, 0x000000, 0);
        rsact_set_cell(h, row - 1, frame, 'o', 0xffffff, 0x000000, 0);
    }
    for (int frame = 0; frame < row; frame++) {
        rsact_set_cell(h, frame, 0, 'o', 0xffffff, 0x000000, 0);
        rsact_set_cell(h, frame, col - 1, 'o', 0xffffff, 0x000000, 0);
    }
}
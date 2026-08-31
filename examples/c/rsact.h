#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define RSACT_KEY_UP 1

#define RSACT_KEY_DOWN 2

#define RSACT_KEY_RIGHT 3

#define RSACT_KEY_LEFT 4

#define RSACT_KEY_ENTER 5

#define RSACT_KEY_ESC 6

#define RSACT_KEY_BACKSPACE 7

/**
 * Ctrl+'a'..'z' -> RSACT_KEY_CTRL_BASE + (letter - 'a')
 */
#define RSACT_KEY_CTRL_BASE 256

typedef struct RsactHandle {
  uint8_t _private[0];
} RsactHandle;

typedef struct RsactKeyEvent {
  uint8_t kind;
  uint32_t code;
} RsactKeyEvent;

/**
 * Creates a handle for a `width`×`height` virtual terminal buffer and
 * enters raw mode. Returns a null pointer if raw mode couldn't be enabled
 * (for example, when stdin/stdout isn't a real terminal).
 *
 * # Safety
 * No preconditions on the arguments themselves. Call this only from a
 * single thread at a time — v0.1 has no synchronization around the
 * process's stdin/stdout raw-mode state.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 *
 * unsafe {
 *     let handle = rsact_create(80, 24);
 *     assert!(!handle.is_null());
 *     rsact_destroy(handle);
 * }
 * ```
 */
struct RsactHandle *rsact_create(uint16_t width, uint16_t height);

/**
 * Returns total size of current terminal. If succesfully got the size, it returns `0`. Otherwise, returns (< 0)
 * 
 * # Examples 
 * ```rust,no_run
 * use rsact_ffi::*;
 * 
 * let mut row: u16 = 0;
 * let mut col: u16 = 0;
 * terminal_size(&mut row, &mut col);
 * ```
 */
int32_t rsact_terminal_size(uint16_t *row,
                            uint16_t *col);

/**
 * Destroys a handle created by [`rsact_create`], restoring the terminal
 * (dropping the handle drops its `RawModeGuard`). Does nothing if `handle`
 * is null.
 *
 * # Safety
 * `handle` must be either null or a pointer previously returned by
 * [`rsact_create`] that hasn't already been passed to `rsact_destroy`.
 * Never call this twice on the same pointer, and never use `handle` again
 * afterward.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 *
 * unsafe {
 *     let handle = rsact_create(80, 24);
 *     rsact_destroy(handle);
 * }
 * ```
 */
void rsact_destroy(struct RsactHandle *handle);

/**
 * Sets a single cell of the virtual buffer. `fg_rgb`/`bg_rgb` are packed as
 * `0xRRGGBB`. `attrs` is the *sum* of the ANSI SGR codes to apply — `1` for
 * bold, `4` for underline, `7` for reverse, added together for combinations
 * (e.g. `5` = bold + underline, `12` = all three); any other value is
 * rejected. Returns `0` on success, `-1` if `handle` is null, `-2` if `ch`
 * isn't a valid Unicode scalar value, `-3` if `attrs` isn't one of the
 * recognized sums.
 *
 * # Safety
 * `handle` must be either null or a valid pointer returned by
 * [`rsact_create`] that hasn't been passed to [`rsact_destroy`] yet.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 *
 * unsafe {
 *     let handle = rsact_create(80, 24);
 *     let rc = rsact_set_cell(handle, 0, 0, 'A' as u32, 0xffffff, 0x000000, 0);
 *     assert_eq!(rc, 0);
 *     rsact_destroy(handle);
 * }
 * ```
 */
int32_t rsact_set_cell(struct RsactHandle *handle,
                       uint16_t row,
                       uint16_t col,
                       uint32_t ch,
                       uint32_t fg_rgb,
                       uint32_t bg_rgb,
                       uint8_t attrs);

/**
 * Diffs the virtual buffer against the last-rendered buffer, writes only
 * the changed cells to the terminal in a single write, then commits the
 * virtual buffer as the new baseline for the next call. Returns `0` on
 * success, `-1` if `handle` is null or the write failed.
 *
 * # Safety
 * `handle` must be either null or a valid pointer returned by
 * [`rsact_create`] that hasn't been passed to [`rsact_destroy`] yet.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 *
 * unsafe {
 *     let handle = rsact_create(80, 24);
 *     rsact_set_cell(handle, 0, 0, 'A' as u32, 0xffffff, 0x000000, 0);
 *     let rc = rsact_render(handle);
 *     assert_eq!(rc, 0);
 *     rsact_destroy(handle);
 * }
 * ```
 */
int32_t rsact_render(struct RsactHandle *handle);

/**
 * Reads and decodes the next key from stdin into `out_event`. Blocks until
 * a key arrives — v0.1 has no non-blocking mode. Returns `1` and fills
 * `out_event` when a key was read, `0` on EOF, `-1` on error or a null
 * `handle`.
 *
 * # Safety
 * `handle` must be either null or a valid pointer returned by
 * [`rsact_create`] that hasn't been passed to [`rsact_destroy`] yet.
 * `out_event` must be a valid, non-null, properly aligned pointer to
 * writable memory for a [`RsactKeyEvent`] — it is not currently
 * null-checked, unlike `handle`.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 * use std::mem::MaybeUninit;
 *
 * unsafe {
 *     let handle = rsact_create(80, 24);
 *     let mut event = MaybeUninit::<RsactKeyEvent>::uninit();
 *     if rsact_poll_key(handle, event.as_mut_ptr()) == 1 {
 *         let event = event.assume_init();
 *         println!("kind={} code={}", event.kind, event.code);
 *     }
 *     rsact_destroy(handle);
 * }
 * ```
 */
int32_t rsact_poll_key(struct RsactHandle *handle, struct RsactKeyEvent *out_event);

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

/**
 * `layout` value for `rsact_element_container`: stack children top to
 * bottom, each spanning the container's full width.
 */
#define RSACT_LAYOUT_VERTICAL 0

/**
 * `layout` value for `rsact_element_container`: stack children left to
 * right, each spanning the container's full height.
 */
#define RSACT_LAYOUT_HORIZONTAL 1

typedef struct RsactHandle {
  uint8_t _private[0];
} RsactHandle;

typedef struct RsactKeyEvent {
  uint8_t kind;
  uint32_t code;
} RsactKeyEvent;

/**
 * An opaque, owned `Element` (sub)tree, built up with the
 * `rsact_element_*` functions below. Passing one to `rsact_element_container`
 * (as one of its `children`) or `rsact_tree_present` transfers ownership —
 * never touch or free it again afterward. An `Element` you built but never
 * attached anywhere must be freed with `rsact_element_free`.
 */
typedef struct RsactElement {
  uint8_t _private[0];
} RsactElement;

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
 * rsact_terminal_size(&mut row, &mut col);
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

/**
 * Creates a single-line text leaf element with its width and style set up
 * front, so it's ready to nest straight into a `rsact_element_container`
 * call — no separate setter calls needed. `fg_rgb`/`bg_rgb`/`attrs` use the
 * same encoding as `rsact_set_cell`. Returns null if `key`/`content` is
 * null or not valid UTF-8, or `attrs` isn't one of the recognized sums.
 *
 * # Safety
 * `key` and `content` must each be either null or a valid, NUL-terminated,
 * UTF-8 C string.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 * use std::ffi::CString;
 *
 * unsafe {
 *     let key = CString::new("label").unwrap();
 *     let content = CString::new("hello").unwrap();
 *     let elem = rsact_element_text(key.as_ptr(), content.as_ptr(), 20, 0xffffff, 0x000000, 0);
 *     assert!(!elem.is_null());
 *     rsact_element_free(elem);
 * }
 * ```
 */
struct RsactElement *rsact_element_text(const char *key,
                                        const char *content,
                                        uint16_t width,
                                        uint32_t fg_rgb,
                                        uint32_t bg_rgb,
                                        uint8_t attrs);

/**
 * Creates a container with `children` already attached, so a whole subtree
 * can be built as one nested expression — pass a C99 compound literal
 * array of `rsact_element_text`/`rsact_element_container` calls directly
 * as `children` (or a heap-allocated array, for a runtime-determined
 * count), instead of building each child as a separate named variable and
 * wiring it in afterward:
 *
 * ```c
 * RsactElement *row = rsact_element_container(
 *     "row", RSACT_LAYOUT_HORIZONTAL, 40, 1,
 *     (RsactElement *[]){
 *         rsact_element_text("label", "left", 20, 0xffffff, 0, 0),
 *         rsact_element_text("value", "0",    20, 0xffffff, 0, 0),
 *     }, 2);
 * ```
 *
 * Always consumes every non-null pointer in `children` — never use or free
 * any of them again after this call, whether or not it succeeds.
 * `layout` is `RSACT_LAYOUT_VERTICAL` or `RSACT_LAYOUT_HORIZONTAL`. Returns
 * null if `key` is null/not valid UTF-8, `layout` isn't one of those two
 * values, or any pointer in `children` is null.
 *
 * # Safety
 * `key` must be either null or a valid, NUL-terminated, UTF-8 C string.
 * `children` must be either null (with `n_children == 0`) or point to an
 * array of exactly `n_children` valid `*mut RsactElement` pointers, each
 * meeting the pointer requirements of `rsact_element_free` — and no two of
 * them (including nested descendants already attached to one of them) may
 * alias each other.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 * use std::ffi::CString;
 *
 * unsafe {
 *     let child_key = CString::new("child").unwrap();
 *     let child_content = CString::new("hi").unwrap();
 *     let child = rsact_element_text(child_key.as_ptr(), child_content.as_ptr(), 10, 0xffffff, 0, 0);
 *
 *     let root_key = CString::new("root").unwrap();
 *     let children = [child];
 *     let root = rsact_element_container(root_key.as_ptr(), RSACT_LAYOUT_VERTICAL, 10, 1, children.as_ptr(), children.len());
 *     assert!(!root.is_null());
 *     rsact_element_free(root); // also frees the attached child
 * }
 * ```
 */
struct RsactElement *rsact_element_container(const char *key,
                                             uint8_t layout,
                                             uint16_t width,
                                             uint16_t height,
                                             struct RsactElement *const *children,
                                             uintptr_t n_children);

/**
 * Frees an element (sub)tree that was never attached to a
 * `rsact_element_container` call or `rsact_tree_present`. Does nothing if
 * `elem` is null.
 *
 * # Safety
 * `elem` must be either null or a valid pointer returned by
 * `rsact_element_text`/`rsact_element_container` that hasn't already been
 * passed as a child to `rsact_element_container`, to `rsact_tree_present`,
 * or to `rsact_element_free`. Never call this twice on the same pointer.
 */
void rsact_element_free(struct RsactElement *elem);

/**
 * Renders `element` (taking ownership of it) through the component-tree
 * reconciler and draws only what changed, on the same `RsactHandle`
 * `rsact_create` already gave you — there's no separate handle type or
 * create/destroy call for the component tree. The `Tree` behind this is
 * created on the first call, sized to match the handle's `width`/`height`;
 * every later call reconciles against what the previous call drew.
 *
 * Don't also call `rsact_set_cell`/`rsact_render` on a handle you use this
 * way: each API tracks its own independent "what's actually on screen"
 * baseline, and interleaving them will leave one of those baselines stale,
 * causing incorrect (missing) repaints. Pick one API per handle.
 *
 * Returns `0` on success, `-1` if `handle`/`element` is null or the write
 * failed.
 *
 * # Safety
 * `handle` must be either null or a valid pointer returned by
 * `rsact_create` that hasn't been passed to `rsact_destroy` yet. `element`
 * must be either null or a valid pointer returned by
 * `rsact_element_text`/`rsact_element_container` that hasn't yet been
 * passed as a child to `rsact_element_container`, to another
 * `rsact_tree_present` call, or to `rsact_element_free`.
 *
 * # Examples
 * ```rust,no_run
 * use rsact_ffi::*;
 * use std::ffi::CString;
 *
 * unsafe {
 *     let handle = rsact_create(20, 1);
 *     let key = CString::new("greeting").unwrap();
 *     let content = CString::new("hi").unwrap();
 *     let root = rsact_element_text(key.as_ptr(), content.as_ptr(), 10, 0xffffff, 0, 0);
 *     assert_eq!(rsact_tree_present(handle, root), 0);
 *     rsact_destroy(handle);
 * }
 * ```
 */
int32_t rsact_tree_present(struct RsactHandle *handle, struct RsactElement *element);

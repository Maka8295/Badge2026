#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Builds an RGB565 color from 8-bit red, green and blue values. */
#define BADGE_RGB(r, g, b) ((uint16_t)((((r) & 0xf8) << 8) | (((g) & 0xfc) << 3) | ((b) >> 3)))

/* Key code of the key at matrix row and column (both start at 1). */
#define BADGE_KEY_CODE(row, col) ((row) * 16 + (col))

/* Returned by badge_key_pressed when no new key was pressed. */
#define BADGE_NO_KEY (-1)

#define BADGE_KEY_ESC   BADGE_KEY_CODE(1, 1)
#define BADGE_KEY_DEL   BADGE_KEY_CODE(1, 14)
#define BADGE_KEY_TAB   BADGE_KEY_CODE(2, 1)
#define BADGE_KEY_SHIFT BADGE_KEY_CODE(3, 2)
#define BADGE_KEY_OK    BADGE_KEY_CODE(3, 14)
#define BADGE_KEY_DOWN  BADGE_KEY_CODE(5, 1)
#define BADGE_KEY_LEFT  BADGE_KEY_CODE(5, 2)
#define BADGE_KEY_UP    BADGE_KEY_CODE(6, 1)
#define BADGE_KEY_A     BADGE_KEY_CODE(6, 2)
#define BADGE_KEY_RIGHT BADGE_KEY_CODE(7, 1)
#define BADGE_KEY_B     BADGE_KEY_CODE(7, 2)

/* Text sizes for badge_draw_text. */
#define BADGE_TEXT_SMALL 0
#define BADGE_TEXT_BODY  1
#define BADGE_TEXT_TITLE 2

/* Fills the whole screen with one color. */
void badge_clear(uint16_t rgb565);

/* Fills a rectangle. Parts outside the screen are clipped. */
void badge_fill_rect(int x, int y, int w, int h, uint16_t rgb565);

/* Draws text with its top-left corner at (x, y). size is one of BADGE_TEXT_*. */
void badge_draw_text(int x, int y, const char *text, uint16_t rgb565, int size);

/* Screen size in pixels (320 x 240). */
int badge_screen_width(void);
int badge_screen_height(void);

/* Returns the code of the next newly pressed key, or BADGE_NO_KEY. */
int badge_key_pressed(void);

/* Returns true while the key with this code is held down. */
bool badge_key_held(int code);

/* Milliseconds since boot. Wraps after about 49 days. */
uint32_t badge_millis(void);

/* Sleeps and lets the rest of the firmware run. Call it in every loop. */
void badge_delay_ms(uint32_t ms);

/* Name of this badge, for example "NeonTengu3f". */
const char *badge_name(void);

#ifdef __cplusplus
}
#endif

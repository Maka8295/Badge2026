#include <stdbool.h>

#include "badge.h"

#define BACKGROUND BADGE_RGB(9, 9, 9)
#define FOREGROUND BADGE_RGB(230, 233, 235)
#define MUTED BADGE_RGB(197, 197, 197)
#define ACCENT BADGE_RGB(237, 37, 45)

#define MARGIN 12
#define TITLE_Y 8
#define PLAY_TOP 48
#define HINT_BOTTOM_GAP 22
#define SQUARE_SIZE 20
#define STEP 4
#define FRAME_MS 16
#define HINT "D-pad move   B quit"

struct square {
    int x;
    int y;
};

static int clamp(int value, int low, int high)
{
    if (value < low) {
        return low;
    }
    return value > high ? high : value;
}

static void draw_square(struct square s, uint16_t color)
{
    badge_fill_rect(s.x, s.y, SQUARE_SIZE, SQUARE_SIZE, color);
}

static void draw_screen(void)
{
    badge_clear(BACKGROUND);
    badge_draw_text(MARGIN, TITLE_Y, badge_name(), FOREGROUND, BADGE_TEXT_TITLE);
    int hint_y = badge_screen_height() - HINT_BOTTOM_GAP;
    badge_draw_text(MARGIN, hint_y, HINT, MUTED, BADGE_TEXT_SMALL);
}

static struct square moved(struct square s)
{
    s.x += STEP * (badge_key_held(BADGE_KEY_RIGHT) - badge_key_held(BADGE_KEY_LEFT));
    s.y += STEP * (badge_key_held(BADGE_KEY_DOWN) - badge_key_held(BADGE_KEY_UP));
    s.x = clamp(s.x, 0, badge_screen_width() - SQUARE_SIZE);
    s.y = clamp(s.y, PLAY_TOP, badge_screen_height() - HINT_BOTTOM_GAP - SQUARE_SIZE);
    return s;
}

static bool quit_pressed(void)
{
    int key;
    while ((key = badge_key_pressed()) != BADGE_NO_KEY) {
        if (key == BADGE_KEY_B) {
            return true;
        }
    }
    return false;
}

int app_main(void)
{
    struct square s = { badge_screen_width() / 2, badge_screen_height() / 2 };
    draw_screen();
    draw_square(s, ACCENT);
    while (!quit_pressed()) {
        struct square next = moved(s);
        if (next.x != s.x || next.y != s.y) {
            draw_square(s, BACKGROUND);
            draw_square(next, ACCENT);
            s = next;
        }
        badge_delay_ms(FRAME_MS);
    }
    return 0;
}

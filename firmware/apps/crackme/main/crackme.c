#include <stdint.h>

#include "badge.h"
#include "crackme_secret.h"
#include "machine.h"

#define BACKGROUND BADGE_RGB(9, 9, 9)
#define FOREGROUND BADGE_RGB(230, 233, 235)
#define MUTED BADGE_RGB(197, 197, 197)
#define ACCENT BADGE_RGB(237, 37, 45)
#define SUCCESS BADGE_RGB(60, 220, 120)

#define MARGIN 12
#define TITLE_Y 8
#define COUNT_Y 44
#define INPUT_Y 70
#define INPUT_HEIGHT 36
#define MESSAGE_Y 120
#define MESSAGE_HEIGHT 24
#define ANSWER_LABEL_Y 60
#define ANSWER_Y 96
#define ANSWER_LINE_HEIGHT 36
#define ANSWER_LINE_CHARS 16
#define HINT_BOTTOM_GAP 22
#define FRAME_MS 16
#define NOPE_MS 1200
#define TITLE "LOCKED"
#define HINT "Arrows/A enter  DEL undo  OK try  B quit"
#define NOPE "nope"
#define CHECKING "..."
#define ANSWER_LABEL "Next answer:"

#define MAX_INPUT 48
#define SHOWN_KEYS 16
#define COUNT_TEXT_SIZE 3
#define DECIMAL 10
#define NOT_A_MOVE (-1)
#define MOVE_COUNT 5

#define START_STATE 0x73880FEE7E4D6413ull
#define GOLDEN 0x9E3779B97F4A7C15ull
#define MIX_A 0xBF58476D1CE4E5B9ull
#define MIX_B 0x94D049BB133111EBull
#define MIX_SHIFT_A 30
#define MIX_SHIFT_B 27
#define MIX_SHIFT_C 31
#define SLOW_CHUNKS 8
#define CHUNK_ROUNDS (1u << 18)
#define CHUNK_PAUSE_MS 1
#define SEAL_ROUNDS 64u
#define TAG_SALT 0x29C3EC25ECCEAE36ull
#define KEY_SALT 0xD4988BA72D84E3F3ull
#define BLOCK_BYTES 8

static const int MOVE_KEYS[MOVE_COUNT] = {
    BADGE_KEY_UP, BADGE_KEY_DOWN, BADGE_KEY_LEFT, BADGE_KEY_RIGHT, BADGE_KEY_A,
};
static const char MOVE_LETTERS[MOVE_COUNT] = { 'U', 'D', 'L', 'R', 'A' };

struct input {
    uint8_t moves[MAX_INPUT];
    int length;
};

struct position {
    int cell;
    int progress;
};

enum outcome { KEEP_GOING, QUIT, SOLVED };

static int move_of_key(int key)
{
    for (int i = 0; i < MOVE_COUNT; i++) {
        if (MOVE_KEYS[i] == key) {
            return i;
        }
    }
    return NOT_A_MOVE;
}

static uint32_t step(struct position *at, int move)
{
    struct machine vm = { .pc = 0 };
    vm.reg[CELL_REGISTER] = (uint32_t)at->cell;
    vm.reg[PROGRESS_REGISTER] = (uint32_t)at->progress;
    vm.reg[MOVE_REGISTER] = (uint32_t)move;
    run_machine(&vm);
    at->cell = (int)vm.reg[CELL_REGISTER];
    at->progress = (int)vm.reg[PROGRESS_REGISTER];
    return vm.reg[WORD_REGISTER];
}

static uint64_t mix(uint64_t value)
{
    value = (value ^ (value >> MIX_SHIFT_A)) * MIX_A;
    value = (value ^ (value >> MIX_SHIFT_B)) * MIX_B;
    return value ^ (value >> MIX_SHIFT_C);
}

static uint64_t absorb(const struct input *in)
{
    struct position at = { .cell = START_CELL, .progress = 0 };
    uint64_t state = START_STATE;
    for (int i = 0; i < in->length; i++) {
        uint32_t word = step(&at, in->moves[i]);
        state = mix((state ^ word) + GOLDEN);
    }
    return state;
}

static uint64_t one_way(uint64_t state, uint32_t rounds)
{
    for (uint32_t i = 0; i < rounds; i++) {
        state += GOLDEN;
        state ^= mix(state);
    }
    return state;
}

static uint64_t slow_mix(uint64_t state)
{
    for (int chunk = 0; chunk < SLOW_CHUNKS; chunk++) {
        state = one_way(state, CHUNK_ROUNDS);
        badge_delay_ms(CHUNK_PAUSE_MS);
    }
    return state;
}

static void decrypt_secret(uint64_t value, char *out)
{
    uint64_t key = value ^ KEY_SALT;
    uint64_t block = 0;
    for (int i = 0; i < SECRET_LENGTH; i++) {
        if (i % BLOCK_BYTES == 0) {
            key += GOLDEN;
            block = one_way(key, SEAL_ROUNDS);
        }
        out[i] = (char)(ENCRYPTED_SECRET[i] ^ (uint8_t)(block >> (BYTE_BITS * (i % BLOCK_BYTES))));
    }
    out[SECRET_LENGTH] = '\0';
}

static void count_text(int number, char *out)
{
    out[0] = (char)('0' + number / DECIMAL);
    out[1] = (char)('0' + number % DECIMAL);
    out[2] = '\0';
}

static void draw_input(const struct input *in)
{
    char text[SHOWN_KEYS + 1];
    char count[COUNT_TEXT_SIZE];
    int first = in->length > SHOWN_KEYS ? in->length - SHOWN_KEYS : 0;
    for (int i = first; i < in->length; i++) {
        text[i - first] = MOVE_LETTERS[in->moves[i]];
    }
    text[in->length - first] = '\0';
    count_text(in->length, count);
    badge_fill_rect(0, COUNT_Y, badge_screen_width(), INPUT_Y + INPUT_HEIGHT - COUNT_Y, BACKGROUND);
    badge_draw_text(MARGIN, COUNT_Y, count, MUTED, BADGE_TEXT_BODY);
    badge_draw_text(MARGIN, INPUT_Y, text, FOREGROUND, BADGE_TEXT_TITLE);
}

static void draw_screen(void)
{
    badge_clear(BACKGROUND);
    badge_draw_text(MARGIN, TITLE_Y, TITLE, ACCENT, BADGE_TEXT_TITLE);
    int hint_y = badge_screen_height() - HINT_BOTTOM_GAP;
    badge_draw_text(MARGIN, hint_y, HINT, MUTED, BADGE_TEXT_SMALL);
}

static void show_message(const char *text, uint16_t color)
{
    badge_fill_rect(0, MESSAGE_Y, badge_screen_width(), MESSAGE_HEIGHT, BACKGROUND);
    badge_draw_text(MARGIN, MESSAGE_Y, text, color, BADGE_TEXT_BODY);
}

static void show_nope(void)
{
    show_message(NOPE, ACCENT);
    badge_delay_ms(NOPE_MS);
    badge_fill_rect(0, MESSAGE_Y, badge_screen_width(), MESSAGE_HEIGHT, BACKGROUND);
}

static void draw_answer_lines(const char *secret)
{
    char line[ANSWER_LINE_CHARS + 1];
    for (int start = 0; start < SECRET_LENGTH; start += ANSWER_LINE_CHARS) {
        int n = 0;
        while (n < ANSWER_LINE_CHARS && secret[start + n] != '\0') {
            line[n] = secret[start + n];
            n++;
        }
        line[n] = '\0';
        int y = ANSWER_Y + (start / ANSWER_LINE_CHARS) * ANSWER_LINE_HEIGHT;
        badge_draw_text(MARGIN, y, line, SUCCESS, BADGE_TEXT_TITLE);
    }
}

static void show_answer(uint64_t value)
{
    char secret[SECRET_LENGTH + 1];
    decrypt_secret(value, secret);
    draw_screen();
    badge_draw_text(MARGIN, ANSWER_LABEL_Y, ANSWER_LABEL, FOREGROUND, BADGE_TEXT_BODY);
    draw_answer_lines(secret);
}

static void submit(struct input *in, enum outcome *result)
{
    show_message(CHECKING, MUTED);
    uint64_t value = slow_mix(absorb(in));
    if (one_way(value ^ TAG_SALT, SEAL_ROUNDS) == CHECK_TAG) {
        show_answer(value);
        *result = SOLVED;
        return;
    }
    show_nope();
    in->length = 0;
    draw_input(in);
}

static void add_move(struct input *in, int move)
{
    if (in->length < MAX_INPUT) {
        in->moves[in->length++] = (uint8_t)move;
        draw_input(in);
    }
}

static void remove_move(struct input *in)
{
    if (in->length > 0) {
        in->length--;
        draw_input(in);
    }
}

static enum outcome handle_key(struct input *in, int key)
{
    enum outcome result = KEEP_GOING;
    int move = move_of_key(key);
    if (key == BADGE_KEY_B || key == BADGE_KEY_ESC) {
        result = QUIT;
    } else if (move != NOT_A_MOVE) {
        add_move(in, move);
    } else if (key == BADGE_KEY_DEL) {
        remove_move(in);
    } else if (key == BADGE_KEY_OK) {
        submit(in, &result);
    }
    return result;
}

static enum outcome read_keys(struct input *in)
{
    int key;
    while ((key = badge_key_pressed()) != BADGE_NO_KEY) {
        enum outcome result = handle_key(in, key);
        if (result != KEEP_GOING) {
            return result;
        }
    }
    return KEEP_GOING;
}

static void wait_for_quit(void)
{
    int key;
    while ((key = badge_key_pressed()) != BADGE_KEY_B && key != BADGE_KEY_ESC) {
        badge_delay_ms(FRAME_MS);
    }
}

int app_main(void)
{
    struct input in = { .length = 0 };
    enum outcome result = KEEP_GOING;
    draw_screen();
    draw_input(&in);
    while ((result = read_keys(&in)) == KEEP_GOING) {
        badge_delay_ms(FRAME_MS);
    }
    if (result == SOLVED) {
        wait_for_quit();
    }
    return 0;
}

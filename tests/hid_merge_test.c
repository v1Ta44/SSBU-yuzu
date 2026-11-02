/* Execute the real Npad writer and F1 button aggregator with virtual device inputs.
 * Device/renderer/rollback boundaries are stubs; no physical controller is opened. */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <assert.h>
#define input_trace_pad recorded_input_trace_pad
#include "../runtime/input_trace.h"
#undef input_trace_pad
static void input_trace_pad(unsigned id, const void *state);
#include "../runtime/nnhid.c"

static NnPadState test_pad;
static int test_connected = 1, test_blocked, trace_calls, check_final, expected_kind;
static unsigned test_menu_keys = 3;
static void input_trace_pad(unsigned id, const void *state) {
    unsigned char before[48]; size_t n = expected_kind == 2 ? 48 : 40;
    memcpy(before, state, n);
    if (check_final) {
        uint64_t a = (uint64_t)(uintptr_t)state;
        assert(id == 1);
        assert(rd32(a + 0x20) == (test_connected ? 3 : 0));
        assert(rd64(a + 8) == (test_connected && !test_blocked ? test_pad.buttons : 0));
        if (expected_kind == 2) {
            assert(rd32(a + 0x24) == (test_connected && !test_blocked && test_pad.gc ? (unsigned)test_pad.trig_l : 0));
            assert(rd32(a + 0x28) == (test_connected && !test_blocked && test_pad.gc ? (unsigned)test_pad.trig_r : 0));
        }
    }
    recorded_input_trace_pad(id, state);
    assert(!memcmp(before, state, n)); trace_calls++;
}
int nnpad_init(void) { return 1; }
void nnpad_poll(int force) { (void)force; }
int nnpad_present(int p) { return p == 1 && test_connected; }
int nnpad_is_gc(int p) { return p == 1 && test_pad.gc; }
int nnpad_gc_native(void) { return 1; }
int nnpad_get(int p, NnPadState *out) { if (!nnpad_present(p)) return 0; *out = test_pad; return 1; }
int nnpad_keyboard_player(void) { return -1; }
int nnpad_kb_vk(int k) { (void)k; return 0; }
unsigned nnpad_menu_keys(void) { return test_menu_keys; }
void nnpad_rumble(int p, float a, float b) { (void)p; (void)a; (void)b; }
void nnpad_disconnect(int p) { (void)p; }
void nnpad_swap(int a, int b) { (void)a; (void)b; }
int nnpad_selftest(const NnHidHooks *h) { (void)h; abort(); }
int menu_input_blocked(void) { return test_blocked; }
int fight_seq_ticks(void) { return 0; }
void nvn_front_shot_fight(int f) { (void)f; abort(); }
long long nvn_front_frame_next(void) { return 0; }
void display_l1_poll(uint64_t b, long long f, long long p) { (void)b; (void)f; (void)p; }
void r1_wrote(uint64_t a, uint64_t n) { (void)a; (void)n; }
void r1_init(void) {}
void guest_trap(const char *s, uint64_t a) { (void)s; (void)a; abort(); }
int r1_on(void) { return 0; }
int r1_poll(int t) { (void)t; abort(); }
int r1_net_on(void) { return 0; }
int r1_ra_phase(void) { return 0; }
void r1_pad(uint64_t *b, int *lx, int *ly, int *rx, int *ry) { (void)b; (void)lx; (void)ly; (void)rx; (void)ry; }
void register_imports(const ImportDef *d, int n) { (void)d; (void)n; abort(); }
void trace_printf(const char *f, ...) { (void)f; }

static void sample(int kind, unsigned char *buf) {
    expected_kind = kind;
    memset(buf, 0xA5, 64); int before = trace_calls;
    fill_pad((uint64_t)(uintptr_t)buf, 1, kind);
    assert(trace_calls == before + 1);
    for (int i = kind == 2 ? 48 : 40; i < 64; i++) assert(buf[i] == 0xA5);
}
int main(int argc, char **argv) {
    if (argc > 1 && !strcmp(argv[1], "--xinput-selftest")) {
        Ctx c = {0}; expected_kind = 1; int bad = nnhid_selftest_xinput(&c); assert(!bad); return bad;
    }
    assert(argc > 1); _putenv_s("SSBU_PAD_TRACE", argv[1]);
    g_hid_ready = g_sdl_pad = g_pad_test = 1; g_keyboard_player = -1;
    test_pad = (NnPadState){.buttons = 1ull << BTN_A, .lx = 120, .ly = -240, .rx = 360, .ry = -480,
        .wired = 1, .gc = 1, .trig_l = 1234, .trig_r = 30000};
    unsigned char buf[64]; check_final = 1;
    sample(2, buf); assert(rd32((uint64_t)(uintptr_t)buf + 0x24) == 1234 && rd32((uint64_t)(uintptr_t)buf + 0x28) == 30000);
    sample(2, buf); /* same data: trace must deduplicate */
    test_pad.buttons |= 1ull << BTN_B; test_pad.ry = 900; sample(2, buf);
    test_blocked = 1; sample(2, buf); /* F1 consumes input before the game */
    uint64_t menu = nnhid_menu_buttons();
    assert((menu & 3) == 3 && (menu & (3ull << 16)) == (3ull << 16)); /* F1 still sees pad/Home/Capture */
    test_blocked = 0; test_pad.gc = 0; sample(1, buf); /* Pro layout still writes only 40 bytes */
    test_connected = 0; sample(1, buf); /* release/disconnect reaches the observer */
    assert(trace_calls == 6);
    puts("PASS real Npad writer: GameCube triggers before observation, Pro layout, menu input blocking, F1 controls, disconnect and unchanged guest bytes");
    return 0;
}

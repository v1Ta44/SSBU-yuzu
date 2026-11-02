/* Real nn::oe and nn::os focus/event code with game/renderer boundaries stubbed.
 * Window queries come from oe_focus_test.c; this never changes desktop focus. */
#define main focus_cases_main
#include "oe_focus_test.c"
#undef main

volatile LONG g_guest_frozen;
MainLayout g_main;
int g_game_build;
const DynSym dyn_syms[1] = {{0}};
const int dyn_sym_count = 0;
static int exit_expected;
void trace_printf(const char *fmt, ...) { (void)fmt; }
int display_l1_trace(void) { return 0; }
double display_l1_ms(void) { return 0; }
void display_perf_thread(void) {}
long long nvn_front_frame_next(void) { return 0; }
unsigned long nvn_front_present_tid(void) { return 0; }
void hitch_cur_ctx(Ctx *c) { (void)c; abort(); }
void call_guest(Ctx *c, uint64_t a) { (void)c; (void)a; abort(); }
void call_indirect(Ctx *c, uint64_t a) { (void)c; (void)a; abort(); }
void register_imports(const ImportDef *d, int n) { (void)d; (void)n; abort(); }
ImportFn import_lookup(const char *s) { (void)s; abort(); }
void *import_data(const char *s) { (void)s; abort(); }
uint64_t hostfn_address(const char *s, ImportFn f) { (void)s; (void)f; abort(); }
uint64_t ga(uint64_t a) { (void)a; abort(); }
const char *gfx_language(void) { abort(); }
void guest_trap(const char *s, uint64_t a) { (void)s; (void)a; abort(); }
void report_and_exit(int code) {
    assert(exit_expected && code == 0 && g_oe_exit_requested && !g_oe_exit_locked && !g_oe_exit_nest);
    puts("PASS upstream nested exit handling reaches process termination after the outer section closes");
    exit(code);
}

int main(int argc, char **argv) {
    if (argc < 2 || strncmp(argv[1], "--exit-", 7)) return focus_cases_main(argc, argv);
    int suspended = !strcmp(argv[1], "--exit-suspended");
    int background = !strcmp(argv[1], "--exit-background");
    _putenv("SSBU_FREEZE_TEST=");
    _putenv(background ? "SSBU_FOCUS_NOTIFY=" : "SSBU_FOCUS_NOTIFY=1");
    InitializeCriticalSection(&g_oe_lock);
    Ctx c = {0}; oe_GetNotificationMessageEvent(&c); notification_event = c.x[0];
    assert(!oe_exit_request() && !g_oe_exit_requested); /* unlocked: let the window close */
    oe_EnterExitRequestHandlingSection(&c); oe_EnterExitRequestHandlingSection(&c);
    assert(g_oe_exit_nest == 2 && g_oe_exit_locked);
    mode(3); change_focus(1, 0);
    for (int i = 0; i < 200; i++) {
        EnterCriticalSection(&g_oe_lock); int seen = g_oe_was_foreground; LeaveCriticalSection(&g_oe_lock);
        if (seen) break; Sleep(5);
    }
    assert(g_oe_was_foreground);
    if (suspended || background) { change_focus(0, 1); if (suspended) wait_frozen(1); else { Sleep(100); wait_frozen(0); } }
    drain();
    assert(oe_exit_request() && g_oe_exit_requested);
    wait_frozen(0); /* exit requests remain runnable even with upstream AlwaysSuspend */
    int found = 0;
    EnterCriticalSection(&g_oe_lock);
    for (int i = 0; i < g_oe_count; i++) if (g_oe_queue[i] == OE_MSG_EXIT_REQUEST) found++;
    LeaveCriticalSection(&g_oe_lock); assert(found == 1 && ev_wait_ms(obj(notification_event), 0));
    oe_LeaveExitRequestHandlingSection(&c); assert(g_oe_exit_locked && g_oe_exit_nest == 1);
    exit_expected = 1; oe_LeaveExitRequestHandlingSection(&c);
    abort();
}

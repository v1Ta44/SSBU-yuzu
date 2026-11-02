/* Protocol selection shares online_mode with the upstream menu. A selection is
 * fixed for a process: changing the settings takes effect on the next start. */
#include "network_protocol.h"
#if SSBU_ENABLE_YUZU
#include <winsock2.h>
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "yuzu_net_internal.h"
static INIT_ONCE once = INIT_ONCE_STATIC_INIT;
static int selected = SSBU_PROTOCOL_YUZU;
static int off_word(const char *s) {
    return !*s || !strcmp(s, "0") || !_stricmp(s, "off") || !_stricmp(s, "false") || !_stricmp(s, "no");
}
static int parse(const char *s) {
    if (off_word(s)) return SSBU_PROTOCOL_OFF;
    if (!_stricmp(s, "yuzu")) return SSBU_PROTOCOL_YUZU;
    /* The upstream accepts host:port and host/join:code as well as its mode names. */
    if (!_stricmp(s, "lobby") || !_stricmp(s, "lan") ||
        !_stricmp(s, "host") || !_stricmp(s, "join") || !_stricmp(s, "1") ||
        !_stricmp(s, "on") || !_stricmp(s, "auto") || strchr(s, ':')) return SSBU_PROTOCOL_UPSTREAM;
    fprintf(stderr, "network: unknown online_mode '%s'; networking disabled\n", s);
    return SSBU_PROTOCOL_OFF;
}
/* Same path precedence as gfx_options.c, including a bare controllers.ini path. */
static const char *config_path(char out[MAX_PATH]) {
    const char *e = getenv("SSBU_GRAPHICS_CONFIG"), *c = getenv("SSBU_CONFIG");
    if (e && *e) return e;
    char path[MAX_PATH] = {0};
    if (c && *c) snprintf(path, sizeof path, "%s", c);
    else { GetModuleFileNameA(NULL, path, sizeof path); path[sizeof path - 1] = 0; }
    char *sep = strrchr(path, '\\'), *slash = strrchr(path, '/');
    if (!sep || (slash && slash > sep)) sep = slash;
    if (sep) *sep = 0;
    if (c && *c) snprintf(out, MAX_PATH, "%s%sgraphics.ini", sep ? path : "", sep ? "\\" : "");
    else snprintf(out, MAX_PATH, "%s\\..\\config\\graphics.ini", sep ? path : ".");
    return out;
}
static BOOL CALLBACK load(PINIT_ONCE o, PVOID p, PVOID *ctx) {
    (void)o; (void)p; (void)ctx;
    const char *e = getenv("SSBU_ONLINE");
    if (e) { selected = parse(e); return TRUE; }
    char path[MAX_PATH], line[512];
    FILE *f = fopen(config_path(path), "r");
    if (!f) return TRUE;
    int have_mode = 0, legacy_server = -1;
    while (fgets(line, sizeof line, f)) {
        char *s = line;
        if (!strncmp(s, "\xef\xbb\xbf", 3)) s += 3;
        while (*s == ' ' || *s == '\t') s++;
        int is_mode = !_strnicmp(s, "online_mode", 11);
        int is_server = !_strnicmp(s, "online_server", 13);
        if (!is_mode && !is_server) continue;
        s += is_mode ? 11 : 13;
        while (*s == ' ' || *s == '\t') s++;
        if (*s++ != '=') continue;
        while (*s == ' ' || *s == '\t') s++;
        char *end = s + strcspn(s, ";#\r\n");
        while (end > s && (end[-1] == ' ' || end[-1] == '\t')) end--;
        *end = 0;
        if (is_mode) { selected = parse(s); have_mode = 1; break; }
        if (legacy_server < 0) legacy_server = off_word(s) ? SSBU_PROTOCOL_OFF : SSBU_PROTOCOL_UPSTREAM;
    }
    if (!have_mode && legacy_server >= 0) selected = legacy_server;
    fclose(f); return TRUE;
}
int network_protocol(void) { InitOnceExecuteOnce(&once, load, NULL, NULL); return selected; }
void network_protocol_ldn(int *role, int *players) {
    *role = *players = 0;
    if (network_protocol() != SSBU_PROTOCOL_YUZU) return;
    yn_lock();
    if (ssbu_yn.snapshot.state == SSBU_NET_JOINED) {
        *role = ssbu_yn.ldn_state == 3 ? 1 : ssbu_yn.ldn_state == 5 ? 2 : 0;
        if (*role) *players = ssbu_yn.network[0x67];
    }
    yn_unlock();
}
int network_protocol_stats(NetStats *out) {
    int protocol = network_protocol();
    if (protocol == SSBU_PROTOCOL_UPSTREAM) return 0;
    memset(out, 0, sizeof *out); out->ping_ms = -1;
    if (protocol == SSBU_PROTOCOL_OFF) return 1;
    /* Rendering never waits for DNS, the room worker or its lock. */
    if (!yn_try_lock()) return 1;
    unsigned state = ssbu_yn.snapshot.state;
    out->state = state == SSBU_NET_JOINED ? NET_CONNECTED :
        state == SSBU_NET_CONNECTING || state == SSBU_NET_JOINING ? NET_CONNECTING :
        state == SSBU_NET_FAILED ? NET_DISCONNECTED : NET_OFFLINE;
    if (state == SSBU_NET_JOINED) {
        int in_room = ssbu_yn.ldn_state == 3 || ssbu_yn.ldn_state == 5;
        out->players = in_room ? ssbu_yn.network[0x67] : 1;
        out->ping_ms = ssbu_yn.ping_ms;
    }
    yn_unlock(); return 1;
}
#endif

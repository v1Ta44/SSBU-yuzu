/* Exercise the real final network registration, SDK adapters and protocol worker.
 * Reuse the independent ENet room fixture. Runtime imports/transport are bounded stubs. */
#define main legacy_yuzu_fixture
#include "yuzu_backend.c"
#undef main
#include "hostrt.h"
#include "network_protocol.h"
#include "net_relay.h"
#include "network_abi.h"
static ImportDef bound[512];
static int bound_count, registrations, upstream_starts, sdk_errno;
static void offline(Ctx *c) { c->x[0] = 0x2ecb; }
static void errno_location(Ctx *c) { c->x[0] = (uint64_t)(uintptr_t)&sdk_errno; }
void *import_data(const char *name) { (void)name; return (void *)1; }
ImportFn import_lookup(const char *name) {
    if (!strcmp(name, "__errno_location")) return errno_location;
    for (int i = 0; i < bound_count; i++) if (!strcmp(bound[i].name, name)) return bound[i].fn;
    return offline;
}
/* Shared by protocol and Console-pipe fixtures. Unknown plugin exports stay absent. */
ImportFn extra_import_lookup(const char *name) {
    if (!strcmp(name, "__errno_location")) return errno_location;
    for (int i = 0; i < bound_count; i++) if (!strcmp(bound[i].name, name)) return bound[i].fn;
    return NULL;
}
void register_imports(const ImportDef *defs, int n) {
    CHECK(n <= 256); registrations++;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < i; j++) CHECK(strcmp(defs[i].name, defs[j].name));
        int slot = 0; while (slot < bound_count && strcmp(bound[slot].name, defs[i].name)) slot++;
        CHECK(slot < 512); bound[slot] = defs[i]; if (slot == bound_count) bound_count++;
    }
}
void trace_printf(const char *format, ...) { (void)format; }
void input_mark(const char *what) { (void)what; }
void os_init_event(uint64_t key, int autoclear) { CHECK(key == 123); (void)autoclear; }
void net_stats_get(NetStats *out) { if (!network_protocol_stats(out)) {memset(out,0,sizeof *out);out->ping_ms=-1;} }
int net_online(void) { return network_protocol() == SSBU_PROTOCOL_UPSTREAM; }
int net_mode(void) { return NET_MODE_LOBBY; }
int net_log_level(void) { return 0; }
uint32_t net_my_ip(void) { return 0x0a0d0002; }
int net_lobby_gone(void) { return 0; }
void net_start(void) { CHECK(network_protocol() == SSBU_PROTOCOL_UPSTREAM); upstream_starts++; }
void net_send(uint32_t src, uint16_t sport, uint32_t dst, uint16_t dport, const void *data, uint32_t len) {
    (void)src; (void)sport; (void)dst; (void)dport; (void)data; (void)len;
    CHECK(network_protocol() == SSBU_PROTOCOL_UPSTREAM);
}
void net_list_rooms(uint32_t seq) { (void)seq; }
int net_room_refusal(uint32_t host, char *why, size_t n) { (void)host; (void)why; (void)n; return 0; }
uint32_t net_match_host(void) { return 0; }
void report_and_exit(int code) { ExitProcess((UINT)code); }
static uint64_t call(const char *name, Ctx *c) { ImportFn f = import_lookup(name); CHECK(f != offline); f(c); return c->x[0]; }
#define LDN(s) "_ZN2nn3ldn" s
/* Calls unrelated to networking are traps: an accidental new dependency fails the test. */
static void unexpected(void) { fprintf(stderr,"FAIL unexpected game/renderer call\n"); ExitProcess(99); }
int sdk_image_ready(void) { unexpected(); return 0; }
int sdk_install(const char *n) { (void)n; unexpected(); return 0; }
uint64_t sdk_symbol(const char *n) { (void)n; unexpected(); return 0; }
int account_user_is_open(uint64_t lo,uint64_t hi) { (void)lo; (void)hi; unexpected(); return 0; }
void guest_trap(const char *what,uint64_t addr) { (void)what; (void)addr; unexpected(); }
volatile LONG g_bcat_mounted;
int g_game_build;
int bcat_dirname_valid(const char *s) { (void)s; unexpected(); return 0; }
int net_rb_send(const void *p,int n) { (void)p; (void)n; unexpected(); return 0; }
void net_match_find_rb(void) { unexpected(); }
void menu_popup(const char *a,const char *b) { (void)a; (void)b; unexpected(); }
long long nvn_front_presents(void) { unexpected(); return 0; }
void net_match_get(NetMatch *out) { (void)out; unexpected(); }
void net_match_cancel(void) { unexpected(); }
void menu_notice(const char *text) { (void)text; unexpected(); }
int net_rb_ready(void) { unexpected(); return 0; }
int net_rb_no_destination(void) { unexpected(); return 0; }
void nvn_front_osk_redraw(void) { unexpected(); }
void host_fixed_random_reseed(uint64_t seed) { (void)seed; unexpected(); }
uint64_t nnhid_menu_buttons(void) { unexpected(); return 0; }
uint64_t ga(uint64_t addr) { (void)addr; unexpected(); return 0; }
void nnnet_register_final(void);
int rbn_on(void);
static void adapter_run(void) {
    CHECK(!enet_initialize()); ENetAddress address = {0};
    CHECK(!enet_address_set_host_ip(&address, "127.0.0.1"));
    server = enet_host_create(&address, 1, 1, 0, 0); CHECK(server);
    CHECK(!enet_socket_get_address(server->socket, &address));
    CHECK(!ssbu_net_set_profile("PCPlayer"));
    SsbuNetConfig config = {0}; config.size = sizeof config; config.abi = SSBU_NET_ABI;
    config.port = address.port; strcpy(config.host, "127.0.0.1"); strcpy(config.nickname, "PCPlayer");
    strcpy(config.password, "pw"); strcpy(config.token, "SECRET_TOKEN");
    CHECK(!ssbu_net_join(&config)); receive(4, -1); wait_state(SSBU_NET_JOINED);
    nnnet_register_final(); CHECK(!upstream_starts);
    Ctx c = {0}; void *pool = _aligned_malloc(0x200000,4096); CHECK(pool);
    c.x[0]=(uintptr_t)pool;c.x[1]=0x200000;c.x[2]=0x100000;c.x[3]=1;
    CHECK(!call("_ZN2nn6socket10InitializeEPvmmi", &c)); CHECK(!call(LDN("10InitializeEv"), &c));
    c.x[0] = 123; CHECK(!call(LDN("22AttachStateChangeEventEPNS_2os15SystemEventTypeE"), &c));
    CHECK(!call(LDN("15OpenAccessPointEv"), &c));
    uint8_t config_bytes[32] = {0}, security[68] = {0}, user[48] = "PCPlayer", peer_node[64];
    config_bytes[0] = 0x42; config_bytes[18] = 4; config_bytes[20] = 1;
    c.x[0] = (uintptr_t)config_bytes; c.x[1] = (uintptr_t)security; c.x[2] = (uintptr_t)user;
    CHECK(!call(LDN("13CreateNetworkERKNS0_13NetworkConfigERKNS0_14SecurityConfigERKNS0_10UserConfigE"), &c));
    CHECK(nnldn_players() == 1);
    node(peer_node, remote, 0, "YuzuUser"); ldn(2, peer_node, 64); receive(6, 3);
    CHECK(nnldn_players() == 2);
    int role, players; network_protocol_ldn(&role, &players); CHECK(role == 1 && players == 2);
    NetStats stats; CHECK(network_protocol_stats(&stats)); CHECK(stats.state == NET_CONNECTED && stats.players == 2);
    c.x[0] = 2; c.x[1] = 2; c.x[2] = 0; int fd = (int)call("nnsocketSocket", &c); CHECK(fd >= YN_VFD_BASE);
    uint8_t sa[16] = {16,2,0x0f,0xa0};
    c.x[0] = fd; c.x[1] = (uintptr_t)sa; c.x[2] = sizeof sa; CHECK(!call("nnsocketBind", &c));
    sa[2] = 0x13; sa[3] = 0x88; memcpy(sa+4, remote, 4);
    const char data[] = "protocol-data";
    c.x[0] = fd; c.x[1] = (uintptr_t)data; c.x[2] = sizeof data; c.x[3] = 0; c.x[4] = (uintptr_t)sa; c.x[5] = sizeof sa;
    CHECK(call("_ZN2nn6socket6SendToEiPKvmiPK8sockaddrj", &c) == sizeof data);
    Packet proxy = receive(5, -1); memcpy(proxy.b+2, remote, 4); memcpy(proxy.b+9, own, 4);
    proxy.b[6]=0x13;proxy.b[7]=0x88;proxy.b[13]=0x0f;proxy.b[14]=0xa0;send_packet(&proxy);
    ULONGLONG end = GetTickCount64()+1500; Packet ignored;
    while (!(yn_socket_poll(fd,1)&1) && GetTickCount64()<end) pump(&ignored);
    char result[64] = {0}; c.x[0]=fd;c.x[1]=(uintptr_t)result;c.x[2]=sizeof result;c.x[3]=0x80;
    CHECK(call("nnsocketRecv",&c)==sizeof data && !memcmp(result,data,sizeof data));
    c.x[0]=fd;CHECK(!call("nnsocketClose",&c));
    CHECK(!call(LDN("14DestroyNetworkEv"),&c));CHECK(!call(LDN("8FinalizeEv"),&c));
    CHECK(!call("_ZN2nn6socket8FinalizeEv",&c));_aligned_free(pool);
    ssbu_net_leave(); CHECK(network_protocol_stats(&stats) && stats.state==NET_OFFLINE && !stats.players && stats.ping_ms==-1);
    enet_host_destroy(server);enet_deinitialize();puts("PASS unified registration, game SDK room/UDP calls, membership and stats");
}
#ifndef NETWORK_PROTOCOL_NO_MAIN
int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--mode")) { printf("%d",network_protocol());return 0; }
    if (argc == 2 && !strcmp(argv[1], "--cache")) {
        int selected = network_protocol(); _putenv_s("SSBU_ONLINE", selected == SSBU_PROTOCOL_YUZU ? "off" : "yuzu");
        CHECK(network_protocol() == selected); puts("PASS protocol remains fixed until restart"); return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "--loop")) {
        _putenv_s("SSBU_RB_NET", "loop,4"); CHECK(rbn_on()); puts("PASS local rollback diagnostic remains available"); return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "--select")) {
        int protocol = network_protocol(); nnnet_register_final();
        CHECK((import_lookup(LDN("10InitializeEv")) == yuzu_protocol_handler(NET_ABI_LDN_INIT)) == (protocol == SSBU_PROTOCOL_YUZU));
        CHECK((import_lookup("nnsocketSocket") == yuzu_protocol_handler(NET_ABI_SOCKET_SOCKET)) == (protocol == SSBU_PROTOCOL_YUZU));
        CHECK((import_lookup("_ZN2nn6socket8FinalizeEv") == yuzu_protocol_handler(NET_ABI_SOCKET_FINALIZE)) == (protocol == SSBU_PROTOCOL_YUZU));
        CHECK(upstream_starts == (protocol == SSBU_PROTOCOL_UPSTREAM));
        if (protocol != SSBU_PROTOCOL_YUZU) CHECK(ssbu_net_join(NULL)==SSBU_NET_BUSY);
        _putenv_s("SSBU_RB_NET","net"); CHECK(rbn_on() == (protocol == SSBU_PROTOCOL_UPSTREAM));
        /* Even stale backend membership cannot enable manual delay in another protocol. */
        yn_lock(); ssbu_yn.snapshot.state=SSBU_NET_JOINED; ssbu_yn.ldn_state=3;
        ssbu_yn.network[0x67]=2; ssbu_yn.epoch=9; ssbu_yn.ldn_generation=7; yn_unlock();
        SsbuNetLdnSnapshot delay={0};delay.size=sizeof delay;delay.abi=SSBU_NET_ABI;
        CHECK(!ssbu_net_ldn_snapshot(&delay));
        if(protocol==SSBU_PROTOCOL_YUZU) CHECK(delay.room_state==SSBU_NET_JOINED && delay.ldn_state==3 && delay.node_count==2 && delay.room_epoch==9 && delay.ldn_generation==7);
        else CHECK(delay.room_state==SSBU_NET_IDLE && !delay.ldn_state && !delay.node_count && !delay.room_epoch && !delay.ldn_generation);
        puts("PASS selected protocol owns registration; unsupported network rollback stays off; delay session is isolated");return 0;
    }
    adapter_run();return 0;
}

#endif /* NETWORK_PROTOCOL_NO_MAIN */

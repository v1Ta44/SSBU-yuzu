/* The game's networking has one registration entry. Protocol-specific room and
 * socket behavior lives behind it; SSBU_ENABLE_YUZU is a Windows build option. */
#ifndef SSBU_NETWORK_PROTOCOL_H
#define SSBU_NETWORK_PROTOCOL_H
#ifndef SSBU_ENABLE_YUZU
#define SSBU_ENABLE_YUZU 0
#endif
#include "net_stats.h"

#ifdef __cplusplus
extern "C" {
#endif
enum { SSBU_PROTOCOL_OFF, SSBU_PROTOCOL_UPSTREAM, SSBU_PROTOCOL_YUZU };
#if SSBU_ENABLE_YUZU
int network_protocol(void);
int network_protocol_stats(NetStats *out);
void network_protocol_ldn(int *role, int *players);
#else
static inline int network_protocol(void) { return SSBU_PROTOCOL_UPSTREAM; }
static inline int network_protocol_stats(NetStats *out) { (void)out; return 0; }
static inline void network_protocol_ldn(int *role, int *players) { *role = *players = 0; }
#endif
#ifdef __cplusplus
}
#endif
#endif

/* Shared game ABI operations. Symbol spellings are owned by nnldn.c only;
 * a protocol supplies behavior for each operation without registering imports. */
#ifndef SSBU_NETWORK_ABI_H
#define SSBU_NETWORK_ABI_H
#include "hostrt.h"
#include "network_protocol.h"
typedef enum {
    NET_ABI_SOCKET_FINALIZE,
    NET_ABI_SOCKET_SOCKET,
    NET_ABI_SOCKET_CLOSE,
    NET_ABI_SOCKET_BIND,
    NET_ABI_SOCKET_BIND_TYPED,
    NET_ABI_SOCKET_CONNECT,
    NET_ABI_SOCKET_LISTEN,
    NET_ABI_SOCKET_SHUTDOWN,
    NET_ABI_SOCKET_ACCEPT,
    NET_ABI_SOCKET_SOCKNAME,
    NET_ABI_SOCKET_PEERNAME,
    NET_ABI_SOCKET_RECV,
    NET_ABI_SOCKET_RECVFROM,
    NET_ABI_SOCKET_SEND,
    NET_ABI_SOCKET_SENDTO,
    NET_ABI_SOCKET_FCNTL,
    NET_ABI_SOCKET_POLL,
    NET_ABI_SOCKET_GETOPT,
    NET_ABI_SOCKET_SETOPT,
    NET_ABI_LDN_INIT,
    NET_ABI_LDN_STATE,
    NET_ABI_LDN_FINAL,
    NET_ABI_LDN_EVENT,
    NET_ABI_LDN_CONTROLLER,
    NET_ABI_LDN_OPEN_STA,
    NET_ABI_LDN_CLOSE_STA,
    NET_ABI_LDN_OPEN_AP,
    NET_ABI_LDN_CLOSE_AP,
    NET_ABI_LDN_DISCONNECT,
    NET_ABI_LDN_DESTROY,
    NET_ABI_LDN_INFO,
    NET_ABI_LDN_CONFIG,
    NET_ABI_LDN_IP,
    NET_ABI_LDN_SECURITY,
    NET_ABI_LDN_ADVERTISE,
    NET_ABI_LDN_POLICY,
    NET_ABI_LDN_REJECT,
    NET_ABI_LDN_SCAN,
    NET_ABI_LDN_CREATE,
    NET_ABI_LDN_CREATE_PRIVATE,
    NET_ABI_LDN_CONNECT,
    NET_ABI_LDN_CONNECT_PRIVATE,
    NET_ABI_LDN_REASON,
    NET_ABI_COUNT
} NetworkAbi;
typedef struct { const char *name; ImportFn fn; NetworkAbi operation; } NetworkBinding;
#if SSBU_ENABLE_YUZU
ImportFn yuzu_protocol_handler(NetworkAbi operation);
#endif
#endif
